# Performance Philosophy

Ontopolis optimizes simulated causal complexity per wall-clock second.

A million inert agents are not success. A smaller number of agents with genuine perceptual processing, concept formation, causal inference, language decoding, and practice execution is the target.

## What We Measure

Primary metrics:

- simulated days per wall-clock second
- active updates per tick
- perceptual features processed
- concepts updated
- utterances decoded
- lexical associations updated
- causal edges emitted
- practice executions
- resolution transitions

Resource metrics:

- peak RSS
- bytes per persistent resident
- observer overhead
- Explanation Engine query latency

## Benchmark Observer States

Performance must be measured under varying observer load:

- **no observer** - headless simulation throughput
- **idle observer** - observer connected but receiving minimal data
- **normal UI** - typical panel configuration with map and inspectors
- **heavy inspection** - multiple deep entity inspections active
- **causal explanation query workload** - explanation system under query load

## Architectural Performance

Performance is not patched in later. It is an architectural concern from the beginning.

Key principles:

- dense data arrays over scattered objects
- structure-of-arrays layout
- cache locality as a design constraint
- deterministic batch execution
- active sets over full population iteration
- sparse cold stores for rarely accessed state

## GPU Acceleration

GPU utilization is not a project goal. End-to-end simulation throughput is.

Candidate GPU systems:

- mana field operations
- terrain field operations
- hydrological batch calculations
- spatial transforms
- large candidate similarity scoring
- generic feature extraction batches

Poor GPU candidates:

- irregular social graph traversal
- agent decision logic
- language semantic inference
- organizational governance

Every GPU kernel requires a CPU reference implementation, defined inputs and outputs, correctness tests, transfer-inclusive benchmark, and workload crossover measurement.

## What We Do Not Optimize For

- maximum entity count at zero activity
- GPU utilization percentage
- lines of code reduction
- framework convenience

## What We Do Optimize For

- causal depth per CPU cycle
- meaningful agent activity per megabyte
- reconstructable emergence per hour of simulation
