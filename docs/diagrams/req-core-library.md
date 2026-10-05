# Requirement diagram — core library

The archive-reading requirements and the one deferred scanning requirement. The reason the archive
half can ship ahead of the analyzers is visible here: `zip_reader` satisfies two committed
requirements and depends on nothing, while `scanner_framework` carries a `DRAFT-` ID because the only
requirement it serves is draft.

Index and notation: [`index.md`](./index.md).

```mermaid
requirementDiagram

    direction TB

    element zip_reader {
        type: "module"
        docRef: "Read-only, size-limited ZIP access"
    }

    element scanner_framework {
        type: "module"
        docRef: "Evidence scanners over logcat/dumpstate"
    }

    element core_library {
        type: "module"
        docRef: "Reusable archive-access library"
    }

    requirement read_only_zip {
        id: "REQ-GEN-01"
        text: "Open bugreport ZIP archives without extracting files to disk"
        risk: high
        verifymethod: test
    }

    performanceRequirement safety_limits {
        id: "REQ-GEN-02"
        text: "Refuse any archive exceeding 10000 entries, 256 MiB per entry, 1 GiB total, or 2 GiB archive size"
        risk: high
        verifymethod: test
    }

    requirement evidence_extraction {
        id: "DRAFT-REQ-GEN-03"
        text: "Scan logcat/dumpstate for ANR, crashes, memory pressure, and CPU contention"
        risk: high
        verifymethod: test
    }

    designConstraint library_distribution {
        id: "REQ-LIB-01"
        text: "Distribute the archive engine as a library so other Rust crates can depend on it"
        risk: medium
        verifymethod: inspection
    }

    zip_reader - satisfies -> read_only_zip
    zip_reader - satisfies -> safety_limits
    scanner_framework - satisfies -> evidence_extraction
    core_library - satisfies -> library_distribution
```

The four numeric limits are stated once, in
[`../requirements.md`](../requirements.md#general). This diagram repeats them in abbreviated form
only so the node is self-describing in isolation; the specification is the authority.
