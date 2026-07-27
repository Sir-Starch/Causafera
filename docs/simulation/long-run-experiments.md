# Long-Run Experiments

Phase 24 provides the first executable deterministic Causafera simulation path. It is deliberately a small causal field experiment rather than a fabricated city or population.

## Executable loop

The bounded actor/material/mana path runs through the existing scheduler and commits:

```text
actor contact
→ chart-qualified material-surface transition and trace
→ canonical repeated physical samples
→ fixed-point mana proposal
→ per-cell Ground Truth commits
→ mana-mediated material-surface transition
→ range-limited physical signal
→ generic feature, subjective scene, and later action
```

Runtime failures are captured by the phase systems and returned from `Runtime::tick`. A canonical
256-bit state fingerprint covers simulation time, chart-qualified material-surface state and its
bounded transition history, mana state, resolution state, and the complete committed event/effect
ancestry.

## Laboratory workflow

`ExperimentRunner` executes bounded checkpointed runs. Strict replay executes the same plan twice and requires exact equality of deterministic results. The long-run suite compares:

- a control trajectory with continuous physical recurrence;
- an intervention trajectory with recurrence temporarily suppressed.

The suite requires the intervention to change the final canonical trajectory. This proves that the configured physical intervention has a causal effect; it does not prove emergence, intelligence, divinity, or a final attractor model.

## Commands

```text
cargo run -p causafera-cli -- doctor
cargo run -p causafera-cli -- run --seed 42 --ticks 1000
cargo run -p causafera-cli -- lab long-run --seed 42 --ticks 1000 \
  --checkpoint-interval 100 --suppression-from 400 --suppression-through 600
```

CLI names and rendered summaries are non-authoritative developer metadata. Wall-clock measurements are excluded from canonical replay results.

## Bounded material-surface diagnostic

The production `Runtime::new` benchmark harness was run twice in release mode with one promoted
actor, one sensor, bootstrap population eight, one chart-qualified material site, a four-tick
warm-up, and 32 measured scheduler ticks. Both runs produced 34 actor contacts and one
mana-to-material transition. Each produced a 160,989-byte snapshot, provenance growth of 734,
and a 15,834-byte bounded `world_chunks` response.

Observer-off ticks took 2,828,303 ns and 2,475,761 ns; equivalent bounded-query ticks took
2,398,500 ns and 2,707,732 ns. The resulting signed deltas were -429,803 ns and 231,971 ns.
Those short local observations deliberately do not estimate query overhead, throughput, memory
usage, or scale. The benchmark demonstrates only that the bounded production path exercised the
material loop and observer projection under its stated envelope.

## Bounds and deferred scope

Runs, checkpoints, field dimensions, pattern batches, resolution signals, and research observations are capped. Provenance grows with accepted state transitions, so substantially larger workloads require TODO-PERF-001 benchmarks and later persistence/compaction work.

`plans/performance-baseline-and-digest-cost.md` (Draft) measured the concrete mechanism behind this
caveat: `RuntimeState::snapshot`, called on every tick and every observer poll, recomputes two digests
from scratch every time rather than incrementally. `history_digest` re-scans the *entire* causal trace
store from tick 0; `physical_state_digest` re-scans, among other state, the unpruned `thermal_receipts`/
`thermal_conservation_receipts` maps, which also gain one new entry every tick with no eviction found
anywhere in the runtime. At a fixed, unchanging small workload, this made mean per-tick wall time grow
6.8x over 512 ticks, driven mainly by trace-store accumulation (`history_digest` alone rose from 9.7 ms
to 125.7 ms across the same span) with a smaller contribution from `physical_state_digest`'s own
unbounded thermal-receipt growth (4.8 ms to 7.7 ms) — a long-run experiment's cost is not currently
constant per tick, it grows over the run's own length, and by more than one mechanism.

The plan proposes making only `history_digest`'s trace-event scan incremental, without a schema
change: that input is written first in the digest sequence, with nothing but cheap bounded content
after it, so a running accumulator can be resumed and a small bounded tail appended each tick.
`physical_state_digest`'s thermal-receipt growth is measured and real but is **not** fixed by that
plan: the unbounded maps sit in the *middle* of its write sequence, with further arbitrarily-mutable
current-tick state written after them, so the same technique does not apply without reordering the
write sequence — which would itself change the digest's output and require a deliberate schema
migration this plan does not attempt. It is recorded as an open follow-up (retention/compaction of
thermal receipts, a reordered-and-versioned digest, or a composable digest primitive) rather than
closed by that plan.

This bounded path includes a causally bootstrapped promoted actor, physical perception, subjective
scene construction, and later action. It does not establish a broad resident population model,
language, economy, city growth, historical synthesis adapters, or social emergence. Those domain
libraries remain available foundations, but inventing placeholder populations would violate the
roadmap constraints.
