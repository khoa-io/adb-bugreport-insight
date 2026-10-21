//! stderr diagnostics, and nothing else.
//!
//! `eprintln!` cannot reach stdout by construction, which is what makes the
//! stdout-purity guarantee a property of the code rather than a convention.
//! `tracing-subscriber` is absent from the offline cache and is not needed:
//! there is exactly one log sink and it is stderr.

use std::sync::atomic::{AtomicBool, Ordering};

static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

pub fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

/// Always emitted: startup failures, and anything that changes what the agent
/// can rely on.
pub fn info(msg: &str) {
    eprintln!("bugreport-insight: {msg}");
}

/// Emitted only under `--verbose` (`REQ-CLI-05`). Still stderr.
pub fn debug(msg: &str) {
    if is_verbose() {
        eprintln!("bugreport-insight: {msg}");
    }
}

pub fn error(msg: &str) {
    eprintln!("bugreport-insight: error: {msg}");
}
