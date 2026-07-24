use crate::*;
use causafera_core::*;
use causafera_domains::*;
use causafera_perception::{PhysicalSignal, SignalMagnitude};
use causafera_types::*;
use std::collections::{BTreeMap, BTreeSet};

use causafera_types::{CHUNK_SIZE, ChartChunkCoord, SimulationTime, TraceId};

pub const MAX_MATERIAL_SURFACE_TRANSITIONS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaterialSurfaceId {
    pub chunk: ChartChunkCoord,
    pub cell_index: u16,
}

impl MaterialSurfaceId {
    pub const fn new(chunk: ChartChunkCoord, cell_index: u16) -> Self {
        Self { chunk, cell_index }
    }

    pub const fn is_within_extent(self, extent: u8) -> bool {
        let side = extent as u16;
        self.cell_index < side.saturating_mul(side).saturating_mul(side)
    }

    pub const fn has_valid_cell_ordinal(self) -> bool {
        self.is_within_extent(CHUNK_SIZE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceManaGate {
    pub active: bool,
    pub last_transition: Option<TraceId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurface {
    pub condition: i64,
    pub contact_count: u64,
    pub last_transition: TraceId,
    pub last_contact_trace: Option<TraceId>,
    pub gate: MaterialSurfaceManaGate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceRecordSnapshot {
    pub id: MaterialSurfaceId,
    pub surface: MaterialSurface,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceSnapshot {
    pub records: Vec<MaterialSurfaceRecordSnapshot>,
    pub pending_physical_changes: Vec<MaterialSurfaceId>,
    pub transitions: Vec<MaterialSurfaceTransition>,
    pub gate_transitions: Vec<MaterialSurfaceGateTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceTransition {
    pub id: MaterialSurfaceId,
    pub occurred_at: SimulationTime,
    pub before_condition: i64,
    pub after_condition: i64,
    pub mana_total: i64,
    pub contact_trace: Option<TraceId>,
    pub mana_effect_trace: Option<TraceId>,
    pub transition_trace: TraceId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialSurfaceGateTransition {
    pub id: MaterialSurfaceId,
    pub occurred_at: SimulationTime,
    pub before_active: bool,
    pub after_active: bool,
    pub local_mana_before: i64,
    pub local_mana_after: i64,
    pub local_mana_trace: TraceId,
    pub contact_trace: Option<TraceId>,
    pub transition_trace: TraceId,
}
pub(crate) fn validate_material_surface_object_ids(
    ids: impl Iterator<Item = MaterialSurfaceId>,
) -> Result<(), RuntimeError> {
    let mut object_ids = BTreeSet::new();
    for id in ids {
        if !object_ids.insert(material_surface_object_id(id)) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface object ID collision",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_mana_cell_object_ids(fields: &ManaFieldSet) -> Result<(), RuntimeError> {
    let mut object_ids = BTreeSet::new();
    for (chunk, field) in fields.fields() {
        for index in 0..field.intensity().len() {
            let cell_index = u16::try_from(index)
                .map_err(|_| RuntimeError::InvalidSnapshot("mana field cell index exceeds u16"))?;
            if !object_ids.insert(cell_object_id(*chunk, cell_index)) {
                return Err(RuntimeError::InvalidSnapshot(
                    "mana cell object ID collision",
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_material_surface_transition(
    traces: &CausalTraceStore,
    transition: &MaterialSurfaceTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.time != transition.occurred_at {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition time does not match anchor",
        ));
    }
    match (transition.contact_trace, transition.mana_effect_trace) {
        (None, None) => {
            if event.kind != EventKindId::new(MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND)
                || event.phase != Phase::Lifecycle
                || transition.mana_total != 0
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface bootstrap transition has invalid lifecycle semantics",
                ));
            }
        }
        (Some(contact_trace), None) => {
            if contact_trace != transition.transition_trace
                || event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                || event.phase != Phase::Action
                || transition.mana_total != 0
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface contact anchor is not an actor contact",
                ));
            }
        }
        (Some(contact_trace), Some(mana_effect_trace)) => {
            if mana_effect_trace != transition.transition_trace
                || event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                || event.phase != Phase::Mana
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface mana anchor is not a mana effect",
                ));
            }
            if !event.causes.contains(&contact_trace) {
                return Err(RuntimeError::InvalidSnapshot(
                    "material surface mana anchor does not cite contact trace",
                ));
            }
        }
        (None, Some(_)) => {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface mana transition is missing a contact anchor",
            ));
        }
    }
    let material_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(transition.id)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface transition effect target does not match surface",
        ))?;
    if material_effect.target().property()
        != StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect property is not condition",
        ));
    }
    if !material_surface_fingerprint_matches_condition(
        material_effect.before(),
        transition.before_condition,
    ) || !material_surface_fingerprint_matches_condition(
        material_effect.after(),
        transition.after_condition,
    ) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect fingerprint does not match declared condition",
        ));
    }
    let before_contact_count = material_surface_fingerprint_contact_count(material_effect.before());
    let after_contact_count = material_surface_fingerprint_contact_count(material_effect.after());
    let contact_count_is_valid = match (transition.contact_trace, transition.mana_effect_trace) {
        (None, None) => before_contact_count == 0 && after_contact_count == 0,
        (Some(_), None) => after_contact_count == before_contact_count.saturating_add(1),
        (Some(_), Some(_)) => after_contact_count == before_contact_count,
        (None, Some(_)) => false,
    };
    if !contact_count_is_valid {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface transition effect has invalid contact count semantics",
        ));
    }
    if transition.mana_effect_trace.is_some() {
        validate_material_surface_mana_contact_parent(traces, transition, material_effect)?;
    }
    Ok(())
}

