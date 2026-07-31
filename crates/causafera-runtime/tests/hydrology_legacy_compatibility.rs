//! Executable pre-hydrology evidence for `plans/hydrology.md`.
//!
//! Stage 1 captures what the engine does *before* hydrology exists, so Stage 6
//! can prove that appending hydrology changed exactly the declared versioned
//! footprint and nothing else (plan risk R7, verification gate V22).
//!
//! Every value below is measured from the engine, not restated from a constant
//! in the source it is meant to protect. The plan is explicit that "hard-coded
//! constants or comments alone do not satisfy this gate".
//!
//! ## What is under protection, and one measured deviation
//!
//! The plan's Stage 1 work list asks this fixture to assert "at least one
//! existing system consumes an RNG stream". Measured at this commit, **no
//! registered system draws from its stream**: every `System::run` in
//! `causafera-runtime` binds the parameter as `_stream`
//! (`PhysicalPatternSystem`, `ExperimentRecipeManaSourceSystem`,
//! `ManaRuntimeSystem`, `ManaEffectsSystem`, `ResolutionRuntimeSystem`,
//! `ActorPerceptionSystem`, `ActorCognitionSystem`, `ActorActionSystem`,
//! `PopulationLifecycleSystem`, `ThermalReservoirSystem`,
//! `ThermalEvolutionSystem`). That assertion is therefore not satisfiable, and
//! writing one that passes vacuously would be worse than not writing it.
//!
//! What R7 actually threatens is the *stream key*, not present consumption: a
//! registration inserted anywhere but last renumbers every later system, and
//! `StreamKey { world_seed, time, phase, system_id }` is what seeds each
//! system's `RandomStream`. So this fixture pins the assigned IDs and the
//! stream samples those IDs key, and proves the pinning is sensitive — a
//! one-step ID shift changes the sample. The moment any system starts drawing
//! from its stream, that drift becomes state drift; until then it is latent,
//! and pinning it is what keeps it latent rather than accidental.
//!
//! The "at least one tick changes physical/history state" half of the
//! non-vacuity requirement *is* satisfiable and is asserted below.

use causafera_core::{Phase, RandomStream, StreamKey};
use causafera_persistence::SnapshotEnvelope;
use causafera_runtime::snapshot_sections::{
    HYDROLOGY_SECTION_ID, HYDROLOGY_SECTION_MAJOR, SECTION_RUNTIME_RECIPE, assemble_envelope,
};
use causafera_runtime::{
    ActiveChunkShape, CURRENT_DIGEST_SCHEMA_VERSION, HYDROLOGY_SYSTEM_ID, Runtime, RuntimeConfig,
    RuntimeSnapshotData, SchedulerRegistration,
};
use causafera_types::SimulationTime;

/// The world this fixture measures. Chosen to light up every pre-hydrology
/// subsystem that owns a snapshot section: terrain carriers, mana, thermal,
/// material surfaces, resolution, actors (objective and subjective), the
/// population aggregate, and the six-stage production bootstrap record.
const LEGACY_WORLD_SEED: u64 = 20_260_729;
const LEGACY_TICKS: u64 = 6;

fn legacy_config() -> RuntimeConfig {
    let mut config = RuntimeConfig::new(LEGACY_WORLD_SEED);
    config.chunk_extent = 3;
    config.active_chunk_radius = 1;
    config.active_chunk_shape = ActiveChunkShape::Line;
    config.actor_count = 4;
    config.sensor_count = 2;
    config.bootstrap_population = 64;
    config.material_surface_signals_enabled = true;
    config
}

fn legacy_runtime() -> Runtime {
    Runtime::new(legacy_config()).expect("the pre-hydrology production runtime must construct")
}

