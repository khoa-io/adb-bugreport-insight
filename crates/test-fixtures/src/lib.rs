//! Shared ZIP fixtures for the `mcp` and `cli` test suites.
//!
//! **Why there is exactly one copy.** Two hand-rolled writers drifted: the
//! central-directory header is 46 bytes before the name, and a copy that writes
//! 44 produces an archive `zip` rejects with `InvalidArchive` — which surfaces as
//! `NotAZipArchive`, not as a clear test failure. Keeping one copy means a field
//! cannot be missed in only one suite.
//!
//! **Why this is a crate rather than a `#[path]` module.** Both suites need it
//! and they live in different packages. A `#[path = "../../mcp/…"]` module
//! resolves under `rustc` but not under rust-analyzer, which reports it as an
//! unresolved module and then loses every fixture behind it. A `dev-dependency`
//! resolves identically in both tools and is checked by cargo itself.
//!
//! Reachable from test builds only: this is a dev-dependency of `crates/mcp` and
//! `crates/cli` and of nothing else.

use std::path::PathBuf;

pub fn scratch_dir() -> PathBuf {
    // This crate sits at `crates/<name>/`, so two levels up is the workspace
    // root and `target/` is shared with every other suite.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/test-fixtures");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Minimal **stored**-entry ZIP writer.
///
/// Stored keeps entries byte-exact, so a test can corrupt one at a known offset
/// and produce "a valid central directory over bytes that do not decode".
pub struct ZipBuilder {
    buf: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Default for ZipBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipBuilder {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            central: Vec::new(),
            count: 0,
        }
    }

    pub fn add(mut self, name: &str, data: &[u8]) -> Self {
        let crc = crc32(data);
        let offset = self.buf.len() as u32;

        // Local file header: 30 bytes, then name and payload.
        self.buf.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // flags
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // mod time
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // mod date
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes()); // compressed
        self.buf
            .extend_from_slice(&(data.len() as u32).to_le_bytes()); // uncompressed
        self.buf
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // extra length
        self.buf.extend_from_slice(name.as_bytes());
        self.buf.extend_from_slice(data);

        // Central directory header: 46 bytes, then the name.
        self.central
            .extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        self.central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        self.central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        self.central.extend_from_slice(&0u16.to_le_bytes()); // flags
        self.central.extend_from_slice(&0u16.to_le_bytes()); // method
        self.central.extend_from_slice(&0u16.to_le_bytes()); // mod time
        self.central.extend_from_slice(&0u16.to_le_bytes()); // mod date
        self.central.extend_from_slice(&crc.to_le_bytes());
        self.central
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.central
            .extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.central
            .extend_from_slice(&(name.len() as u16).to_le_bytes());
        self.central.extend_from_slice(&0u16.to_le_bytes()); // extra length
        self.central.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.central.extend_from_slice(&0u16.to_le_bytes()); // disk number start
        self.central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        self.central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        self.central.extend_from_slice(&offset.to_le_bytes());
        self.central.extend_from_slice(name.as_bytes());

        self.count += 1;
        self
    }

    pub fn finish(mut self) -> Vec<u8> {
        let cd_offset = self.buf.len() as u32;
        let cd_size = self.central.len() as u32;
        self.buf.extend_from_slice(&self.central);
        self.buf.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // disk number
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // cd start disk
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&self.count.to_le_bytes());
        self.buf.extend_from_slice(&cd_size.to_le_bytes());
        self.buf.extend_from_slice(&cd_offset.to_le_bytes());
        self.buf.extend_from_slice(&0u16.to_le_bytes()); // comment length
        self.buf
    }
}

/// A normal archive: three entries, one of them containing a known pattern.
///
/// Built **once** per process. Cargo runs tests in parallel threads and several
/// tests call this; writing a shared file each time was a race that surfaced as
/// `NotAZipArchive` in whichever test lost.
pub fn good_archive() -> PathBuf {
    static GOOD: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    GOOD.get_or_init(|| {
        let path = scratch_dir().join("good.zip");
        let bytes = ZipBuilder::new()
            .add("version.txt", b"2\n")
            .add("ANR/traces.txt", b"FATAL EXCEPTION: main\nsecond line\n")
            .add("ANR/other.txt", b"nothing here\n")
            .finish();
        std::fs::write(&path, bytes).expect("write good fixture");
        path
    })
    .clone()
}

/// The same archive with one entry's **stored bytes** altered so its CRC no
/// longer matches: a valid central directory over a corrupt payload. This is
/// the fixture that catches an implementation returning `empty` for a read it
/// could not perform.
pub fn corrupt_entry_archive() -> PathBuf {
    static CORRUPT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    CORRUPT
        .get_or_init(|| {
            let mut bytes = std::fs::read(good_archive()).expect("read good fixture");
            let needle = b"FATAL EXCEPTION: main\n";
            let pos = bytes
                .windows(needle.len())
                .position(|w| w == needle)
                .expect("fixture must contain the payload");
            bytes[pos] = b'X';
            let path = scratch_dir().join("corrupt.zip");
            std::fs::write(&path, bytes).expect("write corrupt fixture");
            path
        })
        .clone()
}

/// A file that is not a ZIP at all.
pub fn not_a_zip() -> PathBuf {
    static NOT_ZIP: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    NOT_ZIP
        .get_or_init(|| {
            let path = scratch_dir().join("not-a-zip.bin");
            std::fs::write(&path, b"this is definitely not a zip archive")
                .expect("write non-zip fixture");
            path
        })
        .clone()
}
