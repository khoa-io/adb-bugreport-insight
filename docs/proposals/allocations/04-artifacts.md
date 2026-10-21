# Deferred allocation view — batch artifact extensions

Single-report artifact generation is committed under `REQ-INSIGHT-01` through `REQ-INSIGHT-05`.
This page tracks only the remaining batch manifest, NDJSON, and session-segmentation proposal.

No committed scope depends on this batch extension.

Definitions: [deferred requirements](../02-requirements.md).
Notation: [conventions](../../conventions.md).

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [DRAFT-REQ-GEN-04](../02-requirements.md#draft-req-gen-04) | requirement | `batch_writer` | medium | test after schema definition |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `batch_writer` | module | Manifest, NDJSON, and cross-report session segmentation |

The seven single-report artifact schemas are maintained in [`../artifact-schemas/`](../artifact-schemas/index.md).

Field-level schemas for these artifacts are in [`../draft/artifact-schemas/`](../artifact-schemas/index.md).