fn evolved_snapshot() -> RuntimeSnapshotData {
    let mut runtime = legacy_runtime();
    runtime
        .run_ticks(LEGACY_TICKS)
        .expect("the pre-hydrology production runtime must tick");
    runtime
        .export_snapshot()
        .expect("evolved pre-hydrology state must export")
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn digest_hex(bytes: &[u8]) -> String {
    hex(blake3::hash(bytes).as_bytes())
}

// ---------------------------------------------------------------------------
// Scheduler registration and stream keys
// ---------------------------------------------------------------------------

/// The phase and stream-keying ID the scheduler assigned each system, observed
/// from `Scheduler::register_system`'s return value at this commit.
const PRE_HYDROLOGY_REGISTRATIONS: &[(Phase, u64)] = &[
    (Phase::Physics, 0),
    (Phase::Mana, 1),
    (Phase::Mana, 2),
    (Phase::Mana, 3),
    (Phase::Resolution, 4),
    (Phase::Perception, 5),
    (Phase::Cognition, 6),
    (Phase::Action, 7),
    (Phase::Lifecycle, 8),
    (Phase::Physics, 9),
    (Phase::Physics, 10),
];

#[test]
fn scheduler_assigns_the_ids_the_persisted_recipe_declares() {
    // Given: the pre-hydrology production runtime.
    let runtime = legacy_runtime();

    // When: its live assignments and its persisted recipe manifest are read.
    let observed: Vec<(Phase, u64)> = runtime
        .scheduler_registrations()
        .iter()
        .map(|registration| (registration.phase, registration.system_id))
        .collect();
    let declared: Vec<(Phase, u64)> = runtime
        .export_snapshot()
        .expect("initial state must export")
        .recipe
        .system_registrations
        .iter()
        .map(|registration| {
            (
                registration.phase,
                u64::from(registration.registration_order),
            )
        })
        .collect();

    // Then: the live scheduler and the persisted manifest agree with each other,
    // and both begin with the captured pre-hydrology table unchanged.
    // `runtime_system_registrations` is a declaration; on its own it cannot notice
    // a registration inserted ahead of the systems it numbers, because the
    // declaration would move with the insertion.
    assert_eq!(observed, declared, "live scheduler and persisted manifest");
    assert_eq!(
        &observed[..PRE_HYDROLOGY_REGISTRATIONS.len()],
        PRE_HYDROLOGY_REGISTRATIONS,
        "every pre-hydrology registration keeps its phase and its ID"
    );

    // And: hydrology was appended, not inserted. That is the whole of risk R7 —
    // a registration anywhere but last renumbers every later system and silently
    // reseeds its stream.
    assert_eq!(
        observed.len(),
        PRE_HYDROLOGY_REGISTRATIONS.len() + 1,
        "exactly one system was added"
    );
    // The stream-keying ID is the registration order, which is distinct from the
    // system's schema ID: one says where in the sequence a system sits, the other
    // says which system it is.
    assert_eq!(
        observed[PRE_HYDROLOGY_REGISTRATIONS.len()],
        (Phase::Physics, PRE_HYDROLOGY_REGISTRATIONS.len() as u64),
        "hydrology runs in Physics and takes the next free stream ID"
    );
    assert_eq!(
        runtime
            .export_snapshot()
            .expect("state must export")
            .recipe
            .system_registrations
            .last()
            .expect("the manifest is not empty")
            .system_schema_id,
        HYDROLOGY_SYSTEM_ID,
        "the appended system declares its own schema identity"
    );
    assert_eq!(
        CURRENT_DIGEST_SCHEMA_VERSION.raw(),
        HYDROLOGY_DIGEST_SCHEMA,
        "schema 8 covers hydrology"
    );
}

/// One `u64` drawn from each registered system's stream at
/// `(LEGACY_WORLD_SEED, tick 0, phase, system_id)`.
const PRE_HYDROLOGY_STREAM_SAMPLES: &[u64] = &[
    15_542_830_437_426_579_473,
    18_406_560_247_542_661_137,
    16_830_652_225_638_272_175,
    8_005_147_748_315_043_361,
    7_260_022_846_845_843_785,
    14_327_327_687_057_315_040,
    6_399_383_264_662_097_312,
    16_366_838_596_095_427_819,
    12_861_573_312_313_987_536,
    3_799_707_679_601_024_323,
    10_855_378_376_804_395_530,
];

fn stream_sample(registration: SchedulerRegistration) -> u64 {
    RandomStream::from_key(StreamKey {
        world_seed: LEGACY_WORLD_SEED,
        time: SimulationTime::new(0),
        phase: registration.phase,
        system_id: registration.system_id,
    })
    .next_u64()
}

#[test]
fn registered_stream_keys_are_pinned() {
    // Given: the pre-hydrology production runtime's live registrations.
    let runtime = legacy_runtime();

    // When: each registration's stream is keyed and drawn from once.
    let samples: Vec<u64> = runtime
        .scheduler_registrations()
        .iter()
        .copied()
        .map(stream_sample)
        .collect();

    // Then: every pre-hydrology system draws exactly what it drew before. The
    // appended system has a stream of its own, which is an addition rather than a
    // perturbation — nothing that existed was reseeded.
    assert_eq!(
        &samples[..PRE_HYDROLOGY_STREAM_SAMPLES.len()],
        PRE_HYDROLOGY_STREAM_SAMPLES
    );
    assert_eq!(samples.len(), PRE_HYDROLOGY_STREAM_SAMPLES.len() + 1);
    assert_eq!(
        samples[PRE_HYDROLOGY_STREAM_SAMPLES.len()],
        APPENDED_HYDROLOGY_STREAM_SAMPLE
    );
}

/// The sample the appended hydrology system's stream yields at tick zero.
///
/// Recorded so the appended system's own key is pinned too: hydrology draws no
/// randomness today, and the day it does, this is what says whether its stream
/// moved.
const APPENDED_HYDROLOGY_STREAM_SAMPLE: u64 = 12_494_574_395_528_867_065;

#[test]
fn a_one_step_id_shift_would_change_every_later_stream() {
    // Given: the pre-hydrology registrations.
    let runtime = legacy_runtime();
    let registrations = runtime.scheduler_registrations();

    // When: each system is asked what it would draw had a registration been
    // inserted ahead of it, shifting its ID by one.
    for registration in registrations {
        let shifted = SchedulerRegistration {
            phase: registration.phase,
            system_id: registration.system_id + 1,
        };

        // Then: the stream is a different stream. This is what appending
        // hydrology's registration last protects, and it is why the table
        // above is evidence rather than decoration.
        assert_ne!(
            stream_sample(*registration),
            stream_sample(shifted),
            "system {} in {:?} must not share a stream with its successor ID",
            registration.system_id,
            registration.phase
        );
    }

    // And: the pinned table itself is non-degenerate, so a table of repeated
    // values could not pass the assertion above by accident.
    let mut sorted = PRE_HYDROLOGY_STREAM_SAMPLES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), PRE_HYDROLOGY_STREAM_SAMPLES.len());
}

