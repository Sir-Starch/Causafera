//! The digest schema 9 field inventory, held to the code that implements it.
//!
//! `docs/rfc/RFC-PERSIST-002.md` ("Schema 9 Field Inventory") classifies every
//! bounded, history-shaped field of `RuntimeState` and `HydrologyRuntimeState`
//! as exactly one of three things. This module is the executable mirror of that
//! table: it fails when a field is added without a class, when the RFC and the
//! code disagree, when the digest source writes a field its class forbids, and
//! when a classified field does not behave as classified.
//!
//! See `plans/causal-history-storage.md` Todo 7.

use std::collections::BTreeMap;

/// What a field is to the schema 9 digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldClass {
    /// Changes future physical evolution or identifies current physical state.
    /// Written by `physical_state_digest` under every schema.
    Physical,
    /// A bounded window of history whose only readers are Explanation, the
    /// observer and snapshot export. Written by the schema-8 physical digest
    /// only; under schema 9 its records are durable evidence in the stream the
    /// history digest folds.
    EvidenceOnly,
    /// Not written by the physical digest under either schema: configuration,
    /// scratch, caches and counters.
    NotDigested,
}

impl FieldClass {
    const fn rfc_label(self) -> &'static str {
        match self {
            Self::Physical => "Physical",
            Self::EvidenceOnly => "Evidence-only",
            Self::NotDigested => "Not digested",
        }
    }
}

use FieldClass::{EvidenceOnly, NotDigested, Physical};

/// Every `RuntimeState` field.
const RUNTIME_STATE_FIELDS: &[(&str, FieldClass)] = &[
    ("config", NotDigested),
    ("traces", NotDigested),
    ("history", NotDigested),
    ("tick_history", NotDigested),
    ("tick_undo", NotDigested),
    ("mana", Physical),
    ("thermal_fields", Physical),
    ("thermal_active_region", Physical),
    ("thermal_boundary_records", EvidenceOnly),
    ("thermal_reservoirs", Physical),
    ("thermal_parameters", Physical),
    ("pending_thermal_injections", NotDigested),
    ("hydrology", Physical),
    ("thermal_receipts", EvidenceOnly),
    ("thermal_conservation_receipts", EvidenceOnly),
    ("resolution", Physical),
    ("resolution_policy", NotDigested),
    ("carrier_adapters", NotDigested),
    ("active_chunks", Physical),
    ("actors", Physical),
    ("actor_ancestry", Physical),
    ("actor_objects", Physical),
    ("population_aggregates", Physical),
    ("aggregate_actor_pool", Physical),
    ("bootstrap", Physical),
    ("actor_action_bounds", NotDigested),
    ("pending_samples", NotDigested),
    ("pattern_history", Physical),
    ("material_surfaces", Physical),
    ("pending_material_surface_changes", Physical),
    ("material_surface_transitions", EvidenceOnly),
    ("material_surface_gate_transitions", EvidenceOnly),
    ("material_surface_thermal_transitions", EvidenceOnly),
    ("latest_physical_trace", NotDigested),
    ("latest_mana_trace", NotDigested),
    ("executed_experiment_recipe_mana_sources", Physical),
    ("advanced_through", Physical),
    ("physical_events", NotDigested),
    ("mana_cell_changes", NotDigested),
    ("mana_physical_effects", NotDigested),
    ("resolution_changes", NotDigested),
    ("resolution_transitions", NotDigested),
    ("perceived_actor_features", NotDigested),
    ("subjective_actor_objects", NotDigested),
    ("actor_actions_committed", NotDigested),
    ("actor_actions_rejected", NotDigested),
    ("population_births", NotDigested),
    ("population_deaths", NotDigested),
    ("population_movements", NotDigested),
    ("actor_promotions", NotDigested),
    ("actor_demotions", NotDigested),
    ("material_activity_events", NotDigested),
    ("next_actor_id", NotDigested),
    ("last_mana_changes", NotDigested),
    ("physical_scan_count", NotDigested),
    ("failure", NotDigested),
];

/// Every `HydrologyRuntimeState` field.
const HYDROLOGY_FIELDS: &[(&str, FieldClass)] = &[
    ("enabled", Physical),
    ("fields", Physical),
    ("conveyance", Physical),
    ("boundaries", Physical),
    ("metrics", Physical),
    ("active", NotDigested),
    ("resolution", Physical),
    ("resolution_policy", Physical),
    ("forcing", Physical),
    ("registry", Physical),
    ("next_node_id", Physical),
    ("receipts", NotDigested),
    ("conservation_receipts", EvidenceOnly),
    ("retained_batches", EvidenceOnly),
];

