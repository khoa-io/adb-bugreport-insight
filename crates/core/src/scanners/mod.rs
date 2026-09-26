//! Pattern-based scanners that turn a primary bugreport text (and, for native crashes, the raw
//! ZIP archive) into schema-versioned `Finding`s. Each scanner tolerates missing sections and
//! produces zero findings rather than failing when its signal is absent.
//!
//! Scanners owned by a built-in subcommand (`anr`, `apps`, native/Java crashes for
//! `stacktrace`, `memory`, CPU/monitor-contention for `cpu`) live in their respective
//! `bugreport-insight-<verb>` crates instead of here, so each is independently reusable as a
//! library (by the `bugreport-insight` CLI, or by a third-party subcommand) without pulling in
//! the whole scanner suite. This module keeps the scanners with no owning subcommand
//! (`watchdog`, `broadcast_latency`, `activity_timeline`) plus the small pattern-matching
//! helpers below, which the subcommand crates also depend on.

pub mod activity;
pub mod crash;
pub mod performance;

pub use activity::scan_activity_timeline;
pub use crash::scan_watchdog;
pub use performance::scan_broadcast_latency;

/// Evidence lines kept per finding; scanners may observe far more occurrences than this, but
/// truncate what they attach so `findings.json` stays bounded.
pub const MAX_EVIDENCE_PER_FINDING: usize = 10;

/// Returns the report lines in `(index + 1)..=(index + span)`, clamped to bounds, so
/// callers can look for a follow-up line (e.g. `Reason:` after `ANR in`) without panicking.
pub fn lookahead<'a>(lines: &'a [&'a str], index: usize, span: usize) -> &'a [&'a str] {
    let start = (index + 1).min(lines.len());
    let end = (index + 1 + span).min(lines.len());
    &lines[start..end]
}

/// All `(0-based index, line)` pairs containing `needle`.
pub fn find_all<'a>(report: &'a str, needle: &str) -> Vec<(usize, &'a str)> {
    report
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(needle))
        .collect()
}

/// A short, de-duplicated, order-preserving sample of `values` for use as finding metadata.
pub fn unique_sample(values: &[String], cap: usize) -> String {
    let mut seen: Vec<&String> = Vec::new();
    for value in values {
        if !seen.contains(&value) {
            seen.push(value);
        }
        if seen.len() >= cap {
            break;
        }
    }
    seen.into_iter().cloned().collect::<Vec<_>>().join(", ")
}
