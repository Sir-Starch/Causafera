# Codebase Knowledge Graph

`codebase-memory-mcp` maintains and queries a semantic knowledge graph of the codebase. It is an
**optional convenience**, not a repository requirement: any equivalent structural tooling — LSP, call
hierarchy, another semantic index — satisfies the same need, and exact text search remains the right
tool for literals. See the code-discovery guidance in [`AGENTS.md`](/AGENTS.md). This page documents
how to use the graph when it is available.

## Priority Order

1. `search_graph` - Find functions, classes, routes, variables by pattern
2. `trace_path` - Trace who calls a function or what it calls
3. `get_code_snippet` - Read specific function/class source code
4. `query_graph` - Run Cypher queries for complex patterns
5. `get_architecture` - High-level project summary

## Guidelines

- Prefer a structural tool — this graph, LSP, or call hierarchy — for relationships, call paths, and
  change-impact analysis.
- Keep the graph up to date as new crates, modules, structures, traits, and routes are introduced, if
  you are using it.
- Keep queries narrow and bounded, and read current source before relying on an index result.

## When Text Search Is the Right Tool

Use grep/glob for:

- identifiers, TODO IDs, filenames, string literals, and error messages;
- config values;
- non-code files (Dockerfiles, shell scripts, configs, documentation);
- whenever graph results are stale or insufficient.

Do not run the same discovery through both mechanisms unless verifying a concrete uncertainty.

## Maintenance

Re-index the codebase when:

- significant new crates are added;
- major refactors change call graphs;
- new public APIs are introduced;
- before major architectural decisions.

## Related Documents

- `AGENTS.md` - Canonical agent guidelines, including tool-neutral code discovery
- `docs/development/contributing.md` - Contributing guidelines
