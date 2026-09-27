# Agentic Coding Guidelines

## Toolchains
* Language: Rust 2024
* Build System: Cargo Workspaces
* Documentation: Markdown
  * Diagrams: Mermaid

## Project General Instructions
* Before reading the code, read the [documentation](./docs) first. They contain the context and reasoning behind the code, and are up-to-date with the codebase.
* Always update the documentation when you make changes to the codebase. The documentation is the source of truth for the codebase.
* There are specific roles for each kind of task, which are located in the `.agents/agents` folder. See [Specific Roles](#specific-roles) before picking a role to read.
* There are specific instructions for each kind of task, which are located in the `.agents/instructions` folder. See [Specific Instructions](#specific-instructions) before picking a instructions file to read.

## Specific Roles
* [Principal Software Engineer](./.agents/agents/principal-software-engineer.agent.md)
* [AI Team Dev](./.agents/agents/ai-team-dev.agent.md)
* [AOSP Team Dev](./.agents/agents/aosp-team-dev.agent.md)

## Specific Instructions
* [Agent Safety & Governance](./.agents/instructions/agent-safety.instructions.md)
* [Memory Bank](./.agents/instructions/memory-bank.instructions.md)
* [Rust](./.agents/instructions/rust.instructions.md)
* [Rust MCP Server](./.agents/instructions/rust-mcp-server.instructions.md)
* [Markdown](./.agents/instructions/markdown.instructions.md)
* [Agent Skills File Guidelines](./.agents/instructions/agent-skills.instructions.md)
* [Design Patterns for Object-Oriented Programming for Clean Code](./.agents/instructions/oop-design-patterns.instructions.md)
* [Prompts Guidelines](./.agents/instructions/prompt.instructions.md)
* [Self-explanatory Code Commenting](./.agents/instructions/self-explanatory-code-commenting.instructions.md)
