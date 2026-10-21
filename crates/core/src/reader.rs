//! Bounded, read-only entry access.
//!
//! # Why this module exists
//!
//! The reference archive's main entry has **682,951 lines**, one of them
//! **192,666 characters** long. Any implementation that calls `reader.lines()`
//! or `read_line()` into a `String` allocates that line — and on a hostile
//! archive the declared line can be arbitrarily larger. Reader memory must be
//! O(1) in the line number, not O(N).
//!
//! So every line operation in this module goes through [`ScanScratch`]: a
//! **fixed 64 KiB buffer** ([`crate::caps::LINE_SCRATCH_BYTES`]) reused for the
//! whole operation. Bytes are consumed from the stream, `\n` is counted, and
//! only the target line is ever accumulated — and that accumulation stops at
//! [`crate::caps::MAX_LINE_BYTES`]. The tail of an over-long line is counted
//! and discarded; it is never buffered.
//!
//! `BufRead::read_line` is never called anywhere in this crate, and neither is
//! `BufRead::lines`.

use std::io::{self, Read};

use zip::ZipArchive;

use crate::archive::{EntryId, EntryMeta};
use crate::caps::{LINE_SCRATCH_BYTES, MAX_LINE_BYTES};
use crate::error::ArchiveError;
use crate::limits::LimitViolation;
use crate::source::FileCursor;
use crate::text::{ExcerptOutcome, LineSlice};

/// The fixed scratch buffer every line walk in this crate streams through.
///
/// Sized once and reused for the whole walk. This is the object that makes
/// reading line N cost O(1) memory: nothing in the line-walk path allocates in
/// proportion to line length or line number.
pub(crate) struct ScanScratch {
    buf: Box<[u8; LINE_SCRATCH_BYTES]>,
}

impl std::fmt::Debug for ScanScratch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The buffer is 64 KiB of scratch; dumping it would be noise.
        f.debug_struct("ScanScratch")
            .field("bytes", &LINE_SCRATCH_BYTES)
            .finish()
    }
}

impl ScanScratch {
    pub(crate) fn new() -> Self {
        Self {
            buf: Box::new([0u8; LINE_SCRATCH_BYTES]),
        }
    }