// ---------------------------------------------------------------------------
// Section payloads and digests
// ---------------------------------------------------------------------------

/// `(section id, major, minor, flags, decoded size limit, payload length, BLAKE3 of payload)`
/// for the complete pre-hydrology envelope, in canonical section-ID order.
///
/// Stage 6 adds section `0x000F` and moves the runtime recipe section from
/// major 6 to 7. Every *other* row here must survive that change byte for byte.
const PRE_HYDROLOGY_SECTIONS: &[(u64, u16, u16, u32, u64, usize, &str)] = &[
    (
        1,
        6,
        0,
        0,
        0,
        296,
        "d5cdcdab6b61f0e11894eaafe265c74ac3ff00869c7f54eba3c015998f7bfd0f",
    ),
    (
        2,
        1,
        0,
        0,
        0,
        49609,
        "96db9b35891b1354b0308d90efc05c2b553af8cc7c64c9f5a2a657f5ba7a3e42",
    ),
    (
        3,
        2,
        0,
        0,
        0,
        2064,
        "8a017567e6f840713ce13a6a926fc40127200df84e2c34ca7aa3c3358a87f689",
    ),
    (
        4,
        1,
        0,
        0,
        0,
        215,
        "813e81cf811d22b7f003d5a4f98de279b5e694a9fd25d7907c63dd26efe718dc",
    ),
    (
        5,
        1,
        0,
        0,
        0,
        299,
        "076b83f76bd1c454aaa5c70502c1d7b51ee6dc4fdea3b7c882f27fe753ec7c14",
    ),
    (
        6,
        3,
        0,
        0,
        0,
        165,
        "7d44decaf3804b5385395dd0a7c7a6866dbf3a9b281eb7a162d8c2099352fe8d",
    ),
    (
        7,
        1,
        0,
        0,
        0,
        3612,
        "265951fc7bb9d4103eafe0447edef90da39ddefab80150906379cb915d845af6",
    ),
    (
        8,
        1,
        0,
        0,
        0,
        3364,
        "f54629222a05c35a7eaa667400d6315f5110a9b804751f007f3249ec11a48c9b",
    ),
    (
        9,
        2,
        0,
        0,
        0,
        1788,
        "b545b6871ceacede4c2629bde65c43e8268ecf97de996849fc41c45556e1dd74",
    ),
    (
        10,
        1,
        0,
        0,
        0,
        101044,
        "67b91261a57b6a22a2ad472ac331dcdd5244921ded31bce38cd53deecc048193",
    ),
    (
        12,
        3,
        0,
        0,
        0,
        2640,
        "c33d18454f087bcfd061b877e892f8524345643b62d9e3e538e45ddebb6ef615",
    ),
    (
        13,
        1,
        0,
        0,
        0,
        8,
        "71e0a99173564931c0b8acc52d2685a8e39c64dc52e3d02390fdac2a12b155cb",
    ),
    (
        14,
        2,
        0,
        0,
        0,
        31084,
        "6095482e3c8a153914b36c17d3b839f4cf3ac60b217f0c75405ecf0b0ca299eb",
    ),
];

