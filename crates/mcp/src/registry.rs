//! The tool seam: seven handlers, no protocol types.
//!
//! Nothing in this module — or anywhere in [`crate::tools`] — names an `rmcp`
//! symbol. That is what makes the whole tool layer, and the `REQ-MCP-08`
//! four-way status matrix, unit-testable with no server running.

use serde_json::Value;

use crate::error::ToolError;
use crate::tools::envelope::ToolEnvelope;
use crate::tools::handlers::{self, Ctx};

pub trait ToolHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn call(&self, params: Value) -> Result<ToolEnvelope, ToolError>;
}

/// Exactly seven, in the order `architecture.md` §5 lists them.
///
/// `tools/list` is asserted against this, so an eighth tool is a deliberate act
/// rather than an accident. Every handler shares the same `Arc<Archive>`: one
/// immutable index, no lock, no per-tool state.
pub fn registry(ctx: Ctx) -> Vec<Box<dyn ToolHandler>> {
    vec![
        Box::new(handlers::ArchiveInfo::new(ctx.clone())),
        Box::new(handlers::ListEntries::new(ctx.clone())),
        Box::new(handlers::FindEntries::new(ctx.clone())),
        Box::new(handlers::EntryInfo::new(ctx.clone())),
        Box::new(handlers::FindByType::new(ctx.clone())),
        Box::new(handlers::ReadEntry::new(ctx.clone())),
        Box::new(handlers::GrepArchive::new(ctx)),
    ]
}
