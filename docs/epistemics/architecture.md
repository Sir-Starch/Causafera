# Epistemic Architecture

The epistemic infrastructure lives in the `causafera-epistemics` crate. It simulates not only what agents believe but the mechanisms societies use to improve, preserve, and transmit beliefs.

## Core Principle

Science is not a technology tree. Knowledge progress depends on observation, measurement, concepts, hypotheses, experiments, documents, replication, and institutions. Each of these is a distinct system with its own dynamics and failure modes.

## Knowledge Progress Components

```text
observation
↓
measurement
↓
concepts
↓
hypotheses
↓
experiments
↓
documents
↓
replication
↓
institutions
```

Breakdown at any stage can stall or distort knowledge progress. A society may have excellent observation but poor measurement, or good experiments but no replication culture.

## Separation from Technology

Knowing that something is possible does not imply capability to reproduce it. Technology requires:

```text
concept
+
materials
+
tools
+
measurement
+
procedural knowledge
+
social transmission
```

An agent may understand a principle but lack any of the other prerequisites. This separation is essential for modeling realistic technological development and loss.

## Epistemic Domains

The epistemic crate covers:

- **Knowledge types** - Declarative, procedural, perceptual, and motor knowledge
- **Measurement** - How agents quantify properties of the world
- **Metrology** - Systems of units, standards, and calibration
- **Instruments** - Tools that extend perceptual or measurement capability
- **Experiments** - Structured interventions to test hypotheses
- **Replication** - Independent verification of claimed results
- **Science** - Institutionalized knowledge-seeking practices
- **Writing** - Technologies for persistent external memory
- **Documents** - Physical information carriers with provenance
- **Document lineage** - Ancestry and transmission history of documents

## Interaction with Other Domains

- **Cognition**: Belief formation, memory, and bounded rationality shape what agents can know
- **Language**: Documents encode language; scientific terms spread through speech communities
- **Mana**: Measurement systems may unintentionally create patterns that couple with mana
- **Isekai**: Imported Earth knowledge may include scientific concepts without supporting infrastructure

## Related Documents

- `docs/epistemics/knowledge-types.md` - Varieties of knowledge
- `docs/epistemics/measurement.md` - Quantification and observation
- `docs/epistemics/metrology.md` - Units and standards
- `docs/epistemics/instruments.md` - Tools for extended perception
- `docs/epistemics/experiments.md` - Structured hypothesis testing
- `docs/epistemics/replication.md` - Verification and reproducibility
- `docs/epistemics/science.md` - Institutionalized knowledge practices
- `docs/epistemics/writing.md` - Persistent information encoding
- `docs/epistemics/documents.md` - Physical information carriers
- `docs/epistemics/document-lineage.md` - Document ancestry and copying

## Phase 16 Foundation Status

The implemented foundation uses opaque quantity and unit identities, rational scale, fixed-point uncertainty, bounded calibration ancestry, and physically accessible observations. Documents are bounded physical glyph sequences with explicit deterministic copy edits and ancestry. Instruments, experiments, replication, semantic reading, and scientific institutions remain future work.
