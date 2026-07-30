//! Turning a hydrology proposal into a committed causal batch.
//!
//! The domain emits a logical DAG: proposal keys, causes that may name siblings,
//! and effects naming a carrier, a property, and two fingerprints. This module
//! supplies what is runtime schema — object kinds, property IDs, event-kind
//! numbers, the dense object registry, and the persisted synthetic-node counter —
//! and builds the two aggregation trees that bind a tick's terminal state and its
//! coarse process inputs into a bounded ancestry.
//!
//! See `plans/hydrology.md` §8 and §9.

use std::collections::{BTreeMap, BTreeSet};

use causafera_core::provenance::{
    CausalEffect, CausalEventDagCause, CausalEventDagProposal, CausalEventProposalKey,
    CausalTarget, StateFingerprint,
};
use causafera_domains::{
    HYDROLOGY_AGGREGATION_ARITY, HydrologyCoarseProcess, HydrologyConservationReceipt,
    HydrologyEventKind, HydrologyEventPlan, HydrologyProperty, HydrologyTerminalLeaf, process,
    substage,
};
use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey, HydrologyEdgeKey};
use causafera_types::{ChartChunkCoord, EventKindId, StateObjectKindId, StatePropertyId, TraceId};

use crate::RuntimeError;

// ---------------------------------------------------------------------------
// Allocated runtime schema identifiers
// ---------------------------------------------------------------------------
//
// Every value below was verified unused at the commit this was written against:
// runtime event kinds reached 34, object kinds reached 13, and state properties
// reached 24. Nothing is renumbered.

pub const HYDROLOGY_SYSTEM_ID: u64 = 13;

pub const HYDROLOGY_FORCING_EVENT_KIND: u64 = 35;
pub const HYDROLOGY_CELL_CHANGE_EVENT_KIND: u64 = 36;
pub const HYDROLOGY_EDGE_TRANSFER_EVENT_KIND: u64 = 37;
pub const HYDROLOGY_CONSERVATION_EVENT_KIND: u64 = 38;
pub const HYDROLOGY_BOOTSTRAP_EVENT_KIND: u64 = 39;
pub const HYDROLOGY_REPRESENTATION_EVENT_KIND: u64 = 40;
pub const HYDROLOGY_BATCH_AGGREGATE_EVENT_KIND: u64 = 41;
pub const HYDROLOGY_COARSE_INPUT_LEAF_EVENT_KIND: u64 = 42;
pub const HYDROLOGY_COARSE_INPUT_AGGREGATE_EVENT_KIND: u64 = 43;
pub const HYDROLOGY_COARSE_PROCESS_EVENT_KIND: u64 = 44;

pub const HYDROLOGY_CELL_OBJECT_KIND: u64 = 14;
pub const HYDROLOGY_EDGE_OBJECT_KIND: u64 = 15;
pub const HYDROLOGY_FORCING_OBJECT_KIND: u64 = 16;
pub const HYDROLOGY_BOOTSTRAP_OBJECT_KIND: u64 = 17;
pub const HYDROLOGY_RESOLUTION_OBJECT_KIND: u64 = 18;
pub const HYDROLOGY_BATCH_OBJECT_KIND: u64 = 19;