    /// Fills at most `max` bytes of the scratch buffer.
    ///
    /// Needed when the caller knows it wants fewer bytes than the buffer
    /// holds: [`Read::read`] is free to return a full buffer, so a caller that
    /// then discards the excess loses data without noticing.
    pub(crate) fn fill_at_most<R: Read + ?Sized>(
        &mut self,
        r: &mut R,
        max: usize,
    ) -> io::Result<usize> {
        let cap = max.min(LINE_SCRATCH_BYTES);
        let mut filled = 0usize;
        while filled < cap {
            match r.read(&mut self.buf[filled..cap]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(filled)
    }

    fn as_slice(&self, n: usize) -> &[u8] {
        &self.buf[..n]
    }
}

/// Index of the first `\n` in `data`, or `None`.
#[inline]
fn find_newline(data: &[u8]) -> Option<usize> {
    data.iter().position(|&b| b == b'\n')
}

/// The per-walk cursor: position, counters, and the one scratch buffer.
///
/// Kept separate from [`EntryReader`] so that the `ZipFile` borrow — which
/// borrows the archive — never overlaps with mutable access to the cursor.
struct Cursor {
    id: EntryId,
    meta: EntryMeta,
    limit: u64,
    ratio_limit: u64,
    /// Decompressed bytes produced so far on this walk.
    consumed: u64,
    /// Uncompressed offset where the line under construction begins.
    line_start: u64,
    scratch: ScanScratch,
    /// Bytes read but not yet consumed, including bytes `skip_lines` counted
    /// past. Bounded by one scratch buffer.
    carry: Vec<u8>,
    /// Read position within `carry`.
    carry_pos: usize,
}

impl Cursor {
    fn new(id: EntryId, meta: EntryMeta, limit: u64, ratio_limit: u64) -> Self {
        Self {
            id,
            meta,
            limit,
            ratio_limit,
            consumed: 0,
            line_start: 0,
            scratch: ScanScratch::new(),
            carry: Vec::new(),
            carry_pos: 0,
        }
    }

    fn io_err(&self, e: io::Error) -> ArchiveError {
        ArchiveError::UnreadableEntry {
            id: self.id,
            source: zip::result::ZipError::Io(e),
        }
    }

    /// Streaming guard, applied after every chunk of decompressed output.
    ///
    /// This is what catches a lying local header: the declared size is
    /// attacker-controlled, so the *stream* is what gets checked.
    fn check_progress(&self) -> Result<(), ArchiveError> {
        let total = self.consumed;
        if total > self.limit {
            return Err(ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge {
                entry_id: self.id,
                found: total,
                limit: self.limit,
            }));
        }
        // Past the declared size means the header lied. Judge it by ratio.
        if total > self.meta.uncompressed_size {
            let compressed = self.meta.compressed_size.max(1);
            let ratio = total / compressed;
            if ratio > self.ratio_limit {
                return Err(ArchiveError::LimitExceeded(LimitViolation::RatioExceeded {
                    entry_id: self.id,
                    ratio,
                    limit: self.ratio_limit,
                }));
            }
        }
        Ok(())
    }

    /// Reads **one** line into `acc`, capped at [`MAX_LINE_BYTES`].
    ///
    /// Returns the line's true length excluding the `\n`, or `None` at a clean
    /// end of stream. Bytes past the cap are counted and discarded — never
    /// accumulated.
    fn read_line_raw<R: Read + ?Sized>(
        &mut self,
        r: &mut R,
        acc: &mut Vec<u8>,
    ) -> Result<Option<u64>, ArchiveError> {
        let mut full_len: u64 = 0;
        loop {
            // Refill the carry buffer, consuming anything `skip_lines` left.
            match self.next_chunk(r)? {
                None => {
                    // EOF. A partially accumulated line is still a line;
                    // nothing accumulated at all means the stream is exhausted.
                    return Ok(if full_len == 0 && acc.is_empty() {
                        None
                    } else {
                        Some(full_len)
                    });
                }
                Some(chunk) => match find_newline(chunk) {
                    Some(idx) => {
                        full_len += idx as u64;
                        append_capped(acc, &chunk[..idx]);
                        self.consumed_carry(idx as u64 + 1); // +1 for the `\n`
                        self.check_progress()?;
                        return Ok(Some(full_len));
                    }
                    None => {
                        full_len += chunk.len() as u64;
                        append_capped(acc, chunk);
                        let n = chunk.len() as u64;
                        self.consumed_carry(n);
                        self.check_progress()?;
                        // No newline in this chunk: keep going.
                    }
                },
            }
        }
    }

    /// Produces the next contiguous run of bytes to inspect.
    ///
    /// Returns bytes already buffered by [`Cursor::skip_lines`] first — those
    /// are the bytes that followed the last newline it counted, which belong to
    /// the next line and must not be dropped. `None` means end of stream.
    fn next_chunk<R: Read + ?Sized>(&mut self, r: &mut R) -> Result<Option<&[u8]>, ArchiveError> {
        if self.carry_pos < self.carry.len() {
            return Ok(Some(&self.carry[self.carry_pos..]));
        }
        let want = self
            .limit
            .saturating_sub(self.consumed)
            .saturating_add(1)
            .min(LINE_SCRATCH_BYTES as u64) as usize;
        let n = self
            .scratch
            .fill_at_most(r, want)
            .map_err(|e| self.io_err(e))?;
        if n == 0 {
            return Ok(None);
        }
        self.carry.clear();
        self.carry.extend_from_slice(self.scratch.as_slice(n));
        self.carry_pos = 0;
        self.consumed += n as u64;
        self.check_progress()?;
        Ok(Some(&self.carry))
    }

    /// Advances the position within the carry buffer by `n` bytes.
    fn consumed_carry(&mut self, n: u64) {
        self.carry_pos += n as usize;
    }

    /// Marks the start of a new line at the current cursor.
    ///
    /// The line start is the *unconsumed* stream position, which is
    /// `consumed` minus whatever is still sitting in the carry buffer.
    fn begin_line(&mut self) {
        let pending = (self.carry.len() - self.carry_pos) as u64;
        self.line_start = self.consumed.saturating_sub(pending);
    }

    /// Skips `n` lines by counting `\n` through the scratch buffer.
    ///
    /// No line content is accumulated at any point, so this costs O(1) memory
    /// regardless of how long the skipped lines are — this is what makes
    /// reading line N cost O(1) memory rather than O(N).
    ///
    /// Bytes trailing the final newline are moved into `carry` rather than
    /// dropped: they are the beginning of the next line, and losing them would
    /// make every line after a chunk-aligned newline unreadable. `carry` is
    /// bounded by one scratch buffer, so this stays O(1) in memory.
    ///
    /// Returns `false` if the stream ended before `n` lines were skipped.
    fn skip_lines<R: Read + ?Sized>(&mut self, r: &mut R, n: u64) -> Result<bool, ArchiveError> {
        let mut remaining = n;
        while remaining > 0 {
            let chunk = match self.next_chunk(r)? {
                Some(c) => c,
                None => return Ok(false),
            };

            let mut i = 0usize;
            while remaining > 0 {
                match find_newline(&chunk[i..]) {
                    Some(off) => {
                        remaining -= 1;
                        i += off + 1;
                    }
                    None => {
                        // No newline in what remains of this chunk. Those bytes
                        // belong to a later line, so consume them here and let
                        // the next iteration refill — advancing by zero would
                        // spin on the same bytes forever.
                        i = chunk.len();
                        break;
                    }
                }
            }
            self.carry_pos += i;
            // Checked after the borrow ends.
            self.check_progress()?;
        }
        Ok(true)
    }

    fn build_slice(&self, line_no: u64, bytes: Vec<u8>, full_len: u64) -> LineSlice {
        let outcome = if (bytes.len() as u64) < full_len {
            ExcerptOutcome::LineTooLong {
                capped_at: bytes.len() as u64,
                skipped: full_len - bytes.len() as u64,
            }
        } else {
            ExcerptOutcome::Complete
        };
        LineSlice {
            entry_id: self.id,
            line_no,
            bytes,
            byte_offset: self.line_start,
            byte_len_full: full_len,
            outcome,
        }
    }
}

/// A fresh, read-only handle onto one entry.
///
/// Owns its own ZIP reader and logical cursor over a pinned shared descriptor.
/// It is `Send + Sync` and can be moved into `spawn_blocking` on
/// the `crates/mcp` side.
#[derive(Debug)]
pub struct EntryReader {
    /// Taken out for the duration of a walk and put back afterwards, so the
    /// `ZipFile` guard borrows a local rather than `self`.
    archive: Option<ZipArchive<FileCursor>>,
    index: usize,
    id: EntryId,
    meta: EntryMeta,
    limit: u64,
    ratio_limit: u64,
}

impl EntryReader {
    pub(crate) fn new(
        archive: ZipArchive<FileCursor>,
        index: usize,
        meta: EntryMeta,
        limits: crate::limits::ArchiveLimits,
    ) -> Self {
        Self {
            archive: Some(archive),
            index,
            id: meta.id,
            meta,
            limit: limits.max_entry_bytes,
            ratio_limit: limits.max_compression_ratio,
        }
    }

