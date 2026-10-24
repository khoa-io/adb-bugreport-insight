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
use crate::resources::{InsightResources, ResourceError};
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
    resources: std::sync::Arc<InsightResources>,
    resource_confirmations: std::sync::Arc<crate::confirmation::ConfirmationGate>,
    stored_archive_bytes: u64,
    bound_bugreport: Option<String>,
}

impl ArchiveServer {
    pub fn new(ctx: Ctx) -> Self {
        Self {
            resource_confirmations: std::sync::Arc::new(
                crate::confirmation::ConfirmationGate::default(),
            ),
            stored_archive_bytes: ctx.index().archive_file_size(),
            bound_bugreport: ctx
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned),
            resources: std::sync::Arc::new(InsightResources::new(ctx.clone())),
            handlers: std::sync::Arc::new(registry(ctx)),
        }
    }

    fn dispatch(&self, name: &str, params: Value) -> Result<ToolEnvelope, ToolError> {
        match params.get("bugreport") {
            None | Some(Value::Null) => {}
            Some(Value::String(bugreport))
                if self.bound_bugreport.as_deref() == Some(bugreport) => {}
            Some(Value::String(_)) => {
                return Err(ToolError::protocol(
                    "invalid_bugreport",
                    "bugreport must match the server's bound ZIP basename",
                ));
            }
            Some(_) => {
                return Err(ToolError::protocol(
                    "invalid_params",
                    "bugreport must be a string basename or null",
                ));
            }
        }
        let handler = self
            .handlers
            .iter()
            .find(|h| h.name() == name)
            .ok_or_else(|| ToolError::protocol("method_not_found", format!("no tool `{name}`")))?;
        handler.call(params)
    }
}

/// Both failure modes in one place, per `docs/swe.3-detailed-design/03-mcp-server.md`.
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
        description = "Potentially expensive: optional full stored-ZIP SHA-256 plus two bounded marker reads. \
        Preview returns confirmation_required; retry identical arguments with confirmation token. \
        Digest choice is cached; markers are reread. Small output does not bound IO."
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
        description = "Confirmation required: one 8 KiB detect sample plus fresh ZIP metadata reads. Authoritative compressed and \
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
        description = "Confirmation required: an 8 KiB detect sample per file, including beyond the output limit. Returns matching entries, \
        classification basis, explicit failures and sample coverage. Up to 200 matches and \
        200 failure records; incomplete classification is never an empty success."
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
        description = "Confirmation required: reads/decompresses one entry including the skipped prefix. Takes either \
        `line_start`/`line_end` or `byte_start`/`byte_end`, never both. Never returns a whole \
        entry; excerpts cap at 512 bytes. Past EOF returns available content with past_end; reversed ranges are rejected."
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
        description = "Confirmation required for every page: scans entry content. Literal or glob search returning \
        locators — `entry_id`, `line`, `byte_offset`, and a 512-byte excerpt. Up to 50 matches \
        and 32 MiB new source bytes per page; response_budget bounds serialized result bytes, \
        scan_budget bounds scanning. Continue with cursor; omit query fields on continuation."
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

