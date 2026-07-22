# Causafera Candidate Planning Ledger

**Purpose:** Institutional memory for candidate development tranches. Re-evaluated after every accepted tranche against the new authoritative repository state. This is not a committed roadmap or fixed implementation order.

**Assessment baseline:** `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40`, post-acceptance of `plans/local-mana-material-surface-coupling.md`.

---

## Current selection

### Bounded Conserved Thermal Storage and Same-Chart Transfer

- **Ledger ID:** `ledger-2026-07-22-thermal-same-chart`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Revision 6 of `plans/conserved-thermal-energy-carrier.md` accepted 2026-07-23; baseline `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.
- **World capability added:** A deterministic, fixed-point thermal-energy carrier with finite reservoirs, intra-chunk conduction, and same-chart cross-chunk face transfer.
- **Status:** Accepted; implementation branch to be created from this commit.
- **Why selected:** Energy domain is at M1 and needs conserved transfer/storage to support future climate, material, and biological coupling. The slice is architecturally coherent: one state model (`ThermalFieldSet`), one causal loop (reservoir → cell → neighbor cells), one persistence boundary (section `0x000E`), one protocol/Explanation addition (conservation claim), and one verification strategy (exact conservation accounting).
- **Dependencies satisfied:** Mana field provides proven cross-chunk boundary-exchange pattern; scheduler supports registration in `Phase::Physics`; material-surface gate pattern shows how to add a future thermal gate.
- **Major risks:** Integer overflow, conservation violation from implementation bugs, observer/Explanation surface underestimated, scope creep into material coupling.
- **Re-evaluation trigger:** If implementation reveals that same-chart exchange is too costly or that a separate phase is unavoidable, revisit before acceptance.

---

## Serious candidates considered and not selected

### Cross-chart field propagation

- **Ledger ID:** `ledger-2026-07-22-reject-cross-chart-propagation`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Extend any spatial carrier (mana, thermal) across chart seams using registered world-geometry transforms.
- **Why not selected:** Cross-chart transforms and atlas adjacency are not yet implemented. Building them now would balloon the thermal tranche with geometry infrastructure.
- **Dependencies and blockers:** `WorldGeometrySchemaId` registry, chart seam transforms, atlas generation or hand-off, persistence of cross-chart state.
- **Major risks:** Coordinate transform bugs, seam artifacts, non-deterministic ordering across charts, memory blow-up from overlapping chart volumes.
- **Conditions triggering re-evaluation:** Same-chart thermal carrier is stable; world-geometry schema is accepted; need arises for planetary-scale field coherence.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Terrain as a dynamic carrier

- **Ledger ID:** `ledger-2026-07-22-reject-dynamic-terrain`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Mutable terrain state that responds to physical processes (erosion, deposition, excavation, thermal damage).
- **Why not selected:** `TerrainChunk` is currently static after bootstrap. Making terrain dynamic requires new scheduler phases, mutation proposals, conservation of material, and paging/sparse storage.
- **Dependencies and blockers:** Mutable terrain runtime loop; material economy integration; spatial promotion/demotion under RFC-RES-001; production terrain synthesis algorithm.
- **Major risks:** Memory bloat from dynamic dense height fields, serialization cost, loss of deterministic generation provenance, scope overlap with geology/hydrology/climate.
- **Conditions triggering re-evaluation:** Thermal or hydrological carriers need to mutate terrain; production bootstrap produces real terrain; climate demands mutable albedo/roughness.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Richer material responses to thermal exposure

- **Ledger ID:** `ledger-2026-07-22-reject-material-thermal-response`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Material surfaces accumulate thermal exposure, undergo reversible/irreversible changes, or transition phase.
- **Why not selected:** The original `MaterialSurfaceThermalGate` proposal lacked a physically meaningful response rule. Before coupling, the material model needs heat capacity, thermal history, and damage/phase semantics.
- **Dependencies and blockers:** Stable thermal carrier; material-specific heat capacity and response catalogue; provenance for material property changes; observer/Explanation for material thermal history.
- **Major risks:** Semantic shortcut ("condition +1"), infinite state-toggle loops, mass/energy conservation bugs, overlap with mana gate abstraction.
- **Conditions triggering re-evaluation:** Thermal carrier is accepted; a concrete material response model is designed (e.g., retained thermal exposure, expansion, damage accumulation, or phase transition).
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Geometry-based signal occlusion

- **Ledger ID:** `ledger-2026-07-22-reject-geometry-occlusion`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Physical access and perception respect line-of-sight, occlusion, and media attenuation based on real geometry rather than distance thresholds.
- **Why not selected:** Requires mature local 3D geometry, spatial indexing, and acoustic/optical propagation models. Current perception uses range/threshold acquisition.
- **Dependencies and blockers:** Local volumetric geometry representation; terrain/building/body mesh or signed-distance data; efficient deterministic ray/visibility queries; integration with `RFC-PERCEPT-001`.
- **Major risks:** Performance collapse from naive ray casting, geometric aliasing, agent omniscience bugs if occlusion is skipped.
- **Conditions triggering re-evaluation:** Perception needs to distinguish hidden vs visible targets; stealth, cover, or sound propagation become required.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Biological coupling to mana and thermal fields

- **Ledger ID:** `ledger-2026-07-22-reject-biological-field-coupling`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Organisms acquire, retain, and release field state through physical carriers (tissues, rhythms, structures) without intrinsic mana pools or magic classes.
- **Why not selected:** Physiology is a stub (`f32` placeholders); detailed biology state does not exist. Per `plans/biological-mana-coupling.md`, implementation is pending detailed biological state.
- **Dependencies and blockers:** Mature physiology, morphology, development, and heredity runtime; thermal carrier; stable mana field; practice/ritual execution.
- **Major risks:** Hidden scalar aptitude, ritual monoculture, direct belief-to-field coupling, non-conserved personal field state.
- **Conditions triggering re-evaluation:** Biology runtime reaches M2; thermal/mana carriers are stable; need for emergent practitioners or congenital retention.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Full climate system

- **Ledger ID:** `ledger-2026-07-22-reject-full-climate`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Atmosphere/energy state, transport, seasonality, and coupling to terrain, water, ecology, and biomes.
- **Why not selected:** Domain coverage matrix lists Climate at M0 (documentation only). A full climate system depends on hydrology, ecology, and mutable terrain, none of which are runtime yet.
- **Dependencies and blockers:** Thermal carrier; hydrology; mutable terrain; ecological state; long-timescale resolution; benchmarked long horizons.
- **Major risks:** Overwhelming scope, weak coupling to authoritative state, tendency to use scalar global temperature rather than local causal processes.
- **Conditions triggering re-evaluation:** Thermal and hydrological carriers are stable; agriculture, settlement viability, or biome dynamics become required.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

### Cross-domain thermal-economy coupling

- **Ledger ID:** `ledger-2026-07-22-reject-thermal-economy`
- **Date:** 2026-07-22
- **Author:** Sisyphus (OpenCode agent) on behalf of the Causafera design session
- **Source:** Assessment against `main` at `bca3ae383a15607ed268c6cd0aadb47d94fabd40` while selecting the thermal carrier tranche.
- **Capability:** Energy as a tradable, transformable resource (fuel, heat engines, smelting, insulation).
- **Why not selected:** Economy is at M1; production runtime and material transformation catalogue are missing. Thermal economy is a later layer on top of thermal carrier and material responses.
- **Dependencies and blockers:** Thermal carrier; material responses; production runtime; inventory/lot conservation; labor/tool contracts.
- **Major risks:** Energy becomes a semantic resource rather than a conserved physical carrier; conflation of heat with generic "fuel" items.
- **Conditions triggering re-evaluation:** Material thermal responses exist; production and economy reach M2.
- **Baseline assessed:** `bca3ae383a15607ed268c6cd0aadb47d94fabd40`.

---

## Earlier candidates no longer active

- **Global mana-total material gate** — superseded by `plans/local-mana-material-surface-coupling.md`.
- **Experiment-recipe mana source as operator API** — explicitly not an operator API; recipe source is an input control only.
- **Phase 27 optional narrative/LLM surface** — removed from numbered roadmap by Detailed Development rebaseline; remains behind terminal gate.

---

## Ledger maintenance rules

1. Every candidate entry must include a stable `Ledger ID`, `Date`, `Author`, and `Source`. The `Source` must identify the repository baseline (commit hash) and the document or review that produced the entry.
2. After every accepted ExecPlan, re-run candidate selection against the new authoritative state.
3. Update the "Current selection" section with the accepted tranche outcome.
4. For each remaining candidate, update dependencies/blockers, risks, and baseline if the assessment changes.
5. Do not create full ExecPlans for rejected candidates in this file; keep entries compact.
6. When a candidate is selected, move it to "Current selection" and create a dedicated ExecPlan.
