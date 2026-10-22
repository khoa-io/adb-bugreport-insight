# 0015 — Report-scoped archive tools in directory sessions

Status: Accepted

## Context

Combined directory mode previously exposed discovery and insight access but not the seven bounded archive tools.
Adding a session-wide selected report would make concurrent tool calls depend on hidden mutable state and could send an entry ID or grep cursor to the wrong archive.
Reopening an archive for every call would also discard the existing per-archive grep continuation state and could let a pathname replacement change the report between a confirmation preview and its retry.

## Decision

Expose the existing seven archive tools in directory mode and add an optional `bugreport` basename field to each archive-tool input object.
The field is normalized into every directory-mode request before confirmation, so confirmation tokens remain bound to the selected report.
Omission is accepted only when a complete catalog scan finds exactly one recognized report; multiple reports, no reports and incomplete discovery have explicit errors.
For a bound ZIP, omission continues to use the bound file and an optional selector must match its basename.
Successful directory archive results echo `data.bugreport`; `entry_id` values remain report-local, and grep continuation requires the same report selector.

The server caches at most eight per-report handler contexts, each retaining the admitted archive descriptor and its grep cursor state.
An idle context is eligible for eviction after 300 seconds, matching the confirmation and grep-cursor idle lifetimes.
Active calls keep a context alive, and a full active cache returns `archive_context_capacity` rather than evicting a context that may own a pending cursor or confirmation.
All five content tools across the directory session retain REQ-MCP-12's shared 64-token pool; each confirmation also binds to the process-local context generation so a token cannot authorize a reopened same-name archive.
Directory candidate access still uses the pinned directory and descriptor-relative no-follow open; no caller-supplied path is introduced.

## Consequences

Combined directory mode now has the same ten tool names as combined file mode, while ordinary archive mode remains the seven archive tools.
The selector is explicit per call rather than a mutable server selection, so concurrent calls cannot silently switch an archive's identity.
The finite context cap bounds retained indexes, file handles and grep state; the shared confirmation pool retains its existing 64-token capacity.
Existing archive safety, content confirmation, response bounds, result envelopes and directory confinement continue to apply.

[REQ-MCP-14](../swe.1-requirements/02-software-requirements.md#req-mcp-14), [architecture](../swe.2-architecture/02-components-and-dependencies.md#combined-mcp-and-insight-scope) and [MCP design](../swe.3-detailed-design/03-mcp-server.md#directory-archive-tools) own the current contract.
