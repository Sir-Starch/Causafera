# Glossary

This glossary defines terms used throughout the Causafera documentation.

## A

**Active Set** - The subset of simulation entities receiving updates in a given tick. Distinct from total entity count.

**Agent** - A simulated individual with perception, cognition, and behavior. Not a player character.

**Analytical Ontology** - Observer-side categories used to classify simulation structures for human understanding. Not Ground Truth.

**Authoritative State** - The canonical simulation state that determines all outcomes. Only modified by the simulation engine.

## B

**Backpressure** - Flow control mechanisms that prevent slow consumers from overwhelming the system.

**Bootstrap** - Historical initialization that creates pre-simulation state (languages, institutions, geography).

**Bounded Cognition** - The principle that agents have limited attention, memory, and reasoning capacity.

## C

**Causal Carrier** - A mechanism by which one domain affects another.

**Causal Depth** - Number of meaningful causal stages in a phenomenon's history.

**Causal Resolution** - Simulation detail level determined by causal relevance, not just physical distance.

**Concept** - A subjective cognitive structure built by agents from perceptual features.

**Confidence** - Measure of certainty in an analytical classification or explanation claim.

**Cross-Domain Interaction** - Causal connection between two fundamental simulation domains.

## D

**Determinism** - Property that identical inputs produce identical outputs. Required for testing and replay.

**Domain** - A fundamental area of simulation (geography, biology, language, etc.).

**Domain Coverage Matrix** - Analysis ensuring every domain answers required questions before implementation.

**Dungeon** - Target emergent phenomenon: ecological/spatial complex formed from geology, abandoned structures, mana, and ecology.

## E

**Emergent Concept** - A category constructed by agents or societies, not predefined in the engine.

**Emergent Novelty** - Distance of a phenomenon from predefined observer categories.

**Explanation Engine** - System that converts simulation state into human-understandable explanations without modifying state.

**Explanation IR** - Structured intermediate representation for explanations. Contains typed claims and evidence.

## F

**Feature** - Generic perceptual primitive extracted from Ground Truth (change, magnitude, periodicity, etc.).

**Ground Truth** - Objective simulation state, independent of any agent's perception or belief.

## G

**Gloss** - Human-readable label produced by the observer layer for simulation entities. Not simulation state.

## I

**Isekai** - Cross-world transfer of individuals, knowledge, or artifacts from Earth to the simulated world.

## L

**Lexeme** - A socially transmitted linguistic form lineage. Not a string with an objective meaning.

**Lifecycle Audit** - Analysis of an entity's complete lifespan from origin to historical residue.

**Localization** - Adaptation of observer UI text to different human languages. Does not affect simulation state.

## M

**Maintenance** - Practices that preserve infrastructure and equipment. Loss of maintenance knowledge causes historical crises.

**Mana** - Information-sensitive field that responds to real patterns (repetition, geometry, frequency) but not semantic meaning.

**Map Perspective** - Different knowledge states rendered as maps (Ground Truth, agent-known, organization-known, historical).

## O

**Observer** - Read-only layer that provides derived simulation state to UI and analytics. Never authoritative.

**Observer Protocol** - Structured communication interface between simulation and UI using Protocol Buffers.

## P

**Parcel** - Fundamental spatial unit of city organization with boundaries, ownership, and physical characteristics.

**Path Dependence** - Sensitivity of outcomes to early historical conditions.

**Perceptual Feature** - Generic primitive extracted from sensory input, not a semantically meaningful observation.

**Phenomenon Evaluation** - Framework for measuring emergence quality through causal depth, domain coupling, etc.

**Practice** - Structured behavioral program with operations, conditions, timing, and materials.

**Primitive** - Engine-level property required to define the physical or computational universe.

**Provenance** - Historical chain of causation showing how something came to be.

## R

**Read Model** - Derived data structure that supports observer queries without exposing internal simulation storage.

**RFC** - Request for Comments. Proposed architectural decisions requiring review.

## S

**Semantic Drift** - Change in the associations between a lexeme and concepts over time.

**Snapshot** - Complete state capture at a point in time, used for observer synchronization.

**Subjective Concept** - Agent-specific cognitive category that may not match Ground Truth or other agents' concepts.

## T

**Technology** - Capability requiring concept, material, tool, precision, measurement, skill, and social transmission.

**Typed ID** - Strongly typed identifier (AgentId, ConceptId, etc.) preventing accidental cross-domain references.

## U

**UI** - User interface. An observer that never owns authoritative state.

## V

**View** - Independent panel in the desktop application showing a specific aspect of the simulation.