/// Fields the physical digest reaches without naming `self.<field>`: the
/// completed time is passed in by the caller, and the hydrology state is
/// written by its own function.
const REACHED_INDIRECTLY: &[&str] = &["advanced_through"];

#[cfg(test)]
#[path = "../tests/support_hydrology.rs"]
mod support_hydrology;

const RUNTIME_SOURCE: &str = include_str!("runtime.rs");
const DIGESTS_SOURCE: &str = include_str!("digests.rs");
const HYDROLOGY_SOURCE: &str = include_str!("hydrology.rs");
const RFC: &str = include_str!("../../../docs/rfc/RFC-PERSIST-002.md");

/// The field names declared in `pub struct <name> { ... }`.
fn declared_fields(source: &str, name: &str) -> Vec<String> {
    let header = format!("pub struct {name} {{");
    let body = source
        .split_once(&header)
        .unwrap_or_else(|| panic!("{name} is declared in the inspected source"))
        .1;
    let body = &body[..body.find("\n}\n").expect("the struct body ends")];
    body.lines()
        .map(str::trim)
        .filter_map(|line| {
            let line = line
                .strip_prefix("pub(crate) ")
                .or_else(|| line.strip_prefix("pub "))?;
            let (field, _) = line.split_once(':')?;
            Some(field.trim().to_owned())
        })
        .collect()
}

/// `text` mentions `<prefix><field>` as a whole identifier.
fn mentions(text: &str, prefix: &str, field: &str) -> bool {
    let needle = format!("{prefix}{field}");
    text.match_indices(&needle).any(|(index, _)| {
        text[index + needle.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || next == '_'))
    })
}

/// A function body, from its signature to the first line equal to `close`.
fn function_body<'a>(source: &'a str, signature: &str, close: &str) -> &'a str {
    let body = source
        .split_once(signature)
        .unwrap_or_else(|| panic!("`{signature}` is in the inspected source"))
        .1;
    let end = body
        .find(close)
        .unwrap_or_else(|| panic!("`{signature}` has a closing line"));
    &body[..end]
}

/// The text of every `if <condition> { ... }` block in `body`, and the rest.
fn split_gated(body: &str, condition: &str) -> (String, String) {
    let mut gated = String::new();
    let mut ungated = String::new();
    let mut closing: Option<String> = None;
    for line in body.lines() {
        match &closing {
            Some(close) => {
                if line == close {
                    closing = None;
                } else {
                    gated.push_str(line);
                    gated.push('\n');
                }
            }
            None => {
                if line.trim_start().starts_with(&format!("if {condition} {{")) {
                    let indent = &line[..line.len() - line.trim_start().len()];
                    closing = Some(format!("{indent}}}"));
                } else {
                    ungated.push_str(line);
                    ungated.push('\n');
                }
            }
        }
    }
    (gated, ungated)
}

fn rfc_inventory(heading: &str) -> BTreeMap<String, String> {
    let section = RFC
        .split_once(heading)
        .unwrap_or_else(|| panic!("the RFC has a `{heading}` section"))
        .1;
    let section = section.split("\n#").next().expect("section ends");
    section
        .lines()
        .filter_map(|line| {
            let mut cells = line.split('|').map(str::trim);
            cells.next()?;
            let field = cells.next()?.strip_prefix('`')?.strip_suffix('`')?;
            Some((field.to_owned(), cells.next()?.to_owned()))
        })
        .collect()
}

#[test]
fn every_runtime_state_field_has_a_class() {
    let declared = declared_fields(RUNTIME_SOURCE, "RuntimeState");
    let classified = RUNTIME_STATE_FIELDS
        .iter()
        .map(|(field, _)| (*field).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        declared, classified,
        "RuntimeState and its schema 9 digest inventory list different fields; classify the new \
         field here and in RFC-PERSIST-002 before it can reach a snapshot"
    );
    let declared = declared_fields(HYDROLOGY_SOURCE, "HydrologyRuntimeState");
    let classified = HYDROLOGY_FIELDS
        .iter()
        .map(|(field, _)| (*field).to_owned())
        .collect::<Vec<_>>();
    assert_eq!(declared, classified);
}

