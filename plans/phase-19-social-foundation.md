# Phase 19 Social Networks and Organizations

## Goal

Implement bounded, deterministic, trace-backed social-network and collective-record contracts without creating an organization brain or treating organization, law, or contract semantics as objective truth.

## Context

Phase 18 established causal resolution. Phase 19 requires distributed records for relationships, membership/roles, communication, authority, property claims, rules, practices, and agreements. Existing society documents describe these as historical social constructions, while the current code has only an opaque `OrganizationId`.

## Relevant invariants

INV-006, INV-014, INV-015, INV-016, INV-017, INV-019, INV-024, INV-027, and INV-033.

## Proposed architecture

Add a bounded, canonically ordered `SocialState` in `causafera-domains`. Store each carrier as an independent trace-backed record: directed agent relations, role assignments, communication links, authority grants, property claims, institutional rules, practice associations, and attested agreements. Opaque schema/scope/channel IDs carry no built-in meaning. Construction validates references and canonicalizes input. No aggregate object decides, believes, perceives, or acts.

Rules and agreements are records of social claims: rule records reference physical documents and authority grants; agreements reference a physical document and opaque parties. Neither record makes a law universally active, guarantees interpretation, or enforces an outcome.

## Non-goals

Governance simulation, legal adjudication, semantic role or relation taxonomies, organization cognition, shared omniscient knowledge, economy, ownership transfer, enforcement, observer protocol, persistence, and scale claims.

## Implementation stages

1. Add opaque social carrier identifiers and the distributed state contracts.
2. Validate bounds, uniqueness, references, canonical ordering, and causal traces.
3. Test order independence, no-organization-brain structure, rule/agreement interpretation boundaries, and invalid references.
4. Accept a social RFC and update TODO, roadmap, ontology, society docs, changelog, and plan registry.

## Verification

Run workspace tests, strict clippy, formatting and diff checks, architectural searches for semantic strings/enums, and refresh the code knowledge graph.

## Determinism and performance

No RNG, floats, locale, system time, or unordered containers. Every collection is capped and sorted by typed numeric identity; lookups use binary search. No throughput claim is made until benchmarked.

## Progress

- [x] Distributed social contracts implemented.
- [x] Validation and tests implemented.
- [x] Documentation and phase tracking updated.
- [x] Full verification passes.
