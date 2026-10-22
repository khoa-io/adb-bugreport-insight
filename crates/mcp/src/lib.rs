//! `ARCH-MCP-SRV` and `ARCH-MCP-TOOLS-ARC`: the MCP surface over
//! `bugreport-insight-core`.
//!
//! # The seam
//!
//! [`registry::ToolHandler`] carries no protocol type, so all seven tools — and
//! the `REQ-MCP-08` four-way status matrix — are testable with no server
//! running. [`adapter`] is the single module that names an `rmcp` symbol, and it
//! contains no tool logic: it converts parameters, delegates, and maps the
//! outcome onto the wire.
//!
//! # stdout
//!
//! stdout carries JSON-RPC frames and nothing else. Diagnostics go to stderr via
//! [`diag`], which cannot reach stdout by construction.

pub mod adapter;
pub(crate) mod confirmation;
pub mod diag;
pub mod error;
mod insights;
pub mod registry;
pub mod resources;
pub mod server;
pub mod tools;
#[cfg(windows)]
mod winfs;

pub use error::{ToolError, ToolErrorKind};
pub use insights::DIRECTORY_MODE_UNSUPPORTED;
pub use registry::{ToolHandler, registry};
pub use server::{ServeError, serve, serve_with_options};
pub use tools::envelope::ToolEnvelope;

/// The seven committed tool names; array order is not a wire discovery contract.
///
/// Asserted against `tools/list`, so the two cannot drift.
pub const TOOL_NAMES: [&str; 7] = [
    "archive_info",
    "list_entries",
    "find_entries",
    "entry_info",
    "find_by_type",
    "read_entry",
    "grep_archive",
];
