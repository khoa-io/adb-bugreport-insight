# Agentic Coding Guidelines

## Toolchains
* Language: Rust 2024
* Build System: Cargo Workspaces
* Documentation: Markdown
  * UML Diagrams: separate PlantUML `.puml` sources and generated SVGs (UML notation)
  * Other diagrams: Mermaid

## Project General Instructions
* Before reading the code, read the [documentation index](./docs/index.md) first. They contain the context and reasoning behind the code, and are up-to-date with the codebase.
* Always update the documentation when you make changes to the codebase, so the documentation can be the source of truth for the codebase.
* The documentation is not for datetime record. Put datetime record to memory bank.
* There are specific roles for each kind of task, which are located in the `.agents/agents` folder. See [Specific Roles](#specific-roles) before picking a role to read.
* There are specific instructions for each kind of task, which are located in the `.agents/instructions` folder. See [Specific Instructions](#specific-instructions) before picking a instructions file to read.
* When receive a request from user, determine if it needs to go though ASPICE SWE or not.

## Documentation Diagrams
* Edit `.puml` sources; never hand-edit generated SVGs.
Embed adjacent SVGs with descriptive alt text and source links.
* Follow the [diagram guide](docs/guides/03-diagrams.md#diagram-authoring-and-markdown-preview) for the pinned renderer and hook setup.
* Regenerate output with `python3 scripts/diagrams.py render` and run `python3 scripts/diagrams.py check`.
* Use allocation tables linked to canonical requirements/traceability for requirement views.
* Preserve partial staging: the hook renders staged sources/includes/configuration; never replace it with a working-tree render plus `git add`.
* Preserve existing hooks and unrelated staged/unstaged work; do not bypass freshness failures.

## Specific Roles
* [Principal Software Engineer](./.agents/agents/principal-software-engineer.agent.md)
* [AI Team Dev](./.agents/agents/ai-team-dev.agent.md)
* [AOSP Team Dev](./.agents/agents/aosp-team-dev.agent.md)

## Specific Instructions
* [ASPICE SWE — lightweight adaptation](./.agents/instructions/aspice.instructions.md)
* [Agent Safety & Governance](./.agents/instructions/agent-safety.instructions.md)
* [Memory Bank](./.agents/instructions/memory-bank.instructions.md)
* [Rust](./.agents/instructions/rust.instructions.md)
* [Rust MCP Server](./.agents/instructions/rust-mcp-server.instructions.md)
* [Markdown](./.agents/instructions/markdown.instructions.md)
* [Agent Skills File Guidelines](./.agents/instructions/agent-skills.instructions.md)
* [Design Patterns for Object-Oriented Programming for Clean Code](./.agents/instructions/oop-design-patterns.instructions.md)
* [Prompts Guidelines](./.agents/instructions/prompt.instructions.md)
* [Self-explanatory Code Commenting](./.agents/instructions/self-explanatory-code-commenting.instructions.md)