pub(crate) fn validate_material_surface_last_transition(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(surface.last_transition)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let material_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(id)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface last transition effect target does not match surface",
        ))?;
    if material_effect.target().property()
        != StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition effect property is not condition",
        ));
    }
    if material_effect.after() != material_surface_fingerprint(surface) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition effect does not match persisted surface",
        ));
    }
    let before_contact_count = material_surface_fingerprint_contact_count(material_effect.before());
    let after_contact_count = material_surface_fingerprint_contact_count(material_effect.after());
    let semantics_are_valid = match event.kind.raw() {
        MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND => {
            event.phase == Phase::Lifecycle
                && material_effect.before()
                    == material_surface_fingerprint(MaterialSurface {
                        condition: 0,
                        contact_count: 0,
                        last_transition: TraceId::new(0),
                        last_contact_trace: None,
                        gate: MaterialSurfaceManaGate {
                            active: false,
                            last_transition: None,
                        },
                    })
                && before_contact_count == 0
                && after_contact_count == 0
        }
        MATERIAL_SURFACE_CONTACT_EVENT_KIND => {
            event.phase == Phase::Action
                && after_contact_count == before_contact_count.saturating_add(1)
        }
        MATERIAL_SURFACE_MANA_EVENT_KIND => {
            event.phase == Phase::Mana && after_contact_count == before_contact_count
        }
        _ => false,
    };
    if !semantics_are_valid {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface last transition has invalid event semantics",
        ));
    }
    if event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND) {
        let contact_event = event
            .causes
            .iter()
            .find_map(|trace| {
                let candidate = traces.event(*trace)?;
                (candidate.kind == EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                    && candidate.phase == Phase::Action)
                    .then_some(candidate)
            })
            .ok_or(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has no contact parent",
            ))?;
        let contact_effect = contact_event
            .effects
            .iter()
            .find(|effect| {
                effect.target().object_kind()
                    == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                    && effect.target().object_id() == material_surface_object_id(id)
                    && effect.target().property()
                        == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
            })
            .ok_or(RuntimeError::InvalidSnapshot(
                "material surface last mana transition contact parent does not target surface",
            ))?;
        if material_surface_fingerprint_contact_count(contact_effect.after())
            != material_surface_fingerprint_contact_count(contact_effect.before()).saturating_add(1)
        {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has invalid contact parent",
            ));
        }
        let has_condition_parent = event.causes.iter().any(|trace| {
            traces.event(*trace).is_some_and(|candidate| {
                candidate.effects.iter().any(|effect| {
                    effect.target().object_kind()
                        == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                        && effect.target().object_id() == material_surface_object_id(id)
                        && effect.target().property()
                            == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                        && effect.after() == material_effect.before()
                })
            })
        });
        if !has_condition_parent {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface last mana transition has no matching condition parent",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_material_surface_last_contact_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    match (surface.contact_count, surface.last_contact_trace) {
        (0, None) => Ok(()),
        (0, Some(_)) => Err(RuntimeError::InvalidSnapshot(
            "uncontacted material surface has a contact trace",
        )),
        (_, None) => Err(RuntimeError::InvalidSnapshot(
            "contacted material surface is missing a contact trace",
        )),
        (contact_count, Some(trace)) => {
            let event = traces
                .event(trace)
                .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
            let effect = event
                .effects
                .iter()
                .find(|effect| {
                    effect.target().object_kind()
                        == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                        && effect.target().object_id() == material_surface_object_id(id)
                        && effect.target().property()
                            == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                })
                .ok_or(RuntimeError::InvalidSnapshot(
                    "contact trace does not target material surface condition",
                ))?;
            if event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                || event.phase != Phase::Action
                || material_surface_fingerprint_contact_count(effect.after()) != contact_count
                || material_surface_fingerprint_contact_count(effect.before())
                    != contact_count.saturating_sub(1)
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "contact trace does not match persisted material contact count",
                ));
            }
            Ok(())
        }
    }
}

