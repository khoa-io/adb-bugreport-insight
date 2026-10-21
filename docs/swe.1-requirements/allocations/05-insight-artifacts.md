# Committed requirement view — static insight

`docs/traceability.md` owns the requirement-to-architecture join.
This table summarizes ownership and verification intent for `UC-03`.

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [REQ-INSIGHT-01](../02-software-requirements.md#req-insight-01) | interfaceRequirement | `ARCH-CLI-INSIGHT` | medium | process test |
| [REQ-INSIGHT-02](../02-software-requirements.md#req-insight-02) | designConstraint | `ARCH-ARCHIVE`, `ARCH-INSIGHT-WRITER` | high | immutability and atomic-publication tests |
| [REQ-INSIGHT-03](../02-software-requirements.md#req-insight-03) | interfaceRequirement | `ARCH-INSIGHT-WRITER` | medium | artifact-set and parse tests |
| [REQ-INSIGHT-04](../02-software-requirements.md#req-insight-04) | requirement | `ARCH-INSIGHT-ANALYZERS`, `ARCH-INSIGHT-WRITER` | high | schema and domain acceptance tests |
| [REQ-INSIGHT-05](../02-software-requirements.md#req-insight-05) | performanceRequirement | `ARCH-INSIGHT-WRITER`, `ARCH-INSIGHT-ANALYZERS` | high | bounds and hostile-input tests |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `insight_cli` | interface | Selects insight mode and reports process outcomes. |
| `insight_analyzers` | module | Derives attributable system, apps, ANR, crash, memory, and CPU evidence. |
| `artifact_writer` | module | Validates and publishes the seven specified artifacts. |
