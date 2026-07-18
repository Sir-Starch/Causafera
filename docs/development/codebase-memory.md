# Codebase Knowledge Graph

All developers and AI agents working on Causafera must utilize the `codebase-memory-mcp` toolset to maintain and query a semantic knowledge graph of the codebase.

## Priority Order

1. `search_graph` - Find functions, classes, routes, variables by pattern
2. `trace_path` - Trace who calls a function or what it calls
3. `get_code_snippet` - Read specific function/class source code
4. `query_graph` - Run Cypher queries for complex patterns
5. `get_architecture` - High-level project summary

## Rules

- Prefer `codebase-memory-mcp` tools over raw grep, glob, or file search for codebase navigation and structural analysis.
- Keep the knowledge graph up to date as new crates, modules, structures, traits, and routes are introduced.
- Use graph tools as the primary method for code discovery and dependency tracing.

## When to Fall Back

Use grep/glob only for:

- string literals and error messages;
- config values;
- non-code files (Dockerfiles, shell scripts, configs);
- when graph tools return insufficient results.

## Maintenance

Re-index the codebase when:

- significant new crates are added;
- major refactors change call graphs;
- new public APIs are introduced;
- before major architectural decisions.

## Related Documents

- `AGENTS.md` - Agent guidelines including codebase memory usage
- `docs/development/contributing.md` - Contributing guidelines
