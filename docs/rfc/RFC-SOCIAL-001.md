# RFC-SOCIAL-001: Distributed Social and Institutional Records

**Status:** Accepted

## Summary

Represent social structure as bounded, trace-backed records distributed across agents, links, documents, practices, claims, and assignments. An `OrganizationId` groups records but does not identify a cognitive super-agent.

## Objective carrier boundary

The engine may authoritatively record that an assignment, directed relation, communication path, authority grant, property claim, document association, practice association, or attested agreement exists. Every record has opaque numeric schemas or references and causal provenance. These records are evidence of performed social acts and persistent artifacts; they do not establish a single correct social interpretation.

## No organization brain

Organizations have no perception, attention, concepts, beliefs, intent, memory, or decision method. Behavior remains attributable to agents and physical/informational carriers. Organization-level analytics must be derived read-only from the distributed records.

## Primitive versus emergent

Primitive bookkeeping includes typed identity, directed linkage, numeric strength/capacity/weight, time, physical document/practice references, and traces. The meanings of a relation, role, authority scope, communication channel, rule, property status, organization, or agreement remain historically constructed. Opaque IDs must not become a hidden semantic enum.

## Rules and agreements

An `InstitutionalRule` records a source document, separate interpretation and precedent documents, supporting authority grants, and provenance. It does not contain `active: bool`, universal jurisdiction, or automatic enforcement.

An `AttestedAgreement` records a physical text, opaque parties and witnesses, supporting authority, time, and provenance. It does not guarantee shared interpretation, performance, validity, or magical enforcement.

## Determinism and performance

All collections have hard caps and canonical numeric ordering. Construction rejects duplicate identities, duplicate references, missing organizations, missing roles or grants, self-relations, and cross-organization authority references. Lookups use sorted vectors and binary search. No RNG, floating point, unordered traversal, or scale claim is introduced.

## Deferred work

Lifecycle mutation through scheduler proposals, communication delivery, governance, enforcement, ownership transfer, shared-knowledge models, resolution aggregation, observer projection, persistence, economy, legal adjudication, and benchmarks remain future work.

## Decision log

- 2026-07-12: Accept distributed trace-backed records instead of a mutable organization aggregate with behavior.
- 2026-07-12: Model rules and agreements as contestable document-backed claims, never universally active truth.
- 2026-07-12: Keep relation, role, channel, and authority meaning behind opaque IDs.
