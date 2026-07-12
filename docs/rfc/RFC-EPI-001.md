# RFC-EPI-001: Measurement and Metrology

**Status:** Accepted

## Summary

Measurement systems are socially constructed numeric conventions with explicit resolution, uncertainty, procedure, and calibration ancestry. A measurement records a transformation of physically accessible observation; it is not privileged access to truth.

## Decision

Quantities and units use opaque IDs. A unit defines a rational integer scale and non-zero resolution. No unit name or semantic quantity enum exists in authoritative state.

A calibration belongs to a unit, may reference one prior calibration, records systematic bias and uncertainty, and cites the practice used. The bounded registry requires parents to precede children, enforces unit continuity, and caps ancestry depth.

Measurement accepts an accessible numeric observation with access uncertainty. It applies calibration bias, rational scaling, and deterministic quantization, then retains combined uncertainty, calibration identity, procedure, and time. It never accepts or reports a hidden “true value”.

Documents use opaque medium, writing-system, and glyph IDs. A bounded physical mark sequence has optional parent ancestry. Copying takes an explicit ordered edit script and records insertions, removals, and replacements in a transformation record. Glyph meaning remains interpretive and subjective.

## Consequences

- Unit compatibility is structural rather than string-based.
- Calibration drift and disagreement can be represented as competing lineages.
- Copying errors are deterministic causal inputs rather than random lore.
- Instrument physics, experiment design, semantic reading, degradation, and institutions remain future work.

## Rejected alternatives

- Floating-point values and unit strings: non-canonical and semantically privileged.
- Measurements as Ground Truth property reads: violates the physical-access boundary.
- Documents as text strings: erases material form, lineage, and interpretive uncertainty.