pub(crate) fn validate_material_surface_gate_state(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    surface: MaterialSurface,
) -> Result<(), RuntimeError> {
    if surface.contact_count == 0 && surface.gate.last_transition.is_some() {
        return Err(RuntimeError::InvalidSnapshot(
            "uncontacted material surface has a gate transition",
        ));
    }
    let Some(trace) = surface.gate.last_transition else {
        return if surface.gate.active {
            Err(RuntimeError::InvalidSnapshot(
                "active material surface gate is missing a transition",
            ))
        } else {
            Ok(())
        };
    };
    let event = traces
        .event(trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let object_id = material_surface_object_id(id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    let gate_property = StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY);
    let gate_effects: Vec<_> = event
        .effects
        .iter()
        .filter(|effect| {
            effect.target().object_kind() == object_kind
                && effect.target().property() == gate_property
        })
        .collect();
    if gate_effects.len() != 1 || gate_effects[0].target().object_id() != object_id {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition does not target gate property exactly once",
        ));
    }
    if event.effects.iter().any(|effect| {
        effect.target().object_kind() == object_kind && effect.target().object_id() != object_id
    }) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate event contains cross-surface effects",
        ));
    }
    let gate_effect = gate_effects[0];
    let expected_before = material_surface_gate_fingerprint(!surface.gate.active);
    let expected_after = material_surface_gate_fingerprint(surface.gate.active);
    if event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
        || event.phase != Phase::Mana
        || gate_effect.before() != expected_before
        || gate_effect.after() != expected_after
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate state does not match its transition",
        ));
    }
    if traces.iter().any(|candidate| {
        candidate.trace_id > trace
            && candidate.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
            && candidate.phase == Phase::Mana
            && candidate.effects.iter().any(|effect| {
                effect.target().object_kind() == object_kind
                    && effect.target().object_id() == object_id
                    && effect.target().property() == gate_property
            })
    }) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate state is not the latest gate transition",
        ));
    }
    Ok(())
}