/// Advertises tools and read-only insight resources; no prompts or logging.
///
/// `get_info` is written out rather than generated by `#[tool_handler]`: that
/// macro takes its `version` and `instructions` as string literals, so neither
/// can be a named constant or `env!`. Writing it keeps `INSTRUCTIONS` a single
/// source of truth that a test can assert on.
#[tool_handler]
impl rmcp::ServerHandler for ArchiveServer {
    async fn list_resources(
        &self,
        request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListResourcesResult, McpError> {
        if request.as_ref().is_some_and(|p| p.cursor.is_some()) {
            return Err(McpError::invalid_params(
                "insight resources do not use cursors",
                None,
            ));
        }
        let token = resource_confirmation_token(
            request.as_ref().and_then(|p| p.meta.as_ref()),
            &context.meta,
        )?;
        self.resource_confirmations.check(
            "resources/list",
            &serde_json::json!({}),
            token,
            serde_json::json!({
                "stored_archive_bytes": self.stored_archive_bytes,
                "manifest_max_bytes": crate::resources::MAX_RESOURCE_BYTES,
                "potential_work": "Hash the complete ZIP if uncached, read marker entries and validate the complete system.json manifest; inspect artifact metadata.",
            }),
        ).map_err(resource_confirmation_error)?;
        let provider = self.resources.clone();
        let resources = tokio::task::spawn_blocking(move || provider.list())
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(resource_error)?;
        let result = rmcp::model::ListResourcesResult::with_all_items(resources.into_iter().map(|r| {
            rmcp::model::Resource::new(r.uri, r.name)
                .with_mime_type(r.mime_type).with_size(r.size)
                .with_description("Complete generated insight artifact; read-only, not a summary. Run --insight BUGREPORT first.")
        }).collect());
        Ok(result
            .with_ttl_ms(0)
            .with_cache_scope(rmcp::model::CacheScope::Private))
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResponse, McpError> {
        let token = resource_confirmation_token(request.meta.as_ref(), &context.meta)?;
        self.resource_confirmations.check(
            "resources/read",
            &serde_json::json!({ "uri": request.uri }),
            token,
            serde_json::json!({
                "stored_archive_bytes": self.stored_archive_bytes,
                "manifest_max_bytes": crate::resources::MAX_RESOURCE_BYTES,
                "resource_max_bytes": crate::resources::MAX_RESOURCE_BYTES,
                "potential_work": "Hash the complete ZIP if uncached, read marker entries, validate the complete system.json manifest and read the complete requested artifact.",
            }),
        ).map_err(resource_confirmation_error)?;
        let provider = self.resources.clone();
        let (resource, text) = tokio::task::spawn_blocking(move || provider.read(&request.uri))
            .await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?
            .map_err(resource_error)?;
        Ok(rmcp::model::ReadResourceResult::new(vec![
            rmcp::model::ResourceContents::text(text, resource.uri)
                .with_mime_type(resource.mime_type),
        ])
        .with_ttl_ms(0)
        .with_cache_scope(rmcp::model::CacheScope::Private)
        .into())
    }

    fn get_info(&self) -> rmcp::model::ServerConfig {
        rmcp::model::ServerConfig::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
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

fn resource_confirmation_token<'a>(
    request_meta: Option<&'a rmcp::model::RequestMetaObject>,
    context_meta: &'a rmcp::model::RequestMetaObject,
) -> Result<Option<&'a str>, McpError> {
    let value = request_meta
        .and_then(|m| m.0.get("bugreport-insight/confirmation"))
        .or_else(|| context_meta.0.get("bugreport-insight/confirmation"));
    value
        .map(|value| {
            value.as_str().ok_or_else(|| {
                McpError::invalid_params(
                    "confirmation token must be a string",
                    Some(serde_json::json!({ "code": "invalid_confirmation" })),
                )
            })
        })
        .transpose()
}

fn resource_confirmation_error(error: ToolError) -> McpError {
    let mut data = error.data.unwrap_or_else(|| serde_json::json!({}));
    if let Some(object) = data.as_object_mut() {
        object.insert("code".into(), Value::String(error.code.clone()));
    }
    let code = if error.kind == ToolErrorKind::Domain {
        rmcp::model::ErrorCode(-32000)
    } else {
        rmcp::model::ErrorCode::INVALID_PARAMS
    };
    McpError::new(code, error.message, Some(data))
}

fn resource_error(error: ResourceError) -> McpError {
    match error {
        ResourceError::NotFound(_) => McpError::resource_not_found(error.to_string(), None),
        ResourceError::Io(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            McpError::resource_not_found(error.to_string(), None)
        }
        _ => McpError::internal_error(error.to_string(), None),
    }
}

/// At most 40 lines, naming each tool, its cost class, and the intended order.
pub const INSTRUCTIONS: &str = "\
Read-only access to ONE bound Android bugreport ZIP; no extraction or modification.
Startup reads ZIP metadata. initialize/tools/list do not read payloads or hash the ZIP.
Each archive tool accepts an optional `bugreport` basename; when supplied, it must match the bound ZIP.

Start with list_entries/find_entries; use archive_info to establish full identity when needed.
Address entries by entry_id; paths are display filters, never entry-addressing keys.

Before content tools execute, confirmation_required returns cost and a confirmation_token.
Retry identical arguments with confirmation set to that token only if you accept the cost.
Tokens are single-use, expire after 300 seconds and bind one operation and its arguments.
No automatic execution, blanket approval, or approval inherited by later calls/pages.

Actual work; output bounds do not rank execution cost:
  archive_info   may hash the entire stored ZIP; reads two markers (256 source bytes each).
                 First digest choice is cached, including declining it; markers are reread.
  list_entries   immediate index query, 200 items max; segment-aware path_prefix.
  find_entries   immediate index query, 200 items max; literal or glob path matching.
  entry_info     confirmation required; fresh ZIP metadata and an 8 KiB detection sample.
  find_by_type   confirmation required; an 8 KiB sample per file, regardless of output limit.
                 Up to 200 matches and 200 failures; inspect sample coverage.
  read_entry     confirmation required; may decompress a large skipped prefix.
                 Line/byte ranges; 512-byte output excerpts; inspect past_end and clipping.
  grep_archive   confirmation required each page; up to 50 matches/32 MiB new source bytes.
                 Client response/scan budgets; decoder CPU/physical IO are not bounded by bytes.

Successful tool envelopes carry status, truncated, stopped_reason, omitted_count and notes.
Errors and confirmation challenges use separate error shapes without success envelopes.
For grep, follow next_cursor without query fields; each continuation needs its own preview.
Partial/empty pages may continue; coverage is complete only when explicitly reported.

Insight resources require confirmation too; they may hash ZIP/read complete artifacts.
resources/list and resources/read return error.data with cost and confirmation_token.
Retry identical resource params with _meta[bugreport-insight/confirmation] set to the token.
Generate first with --insight BUGREPORT; resources never generate or modify files.
Resources return complete Markdown/JSON, 64 MiB max each; manifests add IO.
Cost previews are conservative estimates, not elapsed-time or physical-IO guarantees.";

/// Additional insight tools over a file or a confined immediate directory.
#[derive(Clone)]
pub(crate) struct InsightServer {
    insights: std::sync::Arc<crate::insights::Insights>,
    archive: Option<ArchiveServer>,
}
impl InsightServer {
    pub(crate) fn new(
        path: &std::path::Path,
        limits: bugreport_insight_core::ArchiveLimits,
        archive: Option<ArchiveServer>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            insights: std::sync::Arc::new(crate::insights::Insights::new(path, limits)?),
            archive,
        })
    }