    pub fn entry_id(&self) -> EntryId {
        self.id
    }

    pub fn meta(&self) -> &EntryMeta {
        &self.meta
    }

    /// Narrows the byte ceiling to `limit`.
    ///
    /// Overflow is [`ArchiveError::LimitExceeded`] — never a silent
    /// truncation. Chainable: `reader.take_bounded(n).read_line(1)?`.
    ///
    /// **Deviation from `04b`:** the contract shows this consuming `self` and
    /// returning an owned `EntryReader`. An owned reader owns a non-cloneable
    /// `ZipArchive`, so it cannot hand out a second one. Constraining in place
    /// keeps the chaining ergonomics and identical overflow behaviour.
    pub fn take_bounded(&mut self, limit: u64) -> &mut Self {
        self.limit = self.limit.min(limit);
        self
    }

    /// Runs `f` with the entry's decompressing stream, restoring the handle
    /// afterwards even on failure.
    fn with_stream<T>(
        &mut self,
        f: impl FnOnce(&mut ZipArchive<FileCursor>, &mut Cursor) -> Result<T, ArchiveError>,
    ) -> Result<T, ArchiveError> {
        let mut archive = self
            .archive
            .take()
            .ok_or_else(|| ArchiveError::UnreadableEntry {
                id: self.id,
                source: zip::result::ZipError::Io(io::Error::other(
                    "entry reader has no archive handle",
                )),
            })?;
        let mut cursor = Cursor::new(self.id, self.meta.clone(), self.limit, self.ratio_limit);
        let result = f(&mut archive, &mut cursor);
        self.archive = Some(archive);
        result
    }