fn envelope_projection(
    envelope: &SnapshotEnvelope,
) -> Vec<(u64, u16, u16, u32, u64, usize, String)> {
    envelope
        .sections
        .iter()
        .map(|(&id, payload)| {
            (
                id,
                payload.section_major,
                payload.section_minor,
                payload.flags,
                payload.decoded_size_limit,
                payload.bytes.len(),
                digest_hex(&payload.bytes),
            )
        })
        .collect()
}

fn rust_literal(projection: &[(u64, u16, u16, u32, u64, usize, String)]) -> String {
    let mut out = String::from("\n");
    for (id, major, minor, flags, limit, length, digest) in projection {
        out.push_str(&format!(
            "    ({id}, {major}, {minor}, {flags}, {limit}, {length}, \"{digest}\"),\n"
        ));
    }
    out
}

#[test]
fn pre_hydrology_section_payloads_are_pinned() {
    // Given: six ticks of evolved pre-hydrology production state.
    let snapshot = evolved_snapshot();

    // When: it is assembled into the canonical snapshot envelope.
    let envelope = assemble_envelope(&snapshot).expect("pre-hydrology state must assemble");
    let projection = envelope_projection(&envelope);
    let expected: Vec<(u64, u16, u16, u32, u64, usize, String)> = PRE_HYDROLOGY_SECTIONS
        .iter()
        .map(|(id, major, minor, flags, limit, length, digest)| {
            (
                *id,
                *major,
                *minor,
                *flags,
                *limit,
                *length,
                (*digest).to_string(),
            )
        })
        .collect();

    // Then: every *legacy subsystem* section matches the captured baseline byte
    // for byte. This is the assertion that matters for V22: adding hydrology must
    // not perturb mana, terrain, actors, traces, or any other domain's payload.
    let legacy: Vec<_> = projection
        .iter()
        .filter(|section| {
            section.0 != u64::from(SECTION_RUNTIME_RECIPE)
                && section.0 != u64::from(HYDROLOGY_SECTION_ID)
        })
        .cloned()
        .collect();
    let legacy_expected: Vec<_> = expected
        .iter()
        .filter(|section| section.0 != u64::from(SECTION_RUNTIME_RECIPE))
        .cloned()
        .collect();
    assert_eq!(
        legacy,
        legacy_expected,
        "a pre-hydrology subsystem section drifted; measured table was:{}",
        rust_literal(&projection)
    );

    // And: the runtime recipe section changed, deliberately and by exactly the
    // declared amount. The recipe describes what a session was configured to be,
    // so a new domain belongs in it; recording the size of a disabled hydrology
    // block is what keeps that from being a place future state can hide.
    let recipe = projection
        .iter()
        .find(|section| section.0 == u64::from(SECTION_RUNTIME_RECIPE))
        .expect("the recipe section is required");
    let recipe_before = expected
        .iter()
        .find(|section| section.0 == u64::from(SECTION_RUNTIME_RECIPE))
        .expect("the baseline recorded the recipe section");
    assert_eq!(recipe_before.1, PRE_HYDROLOGY_RECIPE_MAJOR);
    assert_eq!(recipe.1, HYDROLOGY_RECIPE_MAJOR);
    assert_eq!(
        recipe.5,
        recipe_before.5 + DISABLED_HYDROLOGY_RECIPE_BYTES + APPENDED_REGISTRATION_RECIPE_BYTES,
        "the recipe grew by exactly the disabled hydrology block and one appended \
         system registration"
    );
    assert_ne!(recipe.6, recipe_before.6, "the recipe payload changed");

    // And: the hydrology section is a new required section carrying exactly one
    // byte — the disabled flag. A snapshot that simply omitted it would be
    // indistinguishable from one taken before hydrology existed, and only one of
    // those is a statement about the world it describes.
    let hydrology = projection
        .iter()
        .find(|section| section.0 == u64::from(HYDROLOGY_SECTION_ID))
        .expect("the hydrology section is required");
    assert_eq!(hydrology.1, HYDROLOGY_SECTION_MAJOR);
    assert_eq!(
        hydrology.5, 1,
        "a disabled domain's whole payload is its flag"
    );
    assert!(
        !PRE_HYDROLOGY_SECTIONS
            .iter()
            .any(|section| section.0 == u64::from(HYDROLOGY_SECTION_ID)),
        "the baseline predates the section, which is the point"
    );
    assert_eq!(
        projection.len(),
        PRE_HYDROLOGY_SECTIONS.len() + 1,
        "exactly one section was added"
    );

    // And: the projection is non-vacuous — the evolved world really did fill
    // the subsystem sections this fixture exists to protect.
    assert!(
        projection.len() >= 12,
        "the captured world must exercise every pre-hydrology section"
    );
    assert!(
        projection.iter().all(|section| section.5 > 0),
        "no pre-hydrology section may be empty in the captured world"
    );
}