fn expected_gate_transition_causes(
    transition: &MaterialSurfaceGateTransition,
    prior_gate_trace: Option<TraceId>,
    prior_condition_trace: Option<TraceId>,
) -> Result<Vec<TraceId>, RuntimeError> {
    let mut causes = BTreeSet::new();
    causes.insert(transition.local_mana_trace);
    if let Some(trace) = prior_gate_trace {
        causes.insert(trace);
    }
    if transition.after_active {
        let Some(contact_trace) = transition.contact_trace else {
            return Err(RuntimeError::InvalidSnapshot(
                "rising material surface gate transition is missing contact trace",
            ));
        };
        causes.insert(contact_trace);
        if let Some(prior_condition) = prior_condition_trace
            && prior_condition != contact_trace
        {
            causes.insert(prior_condition);
        }
    }
    Ok(causes.into_iter().collect())
}

fn prior_material_surface_gate_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    before: TraceId,
) -> Option<TraceId> {
    let object_id = material_surface_object_id(id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    let gate_property = StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY);
    traces
        .iter()
        .filter(|event| {
            event.trace_id < before
                && event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
                && event.phase == Phase::Mana
                && event.effects.iter().any(|effect| {
                    effect.target().object_kind() == object_kind
                        && effect.target().object_id() == object_id
                        && effect.target().property() == gate_property
                })
        })
        .map(|event| event.trace_id)
        .max()
}

fn prior_material_surface_condition_trace(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    before: TraceId,
) -> Option<TraceId> {
    let object_id = material_surface_object_id(id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    let condition_property = StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY);
    let is_condition_event = |event: &causafera_core::provenance::CausalEventRef<'_>| {
        ((event.kind == EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
            && event.phase == Phase::Mana)
            || (event.kind == EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
                && event.phase == Phase::Action))
            && event.effects.iter().any(|effect| {
                effect.target().object_kind() == object_kind
                    && effect.target().object_id() == object_id
                    && effect.target().property() == condition_property
            })
    };
    traces
        .iter()
        .filter(|event| event.trace_id < before && is_condition_event(event))
        .map(|event| event.trace_id)
        .max()
}

pub(crate) fn validate_material_surface_gate_transition_history(
    traces: &CausalTraceStore,
    surfaces: &BTreeMap<MaterialSurfaceId, MaterialSurface>,
    _material_transitions: &[MaterialSurfaceTransition],
    gate_transitions: &[MaterialSurfaceGateTransition],
) -> Result<(), RuntimeError> {
    for transition in gate_transitions {
        if !surfaces.contains_key(&transition.id) {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition references unknown surface",
            ));
        }
        let event = traces
            .event(transition.transition_trace)
            .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
        let prior_condition_trace = transition
            .after_active
            .then(|| {
                prior_material_surface_condition_trace(
                    traces,
                    transition.id,
                    transition.transition_trace,
                )
            })
            .flatten();
        let prior_gate_trace =
            prior_material_surface_gate_trace(traces, transition.id, transition.transition_trace);
        let expected_causes =
            expected_gate_transition_causes(transition, prior_gate_trace, prior_condition_trace)?;
        if event.causes != expected_causes {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition has incorrect causal parent set",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_material_surface_gate_transition(
    traces: &CausalTraceStore,
    material_transitions: &[MaterialSurfaceTransition],
    transition: &MaterialSurfaceGateTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.kind != EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND)
        || event.phase != Phase::Mana
        || event.time != transition.occurred_at
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition has invalid event semantics",
        ));
    }
    let gate_effect = event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(transition.id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface gate transition is missing its gate effect",
        ))?;
    if gate_effect.before() != material_surface_gate_fingerprint(transition.before_active)
        || gate_effect.after() != material_surface_gate_fingerprint(transition.after_active)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition gate effect does not match record",
        ));
    }
    let object_id = material_surface_object_id(transition.id);
    let object_kind = StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND);
    if event.effects.iter().any(|effect| {
        effect.target().object_kind() == object_kind && effect.target().object_id() != object_id
    }) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition event contains cross-surface effects",
        ));
    }
    validate_local_mana_transition(traces, transition)?;
    if !event.causes.contains(&transition.local_mana_trace) {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition does not cite local mana trace",
        ));
    }
    match (
        transition.before_active,
        transition.after_active,
        transition.contact_trace,
    ) {
        (false, true, Some(contact_trace)) => {
            if !event.causes.contains(&contact_trace) {
                return Err(RuntimeError::InvalidSnapshot(
                    "rising material surface gate transition does not cite contact trace",
                ));
            }
            validate_material_surface_last_contact_event(traces, transition.id, contact_trace)?;
            if event.effects.len() != 2
                || !material_transitions.iter().any(|condition| {
                    condition.id == transition.id
                        && condition.transition_trace == transition.transition_trace
                        && condition.mana_effect_trace == Some(transition.transition_trace)
                })
            {
                return Err(RuntimeError::InvalidSnapshot(
                    "rising material surface gate transition is missing condition evidence",
                ));
            }
        }
        (true, false, None) if event.effects.len() == 1 => {}
        _ => {
            return Err(RuntimeError::InvalidSnapshot(
                "material surface gate transition has invalid rising or falling shape",
            ));
        }
    }
    Ok(())
}