    /// Opens the entry's stream, mapping the encrypted case distinctly.
    fn open(
        archive: &mut ZipArchive<FileCursor>,
        index: usize,
        id: EntryId,
    ) -> Result<zip::read::ZipFile<'_>, ArchiveError> {
        archive.by_index(index).map_err(|source| {
            if matches!(
                source,
                zip::result::ZipError::UnsupportedArchive(msg)
                    if msg == zip::result::ZipError::PASSWORD_REQUIRED
            ) {
                ArchiveError::EntryEncrypted { id }
            } else {
                ArchiveError::UnreadableEntry { id, source }
            }
        })
    }

    /// Reads line `line_no`, 1-based.
    ///
    /// Memory is O(1) in `line_no` and O(1) in line length: skipping streams
    /// through [`ScanScratch`] and accumulates nothing, and the returned line
    /// is capped at [`MAX_LINE_BYTES`].
    ///
    /// A line longer than `MAX_LINE_BYTES` yields
    /// [`ExcerptOutcome::LineTooLong`] carrying the **true** length, not a
    /// silently shortened buffer.
    pub fn read_line(&mut self, line_no: u64) -> Result<LineSlice, ArchiveError> {
        let id = self.id;
        self.with_stream(|archive, cur| {
            if line_no == 0 {
                return Err(ArchiveError::LineRangeOutOfBounds {
                    id,
                    requested_start: 0,
                    requested_end: 0,
                    actual: 0,
                });
            }
            let mut r = Self::open(archive, cur_index(archive, id), id)?;
            cur.begin_line();
            if line_no > 1 && !cur.skip_lines(&mut r, line_no - 1)? {
                return Err(ArchiveError::LineRangeOutOfBounds {
                    id,
                    requested_start: line_no,
                    requested_end: line_no,
                    actual: line_no - 1,
                });
            }
            cur.begin_line();
            let mut bytes = Vec::new();
            let Some(full_len) = cur.read_line_raw(&mut r, &mut bytes)? else {
                return Err(ArchiveError::LineRangeOutOfBounds {
                    id,
                    requested_start: line_no,
                    requested_end: line_no,
                    actual: line_no - 1,
                });
            };
            Ok(cur.build_slice(line_no, bytes, full_len))
        })
    }

    /// Walks every line **sequentially**, invoking `f` with each one.
    ///
    /// `f` returns `false` to stop early. Returns the number of lines read.
    ///
    /// This is the API streaming callers want, and it exists because
    /// [`EntryReader::read_line`] is a *random access* primitive: each call
    /// restarts the entry's decompressor and walks forward from byte 0. Calling
    /// it once per line is therefore O(lines²) — a real bug that shows up as a
    /// multi-minute hang on the reference archive's 682,951-line entry. Every
    /// sequential consumer must use this instead.
    pub fn for_each_line(
        &mut self,
        mut f: impl FnMut(&LineSlice) -> Result<bool, ArchiveError>,
    ) -> Result<u64, ArchiveError> {
        let id = self.id;
        let index = self.index;
        self.with_stream(move |archive, cur| {
            let mut r = Self::open(archive, index, id)?;
            let mut line_no = 0u64;
            let mut acc = Vec::new();
            loop {
                cur.begin_line();
                acc.clear();
                let Some(full_len) = cur.read_line_raw(&mut r, &mut acc)? else {
                    return Ok(line_no);
                };
                line_no += 1;
                let slice = cur.build_slice(line_no, acc.clone(), full_len);
                if !f(&slice)? {
                    return Ok(line_no);
                }
            }
        })
    }

