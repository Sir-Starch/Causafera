# Performance Philosophy

Ontopolis optimizes simulated causal complexity per wall-clock second.

## Success Criteria

A million inert agents are not success. What matters is:

- simulated days per wall-clock second;
- active updates (not total entities);
- perceptual features processed;
- concepts updated;
- utterances decoded;
- lexical associations updated;
- causal edges emitted;
- practice executions;
- resolution transitions.

## Resource Metrics

Also measure:

- peak RSS;
- bytes per persistent resident;
- observer overhead;
- Explanation Engine query latency.

## Benchmark Observer States

```text
no observer
idle observer
normal UI
heavy inspection
causal explanation query workload
```

## Design Principles

- Performance is architectural, not an afterthought.
- Scale claims require reproducible benchmarks.
- Dense data and active sets beat sparse object graphs.
- Cache locality matters.
- Deterministic batch execution enables predictable throughput.

## Related Documents

- `docs/performance/metrics.md` - Specific performance metrics
- `docs/performance/benchmarks.md` - Benchmark methodology
- `docs/architecture/invariants.md` - INV-017: Performance is architectural
- `docs/architecture/invariants.md` - INV-018: Scale claims require reproducible benchmarks
