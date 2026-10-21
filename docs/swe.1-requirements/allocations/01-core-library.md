# Core library requirement view

Allocation of [committed requirements](../02-software-requirements.md).
[Traceability](../../traceability.md) owns allocation; [conventions](../../conventions.md) define notation.
Records allocation metadata without proving verification; canonical requirements and traceability own obligations.

| Requirement | Kind | Allocated elements | Risk | Verification intent |
| --- | --- | --- | --- | --- |
| [REQ-GEN-01](../02-software-requirements.md#req-gen-01) | requirement | `zip_reader` | high | test |
| [REQ-GEN-02](../02-software-requirements.md#req-gen-02) | performanceRequirement | `zip_reader` | high | metadata boundary and actual-byte read-path tests |
| [REQ-LIB-01](../02-software-requirements.md#req-lib-01) | designConstraint | `core_library` | medium | inspection |

| Element alias | Type | Responsibility |
| --- | --- | --- |
| `zip_reader` | module | Read-only, size-limited ZIP access |
| `core_library` | module | Reusable archive-access library |
