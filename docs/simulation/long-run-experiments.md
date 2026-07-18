# Long-Run Experiments

Phase 24 provides the first executable deterministic Causafera simulation path. It is deliberately a small causal field experiment rather than a fabricated city or population.

## Executable loop

Each tick runs through the existing scheduler and commits:

```text
physical repeated carrier
→ Ground Truth event and trace
→ fixed-point mana proposal
→ per-cell Ground Truth commits
→ opaque trace-backed resolution signal
→ resolution proposal and commit
```

Runtime failures are captured by the phase systems and returned from `Runtime::tick`. A canonical 256-bit state fingerprint covers simulation time, physical counters, mana state, resolution state, and the complete committed event/effect ancestry.

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

## Bounds and deferred scope

Runs, checkpoints, field dimensions, pattern batches, resolution signals, and research observations are capped. Provenance grows with accepted state transitions, so substantially larger workloads require TODO-PERF-001 benchmarks and later persistence/compaction work.

This phase does not simulate residents, cognition, language, economy, city growth, historical synthesis adapters, or social emergence. Those domain libraries remain available foundations, but inventing placeholder populations would violate the roadmap constraints.
