# 0014 — Transport failures and process exits

Status: Accepted

## Context

A clean client disconnect is a normal shutdown, while archive-open, transport startup, response-channel and server-task failures need distinct diagnostics and exit behavior. Individual malformed or failed requests must not terminate a usable session. The SDK may complete its service task without exposing every response-pipe write failure, so shutdown classification also needs output-disconnect detection.

## Decision

Keep archive-open, transport-start, transport IO, and server-task failures as distinct lifecycle outcomes. Report startup archive failures as archive failures; report transport and server-task failures with transport-specific stderr diagnostics. Return process exit 0 for ordinary stdin EOF, before or after initialization, and exit 1 when the stdio response channel fails or the server task fails. Preserve the serving loop after an individual invalid request when the protocol permits subsequent valid requests.

Because rmcp 3.5.0 may finish serving without propagating a response-pipe write failure, the accepted Unix implementation monitors stdout's pipe-disconnect state while the server runs, in addition to checking service-task completion. It stops and joins that monitor during normal shutdown; a monitor failure is a transport failure, not an archive failure. The monitor is limited to detecting a disconnected output descriptor; it does not replace error propagation from the MCP service or claim to detect every possible device-level output fault.

## Consequences and evidence

Process checks cover EOF before and after initialization, a closed stdout pipe during a response, server-task failure and invalid-request recovery. [Transport execution](../../verification/results/transport-exits.md) demonstrates the Unix response-pipe behavior. [SDK fault execution](../../verification/results/server-task-failures.md) demonstrates actual service/send-task panic propagation through the production supervisor and CLI typed-error mapping. The [lifecycle design](../swe.3-detailed-design/03-mcp-server.md#lifecycle-and-exit-behavior) owns implementation details under REQ-CLI-03/05 and REQ-MCP-10. [Native non-Unix qualification](../verification/03-open-qualification.md#transport-exits) remains open; the accepted failure-exit contract is unchanged.
