# Performance Metrics

This document defines the specific metrics used to measure Ontopolis performance.

## Simulation Throughput

- **simulated_days_per_second** - Core throughput metric
- **active_entities_per_tick** - Entities receiving updates
- **total_entities** - Entities in simulation (for ratio analysis)
- **ticks_per_second** - Raw tick rate

## Domain-Specific Activity

- **perceptual_features_extracted_per_tick**
- **concepts_updated_per_tick**
- **utterances_decoded_per_tick**
- **lexical_associations_updated_per_tick**
- **causal_edges_emitted_per_tick**
- **practice_executions_per_tick**
- **resolution_transitions_per_tick**

## Memory

- **peak_rss_mb** - Maximum resident set size
- **bytes_per_persistent_resident** - Memory efficiency
- **active_set_memory_mb** - Hot data memory
- **cold_store_memory_mb** - Archived data memory

## Observer Overhead

- **observer_cpu_percent** - CPU time spent in observer layer
- **observer_memory_mb** - Observer memory usage
- **explanation_query_latency_ms** - Explanation Engine response time
- **protocol_serialization_latency_ms** - Protocol encoding time

## GPU (when applicable)

- **gpu_kernel_time_ms** - Kernel execution time
- **gpu_transfer_time_ms** - Host/device transfer time
- **gpu_utilization_percent** - GPU busy percentage

## Benchmark States

Measure all metrics under:

1. No observer attached
2. Idle observer (connected but no queries)
3. Normal UI operation
4. Heavy inspection (multiple panels open)
5. Causal explanation query workload

## Related Documents

- `docs/performance/philosophy.md` - Performance philosophy
- `docs/performance/benchmarks.md` - Benchmark methodology