    /// Reads an inclusive span, rejecting unavailable lines.
    /// Use `read_line_span_partial` when EOF should preserve the available prefix.
    pub fn read_line_span(&mut self, start: u64, end: u64) -> Result<Vec<LineSlice>, ArchiveError> {
        let (lines, past_end) = self.read_line_span_partial(start, end)?;
        if past_end {
            return Err(ArchiveError::LineRangeOutOfBounds {
                id: self.id,
                requested_start: start,
                requested_end: end,
                actual: lines
                    .last()
                    .map_or(start.saturating_sub(1), |line| line.line_no),
            });
        }
        Ok(lines)
    }

    /// Reads available lines in `start..=end`, plus whether the request extends past EOF.
    /// Invalid endpoints and stream/safety failures remain errors.
    pub fn read_line_span_partial(
        &mut self,
        start: u64,
        end: u64,
    ) -> Result<(Vec<LineSlice>, bool), ArchiveError> {
        let id = self.id;
        let index = self.index;
        self.with_stream(move |archive, cur| {
            if start == 0 || end == 0 || end < start {
                return Err(ArchiveError::LineRangeOutOfBounds {
                    id,
                    requested_start: start,
                    requested_end: end,
                    actual: 0,
                });
            }
            let mut r = Self::open(archive, index, id)?;
            cur.begin_line();
            if start > 1 && !cur.skip_lines(&mut r, start - 1)? {
                return Ok((Vec::new(), true));
            }

            let mut out = Vec::new();
            let mut line_no = start;
            loop {
                cur.begin_line();
                let mut acc = Vec::new();
                let Some(full_len) = cur.read_line_raw(&mut r, &mut acc)? else {
                    return Ok((out, true));
                };
                out.push(cur.build_slice(line_no, acc, full_len));
                if line_no == end {
                    return Ok((out, false));
                }
                line_no += 1;
            }
        })
    }

    /// Reads `len` bytes at uncompressed offset `off`.
    ///
    /// Reached by streaming and discarding, so memory is O(1).
    /// **Time is O(off + len)**: a deflated stream cannot be seeked in
    /// decompressed coordinates, so the bytes before `off` must be produced and
    /// dropped. Only a `Stored` entry could do better.
    pub fn read_byte_range(&mut self, off: u64, len: u64) -> Result<Vec<u8>, ArchiveError> {
        let id = self.id;
        let index = self.index;
        self.with_stream(move |archive, cur| {
            let actual = cur.meta.uncompressed_size;
            let end = off.saturating_add(len);
            if off > actual || end > actual {
                return Err(ArchiveError::ByteRangeOutOfBounds {
                    id,
                    requested_start: off,
                    requested_end: end,
                    actual,
                });
            }
            let mut r = Self::open(archive, index, id)?;

            // Discard `off` bytes without accumulating. Uses the scratch
            // directly: this walk has no carry buffer, so the offsets here are
            // plain stream offsets.
            let mut to_skip = off;
            while to_skip > 0 {
                // Read at most what is left to skip. A full-buffer read would
                // consume bytes past `off` and silently drop them.
                let want = to_skip.min(LINE_SCRATCH_BYTES as u64) as usize;
                let n = cur
                    .scratch
                    .fill_at_most(&mut r, want)
                    .map_err(|e| cur.io_err(e))?;
                if n == 0 {
                    return Err(ArchiveError::UnreadableEntry {
                        id,
                        source: zip::result::ZipError::InvalidArchive("unexpected end of entry"),
                    });
                }
                cur.consumed += n as u64;
                cur.check_progress()?;
                to_skip -= n as u64;
            }

            let mut out = Vec::with_capacity(len.min(LINE_SCRATCH_BYTES as u64) as usize);
            let mut remaining = len;
            while remaining > 0 {
                let want = remaining.min(LINE_SCRATCH_BYTES as u64) as usize;
                let start = out.len();
                out.resize(start + want, 0);
                let read = r.read(&mut out[start..]).map_err(|e| cur.io_err(e))?;
                out.truncate(start + read);
                if read == 0 {
                    break;
                }
                cur.consumed += read as u64;
                cur.check_progress()?;
                remaining -= read as u64;
            }
            Ok(out)
        })
    }