    async fn archive_tool_call(
        &self,
        name: &'static str,
        params: Value,
    ) -> Result<CallToolResult, McpError> {
        if let Some(archive) = &self.archive {
            return into_result(archive.dispatch(name, params));
        }
        let insights = self.insights.clone();
        let outcome = tokio::task::spawn_blocking(move || insights.archive_tool(name, params))
            .await
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        into_result(outcome)
    }
}
#[tool_router]
impl InsightServer {
    #[tool(
        description = "Potentially expensive: optional full stored-ZIP SHA-256 plus two bounded marker reads. Preview returns confirmation_required; retry identical arguments with confirmation token. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn archive_info(
        &self,
        Parameters(p): Parameters<ArchiveInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("archive_info", tool_params(p)?)
            .await
    }

    #[tool(
        description = "Cheap: index only, no content read. Lists ZIP entries with sizes, optionally under one path prefix. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn list_entries(
        &self,
        Parameters(p): Parameters<ListEntriesParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("list_entries", tool_params(p)?)
            .await
    }

    #[tool(
        description = "Cheap: index only, no content read. Finds entry paths by glob or literal. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn find_entries(
        &self,
        Parameters(p): Parameters<FindEntriesParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("find_entries", tool_params(p)?)
            .await
    }

    #[tool(
        description = "Confirmation required: one 8 KiB detection sample plus ZIP metadata. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn entry_info(
        &self,
        Parameters(p): Parameters<EntryInfoParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("entry_info", tool_params(p)?).await
    }

    #[tool(
        description = "Confirmation required: samples each file up to 8 KiB. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn find_by_type(
        &self,
        Parameters(p): Parameters<FindByTypeParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("find_by_type", tool_params(p)?)
            .await
    }

    #[tool(
        description = "Confirmation required: reads one bounded line or byte range, including any decompressed prefix. Directory mode accepts an exact bugreport basename; omission is valid only for one report in a complete catalog."
    )]
    async fn read_entry(
        &self,
        Parameters(p): Parameters<ReadEntryParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("read_entry", tool_params(p)?).await
    }

    #[tool(
        description = "Confirmation required for each page: literal or glob search with bounded matches, source bytes and response size. Repeat the same bugreport basename on every directory-mode continuation; omission is valid only for one report in a complete catalog."
    )]
    async fn grep_archive(
        &self,
        Parameters(p): Parameters<GrepArchiveParams>,
    ) -> Result<CallToolResult, McpError> {
        self.archive_tool_call("grep_archive", tool_params(p)?)
            .await
    }

    #[tool(
        description = "Discover recognized bugreport ZIPs in the bound location only. Metadata only; no recursive or parent search. At most 1000 directory entries and 200 reports/diagnostics; inspect truncation and rejected limit errors."
    )]
    async fn list_bugreports(
        &self,
        Parameters(_): Parameters<crate::insights::ListBugreportsParams>,
    ) -> Result<CallToolResult, McpError> {
        let insights = self.insights.clone();
        into_result(
            tokio::task::spawn_blocking(move || insights.list())
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )
    }
    #[tool(
        description = "Confirmation required: analyze one discovered bugreport in memory and list its seven insight artifacts. May hash the whole ZIP and read bounded source content. Retry identical arguments with confirmation token; no filesystem writes."
    )]
    async fn list_insights(
        &self,
        Parameters(p): Parameters<crate::insights::ListInsightsParams>,
    ) -> Result<CallToolResult, McpError> {
        let insights = self.insights.clone();
        into_result(
            tokio::task::spawn_blocking(move || insights.list_insights(p))
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )
    }
    #[tool(
        description = "Confirmation required: obtain a UTF-8 page of a fixed insight artifact. Analysis runs in memory if uncached. Default 16384 text bytes, maximum 65536. Follow next_byte_start with fresh confirmation on each page. No filesystem writes."
    )]
    async fn read_insight(
        &self,
        Parameters(p): Parameters<crate::insights::ReadInsightParams>,
    ) -> Result<CallToolResult, McpError> {
        let insights = self.insights.clone();
        into_result(
            tokio::task::spawn_blocking(move || insights.read_insight(p))
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?,
        )
    }
}