fn validate_material_surface_last_contact_event(
    traces: &CausalTraceStore,
    id: MaterialSurfaceId,
    trace: TraceId,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
        || event.phase != Phase::Action
        || !event.effects.iter().any(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
        })
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition contact trace is invalid",
        ));
    }
    Ok(())
}

fn validate_local_mana_transition(
    traces: &CausalTraceStore,
    transition: &MaterialSurfaceGateTransition,
) -> Result<(), RuntimeError> {
    let event = traces
        .event(transition.local_mana_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let object_id = cell_object_id(transition.id.chunk, transition.id.cell_index);
    let ordinary = event.kind == EventKindId::new(MANA_EVENT_KIND)
        && event.phase == Phase::Mana
        && event.effects.iter().any(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MANA_OBJECT_KIND)
                && effect.target().object_id() == object_id
                && effect.target().property() == StatePropertyId::new(MANA_PROPERTY)
                && effect.before() == fingerprint_i64(0x0301, transition.local_mana_before)
                && effect.after() == fingerprint_i64(0x0301, transition.local_mana_after)
        });
    let source = event.kind == EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
        && event.phase == Phase::Mana
        && event.effects.iter().any(|effect| {
            effect.target().object_kind()
                == StateObjectKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_OBJECT_KIND)
                && effect.target().object_id() == object_id
                && effect.target().property()
                    == StatePropertyId::new(EXPERIMENT_RECIPE_MANA_SOURCE_PROPERTY)
                && effect.before() == fingerprint_i64(0x0302, transition.local_mana_before)
                && effect.after() == fingerprint_i64(0x0302, transition.local_mana_after)
        });
    if ordinary || source {
        Ok(())
    } else {
        Err(RuntimeError::InvalidSnapshot(
            "material surface gate transition local mana evidence is invalid",
        ))
    }
}

pub(crate) fn material_surface_mana_transition_evidence(
    traces: &CausalTraceStore,
    receipts: &[ExperimentRecipeManaSourceReceipt],
    transition: &MaterialSurfaceTransition,
) -> (Option<TraceId>, Option<i64>, Option<i64>) {
    let Some(mana_effect_trace) = transition.mana_effect_trace else {
        return (None, None, None);
    };
    let Some(mana_effect_event) = traces.event(mana_effect_trace) else {
        return (None, None, None);
    };
    let Some(mana_transition_trace) = mana_effect_event.causes.iter().copied().find(|cause| {
        traces.event(*cause).is_some_and(|event| {
            event.kind == EventKindId::new(MANA_EVENT_KIND)
                || event.kind == EventKindId::new(EXPERIMENT_RECIPE_MANA_SOURCE_EVENT_KIND)
        })
    }) else {
        return (None, None, None);
    };
    let receipt = receipts
        .iter()
        .find(|receipt| receipt.source_trace == mana_transition_trace);
    (
        Some(mana_transition_trace),
        receipt.map(|receipt| receipt.before_intensity),
        receipt.map(|receipt| receipt.after_intensity),
    )
}