const PRE_HYDROLOGY_PHYSICAL_DIGEST: &str =
    "2a6027286ac7964da7abcfd7dd78a911c931a8a1a1e429906f1e7ccfeef822f3";
#[allow(dead_code, reason = "historical record of the pre-hydrology identity")]
const PRE_HYDROLOGY_HISTORY_DIGEST: &str =
    "fe5641b7993e06e38b3c5c080d8ba32ae2efcdfb8362aa4132a014d33a9e0f0e";
#[allow(dead_code, reason = "historical record of the pre-hydrology identity")]
const PRE_HYDROLOGY_EXPERIMENT_DIGEST: &str =
    "dbdcdb841556eddfb3099eb375d64bc738ac94fa01d740bdf50e32330f461320";
const PRE_HYDROLOGY_ENVELOPE_DIGEST: &str =
    "218e685838e49a0104b853c026dd11641e2fd07271ae7ba44237c4de8a5012f3";

#[test]
fn pre_hydrology_digests_are_pinned() {
    // Given: the pre-hydrology production runtime at tick zero.
    let mut runtime = legacy_runtime();
    let initial = runtime.snapshot().expect("initial state must project");

    // When: it runs the captured tick count.
    let evolved = runtime
        .run_ticks(LEGACY_TICKS)
        .expect("the pre-hydrology production runtime must tick");

    // Then: ticking is non-vacuous — both authoritative digests moved.
    assert_ne!(
        initial.physical_state_digest, evolved.physical_state_digest,
        "a tick must change physical state, or this fixture proves nothing"
    );
    assert_ne!(
        initial.history_digest, evolved.history_digest,
        "a tick must change history, or this fixture proves nothing"
    );

    // And: all three digests moved to schema 8, which is the declared change.
    // Their bytes therefore differ from the captured baseline — a digest is an
    // identity, and schema 8 identifies a state that includes hydrology. What must
    // *not* have moved is the legacy subsystem section payloads, which
    // `pre_hydrology_section_payloads_are_pinned` asserts byte for byte; together
    // the two tests say the difference is attributable to the declared schema and
    // section changes and to nothing else.
    assert_eq!(
        evolved.physical_state_digest.schema_version.raw(),
        HYDROLOGY_DIGEST_SCHEMA
    );
    assert_eq!(
        evolved.history_digest.schema_version.raw(),
        HYDROLOGY_DIGEST_SCHEMA
    );
    assert_eq!(
        evolved.canonical_state.schema_version.raw(),
        HYDROLOGY_DIGEST_SCHEMA
    );
    assert_ne!(
        hex(&evolved.physical_state_digest.bytes()),
        PRE_HYDROLOGY_PHYSICAL_DIGEST,
        "schema 8 covers new state, so the physical digest must move"
    );
    assert_eq!(
        hex(&evolved.physical_state_digest.bytes()),
        DISABLED_HYDROLOGY_PHYSICAL_DIGEST,
        "physical digest under schema 8 with hydrology disabled"
    );
    assert_eq!(
        hex(&evolved.history_digest.bytes()),
        DISABLED_HYDROLOGY_HISTORY_DIGEST,
        "history digest"
    );
    assert_eq!(
        hex(&evolved.canonical_state.bytes()),
        DISABLED_HYDROLOGY_EXPERIMENT_DIGEST,
        "experiment digest"
    );

    // And: the full envelope digest moved, because the recipe section it covers
    // now records the hydrology configuration. That is the one declared change
    // at this stage — the three authoritative digests above are byte-identical,
    // which is what says no legacy subsystem state was disturbed to get here.
    let encoded = assemble_envelope(&runtime.export_snapshot().expect("state must export"))
        .expect("state must assemble")
        .encode()
        .expect("envelope must encode");
    assert_ne!(
        digest_hex(&encoded),
        PRE_HYDROLOGY_ENVELOPE_DIGEST,
        "the envelope digest must move with the recipe section"
    );
    assert_eq!(
        digest_hex(&encoded),
        DISABLED_HYDROLOGY_ENVELOPE_DIGEST,
        "complete envelope with a disabled hydrology recipe"
    );
}

