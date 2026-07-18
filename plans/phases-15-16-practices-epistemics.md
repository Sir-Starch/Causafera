# Phases 15–16 Practices and Epistemic Carriers

## Goal

Implement bounded deterministic practice programs, socially constructed measurement systems, and physical document lineages as one causal-carrier batch.

## Context

Language now transmits subjective associations, but the workspace still has placeholder practices using English action strings and epistemic types using floating point and authoritative strings. Measurement procedures depend on executable practices, while documents preserve practice and measurement structure.

## Relevant invariants

INV-006, INV-008, INV-014, INV-016, INV-017, INV-018, INV-025, INV-027, and INV-030.

## Ontology domains affected

Practice, epistemics, language/writing, cognition, and future mana.

## Causal carriers affected

Imitated practices, measurement records, calibration ancestry, glyph sequences, and document-copy ancestry.

## Relevant documents

`docs/architecture/invariants.md`, `docs/ontology/causal-carriers.md`, `docs/ontology/primitive-vs-emergent.md`, epistemics subsystem documents, `RFC-PRACTICE-001`, and `RFC-EPI-001`.

## Current state

`causafera-domains::practices` stores an unbounded vector and English action strings. `causafera-epistemics` stores floating-point measurements with string units and documents with string media. Neither placeholder validates ancestry or bounds memory.

## Proposed architecture

Use opaque schema IDs and fixed-point quantities. A validated practice lineage owns a bounded instruction program with explicit conditions and forward branches. Execution consumes only caller-provided subjective numeric evidence and emits bounded action proposals and timing records. Mutation creates a child lineage and records the changed instruction.

Measurement systems own opaque quantity/unit identity, rational scale, precision, systematic uncertainty, and bounded calibration ancestry. Measurement deterministically transforms an accessible integer observation into a quantized result; it does not expose a Ground Truth value.

Documents are bounded physical mark sequences associated with opaque medium and writing-system IDs. Copying takes an explicit deterministic edit script and creates a child with recorded transformation provenance. Meaning remains listener/reader interpretation, not document state.

## Primitive vs emergent review

Program control flow, integer timing, physical marks, numeric scale, and ancestry are structural primitives. Named actions, ritual types, skills, professions, quantities such as “length”, unit names, document genres, and textual meaning remain emergent or observer glosses.

## Non-goals

Motor simulation, resource authorization, autonomous goals, full practice diffusion, instruments, experiments, science institutions, semantic writing categories, physical material degradation, and mana coupling.

## Implementation stages

1. Replace the practice placeholder with bounded validation, execution, lineage, and mutation.
2. Replace epistemics placeholders with fixed-point measurement/metrology and bounded physical document copying.
3. Add deterministic and boundary tests.
4. Accept both RFCs and update roadmap, TODO, ontology, subsystem documentation, changelog, and plan registry.

## Verification

Run workspace tests, strict clippy, formatting check, diff check, architecture searches for forbidden strings/floats/authoritative cognitive IDs, and refresh the code graph.

## Benchmark plan

No throughput claim is made. Hot collections have hard maxima; future benchmarks must measure practice executions per tick, calibration lookup, and document-copy cost before layout changes.

## Determinism impact

No internal RNG. Programs, evidence, edit scripts, and ancestry are canonically ordered and bounded. Identical inputs produce identical results.

## Memory impact

Practice instructions, execution emissions, calibration chains, glyphs, edits, and ancestry are bounded by public constants.

## Observer impact

Future read models may expose opaque lineage structure and confidence/uncertainty with provenance. No observer protocol changes are made.

## Explanation impact

Lineage and execution records provide structured support for later explanations. Human labels remain downstream.

## Persistence impact

No persistence format change. New state remains plain deterministic Rust data.

## Cross-domain effects

Future scheduler commits may consume action proposals. Cognition may supply subjective condition evidence. Language/writing may interpret glyphs. Mana may later consume only physical repetition and mark patterns, never meanings.

## Risks

- Opaque schemas could hide semantics in integration code; APIs and docs require observer-only glosses.
- Branches could become unbounded; only forward targets are accepted and execution has a hard step budget.
- A measurement could masquerade as truth; results explicitly retain observed input, quantization, and uncertainty.

## Documentation changes

Update practice/epistemics RFCs and subsystem/ontology status documents.

## TODO changes

Complete `TODO-PRACTICE-001`, `TODO-EPI-001`, and `TODO-LANG-004` if all acceptance tests pass.

## Decision log

- 2026-07-12: Batch Phases 15–16 because measurement procedures are practices and documents preserve their transmission.
- 2026-07-12: Keep execution proposal-only; scheduler-controlled authoritative mutation remains outside this phase.

## Progress

- [x] Practice representation, execution, and lineage implemented.
- [x] Measurement and calibration implemented.
- [x] Physical documents and deterministic copying implemented.
- [x] Tests and architectural checks pass.
- [x] RFCs, roadmap, TODO, and documentation updated.