#[test]
fn the_rfc_table_is_the_inventory() {
    for (heading, fields, prefix) in [
        ("### Schema 9 Field Inventory", RUNTIME_STATE_FIELDS, ""),
        (
            "### Schema 9 Hydrology Field Inventory",
            HYDROLOGY_FIELDS,
            "",
        ),
    ] {
        let table = rfc_inventory(heading);
        let code = fields
            .iter()
            .map(|(field, class)| (format!("{prefix}{field}"), class.rfc_label().to_owned()))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            table, code,
            "the `{heading}` table in RFC-PERSIST-002 and digest_inventory.rs disagree"
        );
    }
}

#[test]
fn the_physical_digest_writes_exactly_the_classified_fields() {
    let body = function_body(
        RUNTIME_SOURCE,
        "fn physical_state_digest_for_schema(",
        "\n    }\n",
    );
    let (legacy_only, every_schema) = split_gated(body, "windows_in_physical_digest");
    for (field, class) in RUNTIME_STATE_FIELDS {
        let always = mentions(&every_schema, "self.", field);
        let legacy = mentions(&legacy_only, "self.", field);
        match class {
            Physical if REACHED_INDIRECTLY.contains(field) => {}
            Physical => {
                assert!(always, "{field} is Physical but the digest never writes it");
                assert!(
                    !legacy,
                    "{field} is Physical but only the legacy block writes it"
                );
            }
            EvidenceOnly => {
                assert!(
                    legacy,
                    "{field} is Evidence-only but schema 8 never wrote it"
                );
                assert!(
                    !always,
                    "{field} is Evidence-only but the schema 9 digest still writes it"
                );
            }
            NotDigested => assert!(
                !always && !legacy,
                "{field} is Not digested but the physical digest writes it"
            ),
        }
    }

    let hydrology = function_body(
        DIGESTS_SOURCE,
        "pub(crate) fn write_hydrology_state(",
        "\n}\n",
    );
    let (ledgers, always) = split_gated(hydrology, "retained_batch_ledgers");
    for (field, class) in HYDROLOGY_FIELDS {
        let in_ledgers = mentions(&ledgers, "hydrology.", field);
        let in_always = mentions(&always, "hydrology.", field);
        match class {
            Physical => assert!(in_always, "hydrology.{field} is Physical but never written"),
            EvidenceOnly => assert!(
                in_ledgers && !in_always,
                "hydrology.{field} is Evidence-only but is written outside the legacy ledgers"
            ),
            NotDigested => assert!(
                !in_ledgers && !in_always,
                "hydrology.{field} is Not digested but the digest writes it"
            ),
        }
    }
}

mod behavior {
    use causafera_core::{
        CausalEffect, CausalEventProposal, CausalTarget, Criticality, DomainEvidenceRecord,
        EventProposalKey, EvidenceSchemaId, EvidenceSchemaVersion, Phase, StateFingerprint,
    };
    use causafera_geography::MAX_HYDROLOGY_SECTION_BYTES;
    use causafera_persistence::MAX_TOTAL_SIZE;
    use causafera_types::{
        EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
    };

    use crate::{
        CURRENT_DIGEST_SCHEMA_VERSION, LEGACY_DIGEST_SCHEMA_VERSION, Runtime, RuntimeConfig,
        RuntimeState, RuntimeTickTransaction,
    };

    /// Empties one evidence-only window and says whether it held anything.
    type ClearWindow = fn(&mut RuntimeState) -> bool;

    fn physical_pair(state: &RuntimeState, time: SimulationTime) -> ([u8; 32], [u8; 32]) {
        (
            state.physical_state_digest(time).bytes(),
            state
                .physical_state_digest_for_schema(time, LEGACY_DIGEST_SCHEMA_VERSION)
                .bytes(),
        )
    }

    /// Run `ticks` ticks of `config` and keep the session.
    fn session(config: RuntimeConfig, ticks: u64) -> Runtime {
        let mut runtime = Runtime::new(config).expect("the session constructs");
        runtime.run_ticks(ticks).expect("the session runs");
        runtime
    }

    #[test]
    fn evidence_only_windows_leave_the_schema_9_physical_digest_alone() {
        // Given: a session whose thermal and material windows are populated.
        let runtime = session(crate::benchmark::production_loop_config(1_705), 40);
        let mut state = runtime.lock_state().expect("state");
        let time = state.advanced_through;
        assert!(!state.thermal_receipts.is_empty());
        assert!(!state.thermal_conservation_receipts.is_empty());
        assert!(!state.thermal_boundary_records.is_empty());
        let (schema_9, schema_8) = physical_pair(&state, time);
        let history = state.history_digest();

        // When: each window is emptied, as eviction would.
        state.thermal_receipts.clear();
        state.thermal_conservation_receipts.clear();
        state.thermal_boundary_records.clear();
        state.material_surface_transitions.clear();
        state.material_surface_gate_transitions.clear();
        state.material_surface_thermal_transitions.clear();

        // Then: schema 9 does not move, schema 8 did digest them, and the
        // history digest, which is read from the stream, is untouched.
        let (after_9, after_8) = physical_pair(&state, time);
        assert_eq!(schema_9, after_9);
        assert_ne!(schema_8, after_8);
        assert_eq!(history, state.history_digest());
    }

