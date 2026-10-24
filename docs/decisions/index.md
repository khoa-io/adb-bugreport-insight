# Decisions

Records explain committed design decisions.
SWE.1–3 own current contracts; each decision here explains an active constraint and its tradeoffs.

- [0001: Separate committed archive access from deferred analysis](./0001-committed-archive-scope.md)
- [0002: Seven tools and entry identity](./0002-seven-tools-and-entry-identity.md)
- [0003: Immutable index and independent readers](./0003-independent-readers.md)
- [0004: Lazy archive validation](./0004-lazy-archive-validation.md)
- [0005: CLI composes MCP and core](./0005-cli-composes-mcp-and-core.md)
- [0006: Successful results, errors and metadata](./0006-tool-response-scope.md)
- [0007: Emitted UTF-8 excerpt cap and source progress](./0007-emitted-utf8-excerpt-cap.md)
- [0008: Validated marker metadata](./0008-validated-marker-metadata.md)
- [0009: Client search budgets and cursor pagination](./0009-client-search-budgets-and-pagination.md)

- [0010: Empty results and explicit detection failures](./0010-empty-results-and-detection-failures.md)

- [0011: Unordered tool discovery](./0011-unordered-tool-discovery.md)

- [0012: Partial EOF reads and subtree boundaries](./0012-range-boundaries-and-subtree-filters.md)

- [0013: Confirmation before content IO](./0013-confirmation-before-content-io.md)
- [0014: Transport failures and process exits](./0014-transport-failures-and-process-exits.md)
- [0015: Report-scoped archive tool selection in directory sessions](./0015-directory-archive-tool-selection.md)

Record numbers are permanent.
Changed consequences do not authorize behavior changes; [requirements](../swe.1-requirements/02-software-requirements.md) own current obligations.