pub const HYDROLOGY_SURFACE_PROPERTY: u64 = 25;
pub const HYDROLOGY_SOIL_PROPERTY: u64 = 26;
pub const HYDROLOGY_GROUNDWATER_PROPERTY: u64 = 27;
pub const HYDROLOGY_CONVEYANCE_PROPERTY: u64 = 28;
pub const HYDROLOGY_FORCING_PROPERTY: u64 = 29;
pub const HYDROLOGY_BATCH_SEQUENCE_PROPERTY: u64 = 30;
pub const HYDROLOGY_RESOLUTION_PROPERTY: u64 = 31;
pub const HYDROLOGY_BOOTSTRAP_METRICS_PROPERTY: u64 = 32;
pub const HYDROLOGY_BOOTSTRAP_SUBSTRATE_PROPERTY: u64 = 33;
pub const HYDROLOGY_BOOTSTRAP_STORAGE_PROPERTY: u64 = 34;
pub const HYDROLOGY_BOOTSTRAP_EDGES_PROPERTY: u64 = 35;
pub const HYDROLOGY_BOOTSTRAP_RESOLUTION_PROPERTY: u64 = 36;
pub const HYDROLOGY_BOOTSTRAP_FORCING_PROPERTY: u64 = 37;
pub const HYDROLOGY_BOOTSTRAP_BOUNDARIES_PROPERTY: u64 = 38;
pub const HYDROLOGY_BATCH_ROOT_PROPERTY: u64 = 39;
pub const HYDROLOGY_COARSE_INPUT_PROPERTY: u64 = 40;

/// The synthetic-node object ID reserved for the tick's batch-sequence object.
///
/// The node counter starts at one so this stays distinct from every allocated
/// aggregation node — a conservation event and a tree node sharing an object ID
/// would make two different claims about one target.
pub const HYDROLOGY_BATCH_SEQUENCE_OBJECT_ID: u64 = 0;

// ---------------------------------------------------------------------------
// The dense object registry
// ---------------------------------------------------------------------------

/// Dense `u64` ordinals for every hydrology carrier that can be a causal target.
///
/// `CausalTarget` has a `u64` object slot and a hydrology cell key is 22 bytes,
/// so hashing into the slot would be lossy — two cells could collide and the
/// causal record would say one thing about two places. Bootstrap sorts each key
/// space independently and assigns consecutive ordinals from zero; the tables are
/// persisted and validated as bijections on import.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyObjectRegistry {
    cells: BTreeMap<HydrologyCellKey, u64>,
    edges: BTreeMap<HydrologyEdgeKey, u64>,
    forcing: BTreeMap<(u64, u64), u64>,
    resolution: BTreeMap<ChartChunkCoord, u64>,
}

impl HydrologyObjectRegistry {
    /// Assign ordinals to each key space in canonical order.
    pub fn assign(
        cells: impl IntoIterator<Item = HydrologyCellKey>,
        edges: impl IntoIterator<Item = HydrologyEdgeKey>,
        forcing: impl IntoIterator<Item = (u64, u64)>,
        resolution: impl IntoIterator<Item = ChartChunkCoord>,
    ) -> Self {
        fn dense<K: Ord>(keys: impl IntoIterator<Item = K>) -> BTreeMap<K, u64> {
            let ordered: BTreeSet<K> = keys.into_iter().collect();
            ordered
                .into_iter()
                .enumerate()
                .map(|(ordinal, key)| (key, ordinal as u64))
                .collect()
        }
        Self {
            cells: dense(cells),
            edges: dense(edges),
            forcing: dense(forcing),
            resolution: dense(resolution),
        }
    }

    pub fn cells(&self) -> &BTreeMap<HydrologyCellKey, u64> {
        &self.cells
    }

    pub fn edges(&self) -> &BTreeMap<HydrologyEdgeKey, u64> {
        &self.edges
    }

    pub fn forcing(&self) -> &BTreeMap<(u64, u64), u64> {
        &self.forcing
    }

    pub fn resolution(&self) -> &BTreeMap<ChartChunkCoord, u64> {
        &self.resolution
    }

    /// Whether every table is a bijection onto `0..len`.
    ///
    /// An unknown, duplicate, skipped, or out-of-order assignment would let two
    /// carriers share a causal target, so import checks this rather than trusting
    /// the persisted numbers.
    pub fn is_dense(&self) -> bool {
        fn dense<K: Ord>(table: &BTreeMap<K, u64>) -> bool {
            let mut seen: Vec<u64> = table.values().copied().collect();
            seen.sort_unstable();
            seen.iter()
                .enumerate()
                .all(|(index, ordinal)| *ordinal == index as u64)
        }
        dense(&self.cells) && dense(&self.edges) && dense(&self.forcing) && dense(&self.resolution)
    }

