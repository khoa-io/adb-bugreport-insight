# Open qualification obligations

The [software assessment](assessment.md) and dated runtime results define demonstrated scope.
The obligations below remain open; a completed report or candidate bundle does not close them.

## Transport exits

REQ-CLI-03 requires response-pipe failures to exit with the transport diagnostic and exit code 1.
The Unix monitor implements this; the native non-Unix monitor does not yet detect response-pipe closure.
Implement equivalent native handling and execute it on a non-Unix target, including clean stdin EOF, broken stdout during a response, diagnostics and process exit behavior.
Actual SDK receive/send-task failures already propagate through the production supervisor; [executed evidence](../../verification/results/server-task-failures.md) covers those paths independently.
No native non-Unix runner was available for the recorded assessment; unconditional cross-platform conformance remains unestablished.
See the [lifecycle design](../swe.3-detailed-design/03-mcp-server.md#lifecycle-and-exit-behavior) and [exit ADR](../decisions/0014-transport-failures-and-process-exits.md).

## AOSP runtime

The [agent-led investigation](../../verification/results/agent-led-aosp-crash-investigation.md) supplies Java/display/HAL candidates with scoped executed checks.
The [persistent display correction](../../verification/results/persistent-fixed-avd.md) removes the original inset loop on the matching automotive runtime; framework and native HAL candidates remain uninstalled.
The [longer fresh capture](../../verification/results/fixed-avd-capture-analysis.md) reproduces other Java/native/SystemUI failures and supersedes overall-stability interpretations of earlier 55-second successes.

Remaining acceptance includes:

- Establish exact or justified compatible source/symbol provenance for the matching automotive runtime.
- Verify ClusterHome listener registration/removal across stop/reinitialization and display transitions, including actual framework/overlay integration and panel rendering.
- Build/link the actual HAL and exercise real Binder/H4 delayed, missing and duplicate reset responses, reconnects and historical crash causality.
- Investigate watchdog VHAL null/connection lifecycle and blocking audio-policy/service recovery.
- Repeat cold boot, rotation/restarts and repaired-device capture; compare MCP evidence and HCI traces with the original incidents.

Source-tag similarity, isolated callbacks and translation-unit compilation do not establish exact-build integration or historical cause elimination.
