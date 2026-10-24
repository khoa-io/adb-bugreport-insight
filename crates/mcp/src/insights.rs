//! Protocol-free, confined discovery and in-memory insight access.
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bugreport_insight_core::insight::{InsightFile, generate_insight};
use bugreport_insight_core::{Archive, ArchiveLimits, StoppedReason};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::confirmation::ConfirmationGate;
use crate::registry::{ToolHandler, registry_with_gate};
use crate::{ToolEnvelope, ToolError};

const MAX_DISCOVERY: usize = 1000;
const MAX_LIST: usize = 200;
const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ARCHIVE_TOOL_CONTEXTS: usize = 8;
const ARCHIVE_TOOL_CONTEXT_TTL: Duration = Duration::from_secs(300);

/// Why a directory binding cannot be served on this platform.
///
/// Directory mode needs descriptor-relative, no-follow child access, which only
/// the Unix implementation provides (`REQ-MCP-13`). The message is shared with
/// the CLI so a rejected binding is reported as a startup condition rather
/// than as a transport failure.
pub const DIRECTORY_MODE_UNSUPPORTED: &str =
    "Directory mode requires Unix descriptor-relative file access; use a single ZIP instead";

const ARTIFACTS: [&str; 7] = [
    "README.md",
    "system.json",
    "apps.json",
    "anr.json",
    "crashes.json",
    "memory.json",
    "cpu.json",
];

/// No caller-supplied discovery path: the startup binding defines the scope.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListBugreportsParams {}

/// Generate once in memory and list the seven available artifacts.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInsightsParams {
    /// Exact report basename; no directory components.
    pub bugreport: String,
    /// Single-use token approving this exact operation and its potential analysis cost.
    pub confirmation: Option<String>,
}

/// Read a bounded UTF-8 artifact page, advancing with next_byte_start.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadInsightParams {
    pub bugreport: String,
    /// One fixed filename returned by list_insights (for example system.json).
    pub artifact: String,
    /// UTF-8 byte offset, initially zero; use the previous next_byte_start.
    #[serde(default)]
    pub byte_start: usize,
    /// SHA-256 from the preceding page; required for nonzero byte_start.
    pub source_sha256: Option<String>,
    /// Generation identity returned by the previous page; required for continuation.
    pub snapshot_id: Option<String>,
    /// Output text bytes, 1–65536; defaults to 16384.
    #[serde(default = "default_page")]
    pub max_bytes: usize,
    pub confirmation: Option<String>,
}
fn default_page() -> usize {
    16384
}

struct Cached {
    path: PathBuf,
    sha256: Option<String>,
    files: Vec<InsightFile>,
    snapshot_id: String,
}

struct CachedArchiveTools {
    handlers: Arc<Vec<Box<dyn ToolHandler>>>,
    touched: Instant,
}

struct ArchiveToolCache {
    reports: HashMap<String, CachedArchiveTools>,
    gate: Arc<ConfirmationGate>,
    next_context_id: u64,
}

/// The pinned source of a session.
///
/// A single-file session pins the file; a directory session pins the
/// directory and opens children relative to it, so a pathname replacement
/// cannot redirect a later read.
#[derive(Debug)]
enum Pinned {
    /// A single bound file.
    File(std::fs::File),
    /// A bound directory, enumerated through its handle.
    #[cfg(windows)]
    Directory(crate::winfs::PinnedDirectory),
    /// A bound directory, enumerated through its descriptor.
    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "ios",
        target_os = "freebsd"
    ))]
    Directory(std::fs::File),
}