    #[test]
    fn each_evidence_only_window_is_individually_outside_the_schema_9_digest() {
        let runtime = session(crate::benchmark::production_loop_config(1_705), 40);
        let mut state = runtime.lock_state().expect("state");
        let time = state.advanced_through;
        let windows: [(&str, ClearWindow); 6] = [
            ("thermal_receipts", |s| {
                let had = !s.thermal_receipts.is_empty();
                s.thermal_receipts.clear();
                had
            }),
            ("thermal_conservation_receipts", |s| {
                let had = !s.thermal_conservation_receipts.is_empty();
                s.thermal_conservation_receipts.clear();
                had
            }),
            ("thermal_boundary_records", |s| {
                let had = !s.thermal_boundary_records.is_empty();
                s.thermal_boundary_records.clear();
                had
            }),
            ("material_surface_transitions", |s| {
                let had = !s.material_surface_transitions.is_empty();
                s.material_surface_transitions.clear();
                had
            }),
            ("material_surface_gate_transitions", |s| {
                let had = !s.material_surface_gate_transitions.is_empty();
                s.material_surface_gate_transitions.clear();
                had
            }),
            ("material_surface_thermal_transitions", |s| {
                let had = !s.material_surface_thermal_transitions.is_empty();
                s.material_surface_thermal_transitions.clear();
                had
            }),
        ];
        let mut populated = Vec::new();
        for (name, clear) in windows {
            let (before_9, before_8) = physical_pair(&state, time);
            let had_entries = clear(&mut state);
            let (after_9, after_8) = physical_pair(&state, time);
            assert_eq!(
                before_9, after_9,
                "{name} moved the schema 9 physical digest"
            );
            if had_entries {
                populated.push(name);
                assert_ne!(
                    before_8, after_8,
                    "{name} had entries but schema 8 did not digest them"
                );
            }
        }
        println!("MARKER:digest_inventory populated_windows={populated:?}");
        assert!(
            populated.len() >= 3,
            "the session must populate at least the thermal windows, populated {populated:?}"
        );
    }

    #[test]
    fn evicting_hydrology_ledgers_leaves_the_schema_9_physical_digest_alone() {
        // Given: a wet world that has committed batches.
        let runtime = session(super::support_hydrology::wet_runtime_config(), 3);
        let mut state = runtime.lock_state().expect("state");
        let time = state.advanced_through;
        assert!(!state.hydrology.retained_batches.is_empty());
        assert!(!state.hydrology.conservation_receipts.is_empty());
        let (schema_9, schema_8) = physical_pair(&state, time);

        // When: the retained-batch window is emptied.
        state.hydrology.retained_batches.clear();
        state.hydrology.conservation_receipts.clear();
        state.hydrology.receipts.clear();

        // Then: only the schema-8 digest, which wrote the ledgers, moves.
        let (after_9, after_8) = physical_pair(&state, time);
        assert_eq!(schema_9, after_9);
        assert_ne!(schema_8, after_8);
    }

    #[test]
    fn future_physical_state_moves_the_physical_digest() {
        // Given: one session, and a donor of the same world a while later.
        let runtime = session(crate::benchmark::production_loop_config(55), 16);
        let donor = session(crate::benchmark::production_loop_config(55), 40);
        let mut state = runtime.lock_state().expect("state");
        let donor = donor.lock_state().expect("donor");
        let time = state.advanced_through;

        // When each current-state field is replaced by the donor's value, the
        // physical digest names the difference.
        macro_rules! replace_moves_digest {
            ($($field:ident),+ $(,)?) => {$(
                {
                    let before = state.physical_state_digest(time);
                    state.$field = donor.$field.clone();
                    assert_ne!(
                        before,
                        state.physical_state_digest(time),
                        concat!(
                            stringify!($field),
                            " does not move the physical digest (or the donor's value is equal)"
                        )
                    );
                }
            )+};
        }
        replace_moves_digest!(
            mana,
            pattern_history,
            active_chunks,
            resolution,
            material_surfaces,
        );

        // The completed time is the caller's argument.
        assert_ne!(
            state.physical_state_digest(time),
            state.physical_state_digest(SimulationTime::new(time.raw() + 1))
        );
    }

