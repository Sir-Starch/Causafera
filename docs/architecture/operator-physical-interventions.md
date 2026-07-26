# Operator Physical and Mana Interventions

**Status:** Proposed future architecture. **Not implemented.**

## Direction and boundary

Future operator intervention is a constrained, typed language for composing physical and mana
primitives that the simulation actually supports. The operator specifies causes or source terms;
simulation domains calculate consequences. It is not a catalog of semantic commands such as
destroy building, heal agent, reward worship, or make agent believe.

Conceptual source categories include field sources, boundary conditions, force density, impulses,
energy deposition, charge distributions, matter injection/removal, material-property changes,
reaction initiation, signal emission, and mana excitation/source terms. These are categories,
not binding APIs.

## Supported primitives, not arbitrary code

A future expression may carry unit-checked numbers, scalar/vector quantities, spatial regions,
temporal profiles, bounded functions, explicitly permitted deterministic seeded stochastic
components, operator-visible physical targets, and compositions of supported domain primitives.
Dimensional/domain/resource validation rejects kilograms assigned to temperature, NaN, infinity,
unsupported dimensions, unbounded sources, and unavailable domains before authoritative execution.

This is not unrestricted Python, Lua, Rust, shell, or any general-purpose code with RuntimeState
access. The authoritative environment exposes no filesystem/network/process/memory access,
unbounded loops or recursion, direct component mutation, scheduler bypass, or unvalidated Meta
Truth. Higher-level tools may generate expressions, but runtime receives only validated,
canonical, bounded representations:

operator expression → parsed typed AST → dimensional/domain validation → resource validation →
canonicalization → supported domain proposals → scheduler → authoritative commit → provenance →
physical/mana consequences

Composing supported primitives does not mean the engine can understand every imaginable formula.
An expression can only name represented quantities and laws: naming pressure, conductivity,
combustion, or fracture cannot create their full consequences when those models are absent.

## Lightning example

Within supported models, an operator could specify a target region, electric potential difference
or field source, ionized conductive path, conductivity, duration, and energy/power budget. A
future supported expression might relate E = -grad(phi), J = sigma E, and q = J dot E.
Implemented domains would then calculate any supported current, heating, material damage,
ignition, pressure/acoustic effects, biological effects, and mana coupling.

The operator does not command “destroy this building.” The result depends on represented
material, moisture, geometry, conductivity, grounding, nearby structures, and implemented thermal
or damage models. Causafera does not currently implement complete electrodynamics, combustion,
fracture, or a general physical solver.

## Deterministic authoritative intervention

External intervention never mutates RuntimeState directly. A committed intervention must preserve
enough immutable data for replay and save/resume, potentially canonical expression/AST and hash,
tick, region, units, budgets, deterministic seed, schema/compiler compatibility identifiers, and
proposal lineage. Every authoritative result has causal provenance.

Operator provenance may say an external intervention occurred, but that Meta Truth must not become
ordinary world-observer or Explanation output. The world sees the causal chain:

electric field increases → air becomes conductive → current flows → material heats → structure
ignites/fractures → light/sound/heat/damage propagate → agents perceive bounded signals → agents
form subjective explanations

Agents might call it weather, magic, divine punishment, technology, coincidence, fraud, or an
unknown process. The engine does not insert an interpretation into cognition.

Later convenience commands such as strike-lightning may compile to the same validated electric
source, conductive path, spatial profile, pulse, and energy budget. They must not bypass
proposal/commit.

Interventions need explicit resource limits for energy, mass, charge, momentum, volume, duration,
and mana. Whether they create quantities outside the universe, exchange an operator reservoir,
redistribute in-world quantities, or use a simulation-specific policy remains deliberately open.

## Beliefs and semantics

Direct rewrites of beliefs, loyalty, interpretation, memories, subjective scenes, or religious
identity are outside physical intervention. Normal influence is:

physical or mana event → bounded perception → memory/communication → subjective interpretation →
belief change

Neither mana nor physics receives the operator's semantic intention.

## Current mana compatibility

The current implementation is a bounded, dense, local cubic fixed-point scalar ManaField. It
consumes canonical, bounded physical-pattern samples; responds to recurrence, periodicity,
synchronization, repeated placement, and magnitude; then applies deterministic six-neighbour
diffusion, decay, and saturation. It uses integer arithmetic and canonical order. It is not a
continuous field, arbitrary equation solver, universal physical substrate, or general material
model.

The completed actor/material/mana slice proves one production-path loop:

actor action → durable chart-qualified material-surface contact → measurable repeated material
pattern → mana response → traced material-surface condition change → range-limited physical signal
→ bounded perception → subjective scene → later action → persisted/replayed causal history and
read-only reconstruction

It includes production bootstrap, Action/Physics/Mana proposal-commit phases, provenance,
snapshot save/resume, deterministic replay, bounded observer projection, typed Explanation claims,
and negative controls for disabled mana, absent repetition, and suppressed signal access. It does
not prove broader material physics, arbitrary mana effects, a continuous field, or any claim of
scale. See [Mana topology](../world/mana-topology.md).

Mana is World Ground Truth when represented in authoritative state, never engine Meta Truth,
archive, operator intent, administrator command system, semantic oracle, or LLM layer. It reacts
only through represented carriers. Today there are two concrete carriers, and the difference between
them matters: a changed chart-qualified material-surface condition/contact pattern, which emits
because something happened, and the standing terrain structure of each active chunk, which emits
because something is there. Neither is an operator channel — the second is the world's own relief
and surface material, projected onto the mana lattice. The wider field model allows represented
structure, geometry, repetition, timing, spatial organization, and physical signals; terrain is the
geometric producer of that list. See `plans/terrain-carrier-participation.md`. It never receives
temple, worship, sincerity, priest, sacrifice, or divine-intent facts. A world may call repeated
movement plus persistent geometry/material arrangement/acoustic frequency a ritual, but mana reacts
only to represented carriers.

Future mana primitives may include illustrative names such as ManaFieldSource, excitation,
redistribution, boundary condition, spatial profile, temporal pulse, frequency/phase input, or
coupling parameter only when the model supports them. They enter only as:

external mana intervention → validated proposal → scheduler → authoritative mana transition →
physical coupling → provenance → persistence/replay

Operator policy may inspect Meta Truth to decide when/where/how much source to introduce. Mana
dynamics process the source by in-world rules and never receive that semantic policy. Whether an
operator creates, exchanges, or redistributes mana remains open.

## Compatibility constraints

Future sources must preserve deterministic scheduling, proposal/commit, snapshots/save-resume,
replay, and causal provenance. They must not grant direct RuntimeState mutation or make current
product code depend on this language. Current identifiers and mana abstractions must not be
redesigned merely to anticipate it.