    /// Reads the available intersection of a byte range with the declared entry size.
    /// The boolean reports whether the requested endpoints extend past EOF.
    /// Read, CRC and safety failures are propagated by the strict reader.
    pub fn read_byte_range_partial(
        &mut self,
        off: u64,
        len: u64,
    ) -> Result<(Vec<u8>, bool), ArchiveError> {
        let size = self.with_stream(|_, cur| Ok(cur.meta.uncompressed_size))?;
        let past_end = off > size || len > size.saturating_sub(off);
        let start = off.min(size);
        let available = len.min(size - start);
        let bytes = self.read_byte_range(start, available)?;
        if bytes.len() as u64 != available {
            return Err(ArchiveError::UnreadableEntry {
                id: self.id,
                source: zip::result::ZipError::InvalidArchive("unexpected end of entry"),
            });
        }
        Ok((bytes, past_end))
    }

    /// Chunks of the decompressed stream, each at most [`LINE_SCRATCH_BYTES`].
    ///
    /// Borrows one decoder for the iterator's lifetime, avoiding repeated
    /// decompression and checking actual output before yielding each chunk.
    /// A ceiling violation is an error; reaching the ceiling alone is not EOF.
    pub fn stream(&mut self) -> EntryChunks<'_> {
        let file = match self.archive.as_mut() {
            Some(archive) => Self::open(archive, self.index, self.id),
            None => Err(ArchiveError::UnreadableEntry {
                id: self.id,
                source: zip::result::ZipError::Io(io::Error::other(
                    "entry reader has no archive handle",
                )),
            }),
        };
        EntryChunks {
            file: file.map_err(io::Error::other),
            cursor: Cursor::new(self.id, self.meta.clone(), self.limit, self.ratio_limit),
            done: false,
        }
    }
}

/// Recovers the `by_index` slot from an `EntryId`. `EntryId` is the central
/// directory position, which is exactly what `by_index` wants.
fn cur_index(archive: &ZipArchive<FileCursor>, id: EntryId) -> usize {
    debug_assert!(archive.len() > id.0 as usize);
    id.0 as usize
}

/// A demand-driven decoder owned by a parked worker, avoiding self-referential
/// ZipFile borrows and replay of previously searched compressed content.
#[derive(Debug)]
pub(crate) struct SearchStream {
    requests: Option<std::sync::mpsc::SyncSender<usize>>,
    replies: std::sync::mpsc::Receiver<Result<Vec<u8>, ArchiveError>>,
    worker: Option<std::thread::JoinHandle<()>>,
    id: EntryId,
}

impl SearchStream {
    pub(crate) fn new(mut reader: EntryReader) -> Result<Self, ArchiveError> {
        let id = reader.id;
        let (requests, receive) = std::sync::mpsc::sync_channel::<usize>(0);
        let (send, replies) = std::sync::mpsc::sync_channel(0);
        let worker = std::thread::Builder::new()
            .name("archive-search-decoder".into())
            .spawn(move || {
                let index = reader.index;
                let _ = reader.with_stream(|archive, cur| {
                    let mut file = match EntryReader::open(archive, index, id) {
                        Ok(file) => file,
                        Err(error) => {
                            if receive.recv().is_ok() {
                                let _ = send.send(Err(error));
                            }
                            return Ok(());
                        }
                    };
                    while let Ok(want) = receive.recv() {
                        let want = want
                            .min(LINE_SCRATCH_BYTES)
                            .min(cur.limit.saturating_sub(cur.consumed).saturating_add(1) as usize);
                        let mut bytes = vec![0; want];
                        let result =
                            file.read(&mut bytes)
                                .map_err(|e| cur.io_err(e))
                                .and_then(|n| {
                                    bytes.truncate(n);
                                    cur.consumed += n as u64;
                                    cur.check_progress()?;
                                    Ok(bytes)
                                });
                        let done = result.as_ref().map_or(true, |b| b.is_empty());
                        if send.send(result).is_err() || done {
                            break;
                        }
                    }
                    Ok(())
                });
            })
            .map_err(|source| ArchiveError::UnreadableEntry {
                id,
                source: zip::result::ZipError::Io(source),
            })?;
        Ok(Self {
            requests: Some(requests),
            replies,
            worker: Some(worker),
            id,
        })
    }