fn validate_material_surface_mana_contact_parent(
    traces: &CausalTraceStore,
    mana_transition: &MaterialSurfaceTransition,
    mana_effect: &CausalEffect,
) -> Result<(), RuntimeError> {
    let contact_trace = mana_transition
        .contact_trace
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface mana transition is missing a contact anchor",
        ))?;
    let contact_event = traces
        .event(contact_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    if contact_event.phase != Phase::Action
        || contact_event.kind != EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent is not an actor contact",
        ));
    }
    let contact_effect = contact_event
        .effects
        .iter()
        .find(|effect| {
            effect.target().object_kind() == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                && effect.target().object_id() == material_surface_object_id(mana_transition.id)
                && effect.target().property()
                    == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
        })
        .ok_or(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent does not target declared condition",
        ))?;
    if material_surface_fingerprint_contact_count(contact_effect.after())
        != material_surface_fingerprint_contact_count(contact_effect.before()).saturating_add(1)
    {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana contact parent has invalid contact count semantics",
        ));
    }
    let mana_event = traces
        .event(mana_transition.transition_trace)
        .ok_or(RuntimeError::InvalidSnapshot("unknown trace reference"))?;
    let has_condition_parent = mana_event.causes.iter().any(|trace| {
        traces.event(*trace).is_some_and(|event| {
            event.effects.iter().any(|effect| {
                effect.target().object_kind()
                    == StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND)
                    && effect.target().object_id() == material_surface_object_id(mana_transition.id)
                    && effect.target().property()
                        == StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY)
                    && effect.after() == mana_effect.before()
            })
        })
    });
    if !has_condition_parent {
        return Err(RuntimeError::InvalidSnapshot(
            "material surface mana transition has no matching prior condition parent",
        ));
    }
    Ok(())
}

fn material_surface_fingerprint_matches_condition(
    fingerprint: StateFingerprint,
    condition: i64,
) -> bool {
    material_surface_fingerprint(MaterialSurface {
        condition,
        contact_count: material_surface_fingerprint_contact_count(fingerprint),
        last_transition: TraceId::new(0),
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    }) == fingerprint
}

fn material_surface_fingerprint_contact_count(fingerprint: StateFingerprint) -> u64 {
    let bytes = fingerprint.bytes();
    let mut encoded = [0_u8; 8];
    encoded.copy_from_slice(&bytes[16..24]);
    u64::from_le_bytes(encoded)
}

pub(crate) fn material_surface_object_id(id: MaterialSurfaceId) -> u64 {
    cell_object_id(id.chunk, id.cell_index)
}

pub(crate) fn material_surface_fingerprint(surface: MaterialSurface) -> StateFingerprint {
    fingerprint_pair(
        0x0D01,
        surface.condition,
        i64::try_from(surface.contact_count).unwrap_or(i64::MAX),
    )
}

pub(crate) fn material_surface_gate_fingerprint(active: bool) -> StateFingerprint {
    fingerprint_u64(0x0D03, u64::from(active))
}

pub(crate) fn commit_material_surface_bootstrap_event(
    state: &mut RuntimeState,
    stage: HistoricalStageId,
    ordinal: u64,
    id: MaterialSurfaceId,
    initial_condition: i64,
) -> Result<TraceId, RuntimeError> {
    let before = MaterialSurface {
        condition: 0,
        contact_count: 0,
        last_transition: state.latest_physical_trace,
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    };
    let after = MaterialSurface {
        condition: initial_condition,
        contact_count: 0,
        last_transition: state.latest_physical_trace,
        last_contact_trace: None,
        gate: MaterialSurfaceManaGate {
            active: false,
            last_transition: None,
        },
    };
    let event = CausalEventProposal::new(
        EventProposalKey::new(BOOTSTRAP_SYSTEM_ID, stage.raw(), ordinal),
        EventKindId::new(MATERIAL_SURFACE_BOOTSTRAP_EVENT_KIND),
        vec![state.latest_physical_trace],
        vec![CausalEffect::new(
            CausalTarget::new(
                StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                material_surface_object_id(id),
                StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
            ),
            material_surface_fingerprint(before),
            material_surface_fingerprint(after),
        )?],
    )?;
    let trace = state
        .traces
        .commit_batch(SimulationTime::new(0), Phase::Lifecycle, vec![event])?[0];
    state.latest_physical_trace = trace;
    Ok(trace)
}

