## Summary

Describe the bounded change and why it is needed. External contributions cannot be accepted until
the CLA acceptance workflow is configured.

## Evidence

List the failing-first proof, targeted validation, and real-surface verification. Include exact
commands and exit statuses; do not include credentials, private data, or unredacted local paths.

## Causal and architectural impact

- Authoritative state and owning scheduler phase:
- Physical or informational carriers:
- Provenance and Explanation impact:
- Determinism and RNG impact:
- Persistence and causal-resolution impact:
- Observer/UI impact:
- Performance evidence:
- Explicit non-goals:

Use `Not applicable` only when the change genuinely cannot affect that boundary.

## Checklist

- [ ] I read the required project documents in `CONTRIBUTING.md`.
- [ ] The change has an accepted ExecPlan if it is multi-stage or architectural.
- [ ] No semantic enum or human-language label is treated as authoritative simulation Ground Truth; ordinary UI labels, diagnostics, protocol discriminants, implementation enums, internal state machines, and non-authoritative explanation text remain allowed.
- [ ] No fixture/demo runtime path or digest-distance shortcut was introduced.
- [ ] Relevant Rust, frontend, audit, documentation, determinism, persistence, and benchmark checks passed.
- [ ] Documentation, TODOs, ADRs, RFCs, roadmap, and maturity evidence were updated where required.
- [ ] No credential, private data, generated build output, or unrelated change is included.
- [ ] CLA acceptance has been recorded separately if this is an external contribution.