    fn handoff_cause(cause: TraceId) -> CausalEventProposal {
        CausalEventProposal::new(
            EventProposalKey::new(0x7e57, 0, 0),
            EventKindId::new(crate::ROOT_EVENT_KIND),
            vec![cause],
            vec![
                CausalEffect::new(
                    CausalTarget::new(
                        StateObjectKindId::new(crate::RUNTIME_OBJECT_KIND),
                        0x7e57,
                        StatePropertyId::new(crate::ROOT_PROPERTY),
                    ),
                    StateFingerprint::new([1; 32]),
                    StateFingerprint::new([2; 32]),
                )
                .expect("effect is canonical"),
            ],
        )
        .expect("proposal is canonical")
    }

    fn state_with_one_evidence_record(payload: &[u8]) -> Runtime {
        let runtime = Runtime::new(RuntimeConfig::new(2_024)).expect("construction");
        {
            let mut state = runtime.lock_state().expect("state");
            let transaction = RuntimeTickTransaction::open(
                &mut state,
                MAX_TOTAL_SIZE,
                MAX_HYDROLOGY_SECTION_BYTES,
            )
            .expect("transaction opens");
            let cause = state.latest_physical_trace;
            let anchor = state
                .traces
                .commit_batch(
                    SimulationTime::new(0),
                    Phase::Physics,
                    vec![handoff_cause(cause)],
                )
                .expect("the cause commits")[0];
            let record = DomainEvidenceRecord::new(
                anchor.raw(),
                EvidenceSchemaId::new(0x7f10),
                EvidenceSchemaVersion::new(1, 0),
                Criticality::ExplanationOnly,
                SimulationTime::new(0),
                vec![anchor],
                b"inventory/evidence".to_vec(),
                payload.to_vec(),
                StateFingerprint::new(*blake3::hash(payload).as_bytes()),
            )
            .expect("record is canonical");
            state
                .stage_history_evidence(std::slice::from_ref(&record))
                .expect("staging succeeds");
            let completed = state.advanced_through;
            transaction
                .close(&mut state, completed)
                .expect("tick publishes");
        }
        runtime
    }

    #[test]
    fn a_changed_evidence_payload_moves_history_and_experiment_identity_only() {
        // Given: two sessions whose physical state and causal events are equal
        // and whose single evidence record differs in one payload byte.
        let first = state_with_one_evidence_record(b"payload-a");
        let second = state_with_one_evidence_record(b"payload-b");
        let first = first.lock_state().expect("state");
        let second = second.lock_state().expect("state");
        let time = first.advanced_through;

        // Then: the physical digest is equal, the history digest and the
        // experiment digest are not, and each history digest is its own cold
        // rescan.
        assert_eq!(
            first.physical_state_digest(time),
            second.physical_state_digest(time)
        );
        assert_ne!(first.history_digest(), second.history_digest());
        assert_ne!(
            first.canonical_state(first.physical_state_digest(time), first.history_digest()),
            second.canonical_state(second.physical_state_digest(time), second.history_digest())
        );
        for state in [&first, &second] {
            assert_eq!(
                state.history_digest(),
                state.history_digest_full_rescan().expect("cold rescan")
            );
        }
    }

    #[test]
    fn the_experiment_digest_names_the_world_and_the_recipe() {
        let runtime = session(RuntimeConfig::new(7), 2);
        let state = runtime.lock_state().expect("state");
        let physical = state.physical_state_digest(state.advanced_through);
        let history = state.history_digest();
        let experiment = |seed: u64, recipe: [u8; 32], schema| {
            RuntimeState::canonical_state_for_schema(seed, recipe, physical, history, schema)
        };
        let current = CURRENT_DIGEST_SCHEMA_VERSION;
        let base = experiment(7, [1; 32], current);
        assert_ne!(
            base,
            experiment(8, [1; 32], current),
            "the seed is identity"
        );
        assert_ne!(
            base,
            experiment(7, [2; 32], current),
            "the recipe is identity"
        );
        // The legacy fold never included either, and still must not.
        assert_eq!(
            experiment(7, [1; 32], LEGACY_DIGEST_SCHEMA_VERSION),
            experiment(8, [2; 32], LEGACY_DIGEST_SCHEMA_VERSION)
        );
    }
}