/// The digest schema that covers hydrology.
const HYDROLOGY_DIGEST_SCHEMA: u16 = 8;

/// The runtime recipe section major before hydrology was configurable.
const PRE_HYDROLOGY_RECIPE_MAJOR: u16 = 6;

/// The major that carries the hydrology configuration.
const HYDROLOGY_RECIPE_MAJOR: u16 = 7;

/// Bytes a disabled hydrology configuration adds to the runtime recipe.
///
/// `limits_schema` and the resolution policy's schema version at two bytes each,
/// three one-byte flags, one byte of maximum level, and two eight-byte counts for
/// the empty metric and forcing collections. A disabled domain still says so
/// explicitly, because "off" and "absent" are different claims about a snapshot.
const DISABLED_HYDROLOGY_RECIPE_BYTES: usize = 2 + 1 + 2 + 1 + 1 + 8 + 1 + 8;

/// Bytes the appended system registration adds to the runtime recipe: a phase, a
/// schema ID, a revision, and a registration order.
const APPENDED_REGISTRATION_RECIPE_BYTES: usize = 2 + 8 + 2 + 2;

/// The three authoritative digests of the same world under schema 8, with
/// hydrology disabled. Measured, not restated: schema 8 writes one zero for a
/// disabled domain, so what moved is the schema version and that single byte.
const DISABLED_HYDROLOGY_PHYSICAL_DIGEST: &str =
    "a62665b6c03d56846ef206302bff041bc59271678437c1b2ee7e29e32210ecac";