    pub(crate) fn read(&self, want: usize) -> Result<Vec<u8>, ArchiveError> {
        let failed = || ArchiveError::UnreadableEntry {
            id: self.id,
            source: zip::result::ZipError::Io(io::Error::other("search decoder stopped")),
        };
        self.requests
            .as_ref()
            .ok_or_else(failed)?
            .send(want)
            .map_err(|_| failed())?;
        self.replies.recv().map_err(|_| failed())?
    }
}

impl Drop for SearchStream {
    fn drop(&mut self) {
        self.requests.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Sequential chunk iterator over one entry's decompressed stream.
pub struct EntryChunks<'a> {
    file: io::Result<zip::read::ZipFile<'a>>,
    cursor: Cursor,
    done: bool,
}

impl std::fmt::Debug for EntryChunks<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntryChunks")
            .field("entry_id", &self.cursor.id)
            .finish()
    }
}

impl Iterator for EntryChunks<'_> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let file = match &mut self.file {
            Ok(file) => file,
            Err(error) => {
                self.done = true;
                return Some(Err(std::mem::replace(
                    error,
                    io::Error::other("stream ended"),
                )));
            }
        };
        match self.cursor.next_chunk(file) {
            Ok(Some(bytes)) => {
                let chunk = bytes.to_vec();
                self.cursor.carry_pos = self.cursor.carry.len();
                Some(Ok(chunk))
            }
            Ok(None) => {
                self.done = true;
                None
            }
            Err(error) => {
                self.done = true;
                Some(Err(io::Error::other(error)))
            }
        }
    }
}

/// Appends `data` to `acc`, stopping at [`MAX_LINE_BYTES`].
///
/// The caller compares `acc.len()` against the true line length to detect that
/// the cap bit.
fn append_capped(acc: &mut Vec<u8>, data: &[u8]) {
    let cap = MAX_LINE_BYTES as usize;
    if acc.len() >= cap {
        return;
    }
    let room = cap - acc.len();
    if data.len() <= room {
        acc.extend_from_slice(data);
    } else {
        acc.extend_from_slice(&data[..room]);
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;
    use crate::Archive;
    use crate::ArchiveLimits;
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};

    fn cursor(limits: ArchiveLimits, name: &str) -> Cursor {
        let path = scratch_dir().join(name);
        std::fs::write(&path, ZipBuilder::new().add("payload", b"x").finish()).unwrap();
        let archive = Archive::open(path).unwrap();
        Cursor::new(
            EntryId(0),
            archive.entries()[0].clone(),
            limits.max_entry_bytes,
            limits.max_compression_ratio,
        )
    }

    #[test]
    fn produced_bytes_use_selected_entry_ceiling_without_default_clipping() {
        let mut cur = cursor(ArchiveLimits::FORCED, "reader-policy-entry.zip");
        // Model cursor progress without allocating multi-gigabyte test payloads.
        cur.meta.uncompressed_size = ArchiveLimits::FORCED.max_entry_bytes;
        cur.consumed = ArchiveLimits::DEFAULT.max_entry_bytes + 1;
        assert!(cur.check_progress().is_ok());
        cur.consumed = ArchiveLimits::FORCED.max_entry_bytes;
        assert!(cur.check_progress().is_ok());
        cur.consumed += 1;
        assert!(
            matches!(cur.check_progress(), Err(ArchiveError::LimitExceeded(LimitViolation::EntryTooLarge { limit, .. })) if limit == ArchiveLimits::FORCED.max_entry_bytes)
        );
    }

    #[test]
    fn lying_header_ratio_uses_selected_profile_and_remains_finite() {
        let mut cur = cursor(ArchiveLimits::FORCED, "reader-policy-ratio.zip");
        cur.consumed = 1001;
        assert!(cur.check_progress().is_ok());
        cur.ratio_limit = ArchiveLimits::DEFAULT.max_compression_ratio;
        assert!(matches!(
            cur.check_progress(),
            Err(ArchiveError::LimitExceeded(LimitViolation::RatioExceeded {
                limit: 1000,
                ..
            }))
        ));
        cur.ratio_limit = ArchiveLimits::FORCED.max_compression_ratio;
        cur.consumed = 10_001;
        assert!(matches!(
            cur.check_progress(),
            Err(ArchiveError::LimitExceeded(LimitViolation::RatioExceeded {
                limit: 10_000,
                ..
            }))
        ));
    }
}