/// A fixed file or immediate directory, never a search through ancestors.
pub(crate) struct Insights {
    bound: PathBuf,
    directory: bool,
    pinned: Pinned,
    limits: ArchiveLimits,
    gate: ConfirmationGate,
    cache: Mutex<Option<Cached>>,
    archive_tools: Mutex<ArchiveToolCache>,
    generation: std::sync::atomic::AtomicU64,
}
impl Insights {
    pub(crate) fn new(path: &Path, limits: ArchiveLimits) -> std::io::Result<Self> {
        let bound = path.canonicalize()?;
        // The directory case is decided from `metadata` before any handle is
        // opened: `File::open` on a directory fails on Windows with
        // `PermissionDenied` (os error 5), which would report the binding
        // condition as an IO failure instead of the platform condition it is.
        let directory = std::fs::metadata(&bound)?.is_dir();
        let pinned = if directory {
            #[cfg(windows)]
            {
                Pinned::Directory(crate::winfs::open_directory(&bound)?)
            }
            #[cfg(any(
                target_os = "macos",
                target_os = "linux",
                target_os = "ios",
                target_os = "freebsd"
            ))]
            {
                Pinned::Directory(std::fs::File::open(&bound)?)
            }
            #[cfg(not(any(
                target_os = "macos",
                target_os = "linux",
                target_os = "ios",
                target_os = "freebsd",
                windows
            )))]
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    DIRECTORY_MODE_UNSUPPORTED,
                ));
            }
        } else {
            Pinned::File(std::fs::File::open(&bound)?)
        };

        Ok(Self {
            bound,
            directory,
            pinned,
            limits,
            gate: ConfirmationGate::default(),
            cache: Mutex::new(None),
            archive_tools: Mutex::new(ArchiveToolCache {
                reports: HashMap::new(),
                gate: Arc::new(ConfirmationGate::default()),
                next_context_id: 0,
            }),
            generation: std::sync::atomic::AtomicU64::new(0),
        })
    }

    /// Dispatch one archive tool against an exact report in the confined catalog.
    ///
    /// The selected basename is normalized before dispatch. Confirmation also
    /// binds to the retained context generation, so reopening a basename cannot
    /// reuse a token issued for an older archive snapshot.
    pub(crate) fn archive_tool(
        &self,
        name: &str,
        mut params: Value,
    ) -> Result<ToolEnvelope, ToolError> {
        if !self.directory {
            return Err(ToolError::protocol(
                "archive_tool_unavailable",
                "Directory archive tools require a directory-bound insight session",
            ));
        }
        let requested = match params.get("bugreport") {
            None | Some(Value::Null) => None,
            Some(Value::String(id)) => Some(id.as_str()),
            Some(_) => {
                return Err(ToolError::protocol(
                    "invalid_params",
                    "bugreport must be a string basename or null",
                ));
            }
        };
        let id = self.select_report(requested)?;
        let arguments = params.as_object_mut().ok_or_else(|| {
            ToolError::protocol("invalid_params", "tool arguments must be an object")
        })?;
        arguments.insert("bugreport".into(), Value::String(id.clone()));

        let handlers = self.archive_tool_handlers(&id)?;
        let result = handlers
            .iter()
            .find(|handler| handler.name() == name)
            .ok_or_else(|| ToolError::protocol("method_not_found", format!("no tool `{name}`")))
            .and_then(|handler| handler.call(params));
        self.touch_archive_tool_context(&id);

        match result {
            Ok(mut envelope) => {
                if let Some(data) = envelope.data.as_object_mut() {
                    data.insert("bugreport".into(), Value::String(id));
                }
                Ok(envelope)
            }
            Err(mut error) => {
                if error.kind == crate::ToolErrorKind::Domain {
                    let data = error.data.get_or_insert_with(|| serde_json::json!({}));
                    if let Some(object) = data.as_object_mut() {
                        object.insert("bugreport".into(), Value::String(id));
                    }
                }
                Err(error)
            }
        }
    }

    fn select_report(&self, requested: Option<&str>) -> Result<String, ToolError> {
        if let Some(id) = requested {
            return Ok(id.to_owned());
        }

        let listing = self.list()?;
        let reports = listing.data["bugreports"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["bugreport"].as_str().map(str::to_owned))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !listing.truncated && reports.len() == 1 {
            return Ok(reports[0].clone());
        }
        if !listing.truncated && reports.is_empty() {
            return Err(ToolError::domain(
                "no_bugreports",
                "No recognized bugreport ZIPs are available in the bound directory",
                serde_json::json!({}),
            ));
        }
        Err(ToolError::protocol(
            "bugreport_required",
            "Specify the exact `bugreport` basename of an immediate report; use a list_bugreports identifier when available",
        ))
    }

    fn archive_tool_handlers(&self, id: &str) -> Result<Arc<Vec<Box<dyn ToolHandler>>>, ToolError> {
        let mut cache = self.archive_tools.lock().map_err(|_| {
            ToolError::protocol(
                "archive_context_unavailable",
                "Archive context cache unavailable",
            )
        })?;
        cache.reports.retain(|_, context| {
            context.touched.elapsed() < ARCHIVE_TOOL_CONTEXT_TTL
                || Arc::strong_count(&context.handlers) > 1
        });
        if let Some(context) = cache.reports.get_mut(id) {
            context.touched = Instant::now();
            return Ok(Arc::clone(&context.handlers));
        }
        if cache.reports.len() >= MAX_ARCHIVE_TOOL_CONTEXTS {
            return Err(ToolError::domain(
                "archive_context_capacity",
                "Eight report contexts are active; finish work or wait for an idle context to expire",
                serde_json::json!({
                    "capacity": MAX_ARCHIVE_TOOL_CONTEXTS,
                    "idle_expiry_seconds": ARCHIVE_TOOL_CONTEXT_TTL.as_secs(),
                }),
            ));
        }

        let file = self.resolve(id)?;
        let path = self.bound.join(id);
        let archive = Archive::from_file(file, path, self.limits)
            .map_err(|error| self.archive_error(&error))?;
        if !recognized(&archive) {
            return Err(ToolError::domain(
                "not_bugreport",
                "ZIP does not contain recognized bugreport markers",
                serde_json::json!({ "bugreport": id }),
            ));
        }
        let context_id = cache.next_context_id;
        cache.next_context_id = cache.next_context_id.checked_add(1).ok_or_else(|| {
            ToolError::domain(
                "archive_context_generation_exhausted",
                "No more report context identifiers are available",
                serde_json::json!({}),
            )
        })?;
        let handlers = Arc::new(registry_with_gate(
            Arc::new(archive),
            Arc::clone(&cache.gate),
            Some(json!(context_id)),
        ));
        cache.reports.insert(
            id.to_owned(),
            CachedArchiveTools {
                handlers: Arc::clone(&handlers),
                touched: Instant::now(),
            },
        );
        Ok(handlers)
    }

    fn touch_archive_tool_context(&self, id: &str) {
        if let Ok(mut cache) = self.archive_tools.lock()
            && let Some(context) = cache.reports.get_mut(id)
        {
            context.touched = Instant::now();
        }
    }

    pub(crate) fn list(&self) -> Result<ToolEnvelope, ToolError> {
        let mut reports = Vec::new();
        let mut rejected = Vec::new();
        let mut scanned = 0;
        let mut limited = false;
        let mut omitted = 0;
        let paths: Box<dyn Iterator<Item = std::io::Result<PathBuf>>> = if self.directory {
            Box::new(self.directory_entries()?.into_iter().map(Ok))
        } else {
            Box::new(std::iter::once(Ok(self.bound.clone())))
        };
        for path in paths {
            if scanned == MAX_DISCOVERY {
                limited = true;
                break;
            }
            scanned += 1;
            let path = path.map_err(io_error)?;
            if self.directory
                && !path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
            {
                continue;
            }
            let Some(id) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let file = match self.resolve(id) {
                Ok(file) => file,
                Err(error) if error.code == "invalid_bugreport" => continue,
                Err(error) => return Err(error),
            };
            let result = Archive::from_file(
                file,
                if self.directory {
                    self.bound.join(id)
                } else {
                    self.bound.clone()
                },
                self.limits,
            );
            match result {
                Ok(archive) if !self.directory || recognized(&archive) => {
                    if reports.len() + rejected.len() < MAX_LIST {
                        reports.push(json!({"bugreport":id,"entry_count":archive.entry_count(),"stored_bytes":archive.index().archive_file_size()}));
                    } else {
                        limited = true;
                        omitted += 1;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    if reports.len() + rejected.len() < MAX_LIST {
                        rejected.push(json!({"bugreport":id,"code":error.code(),"message":self.archive_error(&error).message}));
                    } else {
                        limited = true;
                        omitted += 1;
                    }
                }
            }
        }
        reports.sort_by(|a, b| a["bugreport"].as_str().cmp(&b["bugreport"].as_str()));
        rejected.sort_by(|a, b| a["bugreport"].as_str().cmp(&b["bugreport"].as_str()));
        let data =
            json!({"bugreports":reports,"rejected":rejected,"scanned_directory_entries":scanned});
        let out = if reports.is_empty() && rejected.is_empty() && !limited {
            ToolEnvelope::empty(data, "No bugreport ZIPs found in the bound location")
        } else {
            ToolEnvelope::ok(data)
        };
        Ok(if limited {
            out.with_cap(StoppedReason::ItemCap,omitted).with_note("Discovery stopped at 1000 directory entries or 200 result records; unvisited candidates are unknown.")
        } else {
            out
        })
    }

    #[cfg(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "ios",
        target_os = "freebsd"
    ))]
    fn directory_entries(&self) -> Result<Vec<PathBuf>, ToolError> {
        use std::os::fd::AsRawFd;
        let Pinned::Directory(directory) = &self.pinned else {
            return Err(ToolError::protocol(
                "unsupported_directory",
                DIRECTORY_MODE_UNSUPPORTED,
            ));
        };
        // SAFETY: the live pinned descriptor is a directory. Opening "." relative
        // to it creates an independent enumeration offset for concurrent requests.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io_error(std::io::Error::last_os_error()));
        }
        // SAFETY: fd is a new owned directory descriptor; fdopendir takes ownership.
        let raw = unsafe { libc::fdopendir(fd) };
        if raw.is_null() {
            let error = std::io::Error::last_os_error();
            // SAFETY: fdopendir failed and did not take ownership of fd.
            unsafe { libc::close(fd) };
            return Err(io_error(error));
        }
        struct Directory(*mut libc::DIR);
        impl Drop for Directory {
            fn drop(&mut self) {
                // SAFETY: this guard uniquely owns the DIR stream.
                unsafe { libc::closedir(self.0) };
            }
        }
        let directory = Directory(raw);
        let mut paths = Vec::new();
        while paths.len() <= MAX_DISCOVERY {
            // SAFETY: libc exposes this thread's errno storage on these targets.
            #[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
            let errno = unsafe { libc::__error() };
            #[cfg(target_os = "linux")]
            let errno = unsafe { libc::__errno_location() };
            // SAFETY: errno is a valid thread-local slot; directory is uniquely owned.
            let entry = unsafe {
                *errno = 0;
                libc::readdir(directory.0)
            };
            if entry.is_null() {
                // SAFETY: same thread-local errno slot, read before any other libc call.
                let code = unsafe { *errno };
                if code != 0 {
                    return Err(io_error(std::io::Error::from_raw_os_error(code)));
                }
                break;
            }
            // SAFETY: readdir provides a NUL-terminated d_name valid until its next call.
            let name = unsafe { std::ffi::CStr::from_ptr((*entry).d_name.as_ptr()) };
            if name.to_bytes() == b"." || name.to_bytes() == b".." {
                continue;
            }
            use std::os::unix::ffi::OsStrExt;
            paths.push(
                self.bound
                    .join(std::ffi::OsStr::from_bytes(name.to_bytes())),
            );
        }
        Ok(paths)
    }
    /// Enumerates the pinned directory through its handle.
    #[cfg(windows)]
    fn directory_entries(&self) -> Result<Vec<PathBuf>, ToolError> {
        let Pinned::Directory(directory) = &self.pinned else {
            return Err(ToolError::protocol(
                "unsupported_directory",
                DIRECTORY_MODE_UNSUPPORTED,
            ));
        };
        directory.entries().map_err(io_error)
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "ios",
        target_os = "freebsd",
        windows
    )))]
    fn directory_entries(&self) -> Result<Vec<PathBuf>, ToolError> {
        Err(ToolError::protocol(
            "unsupported_directory",
            DIRECTORY_MODE_UNSUPPORTED,
        ))
    }

    fn resolve(&self, id: &str) -> Result<std::fs::File, ToolError> {
        if id.is_empty() || id == "." || id == ".." || id.contains(['/', '\\', '\0']) {
            return Err(ToolError::protocol(
                "invalid_bugreport",
                "Use an exact immediate ZIP basename; directory paths are not accepted",
            ));
        }
        if !self.directory {
            if self.bound.file_name().and_then(|s| s.to_str()) != Some(id) {
                return Err(ToolError::protocol(
                    "invalid_bugreport",
                    "Bugreport is outside the bound file",
                ));
            }
            let Pinned::File(file) = &self.pinned else {
                return Err(ToolError::protocol(
                    "unsupported_directory",
                    DIRECTORY_MODE_UNSUPPORTED,
                ));
            };
            return file.try_clone().map_err(io_error);
        }
        if !Path::new(id)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
        {
            return Err(ToolError::protocol(
                "invalid_bugreport",
                "Only immediate ZIP files are discoverable",
            ));
        }
        #[cfg(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "ios",
            target_os = "freebsd"
        ))]
        {
            use std::os::fd::{AsRawFd, FromRawFd};
            let Pinned::Directory(directory) = &self.pinned else {
                return Err(ToolError::protocol(
                    "unsupported_directory",
                    DIRECTORY_MODE_UNSUPPORTED,
                ));
            };
            let name = std::ffi::CString::new(id)
                .map_err(|_| ToolError::protocol("invalid_bugreport", "NUL in bugreport name"))?;
            // SAFETY: pinned owns a live directory descriptor and name is NUL terminated.
            // O_NOFOLLOW blocks symlink replacements; O_NONBLOCK avoids FIFO hangs.
            let fd = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                let e = std::io::Error::last_os_error();
                if e.raw_os_error() == Some(libc::ELOOP) {
                    return Err(ToolError::protocol(
                        "invalid_bugreport",
                        "Symlink bugreports are not allowed",
                    ));
                }
                return Err(io_error(e));
            }
            // SAFETY: successful openat returns a new owned descriptor.
            let file = unsafe { std::fs::File::from_raw_fd(fd) };
            if !file.metadata().map_err(io_error)?.is_file() {
                return Err(ToolError::protocol(
                    "invalid_bugreport",
                    "Bugreport must be a regular file",
                ));
            }
            Ok(file)
        }
        // `FILE_FLAG_OPEN_REPARSE_POINT` means a symlink is opened as itself,
        // so the reparse-point attribute can refuse it rather than the open
        // silently following it to another file.
        #[cfg(windows)]
        {
            let Pinned::Directory(directory) = &self.pinned else {
                return Err(ToolError::protocol(
                    "unsupported_directory",
                    DIRECTORY_MODE_UNSUPPORTED,
                ));
            };
            match directory.open_child(id).map_err(io_error)? {
                Some(file) => Ok(file),
                None => Err(ToolError::protocol(
                    "invalid_bugreport",
                    "Symlink and non-regular bugreports are not allowed",
                )),
            }
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "linux",
            target_os = "ios",
            target_os = "freebsd",
            windows
        )))]
        {
            Err(ToolError::protocol(
                "unsupported_directory",
                DIRECTORY_MODE_UNSUPPORTED,
            ))
        }
    }

    fn access(
        &self,
        operation: &str,
        id: &str,
        arguments: Value,
        token: Option<&str>,
        expected: Option<(&str, &str)>,
        read: impl FnOnce(&[InsightFile], &str, &str) -> Result<ToolEnvelope, ToolError>,
    ) -> Result<ToolEnvelope, ToolError> {
        let expected_sha256 = expected.map(|p| p.0);
        let expected_snapshot = expected.map(|p| p.1);
        // Preview uses arguments and configured ceilings only, with no filesystem IO.
        self.gate.check(operation,&arguments,token,json!({"stored_bytes_upper_estimate":self.limits.max_archive_file_bytes.saturating_mul(2),"source_bytes_upper_estimate":self.limits.max_total_bytes,"potential_work":"Open and analyze the bound bugreport ZIP, hash its stored bytes, and generate all seven insight artifacts in memory; no filesystem writes. Cache may reduce analysis work. Identity is rehashed on access; byte figures are conservative per-pass estimates, not cumulative IO or elapsed-time guarantees."}))?;
        let file = self.resolve(id)?;
        let path = if self.directory {
            self.bound.join(id)
        } else {
            self.bound.clone()
        };
        let archive =
            Archive::from_file(file, &path, self.limits).map_err(|e| self.archive_error(&e))?;
        if self.directory && !recognized(&archive) {
            return Err(ToolError::domain(
                "not_bugreport",
                "ZIP does not contain recognized bugreport markers",
                json!({"bugreport":id}),
            ));
        }
        let sha256 = archive
            .identity(true)
            .map_err(|e| self.archive_error(&e))?
            .sha256;
        if expected_sha256.is_some_and(|expected| Some(expected) != sha256.as_deref()) {
            return Err(ToolError::domain(
                "insight_source_changed",
                "Bugreport identity changed between pages; restart at byte_start zero",
                json!({"source_sha256":sha256}),
            ));
        }
        let mut cache = self.cache.lock().map_err(|_| {
            ToolError::protocol("insight_unavailable", "Insight cache lock unavailable")
        })?;
        let valid = cache
            .as_ref()
            .is_some_and(|c| c.path == path && c.sha256 == sha256);
        if expected_snapshot
            .is_some_and(|expected| cache.as_ref().is_none_or(|c| c.snapshot_id != expected))
            || (!valid && expected_sha256.is_some())
        {
            return Err(ToolError::domain(
                "insight_snapshot_expired",
                "Artifact cache was evicted; restart at byte_start zero",
                json!({}),
            ));
        }
        if !valid {
            *cache = None;
            let files = generate_insight(&archive).map_err(|e| {
                ToolError::domain(
                    "insight_failed",
                    match &e {
                        bugreport_insight_core::insight::InsightError::LimitExceeded { .. } => {
                            format!("{e}; {}", self.limit_advice())
                        }
                        bugreport_insight_core::insight::InsightError::Archive(e) => {
                            self.archive_error(e).message
                        }
                        _ => e.to_string(),
                    },
                    json!({"bugreport":id}),
                )
            })?;
            if files.iter().map(|f| f.contents.len()).sum::<usize>() > MAX_CACHE_BYTES {
                return Err(ToolError::domain(
                    "insight_cache_limit",
                    "Generated artifacts exceed the 64 MiB memory cache ceiling",
                    json!({"limit":MAX_CACHE_BYTES}),
                ));
            }
            *cache = Some(Cached {
                path: path.clone(),
                sha256: sha256.clone(),
                snapshot_id: self
                    .generation
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    .to_string(),
                files,
            });
        }
        // Recheck the scoped candidate; all analysis IO itself uses the pinned source.
        let current = Archive::from_file(self.resolve(id)?, &path, self.limits)
            .map_err(|e| self.archive_error(&e))?;
        if current
            .identity(true)
            .map_err(|e| self.archive_error(&e))?
            .sha256
            != sha256
        {
            *cache = None;
            return Err(ToolError::domain(
                "bugreport_changed",
                "Bugreport changed during analysis; retry against a stable file",
                json!({"bugreport":id}),
            ));
        }
        let cached = cache.as_ref().ok_or_else(|| {
            ToolError::domain(
                "insight_unavailable",
                "Insight cache unavailable",
                json!({}),
            )
        })?;
        read(
            &cached.files,
            sha256.as_deref().ok_or_else(|| {
                ToolError::domain(
                    "insight_identity",
                    "Mandatory SHA-256 unavailable",
                    json!({}),
                )
            })?,
            &cached.snapshot_id,
        )
    }

    fn limit_advice(&self) -> &'static str {
        if self.limits == ArchiveLimits::FORCED {
            "The --force limit profile was already selected; this exceeds its finite ceiling"
        } else {
            "Bugreport is too much for normal limits; restart with --force to use the larger finite limit profile"
        }
    }
    fn archive_error(&self, e: &bugreport_insight_core::ArchiveError) -> ToolError {
        let mut error = ToolError::from_archive(e);
        if matches!(e, bugreport_insight_core::ArchiveError::LimitExceeded(_)) {
            error.message = format!("{e}; {}", self.limit_advice());
        }
        error
    }

    pub(crate) fn list_insights(&self, p: ListInsightsParams) -> Result<ToolEnvelope, ToolError> {
        self.access("list_insights",&p.bugreport,json!({"bugreport":p.bugreport}),p.confirmation.as_deref(),None,|files,_sha,_snapshot|Ok(ToolEnvelope::ok(json!({"bugreport":p.bugreport,"artifacts":files.iter().map(|f|json!({"artifact":f.name,"bytes":f.contents.len()})).collect::<Vec<_>>()}))))
    }

    pub(crate) fn read_insight(&self, p: ReadInsightParams) -> Result<ToolEnvelope, ToolError> {
        if !ARTIFACTS.contains(&p.artifact.as_str()) || p.max_bytes == 0 || p.max_bytes > 65536 {
            return Err(ToolError::protocol(
                "invalid_insight_range",
                "Use a listed artifact and max_bytes between 1 and 65536",
            ));
        }
        if p.byte_start > 0 && (p.source_sha256.is_none() || p.snapshot_id.is_none()) {
            return Err(ToolError::protocol(
                "invalid_insight_range",
                "source_sha256 and snapshot_id from preceding page are required for nonzero byte_start",
            ));
        }
        self.access("read_insight",&p.bugreport,json!({"bugreport":p.bugreport,"artifact":p.artifact,"byte_start":p.byte_start,"max_bytes":p.max_bytes,"source_sha256":p.source_sha256,"snapshot_id":p.snapshot_id}),p.confirmation.as_deref(),p.source_sha256.as_deref().zip(p.snapshot_id.as_deref()),|files,sha256,snapshot| {
            let file = files.iter().find(|f| f.name == p.artifact).ok_or_else(||ToolError::domain("insight_unavailable","Artifact missing from generated set",json!({})))?;
            let text = std::str::from_utf8(&file.contents).map_err(|_|ToolError::domain("invalid_insight","Artifact is not UTF-8",json!({})))?;
            if p.byte_start > text.len() || !text.is_char_boundary(p.byte_start) { return Err(ToolError::protocol("invalid_insight_range","byte_start must be an existing UTF-8 boundary or EOF")); }
            let mut end = p.byte_start.saturating_add(p.max_bytes).min(text.len());
            while !text.is_char_boundary(end) { end -= 1; }
            if end == p.byte_start && end < text.len() { return Err(ToolError::protocol("invalid_insight_range","max_bytes cannot fit the next UTF-8 character; increase it")); }
            let next = (end < text.len()).then_some(end);
            let out = ToolEnvelope::ok(json!({"bugreport":p.bugreport,"artifact":p.artifact,"text":&text[p.byte_start..end],"byte_start":p.byte_start,"bytes_returned":end-p.byte_start,"total_bytes":text.len(),"next_byte_start":next,"source_sha256":sha256,"snapshot_id":snapshot}));
            Ok(if next.is_some() {out.with_cap(StoppedReason::ByteBudget,0)} else {out})
        })
    }
}
fn recognized(archive: &Archive) -> bool {
    archive.entries().iter().any(|e| {
        !e.is_dir
            && !e.is_symlink
            && (e.path == "main_entry.txt"
                || (e.path.starts_with("bugreport")
                    && e.path.ends_with(".txt")
                    && !e.path.contains('/')))
    })
}
fn io_error(e: std::io::Error) -> ToolError {
    ToolError::domain("bugreport_io", e.to_string(), json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bugreport_insight_test_fixtures::{ZipBuilder, scratch_dir};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQUENCE: AtomicUsize = AtomicUsize::new(0);
    fn directory() -> PathBuf {
        let dir = scratch_dir().join(format!(
            "insight-tools-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
    fn zip(path: &Path, entry: &str, body: &[u8]) {
        std::fs::write(path, ZipBuilder::new().add(entry, body).finish()).unwrap();
    }
    fn approved_list(insights: &Insights, id: &str) -> Result<ToolEnvelope, ToolError> {
        let mut p = ListInsightsParams {
            bugreport: id.into(),
            confirmation: None,
        };
        let preview = insights.list_insights(p.clone()).unwrap_err();
        assert_eq!(preview.code, "confirmation_required");
        p.confirmation = Some(
            preview.data.unwrap()["confirmation_token"]
                .as_str()
                .unwrap()
                .into(),
        );
        insights.list_insights(p)
    }
    fn approved_read(
        insights: &Insights,
        mut p: ReadInsightParams,
    ) -> Result<ToolEnvelope, ToolError> {
        let preview = insights.read_insight(p.clone()).unwrap_err();
        assert_eq!(preview.code, "confirmation_required");
        p.confirmation = Some(
            preview.data.unwrap()["confirmation_token"]
                .as_str()
                .unwrap()
                .into(),
        );
        insights.read_insight(p)
    }
    fn read(id: &str, artifact: &str, start: usize, size: usize) -> ReadInsightParams {
        ReadInsightParams {
            bugreport: id.into(),
            artifact: artifact.into(),
            byte_start: start,
            source_sha256: None,
            snapshot_id: None,
            max_bytes: size,
            confirmation: None,
        }
    }
    #[test]
    fn discovery_confines_scope_and_uses_report_markers() {
        let root = directory();
        let child = root.join("reports");
        std::fs::create_dir(&child).unwrap();
        zip(&root.join("parent.zip"), "bugreport-parent.txt", b"parent");
        zip(&child.join("legacy.ZIP"), "bugreport-legacy.txt", b"legacy");
        zip(&child.join("modern.zip"), "main_entry.txt", b"report.txt");
        zip(&child.join("unrelated.zip"), "version.txt", b"1");
        zip(
            &child.join("nested-marker.zip"),
            "nested/bugreport.txt",
            b"nested",
        );
        std::fs::create_dir(child.join("nested")).unwrap();
        zip(&child.join("nested/hidden.zip"), "bugreport.txt", b"hidden");
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("parent.zip"), child.join("linked.zip")).unwrap();
        let insights = Insights::new(&child, ArchiveLimits::DEFAULT).unwrap();
        let out = insights.list().unwrap();
        let ids: Vec<_> = out.data["bugreports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["bugreport"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["legacy.ZIP", "modern.zip"]);
        for id in [
            "../parent.zip",
            "nested/hidden.zip",
            "linked.zip",
            "/parent.zip",
            "..\\parent.zip",
        ] {
            assert!(approved_list(&insights, id).is_err(), "accepted {id}");
        }
        assert_eq!(
            approved_list(&insights, "unrelated.zip").unwrap_err().code,
            "not_bugreport"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn discovery_surfaces_too_large_and_forced_ceiling() {
        let root = directory();
        zip(&root.join("report.zip"), "bugreport.txt", b"test");
        let limits = ArchiveLimits {
            max_archive_file_bytes: 1,
            ..ArchiveLimits::DEFAULT
        };
        let insights = Insights::new(&root, limits).unwrap();
        let out = insights.list().unwrap();
        assert!(out.data["bugreports"].as_array().unwrap().is_empty());
        assert!(
            out.data["rejected"][0]["message"]
                .as_str()
                .unwrap()
                .contains("--force")
        );
        assert!(
            approved_list(&insights, "report.zip")
                .unwrap_err()
                .message
                .contains("--force")
        );
        let forced = Insights::new(&root, ArchiveLimits::FORCED).unwrap();
        assert!(forced.limit_advice().contains("already selected"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn generation_pages_exact_artifacts_without_writes_and_invalidates_changed_input() {
        let root = directory();
        let path = root.join("report.zip");
        zip(&path, "bugreport.txt", b"first");
        let insights = Insights::new(&root, ArchiveLimits::DEFAULT).unwrap();
        assert_eq!(
            approved_list(&insights, "report.zip").unwrap().data["artifacts"]
                .as_array()
                .unwrap()
                .len(),
            7
        );
        let mut start = 0;
        let mut bytes = String::new();
        let mut source_sha256 = None;
        let mut snapshot_id = None;
        loop {
            let mut request = read("report.zip", "system.json", start, 8192);
            request.source_sha256 = source_sha256;
            request.snapshot_id = snapshot_id;
            let page = approved_read(&insights, request).unwrap();
            source_sha256 = Some(page.data["source_sha256"].as_str().unwrap().to_owned());
            snapshot_id = Some(page.data["snapshot_id"].as_str().unwrap().to_owned());
            let text = page.data["text"].as_str().unwrap();
            assert!(text.len() <= 8192);
            bytes.push_str(text);
            if let Some(next) = page.data["next_byte_start"].as_u64() {
                assert!(page.truncated);
                assert!(next as usize > start);
                start = next as usize;
            } else {
                assert!(!page.truncated);
                break;
            }
        }
        let first: Value = serde_json::from_str(&bytes).unwrap();
        assert!(first["source"].is_object());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        zip(&path, "bugreport.txt", b"other"); // same-length payload; digest invalidates the cache.
        approved_list(&insights, "report.zip").unwrap();
        let next = approved_read(&insights, read("report.zip", "system.json", 0, 65536)).unwrap();
        let second: Value = serde_json::from_str(next.data["text"].as_str().unwrap()).unwrap();
        assert_ne!(first["source"], second["source"]);
        assert_eq!(
            insights
                .read_insight(read("report.zip", "system.json", usize::MAX, 20))
                .unwrap_err()
                .code,
            "invalid_insight_range"
        );
        assert_eq!(
            insights
                .read_insight(read("report.zip", "../system.json", 0, 10))
                .unwrap_err()
                .code,
            "invalid_insight_range"
        );
        assert_eq!(
            insights
                .read_insight(read("report.zip", "system.json", 0, 65537))
                .unwrap_err()
                .code,
            "invalid_insight_range"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn preview_is_io_free_and_tokens_bind_bugreport_and_operation() {
        let root = directory();
        let insights = Insights::new(&root, ArchiveLimits::DEFAULT).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        let p = ListInsightsParams {
            bugreport: "gone.zip".into(),
            confirmation: None,
        };
        let challenge = insights.list_insights(p.clone()).unwrap_err();
        assert_eq!(challenge.code, "confirmation_required");
        let token = challenge.data.unwrap()["confirmation_token"]
            .as_str()
            .unwrap()
            .to_owned();
        let mut wrong = p.clone();
        wrong.bugreport = "other.zip".into();
        wrong.confirmation = Some(token.clone());
        assert_eq!(
            insights.list_insights(wrong).unwrap_err().code,
            "invalid_confirmation"
        );
        let mut original = p;
        original.confirmation = Some(token);
        assert_eq!(
            insights.list_insights(original.clone()).unwrap_err().code,
            "bugreport_io"
        );
        assert_eq!(
            insights.list_insights(original).unwrap_err().code,
            "invalid_confirmation"
        );
    }
    #[test]
    fn continuation_rejects_changed_source_and_evicted_generation() {
        let root = directory();
        let a = root.join("a.zip");
        zip(&a, "bugreport.txt", b"first");
        zip(&root.join("b.zip"), "bugreport.txt", b"second");
        let insights = Insights::new(&root, ArchiveLimits::DEFAULT).unwrap();
        let first = approved_read(&insights, read("a.zip", "system.json", 0, 100)).unwrap();
        let mut next = read("a.zip", "system.json", 100, 100);
        next.source_sha256 = Some(first.data["source_sha256"].as_str().unwrap().into());
        next.snapshot_id = Some(first.data["snapshot_id"].as_str().unwrap().into());
        approved_list(&insights, "b.zip").unwrap();
        assert_eq!(
            approved_read(&insights, next.clone()).unwrap_err().code,
            "insight_snapshot_expired"
        );
        approved_list(&insights, "a.zip").unwrap();
        assert_eq!(
            approved_read(&insights, next.clone()).unwrap_err().code,
            "insight_snapshot_expired"
        );
        zip(&a, "bugreport.txt", b"other");
        assert_eq!(
            approved_read(&insights, next).unwrap_err().code,
            "insight_source_changed"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn pinned_directory_survives_path_replacement_and_refuses_child_symlinks() {
        use std::os::unix::fs::symlink;
        let root = directory();
        let bound = root.join("bound");
        let outside = root.join("outside");
        std::fs::create_dir(&bound).unwrap();
        std::fs::create_dir(&outside).unwrap();
        zip(&bound.join("inside.zip"), "bugreport.txt", b"inside");
        zip(&outside.join("outside.zip"), "bugreport.txt", b"outside");
        let insights = Insights::new(&bound, ArchiveLimits::DEFAULT).unwrap();
        let original = root.join("original");
        std::fs::rename(&bound, &original).unwrap();
        symlink(&outside, &bound).unwrap();
        let listed = insights.list().unwrap();
        assert_eq!(listed.data["bugreports"][0]["bugreport"], "inside.zip");
        assert_eq!(listed.data["bugreports"].as_array().unwrap().len(), 1);
        approved_list(&insights, "inside.zip").unwrap();
        assert!(approved_list(&insights, "outside.zip").is_err());
        std::fs::remove_file(original.join("inside.zip")).unwrap();
        symlink(outside.join("outside.zip"), original.join("inside.zip")).unwrap();
        assert_eq!(
            approved_list(&insights, "inside.zip").unwrap_err().code,
            "invalid_bugreport"
        );
        assert!(
            insights.list().unwrap().data["bugreports"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    /// The Windows session keeps serving the pinned directory after a rename.
    ///
    /// The bound pathname is freed by the rename, so a session that re-resolved
    /// the pathname would start serving whatever took its place. The pinned
    /// handle keeps serving the directory it was opened on.
    ///
    /// The Unix equivalent is `pinned_directory_survives_path_replacement_and_refuses_child_symlinks`,
    /// which additionally covers the child-symlink case that needs a privilege
    /// Windows does not grant to an ordinary test process.
    #[cfg(windows)]
    #[test]
    fn pinned_windows_directory_survives_rename() {
        let root = directory();
        let bound = root.join("bound");
        let outside = root.join("outside");
        std::fs::create_dir(&bound).unwrap();
        std::fs::create_dir(&outside).unwrap();
        zip(&bound.join("inside.zip"), "bugreport.txt", b"inside");
        zip(&outside.join("outside.zip"), "bugreport.txt", b"outside");
        let insights = Insights::new(&bound, ArchiveLimits::DEFAULT).unwrap();
        let original = root.join("original");
        std::fs::rename(&bound, &original).unwrap();
        // The bound pathname is now free. A replacement directory planted
        // there must not become the session's scope.
        std::fs::create_dir(&bound).unwrap();
        zip(&bound.join("planted.zip"), "bugreport.txt", b"planted");
        let listed = insights.list().unwrap();
        assert_eq!(listed.data["bugreports"][0]["bugreport"], "inside.zip");
        assert_eq!(listed.data["bugreports"].as_array().unwrap().len(), 1);
        approved_list(&insights, "inside.zip").unwrap();
        assert!(approved_list(&insights, "outside.zip").is_err());
        assert!(approved_list(&insights, "planted.zip").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn discovery_bounds_work_and_returned_records() {
        let root = directory();
        for i in 0..201 {
            zip(
                &root.join(format!("report-{i:03}.zip")),
                "bugreport.txt",
                b"x",
            );
        }
        let insights = Insights::new(&root, ArchiveLimits::DEFAULT).unwrap();
        let out = insights.list().unwrap();
        assert_eq!(out.data["bugreports"].as_array().unwrap().len(), 200);
        assert!(out.truncated);
        for i in 0..1000 {
            std::fs::write(root.join(format!("ordinary-{i}")), b"").unwrap();
        }
        let out = insights.list().unwrap();
        assert_eq!(out.data["scanned_directory_entries"], 1000);
        assert!(out.truncated);
        std::fs::remove_dir_all(root).unwrap();
    }
}
