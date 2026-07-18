# Determinism Requirements

Causafera requires explicit deterministic execution for testing, replay, causal analysis, and reference experiments.

## Deterministic Random Streams

Randomness must come from explicit deterministic streams keyed by:

- world_seed
- simulation_time
- phase_id
- system_id
- entity_id
- operation_ordinal

Parallel scheduling order must not silently alter random outcomes. The scheduler must ensure that deterministic streams are consumed in a well-defined order regardless of parallelism.

## Deterministic Modes

### Strict deterministic mode

Used for:

- testing
- replay
- causal analysis
- reference experiments

In strict mode, identical inputs must produce identical outputs and identical state hashes across runs.

### Fast experimental mode

May explicitly use weaker reproducibility for performance exploration. Any weaker guarantees must be documented and must not be the default.

## Determinism and Parallelism

Parallel execution must preserve deterministic outcomes. Systems should generally use:

```text
READ
→ PROPOSE
→ REDUCE
→ COMMIT
```

This pattern ensures that parallel reads do not conflict, proposals are collected deterministically, reduction applies rules in a fixed order, and commits are atomic.

Avoid pervasive `Arc<Mutex<T>>` in domain code. Mutex contention introduces non-deterministic scheduling effects.

## Determinism and Language

A key determinism test: identical simulation inputs executed with different observer UI locales must produce identical canonical simulation state hashes. This verifies that no human language has leaked into authoritative state.

## Determinism and RNG

Preserve deterministic RNG rules across all phases. Do not use system randomness, timestamps, or pointer addresses as entropy sources in strict mode.

## Determinism Checklist

Before claiming a system is deterministic, verify:

- [ ] Random streams are explicitly seeded and keyed
- [ ] Parallel execution does not alter stream consumption order
- [ ] No system time or hardware entropy is used
- [ ] No pointer-dependent hashing or ordering
- [ ] Floating point operations are reproducible across runs
- [ ] Observer locale does not affect state hash
- [ ] Same seed produces same state hash on same hardware
- [ ] State hash is computed from canonical representation
