# 0002 — Seven tools and entry identity

Status: Accepted

## Context and alternatives

An agent needs identity, listing, path filtering, type filtering, metadata, bounded reads and content search.
Combining three discovery routes behind a mode/union would require parsing unrelated result shapes.
Path addressing makes duplicate/lossy names ambiguous and complicates traversal avoidance.

## Decision and consequences

Keep seven named capabilities with typed input objects.
Address entries by central-directory EntryId; display paths and allow discovery filters.
Archive discovery remains separate from content reads; these seven archive handlers do not run analyzers.
Combined `--mcp --insight` adds three distinct insight tools under REQ-MCP-13 without merging their contracts into archive handlers.
Locators support follow-up reads, not seekable decompression.
Flat line/byte fields keep read_entry's schema a root object; validation requires exactly one complete range pair.

Current wire contract: [MCP design](../swe.3-detailed-design/03-mcp-server.md).
[Requirements](../swe.1-requirements/02-software-requirements.md#mcp-tools) own obligations and shared bounds.
