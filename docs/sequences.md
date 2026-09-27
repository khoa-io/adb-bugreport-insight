# Sequence Diagrams

## Processing Pipeline

```mermaid
flowchart TB
    A["BUGREPORT.zip"] --> B["open_archive()\n(core::archive)"]
    B --> C["validate_archive()\nentry‑name / size / count limits"]
    C --> D["read_primary_report()\n(core::source)"]
    D --> E["run scanners\n(anr, stacktrace, memory, cpu,\n+ unclaimed: watchdog/broadcast/activity)"]
    E --> F["prepare_output()\nrefuse non‑empty unless --force"]
    F --> G["render()\n(core::render)"]
    G --> H["sessions()\n(core::sessions)"]
    H --> I["write .BUGREPORT/insight/"]
    I --> J["README.md + manifest.json +\nfindings.json/ndjson + sessions.json"]

    style A fill:#dae8fc,stroke:#6c8ebf
    style B fill:#d5e8d4,stroke:#82b366
    style C fill:#d5e8d4,stroke:#82b366
    style D fill:#d5e8d4,stroke:#82b366
    style E fill:#d5e8d4,stroke:#82b366
    style F fill:#fff2cc,stroke:#d6b656
    style G fill:#d5e8d4,stroke:#82b366
    style H fill:#d5e8d4,stroke:#82b366
    style I fill:#fff2cc,stroke:#d6b656
    style J fill:#fff2cc,stroke:#d6b656
```

The pipeline proceeds left‑to‑right, reading the bugreport, validating it, selecting the primary text entry, scanning for findings, and finally rendering the output artifacts.