pub(crate) fn commit_material_surface_contact_events(
    state: &mut RuntimeState,
    time: SimulationTime,
    ordinal: u64,
    actor: &ActorState,
    proposal: MaterialSurfaceContactProposal,
) -> Result<TraceId, RuntimeError> {
    let event = CausalEventProposal::new(
        EventProposalKey::new(ACTOR_ACTION_SYSTEM_ID, proposal.actor.raw(), ordinal),
        EventKindId::new(MATERIAL_SURFACE_CONTACT_EVENT_KIND),
        proposal.causes,
        vec![
            CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(ACTOR_OBJECT_KIND),
                    proposal.actor.raw(),
                    StatePropertyId::new(ACTOR_BODY_PROPERTY),
                ),
                actor_state_fingerprint(actor),
                actor_state_fingerprint(&proposal.next_actor),
            )?,
            CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                    material_surface_object_id(proposal.surface),
                    StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
                ),
                material_surface_fingerprint(proposal.before_surface),
                material_surface_fingerprint(proposal.after_surface),
            )?,
        ],
    )?;
    let trace = state
        .traces
        .commit_batch(time, Phase::Action, vec![event])?[0];
    state.material_surfaces.insert(
        proposal.surface,
        MaterialSurface {
            last_transition: trace,
            last_contact_trace: Some(trace),
            ..proposal.after_surface
        },
    );
    state
        .pending_material_surface_changes
        .insert(proposal.surface);
    record_material_surface_transition(
        state,
        MaterialSurfaceTransition {
            id: proposal.surface,
            occurred_at: time,
            before_condition: proposal.before_surface.condition,
            after_condition: proposal.after_surface.condition,
            mana_total: 0,
            contact_trace: Some(trace),
            mana_effect_trace: None,
            transition_trace: trace,
        },
    );
    Ok(trace)
}