fn tool_params<T: serde::Serialize>(params: T) -> Result<Value, McpError> {
    serde_json::to_value(params).map_err(|error| {
        McpError::invalid_params(format!("cannot re-encode parameters: {error}"), None)
    })
}

impl rmcp::ServerHandler for InsightServer {
    async fn list_tools(
        &self,
        _: Option<rmcp::model::PaginatedRequestParams>,
        _: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListToolsResult, McpError> {
        Ok(rmcp::model::ListToolsResult::with_all_items(
            Self::tool_router().list_all(),
        ))
    }
    fn get_tool(&self, name: &str) -> Option<rmcp::model::Tool> {
        Self::tool_router().get(name).cloned()
    }
    async fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::CallToolResponse, McpError> {
        if Self::tool_router().get(&request.name).is_some() {
            let call = rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            Self::tool_router().call(call).await
        } else {
            Err(McpError::invalid_params(
                "Unknown tool; use tools/list for this server's capabilities",
                None,
            ))
        }
    }
    async fn list_resources(
        &self,
        request: Option<rmcp::model::PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ListResourcesResult, McpError> {
        if let Some(archive) = &self.archive {
            rmcp::ServerHandler::list_resources(archive, request, context).await
        } else {
            Ok(rmcp::model::ListResourcesResult::with_all_items(vec![]))
        }
    }
    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<rmcp::model::ReadResourceResponse, McpError> {
        if let Some(archive) = &self.archive {
            rmcp::ServerHandler::read_resource(archive, request, context).await
        } else {
            Err(McpError::resource_not_found(
                "Directory mode exposes insights through tools; no bound archive resource",
                None,
            ))
        }
    }
    fn get_info(&self) -> rmcp::model::ServerConfig {
        let capabilities = rmcp::model::ServerCapabilities::builder().enable_tools();
        let capabilities = if self.archive.is_some() {
            capabilities.enable_resources().build()
        } else {
            capabilities.build()
        };
        rmcp::model::ServerConfig::new(capabilities)
            .with_server_info(rmcp::model::Implementation::new("bugreport-insight",env!("CARGO_PKG_VERSION")))
            .with_instructions(format!("Insight mode: start with list_bugreports; use its exact basename as bugreport. Directory scope is immediate regular ZIP files only, never parents/subdirectories/symlinks. The seven archive tools are available; pass `bugreport` from discovery when more than one report is present. Omission selects a report only when complete discovery finds exactly one. Each archive result identifies its selected bugreport; entry IDs are local to that report. Grep continuations must retain the same bugreport and cursor. list_insights generates seven artifacts in memory; read_insight pages them. Both require per-request confirmation: retry identical args with confirmation_token from error.data as confirmation only after accepting the analysis cost. Every page requires a new token. Follow next_byte_start, inspect truncation/evidence gaps; absence does not imply device health. No output files are written. For a bound file, the optional bugreport must match the bound ZIP basename and the existing read-only resources remain available with their original confirmations.{}", if self.archive.is_some() { format!("\n{INSTRUCTIONS}") } else { String::new() }))
    }
}
