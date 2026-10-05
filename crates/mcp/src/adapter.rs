//! The `rmcp` adapter: the **only** place an `rmcp` symbol appears.
//!
//! Zero tool logic lives here. Each `#[tool]` method deserialises its parameters
//! through `serde_json`, delegates to the same [`ToolHandler::call`] any unit
//! test calls, and converts the outcome — `Protocol` errors to
//! `ErrorData::invalid_params`, `Domain` errors to a caller-visible
//! `CallToolResult::error`. The seam in [`crate::registry`] is what makes that
//! possible.

use rmcp::ErrorData as McpError;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{tool, tool_handler, tool_router};
use serde_json::Value;

use crate::error::{ToolError, ToolErrorKind};
use crate::registry::{ToolHandler, registry};
use crate::tools::envelope::ToolEnvelope;
use crate::tools::handlers::Ctx;
use crate::tools::params::{
    ArchiveInfoParams, EntryInfoParams, FindByTypeParams, FindEntriesParams, GrepArchiveParams,
    ListEntriesParams, ReadEntryParams,
};

/// Tool dispatch plus the `tools` capability.
///
/// `#[tool_router]` collects the seven `#[tool]` methods; `#[tool_handler]`
/// generates `list_tools` and `call_tool` from that router, so there is no
/// hand-written dispatch table to drift out of sync.
#[derive(Clone)]
pub struct ArchiveServer {
    handlers: std::sync::Arc<Vec<Box<dyn ToolHandler>>>,
}

impl ArchiveServer {
    pub fn new(ctx: Ctx) -> Self {
        Self {
            handlers: std::sync::Arc::new(registry(ctx)),
        }
    }

    fn dispatch(&self, name: &str, params: Value) -> Result<ToolEnvelope, ToolError> {
        let handler = self
            .handlers
            .iter()
            .find(|h| h.name() == name)
            .ok_or_else(|| ToolError::protocol("method_not_found", format!("no tool `{name}`")))?;
        handler.call(params)
    }
}

/// Both failure modes in one place, per `design/05-mcp-server-design.md` §4.
///
/// The choice is by *whose problem it is*: a malformed request the server
/// cannot route is a protocol error, while a well-formed request against a
/// target that could not be read is a tool error whose structured body the
/// agent can act on — retry with a different `entry_id`, for instance.
fn into_result(outcome: Result<ToolEnvelope, ToolError>) -> Result<CallToolResult, McpError> {
    match outcome {
        Ok(envelope) => {
            let body = serde_json::to_string(&envelope).map_err(|e| {
                McpError::internal_error(format!("failed to serialise result envelope: {e}"), None)
            })?;
            Ok(CallToolResult::success(vec![ContentBlock::text(body)]))
        }
        Err(e) if e.kind == ToolErrorKind::Protocol => Err(McpError::invalid_params(
            e.message,
            Some(serde_json::json!({ "code": e.code })),
        )),
        Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(
            e.to_json().to_string(),
        )])),
    }
}