    fn object_id(&self, carrier: HydrologyCarrierKey) -> Result<(u64, u64), RuntimeError> {
        match carrier {
            HydrologyCarrierKey::Cell(cell) => self
                .cells
                .get(&cell)
                .copied()
                .map(|id| (HYDROLOGY_CELL_OBJECT_KIND, id))
                .ok_or(RuntimeError::HydrologyCarrierNotRegistered),
            HydrologyCarrierKey::Edge(edge) => self
                .edges
                .get(&edge)
                .copied()
                .map(|id| (HYDROLOGY_EDGE_OBJECT_KIND, id))
                .ok_or(RuntimeError::HydrologyCarrierNotRegistered),
            HydrologyCarrierKey::ForcingRecord {
                scheduled_tick,
                forcing_id,
            } => self
                .forcing
                .get(&(scheduled_tick, forcing_id))
                .copied()
                .map(|id| (HYDROLOGY_FORCING_OBJECT_KIND, id))
                .ok_or(RuntimeError::HydrologyCarrierNotRegistered),
            HydrologyCarrierKey::ResolutionChunk(chunk) => self
                .resolution
                .get(&chunk)
                .copied()
                .map(|id| (HYDROLOGY_RESOLUTION_OBJECT_KIND, id))
                .ok_or(RuntimeError::HydrologyCarrierNotRegistered),
            // A synthetic node names itself; nothing is looked up.
            HydrologyCarrierKey::BatchNode(id) => Ok((HYDROLOGY_BATCH_OBJECT_KIND, id)),
            // An exterior face is a receipt endpoint, never a stored property. A
            // transfer whose effect target decoded to one would be a change to a
            // place that holds nothing.
            HydrologyCarrierKey::ExteriorFace(_) => {
                Err(RuntimeError::HydrologyCarrierNotAddressable)
            }
        }
    }

    fn target(
        &self,
        carrier: HydrologyCarrierKey,
        property: HydrologyProperty,
    ) -> Result<CausalTarget, RuntimeError> {
        let (object_kind, object_id) = self.object_id(carrier)?;
        Ok(CausalTarget::new(
            StateObjectKindId::new(object_kind),
            object_id,
            StatePropertyId::new(property_id(property)),
        ))
    }
}

const fn property_id(property: HydrologyProperty) -> u64 {
    match property {
        HydrologyProperty::Surface => HYDROLOGY_SURFACE_PROPERTY,
        HydrologyProperty::Soil => HYDROLOGY_SOIL_PROPERTY,
        HydrologyProperty::Groundwater => HYDROLOGY_GROUNDWATER_PROPERTY,
        HydrologyProperty::Conveyance => HYDROLOGY_CONVEYANCE_PROPERTY,
        HydrologyProperty::ForcingInput | HydrologyProperty::ForcingRecord => {
            HYDROLOGY_FORCING_PROPERTY
        }
        HydrologyProperty::Resolution => HYDROLOGY_RESOLUTION_PROPERTY,
        HydrologyProperty::BatchSequence => HYDROLOGY_BATCH_SEQUENCE_PROPERTY,
        HydrologyProperty::BatchRoot => HYDROLOGY_BATCH_ROOT_PROPERTY,
        HydrologyProperty::CoarseInput => HYDROLOGY_COARSE_INPUT_PROPERTY,
    }
}

const fn event_kind(kind: HydrologyEventKind) -> u64 {
    match kind {
        HydrologyEventKind::ForcingApplication => HYDROLOGY_FORCING_EVENT_KIND,
        HydrologyEventKind::ForcingSettlement | HydrologyEventKind::CellChange => {
            HYDROLOGY_CELL_CHANGE_EVENT_KIND
        }
        HydrologyEventKind::EdgeTransfer => HYDROLOGY_EDGE_TRANSFER_EVENT_KIND,
        HydrologyEventKind::Representation => HYDROLOGY_REPRESENTATION_EVENT_KIND,
    }
}

