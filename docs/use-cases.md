# Use Case Diagram

The tool is used in several typical scenarios:

1. **Engineer debugging a bugreport** – Run `bugreport-insight <zip>` to get a concise Markdown report and JSON findings for quick analysis.
2. **Automated CI pipeline** – Invoke the CLI in a CI job to automatically generate artifacts for regression testing of Android builds.
3. **Third‑party tool integration** – Developers add custom subcommand binaries (e.g., `bugreport-insight‑custom`) that the dispatcher runs via `bugreport-insight <verb>`.
4. **Research and data mining** – Analysts batch‑process large collections of bugreports, parse the NDJSON output, and feed it into a data warehouse.

These use cases motivate the requirement for a fast, no‑side‑effects CLI and a modular scanner architecture.
