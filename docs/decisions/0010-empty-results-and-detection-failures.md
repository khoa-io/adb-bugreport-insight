# 0010 — Empty results and explicit detection failures

Status: Accepted

## Context

Collection misses and unreadable content require distinct outcomes.
A zero-match array must not conceal failed classification, and a capped output must not claim a completed empty scope.
Direct addressed-entry errors retain their separate domain-error channel.

## Decision

Completed zero-match collection queries use status empty with unchanged tool-specific data shape and the mandatory four bounding fields.
This applies to list_entries, find_entries and find_by_type, alongside existing grep rules.
A zero-sized output limit that withholds known matches is partial Ok, never Empty.
Collection query failures preserve successful matches and report partial Ok with explicit failures and incomplete coverage, including when all file classifications fail.
Direct addressed-entry requests retain domain errors with entry context; unreadable and not_found do not become successful collection statuses.
Protocol/parameter failures remain on their rejection channels.

find_by_type returns matches, failures, omitted_matches, omitted_failures and coverage on every successful result, including Empty.
Each failure retains entry_id, display path and the mapped structured error code/message/data.
Return at most limit matching records (default/max 200) and at most 200 failure records independently; all failures are counted even when details are capped.
Envelope omitted_count equals omitted_matches plus omitted_failures, counting observed withheld records rather than unknown matches in unreadable content.
Record omission uses item_cap; otherwise failed classification uses skipped_content with truncated true and an explanatory note.
Neither a clipped failure list nor a zero-match result may conceal that classification coverage is incomplete.

Coverage includes entries_in_scope (files only), classified_entries, failed_entries, skipped_directories and coverage_complete.
Every file is attempted; directories have no file content to classify, are excluded from file scope and counted separately.
coverage_complete is true only when every file's promised detection sample was successfully classified; capped returned records do not invalidate that sample coverage.
This is not full-file validation or proof of every file's true format: each successful match retains its existing detection basis and sample size.
Unknown is a successfully obtained classification with no recognized type, never a substitute for read failure.
All-failed collections remain partial Ok, allowing discovery and direct-entry follow-up without losing archive/session access.

## Consequences and evidence

No new status or tool is introduced, and grep pagination/direct-entry errors remain unchanged.
Bounded failure details let clients distinguish confirmed non-matches from files whose type is unknown because their bytes could not be inspected.
Clients must inspect coverage, failures and omission counts rather than infer completeness from matches alone.
[Requirements](../swe.1-requirements/02-software-requirements.md#req-mcp-08), [MCP contract](../swe.3-detailed-design/03-mcp-server.md#collection-outcomes-and-detection-coverage) and [handlers](../../crates/mcp/src/tools/handlers.rs) own the accepted behavior.
[Executed verification](../../verification/results/empty-and-detection-outcomes.md) maps bounded failures, empty/capped outcomes, sample coverage and direct errors to component and real CLI checks.