// ---------------------------------------------------------------------------
// Canonical fingerprints the runtime owns
// ---------------------------------------------------------------------------

const DOMAIN_BATCH_LEAF: &[u8] = b"causafera.hydrology.batch-leaf.v1";
const DOMAIN_BATCH_NODE: &[u8] = b"causafera.hydrology.batch-node.v1";
const DOMAIN_COARSE_INPUT_LEAF: &[u8] = b"causafera.hydrology.coarse-input-leaf.v1";
const DOMAIN_COARSE_INPUT_NODE: &[u8] = b"causafera.hydrology.coarse-input-node.v1";
const DOMAIN_COARSE_PROCESS: &[u8] = b"causafera.hydrology.coarse-process.v1";
const DOMAIN_NODE_ABSENT: &[u8] = b"causafera.hydrology.node-absent.v1";
const DOMAIN_BATCH_SEQUENCE: &[u8] = b"causafera.hydrology.batch-sequence.v1";

fn write_prefixed(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn write_target(hasher: &mut blake3::Hasher, target: CausalTarget) {
    hasher.update(&target.object_kind().raw().to_be_bytes());
    hasher.update(&target.object_id().to_be_bytes());
    hasher.update(&target.property().raw().to_be_bytes());
}

/// The fingerprint a synthetic node's target holds before the node exists.
fn node_absent_fingerprint(object_id: u64, property: u64) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_NODE_ABSENT);
    hasher.update(&object_id.to_be_bytes());
    hasher.update(&property.to_be_bytes());
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// One terminal leaf record's fingerprint.
///
/// One settlement event can be terminal for several buckets of the same cell, so
/// the record — not the event — is what is hashed and ordered. Otherwise a cell
/// whose surface and soil both settled in one event would contribute one leaf and
/// a conserved carrier would drop out of the tree.
fn batch_leaf_fingerprint(
    leaf: &HydrologyTerminalLeaf,
    effects: &[CausalEffect],
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_BATCH_LEAF);
    hasher.update(&[leaf.bucket_tag]);
    write_prefixed(&mut hasher, &leaf.carrier_bytes);
    write_prefixed(&mut hasher, leaf.event.bytes());
    hasher.update(&(effects.len() as u64).to_be_bytes());
    for effect in effects {
        write_target(&mut hasher, effect.target());
        write_prefixed(&mut hasher, &effect.before().bytes());
        write_prefixed(&mut hasher, &effect.after().bytes());
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// One aggregate node's fingerprint, for either tree.
fn node_fingerprint(domain: &[u8], level: u32, children: &[StateFingerprint]) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, domain);
    hasher.update(&level.to_be_bytes());
    hasher.update(&[children.len() as u8]);
    for child in children {
        write_prefixed(&mut hasher, &child.bytes());
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// A causal reference, as the coarse-input leaf fingerprint sees it.
///
/// `plans/hydrology.md` §9 hashes "the ordered current-reference trace IDs". A
/// local reference has no trace ID until the batch it belongs to commits, and the
/// leaf fingerprint is an *input* to that batch — so hashing trace IDs is not
/// computable where it has to be computed. The descriptor below is: a kind tag,
/// then either the existing trace ID or the sibling's canonical proposal key.
/// That is deterministic before commit and strictly more specific than an ID,
/// which depends on how much history the store already holds. See the Decision
/// log.
fn write_reference(hasher: &mut blake3::Hasher, reference: &CausalEventDagCause) {
    match reference {
        CausalEventDagCause::Existing(trace) => {
            hasher.update(&[0]);
            hasher.update(&trace.raw().to_be_bytes());
        }
        CausalEventDagCause::Local(key) => {
            hasher.update(&[1]);
            write_prefixed(hasher, key.bytes());
        }
    }
}

fn coarse_leaf_fingerprint(
    coarse: &HydrologyCoarseProcess,
    member: &causafera_domains::HydrologyCoarseMember,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_COARSE_INPUT_LEAF);
    hasher.update(&coarse.tick.to_be_bytes());
    write_prefixed(&mut hasher, &coarse.block.encode());
    write_prefixed(&mut hasher, &coarse.constitutive.encode());
    hasher.update(&coarse.process_kind.to_be_bytes());
    let (scheduled_tick, forcing_id) = coarse.forcing.unwrap_or((0, 0));
    hasher.update(&scheduled_tick.to_be_bytes());
    hasher.update(&forcing_id.to_be_bytes());
    write_prefixed(
        &mut hasher,
        &HydrologyCarrierKey::Cell(member.cell).encode(),
    );
    hasher.update(&(member.weight as u128).to_be_bytes());
    hasher.update(&(member.ceiling as u128).to_be_bytes());
    hasher.update(&(member.references.len() as u64).to_be_bytes());
    for reference in &member.references {
        write_reference(&mut hasher, reference);
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

fn coarse_process_fingerprint(
    coarse: &HydrologyCoarseProcess,
    input_root: StateFingerprint,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_COARSE_PROCESS);
    hasher.update(&coarse.tick.to_be_bytes());
    write_prefixed(&mut hasher, &coarse.block.encode());
    write_prefixed(&mut hasher, &coarse.constitutive.encode());
    hasher.update(&coarse.process_kind.to_be_bytes());
    // The forcing identity, which the plan's four-part key omits but which is
    // what separates two invocations of one group. See the Decision log.
    let (scheduled_tick, forcing_id) = coarse.forcing.unwrap_or((0, 0));
    hasher.update(&scheduled_tick.to_be_bytes());
    hasher.update(&forcing_id.to_be_bytes());
    hasher.update(&(coarse.raw_candidate as u128).to_be_bytes());
    hasher.update(&(coarse.summed_ceilings as u128).to_be_bytes());
    hasher.update(&(coarse.accepted_total as u128).to_be_bytes());
    write_prefixed(&mut hasher, &input_root.bytes());
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

/// The batch-sequence claim one tick's conservation event settles.
///
/// Covers every ledger term, not just the sequence number, so the effect is a
/// statement about the tick's budget rather than about a counter that happened to
/// advance.
fn batch_sequence_fingerprint(
    batch_sequence: u64,
    receipt: &HydrologyConservationReceipt,
) -> StateFingerprint {
    let mut hasher = blake3::Hasher::new();
    write_prefixed(&mut hasher, DOMAIN_BATCH_SEQUENCE);
    hasher.update(&batch_sequence.to_be_bytes());
    hasher.update(&receipt.tick().to_be_bytes());
    for term in [
        receipt.surface_before(),
        receipt.soil_before(),
        receipt.groundwater_before(),
        receipt.conveyance_before(),
        receipt.surface_after(),
        receipt.soil_after(),
        receipt.groundwater_after(),
        receipt.conveyance_after(),
        receipt.accepted_precipitation(),
        receipt.accepted_external_inflow(),
        receipt.accepted_evapotranspiration(),
        receipt.boundary_exports(),
        receipt.residual(),
    ] {
        hasher.update(&term.to_be_bytes());
    }
    StateFingerprint::new(*hasher.finalize().as_bytes())
}

// ---------------------------------------------------------------------------
// Building the batch
// ---------------------------------------------------------------------------

/// What one committed hydrology event settled, so the runtime can install the
/// resolved trace on the right anchor afterwards.
#[derive(Clone, Debug)]
pub(crate) struct HydrologyBatch {
    pub(crate) proposals: Vec<CausalEventDagProposal>,
    pub(crate) conservation_key: CausalEventProposalKey,
    pub(crate) next_node_id: u64,
}

/// The inputs the runtime hands the batch builder.
pub(crate) struct HydrologyBatchInputs<'a> {
    pub(crate) registry: &'a HydrologyObjectRegistry,
    pub(crate) events: &'a [HydrologyEventPlan],
    pub(crate) terminal_leaves: &'a [HydrologyTerminalLeaf],
    pub(crate) coarse_processes: &'a [HydrologyCoarseProcess],
    pub(crate) conservation: &'a HydrologyConservationReceipt,
    pub(crate) batch_sequence: u64,
    pub(crate) previous_conservation: TraceId,
    pub(crate) forcing_origins: Vec<TraceId>,
    pub(crate) next_node_id: u64,
}

/// A synthetic node the builder allocated, before it becomes a proposal.
struct Allocated {
    key: CausalEventProposalKey,
    fingerprint: StateFingerprint,
}

fn node_key(
    substage_ordinal: u8,
    process_kind: u32,
    object_id: u64,
) -> Result<CausalEventProposalKey, RuntimeError> {
    Ok(CausalEventProposalKey::new(
        substage_ordinal,
        process_kind,
        &HydrologyCarrierKey::BatchNode(object_id).encode(),
        0,
    )?)
}

fn node_proposal(
    kind: u64,
    property: u64,
    object_id: u64,
    key: CausalEventProposalKey,
    causes: Vec<CausalEventDagCause>,
    fingerprint: StateFingerprint,
) -> Result<CausalEventDagProposal, RuntimeError> {
    let target = CausalTarget::new(
        StateObjectKindId::new(HYDROLOGY_BATCH_OBJECT_KIND),
        object_id,
        StatePropertyId::new(property),
    );
    Ok(CausalEventDagProposal::new(
        key,
        EventKindId::new(kind),
        causes,
        vec![CausalEffect::new(
            target,
            node_absent_fingerprint(object_id, property),
            fingerprint,
        )?],
    )?)
}

/// Build one bottom-up 16-ary tree and return its root.
///
/// Nodes are allocated level by level, left to right, so the shared counter's
/// order is a function of the tree's shape rather than of traversal choices. A
/// single leaf still gets a one-child root, and no leaves still get a root — with
/// zero causes and a `child_count` of zero — because the conservation event cites
/// that root unconditionally and an absent root would make an empty tick
/// unrepresentable rather than empty.
#[allow(clippy::too_many_arguments)]
fn build_tree(
    children: Vec<(CausalEventProposalKey, StateFingerprint)>,
    domain: &[u8],
    node_kind: u64,
    node_property: u64,
    substage_ordinal: u8,
    process_kind: u32,
    counter: &mut u64,
    out: &mut Vec<CausalEventDagProposal>,
) -> Result<Allocated, RuntimeError> {
    let mut level_children = children;
    let mut level = 0_u32;
    loop {
        let mut nodes: Vec<(CausalEventProposalKey, StateFingerprint)> = Vec::new();
        // One pass per level. An empty first level still produces one node, which
        // is what keeps the root unconditional.
        let groups = if level_children.is_empty() {
            vec![Vec::new()]
        } else {
            level_children
                .chunks(HYDROLOGY_AGGREGATION_ARITY)
                .map(<[(CausalEventProposalKey, StateFingerprint)]>::to_vec)
                .collect()
        };
        for group in groups {
            let object_id = *counter;
            *counter = counter
                .checked_add(1)
                .ok_or(RuntimeError::HydrologyNodeIdentifiersExhausted)?;
            let key = node_key(substage_ordinal, process_kind, object_id)?;
            let fingerprints: Vec<StateFingerprint> =
                group.iter().map(|(_, fingerprint)| *fingerprint).collect();
            let fingerprint = node_fingerprint(domain, level, &fingerprints);
            // Repeated proposal keys are stable-deduplicated at first occurrence,
            // while every record fingerprint stays in the payload above. That is
            // what lets one settlement event be terminal for two buckets without
            // violating cause uniqueness or erasing a conserved carrier.
            let mut causes: Vec<CausalEventDagCause> = Vec::new();
            let mut seen: BTreeSet<CausalEventProposalKey> = BTreeSet::new();
            for (child_key, _) in &group {
                if seen.insert(child_key.clone()) {
                    causes.push(CausalEventDagCause::Local(child_key.clone()));
                }
            }
            causes.sort();
            out.push(node_proposal(
                node_kind,
                node_property,
                object_id,
                key.clone(),
                causes,
                fingerprint,
            )?);
            nodes.push((key, fingerprint));
        }
        if nodes.len() == 1 {
            let (key, fingerprint) = nodes.remove(0);
            return Ok(Allocated { key, fingerprint });
        }
        level_children = nodes;
        level = level
            .checked_add(1)
            .ok_or(RuntimeError::HydrologyNodeIdentifiersExhausted)?;
    }
}

/// Map the domain's logical events, build both trees, and emit the whole batch.
pub(crate) fn build_hydrology_batch(
    inputs: HydrologyBatchInputs<'_>,
) -> Result<HydrologyBatch, RuntimeError> {
    let registry = inputs.registry;
    let mut counter = inputs
        .next_node_id
        .max(HYDROLOGY_BATCH_SEQUENCE_OBJECT_ID + 1);
    let mut proposals: Vec<CausalEventDagProposal> = Vec::new();

    // Coarse groups first, in canonical identity order: member leaves by cell key,
    // then input nodes bottom-up, then the group's process event. Only after every
    // coarse group has its complete tree does the terminal tree draw from the
    // counter, which is what makes the shared allocation order reproducible on
    // import.
    let mut ordered_coarse: Vec<(usize, &HydrologyCoarseProcess)> =
        inputs.coarse_processes.iter().enumerate().collect();
    ordered_coarse.sort_by_key(|(_, coarse)| coarse.identity());
    let mut coarse_keys: BTreeMap<usize, CausalEventProposalKey> = BTreeMap::new();
    for (index, coarse) in ordered_coarse {
        let mut leaves = Vec::with_capacity(coarse.members.len());
        for member in &coarse.members {
            let object_id = counter;
            counter = counter
                .checked_add(1)
                .ok_or(RuntimeError::HydrologyNodeIdentifiersExhausted)?;
            let key = node_key(
                coarse.substage_ordinal,
                process::COARSE_INPUT_LEAF,
                object_id,
            )?;
            let fingerprint = coarse_leaf_fingerprint(coarse, member);
            let mut causes = member.references.clone();
            causes.sort();
            causes.dedup();
            proposals.push(node_proposal(
                HYDROLOGY_COARSE_INPUT_LEAF_EVENT_KIND,
                HYDROLOGY_COARSE_INPUT_PROPERTY,
                object_id,
                key.clone(),
                causes,
                fingerprint,
            )?);
            leaves.push((key, fingerprint));
        }
        let root = build_tree(
            leaves,
            DOMAIN_COARSE_INPUT_NODE,
            HYDROLOGY_COARSE_INPUT_AGGREGATE_EVENT_KIND,
            HYDROLOGY_COARSE_INPUT_PROPERTY,
            coarse.substage_ordinal,
            process::COARSE_INPUT_AGGREGATE,
            &mut counter,
            &mut proposals,
        )?;
        let object_id = counter;
        counter = counter
            .checked_add(1)
            .ok_or(RuntimeError::HydrologyNodeIdentifiersExhausted)?;
        let key = node_key(coarse.substage_ordinal, process::COARSE_PROCESS, object_id)?;
        proposals.push(node_proposal(
            HYDROLOGY_COARSE_PROCESS_EVENT_KIND,
            HYDROLOGY_COARSE_INPUT_PROPERTY,
            object_id,
            key.clone(),
            vec![CausalEventDagCause::Local(root.key)],
            coarse_process_fingerprint(coarse, root.fingerprint),
        )?);
        coarse_keys.insert(index, key);
    }

    // The domain's own events, with their carriers and properties mapped onto the
    // runtime's schema and their coarse-process cause resolved.
    let mut effects_by_event: BTreeMap<CausalEventProposalKey, Vec<CausalEffect>> = BTreeMap::new();
    for event in inputs.events {
        let mut effects = Vec::with_capacity(event.effects.len());
        for effect in &event.effects {
            effects.push(CausalEffect::new(
                registry.target(effect.carrier, effect.property)?,
                effect.before,
                effect.after,
            )?);
        }
        // Ordered by target, which is the runtime's schema order and need not
        // match the order the domain built them in.
        effects.sort_by_key(|effect| effect.target());
        let mut causes = event.causes.clone();
        if let Some(index) = event.coarse_process {
            let key = coarse_keys
                .get(&index)
                .ok_or(RuntimeError::HydrologyCoarseProcessUnknown)?;
            causes.push(CausalEventDagCause::Local(key.clone()));
        }
        causes.sort();
        causes.dedup();
        effects_by_event.insert(event.key.clone(), effects.clone());
        proposals.push(CausalEventDagProposal::new(
            event.key.clone(),
            EventKindId::new(event_kind(event.kind)),
            causes,
            effects,
        )?);
    }

    // The terminal tree over every leaf record, in the domain's canonical
    // `(carrier bytes, bucket tag, proposal key bytes)` order.
    let mut terminal = Vec::with_capacity(inputs.terminal_leaves.len());
    for leaf in inputs.terminal_leaves {
        let effects = effects_by_event
            .get(&leaf.event)
            .ok_or(RuntimeError::HydrologyTerminalLeafUnknown)?;
        terminal.push((leaf.event.clone(), batch_leaf_fingerprint(leaf, effects)));
    }
    let root = build_tree(
        terminal,
        DOMAIN_BATCH_NODE,
        HYDROLOGY_BATCH_AGGREGATE_EVENT_KIND,
        HYDROLOGY_BATCH_ROOT_PROPERTY,
        substage::CONSERVATION,
        process::BATCH_AGGREGATE,
        &mut counter,
        &mut proposals,
    )?;

    // The terminal conservation event: the always-present root, the previous
    // conservation trace, and at most eight forcing origins. Every event in the
    // batch therefore stays inside the sixteen-cause cap while the durable DAG
    // still reaches every bucket, edge, and coarse group the tick touched.
    let mut origins = inputs.forcing_origins;
    origins.sort_unstable();
    origins.dedup();
    if origins.len() > causafera_geography::MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK {
        return Err(RuntimeError::HydrologyBoundExceeded {
            what: "forcing origins in one tick",
            count: origins.len(),
            max: causafera_geography::MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK,
        });
    }
    let conservation_key = CausalEventProposalKey::new(
        substage::CONSERVATION,
        process::CONSERVATION,
        &HydrologyCarrierKey::BatchNode(HYDROLOGY_BATCH_SEQUENCE_OBJECT_ID).encode(),
        0,
    )?;
    let mut causes = vec![
        CausalEventDagCause::Local(root.key),
        CausalEventDagCause::Existing(inputs.previous_conservation),
    ];
    for origin in origins {
        causes.push(CausalEventDagCause::Existing(origin));
    }
    causes.sort();
    causes.dedup();
    proposals.push(CausalEventDagProposal::new(
        conservation_key.clone(),
        EventKindId::new(HYDROLOGY_CONSERVATION_EVENT_KIND),
        causes,
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(HYDROLOGY_BATCH_OBJECT_KIND),
                HYDROLOGY_BATCH_SEQUENCE_OBJECT_ID,
                StatePropertyId::new(HYDROLOGY_BATCH_SEQUENCE_PROPERTY),
            ),
            batch_sequence_fingerprint(
                inputs.batch_sequence.saturating_sub(1),
                inputs.conservation,
            ),
            batch_sequence_fingerprint(inputs.batch_sequence, inputs.conservation),
        )?],
    )?);

    Ok(HydrologyBatch {
        proposals,
        conservation_key,
        next_node_id: counter,
    })
}
