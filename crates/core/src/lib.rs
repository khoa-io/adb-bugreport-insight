//! `bugreport-insight-core` — read-only Android bugreport scanning and insight-artifact
//! rendering. This crate has no CLI/argument-parsing dependency; see the `bugreport-insight`
//! binary crate for the command-line entry point.

pub mod analyze;
pub mod archive;
pub mod error;
pub mod model;
pub mod render;
pub mod scanners;
pub mod sessions;
pub mod source;
pub mod text;

pub use analyze::{open_report, render_insight, unclaimed_findings};
pub use error::Error;