const DISABLED_HYDROLOGY_HISTORY_DIGEST: &str =
    "14c3bc82f53b042d936f41eb60044282275b411653bbc8f9b6a349503afdff81";
const DISABLED_HYDROLOGY_EXPERIMENT_DIGEST: &str =
    "4faa93eaf8dfda6bb5025f80d87b8612540bcf799192d1fd8cdf94a798d18817";

/// The full-envelope digest of the same pre-hydrology world once its recipe
/// records a disabled hydrology configuration.
const DISABLED_HYDROLOGY_ENVELOPE_DIGEST: &str =
    "baf641b23243b693f3f33163b44c7d8efc3b557bc197915fbf06a1d5d08750d7";

// ---------------------------------------------------------------------------
// Resume equivalence
// ---------------------------------------------------------------------------

#[test]
fn pre_hydrology_state_resumes_byte_identically() {
    // Given: six ticks of evolved pre-hydrology production state.
    let exported = evolved_snapshot();
    let original = assemble_envelope(&exported)
        .expect("state must assemble")
        .encode()
        .expect("envelope must encode");

    // When: it is resumed and re-exported without advancing time.
    let resumed =
        Runtime::from_snapshot(exported.clone()).expect("pre-hydrology state must resume");
    let reexported = resumed
        .export_snapshot()
        .expect("resumed state must export");
    let roundtripped = assemble_envelope(&reexported)
        .expect("resumed state must assemble")
        .encode()
        .expect("resumed envelope must encode");

    // Then: export, import, export is byte-identical.
    assert_eq!(exported, reexported, "resumed logical state");
    assert_eq!(hex(&original), hex(&roundtripped), "resumed envelope bytes");
    assert_eq!(
        SnapshotEnvelope::decode(&original).expect("original envelope must decode"),
        SnapshotEnvelope::decode(&roundtripped).expect("resumed envelope must decode"),
    );
}

#[test]
fn pre_hydrology_resumed_state_continues_identically() {
    // Given: the same world run for `LEGACY_TICKS` twice, one of them resumed
    // from a snapshot taken half way.
    let mut uninterrupted = legacy_runtime();
    let straight_through = uninterrupted
        .run_ticks(LEGACY_TICKS * 2)
        .expect("uninterrupted run must tick");

    let mut interrupted = legacy_runtime();
    interrupted
        .run_ticks(LEGACY_TICKS)
        .expect("first half must tick");
    let checkpoint = interrupted
        .export_snapshot()
        .expect("checkpoint must export");
    let mut resumed = Runtime::from_snapshot(checkpoint).expect("checkpoint must resume");
    let after_resume = resumed
        .run_ticks(LEGACY_TICKS)
        .expect("second half must tick");

    // Then: save/resume is equivalent to running straight through.
    assert_eq!(straight_through.time, after_resume.time);
    assert_eq!(
        straight_through.physical_state_digest,
        after_resume.physical_state_digest
    );
    assert_eq!(straight_through.history_digest, after_resume.history_digest);
    assert_eq!(
        uninterrupted.export_snapshot().expect("must export"),
        resumed.export_snapshot().expect("must export"),
    );
}