pub(crate) fn commit_mana_material_surface_effect_events(
    state: &mut RuntimeState,
    time: SimulationTime,
    proposals: &[LocalManaMaterialSurfaceProposal],
) -> Result<Vec<TraceId>, RuntimeError> {
    let events = proposals
        .iter()
        .map(|proposal| {
            let mut effects = vec![CausalEffect::new(
                CausalTarget::new(
                    StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                    material_surface_object_id(proposal.surface),
                    StatePropertyId::new(MATERIAL_SURFACE_MANA_GATE_PROPERTY),
                ),
                material_surface_gate_fingerprint(proposal.before.gate.active),
                material_surface_gate_fingerprint(proposal.after_active),
            )?];
            if let Some(after_condition) = proposal.after_condition {
                let mut after = proposal.before;
                after.condition = after_condition;
                effects.push(CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(MATERIAL_SURFACE_OBJECT_KIND),
                        material_surface_object_id(proposal.surface),
                        StatePropertyId::new(MATERIAL_SURFACE_CONDITION_PROPERTY),
                    ),
                    material_surface_fingerprint(proposal.before),
                    material_surface_fingerprint(after),
                )?);
            }
            effects.sort_unstable_by_key(|effect| effect.target());
            CausalEventProposal::new(
                proposal.key,
                EventKindId::new(MATERIAL_SURFACE_MANA_EVENT_KIND),
                proposal.causes.clone(),
                effects,
            )
            .map_err(RuntimeError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(state.traces.commit_batch(time, Phase::Mana, events)?)
}

pub(crate) fn apply_local_mana_material_surface_transition(
    state: &mut RuntimeState,
    time: SimulationTime,
    proposal: &LocalManaMaterialSurfaceProposal,
    trace: TraceId,
) {
    let mut after = proposal.before;
    after.gate.active = proposal.after_active;
    after.gate.last_transition = Some(trace);
    if let Some(condition) = proposal.after_condition {
        after.condition = condition;
        after.last_transition = trace;
        state
            .pending_material_surface_changes
            .insert(proposal.surface);
        record_material_surface_transition(
            state,
            MaterialSurfaceTransition {
                id: proposal.surface,
                occurred_at: time,
                before_condition: proposal.before.condition,
                after_condition: condition,
                mana_total: state.mana.total_intensity(),
                contact_trace: proposal.contact_trace,
                mana_effect_trace: Some(trace),
                transition_trace: trace,
            },
        );
    }
    state.material_surfaces.insert(proposal.surface, after);
    record_material_surface_gate_transition(
        state,
        MaterialSurfaceGateTransition {
            id: proposal.surface,
            occurred_at: time,
            before_active: proposal.before.gate.active,
            after_active: proposal.after_active,
            local_mana_before: proposal.local_mana_before,
            local_mana_after: proposal.local_mana_after,
            local_mana_trace: proposal.local_mana_trace,
            contact_trace: proposal.contact_trace,
            transition_trace: trace,
        },
    );
}

pub(crate) fn record_material_surface_transition(
    state: &mut RuntimeState,
    transition: MaterialSurfaceTransition,
) {
    if state.material_surface_transitions.len() == MAX_MATERIAL_SURFACE_TRANSITIONS {
        let evicted = state
            .material_surface_transitions
            .iter()
            .position(|existing| existing.mana_effect_trace.is_none())
            .unwrap_or(0);
        state.material_surface_transitions.remove(evicted);
    }
    state.material_surface_transitions.push(transition);
}

fn record_material_surface_gate_transition(
    state: &mut RuntimeState,
    transition: MaterialSurfaceGateTransition,
) {
    if state.material_surface_gate_transitions.len() == MAX_MATERIAL_SURFACE_TRANSITIONS {
        let evicted = state
            .material_surface_gate_transitions
            .iter()
            .position(|existing| !existing.after_active)
            .unwrap_or(0);
        state.material_surface_gate_transitions.remove(evicted);
    }
    state.material_surface_gate_transitions.push(transition);
}

pub(crate) fn resolve_material_surface(
    state: &RuntimeState,
    position: WorldCoord,
) -> Option<MaterialSurfaceId> {
    state.material_surfaces.keys().copied().min_by_key(|id| {
        let surface_position = WorldCoord::new(
            i64::from(id.chunk.chunk.x),
            i64::from(id.chunk.chunk.y),
            i64::from(id.chunk.chunk.z),
        );
        (
            position.x.abs_diff(surface_position.x)
                + position.y.abs_diff(surface_position.y)
                + position.z.abs_diff(surface_position.z),
            *id,
        )
    })
}

pub(crate) fn material_surface_physical_signals(
    state: &RuntimeState,
    time: SimulationTime,
) -> Vec<PhysicalSignal> {
    const MATERIAL_SURFACE_SIGNAL_GAIN: i64 = 16;
    if !state.config.material_surface_signals_enabled {
        return Vec::new();
    }
    state
        .material_surfaces
        .iter()
        .filter(|(_, surface)| surface.contact_count > 0)
        .map(|(id, surface)| {
            PhysicalSignal::new(
                EntityId::new(material_surface_object_id(*id)),
                SignalChannelId::new(
                    SensorKindId::new(1)
                        .raw()
                        .saturating_add(ACTOR_SIGNAL_CHANNEL),
                ),
                WorldCoord::new(
                    i64::from(id.chunk.chunk.x),
                    i64::from(id.chunk.chunk.y),
                    i64::from(id.chunk.chunk.z),
                ),
                SignalMagnitude::new(
                    surface
                        .condition
                        .saturating_mul(MATERIAL_SURFACE_SIGNAL_GAIN),
                ),
                time,
                surface.last_transition,
            )
        })
        .collect()
}

pub(crate) fn cell_object_id(chunk: ChartChunkCoord, cell_index: u16) -> u64 {
    chart_chunk_hash(chunk) ^ u64::from(cell_index)
}
