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

This bounded path includes a causally bootstrapped promoted actor, physical perception, subjective
scene construction, and later action. It does not establish a broad resident population model,
language, economy, city growth, historical synthesis adapters, or social emergence. Those domain
libraries remain available foundations, but inventing placeholder populations would violate the
roadmap constraints.
