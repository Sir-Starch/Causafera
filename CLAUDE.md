@AGENTS.md

## Claude Code adapter

[`AGENTS.md`](AGENTS.md) is the canonical agent guidance for this repository. Do not duplicate or
restate it here. This file covers only Claude Code specifics.

- This repository does not require any external orchestration framework. Do not invoke
  framework-specific agents, slash commands, or control files unless the current user asks for them
  in this session.
- Claude memory, previous-session summaries, and compacted context are navigation hints. They are not
  authoritative; verify against current source, tests, and the canonical documents before acting.
- `codebase-memory-mcp` is optional. Load it through ToolSearch when a question is genuinely
  structural — architecture, call paths, relationships, or change-impact analysis.
- Use Grep, Glob, and focused Read for exact identifiers, TODO IDs, filenames, error strings,
  configuration, documentation, and literals.
- Do not repeat the same discovery through both MCP and filesystem search without a concrete
  verification reason.
- Use plan mode for ambiguous or multi-stage work. Canonical ExecPlans under `plans/` remain the
  authoritative plan of record; plan mode does not replace them.
- After a resume or a context compaction, re-read the active plan's current Progress and Decision Log
  sections and inspect `git status` before continuing.
