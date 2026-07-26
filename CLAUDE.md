@AGENTS.md

## Claude Code Tooling

For indexed source-code discovery and dependency tracing, use `codebase-memory-mcp` before broad `Grep`, `Glob`, repeated `Read`, or filesystem search through `Bash`.

Use filesystem search directly only for documentation, configuration, generated files, binaries, exact text matching, files outside the index, or when MCP results are insufficient.

Use MCP results to guide subsequent file reads. Do not invoke MCP merely as a formality.