#[tool_router]
impl ArchiveServer {
    #[tool(
        description = "Cheapest call: index only, no bytes read. Identity of the bound archive \
        — basename, size, SHA-256 (memoised, so paid once per process), entry count, version.txt, \
        main entry. Call this first to confirm you are looking at the archive you think you are."
    )]
    fn archive_info(
        &self,
        Parameters(p): Parameters<ArchiveInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("archive_info", params))
    }

    #[tool(
        description = "Cheap: index only, no content read. Lists ZIP entries with sizes, \
        optionally under one path prefix such as `ANR/`. Bounded to 200 items by default and \
        reports what it omitted."
    )]
    fn list_entries(
        &self,
        Parameters(p): Parameters<ListEntriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("list_entries", params))
    }

    #[tool(
        description = "Cheap: index only, no content read. Entries whose path matches a glob \
        (or a literal). Bounded to 200 items by default. Cheaper than `find_by_type`, which \
        samples bytes."
    )]
    fn find_entries(
        &self,
        Parameters(p): Parameters<FindEntriesParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("find_entries", params))
    }

    #[tool(
        description = "Cheap: index plus one 8 KiB detect sample. Authoritative compressed and \
        uncompressed size, compression method, mtime, and content type reported with its basis. \
        Addressed by `entry_id`, never by path."
    )]
    fn entry_info(
        &self,
        Parameters(p): Parameters<EntryInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("entry_info", params))
    }

    #[tool(
        description = "Moderate: an 8 KiB detect sample per entry, no content read. Entries of \
        one detected content type, with the basis that produced the classification. Bounded to \
        200 items."
    )]
    fn find_by_type(
        &self,
        Parameters(p): Parameters<FindByTypeParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("find_by_type", params))
    }

    #[tool(
        description = "Moderate: reads bytes of one entry, bounded. Takes either \
        `line_start`/`line_end` or `byte_start`/`byte_end`, never both. Never returns a whole \
        entry; excerpts cap at 512 bytes and truncation is always flagged."
    )]
    fn read_entry(
        &self,
        Parameters(p): Parameters<ReadEntryParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("read_entry", params))
    }

    #[tool(
        description = "Most expensive: scans entry content. Literal or glob search returning \
        locators — `entry_id`, `line`, `byte_offset`, and a 512-byte excerpt. Capped at 50 matches \
        and 32 MiB scanned per call; both caps are reported, never silent."
    )]
    fn grep_archive(
        &self,
        Parameters(p): Parameters<GrepArchiveParams>,
    ) -> Result<CallToolResult, McpError> {
        let params = serde_json::to_value(p).map_err(|e| {
            McpError::invalid_params(format!("cannot re-encode parameters: {e}"), None)
        })?;
        into_result(self.dispatch("grep_archive", params))
    }
}

/// Advertises the `tools` capability and nothing else.
///
/// No `prompts`, no `resources`, no `logging` — `logging` is deprecated under
/// SEP-2577, and advertising a capability with no handler is a lie to the
/// client.
///
/// `get_info` is written out rather than generated by `#[tool_handler]`: that
/// macro takes its `version` and `instructions` as string literals, so neither
/// can be a named constant or `env!`. Writing it keeps `INSTRUCTIONS` a single
/// source of truth that a test can assert on.
#[tool_handler]
impl rmcp::ServerHandler for ArchiveServer {
    fn get_info(&self) -> rmcp::model::ServerConfig {
        rmcp::model::ServerConfig::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new(
            "bugreport-insight",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(INSTRUCTIONS)
    }
}

/// At most 40 lines, naming each tool, its cost class, and the intended order.
pub const INSTRUCTIONS: &str = "\
Read-only access to ONE bound Android bugreport ZIP. Nothing is ever extracted, written, \
or modified; the archive is opened in place and you cannot change it mid-session.

Call order: archive_info, then find_entries or find_by_type to locate what you need, then \
entry_info for authoritative sizes, then read_entry or grep_archive for content.

Address entries by entry_id, never by path. Paths are output for your reading only.

Cost classes, cheapest first:
  archive_info   index only, no bytes. SHA-256 memoised, paid once per process.
  list_entries   index only. 200 items max. Optional path_prefix to scope a subtree.
  find_entries   index only, glob on the path. 200 items max.
  entry_info     index plus an 8 KiB detect sample of one entry.
  find_by_type   an 8 KiB detect sample per entry, across the archive. 200 items max.
  read_entry     reads bytes of one entry. Line range or byte range, never both.
                 Excerpts cap at 512 bytes; truncation is always flagged.
  grep_archive   scans entry content. 50 matches and 32 MiB per call. Returns locators
                 (entry_id, line, byte_offset, 512-byte excerpt), never raw section text.

Every result carries status, truncated, stopped_reason, omitted_count, and notes. \
status is one of ok, empty, not_found, unreadable: a failure is never shaped like \
an empty success, so 'empty' always means the call worked and there was nothing.

If a result is truncated, narrow the request rather than retrying it wider.";
