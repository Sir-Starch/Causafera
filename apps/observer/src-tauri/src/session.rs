use causafera_observer_api::{
    DeliveryPolicy, ObserverStreamHub, QueryKind, QueryStatus, StreamError, StreamKind, StreamScope,
};
use causafera_observer_wire::{
    ProtocolHandler, WireError, decode_connect_request, decode_field_raster_request, decode_query,
    encode_connect_response, encode_field_raster, encode_observer_snapshot, encode_query_response,
    encode_stream_envelope,
};
use causafera_runtime::{ActiveChunkShape, Runtime, RuntimeConfig, RuntimeError};
use thiserror::Error;

const RUNTIME_STREAM_ID: u64 = 1;
const MAX_ADVANCE_TICKS: u64 = 64;
const DEFAULT_ACTORS: u8 = 8;
const DEFAULT_SENSORS: u8 = 2;
const DEFAULT_POPULATION: u64 = 512;

pub struct ObserverSession {
    runtime: Runtime,
    protocol: ProtocolHandler,
    streams: ObserverStreamHub,
}

impl ObserverSession {
    pub fn new(seed: u64) -> Result<Self, SessionError> {
        let runtime = Runtime::new(session_config(seed))?;
        let mut session = Self {
            runtime,
            protocol: ProtocolHandler::default(),
            streams: ObserverStreamHub::default(),
        };
        session.refresh_protocol()?;
        Ok(session)
    }

    pub fn connect(&mut self, request: &[u8]) -> Result<Vec<u8>, SessionError> {
        self.refresh_protocol()?;
        let request = decode_connect_request(request)?;
        let response = self.protocol.negotiate(&request)?;
        Ok(encode_connect_response(&response))
    }

    pub fn query(&mut self, request: &[u8]) -> Result<Vec<u8>, SessionError> {
        self.refresh_protocol()?;
        // A raster answer depends on what was asked rather than on when, so it
        // cannot come from the payload cache the other kinds share.
        let query = decode_query(request)?;
        if query.kind == QueryKind::FieldRaster {
            return self.field_raster(query.request_id, &query.payload);
        }
        Ok(self.protocol.handle_query(request)?)
    }

    /// One chunk of one measured lattice.
    ///
    /// A malformed request is refused as invalid and a chunk outside the active
    /// set as unavailable: the session states which of the two happened rather
    /// than answering an unbounded read.
    fn field_raster(&mut self, request_id: u64, payload: &[u8]) -> Result<Vec<u8>, SessionError> {
        let request = match decode_field_raster_request(payload) {
            Ok(request) => request,
            Err(_) => {
                return Ok(encode_query_response(
                    request_id,
                    QueryStatus::InvalidRequest,
                    Vec::new(),
                ));
            }
        };
        Ok(match self.runtime.observer_field_raster(&request)? {
            Some(raster) => {
                encode_query_response(request_id, QueryStatus::Ok, encode_field_raster(&raster))
            }
            None => encode_query_response(request_id, QueryStatus::NotAvailable, Vec::new()),
        })
    }

    pub fn open_runtime_stream(&mut self) -> Result<Vec<u8>, SessionError> {
        self.streams = ObserverStreamHub::default();
        self.streams.subscribe(
            RUNTIME_STREAM_ID,
            StreamScope {
                kind: StreamKind::RuntimeSummary,
                chunk: None,
            },
            DeliveryPolicy::LatestStateWins,
            1,
        )?;
        self.publish_runtime(true)
    }

    pub fn advance(&mut self, ticks: u64) -> Result<Vec<u8>, SessionError> {
        if ticks == 0 || ticks > MAX_ADVANCE_TICKS {
            return Err(SessionError::InvalidAdvance(ticks));
        }
        self.runtime.run_ticks(ticks)?;
        self.refresh_protocol()?;
        self.publish_runtime(false)
    }

    pub fn analyze(&mut self, request: &[u8]) -> Result<Vec<u8>, SessionError> {
        let report = self.runtime.observer_material_surface_loop_explanation()?;
        self.protocol.set_explanation_report(&report);
        Ok(self.protocol.handle_query(request)?)
    }

    pub fn reset(&mut self, seed: u64) -> Result<Vec<u8>, SessionError> {
        *self = Self::new(seed)?;
        self.open_runtime_stream()
    }

    fn refresh_protocol(&mut self) -> Result<(), SessionError> {
        let snapshot = self.runtime.snapshot()?;
        let world = self.runtime.observer_world_snapshot()?;
        self.protocol
            .set_runtime_snapshot(&snapshot.observer_snapshot());
        self.protocol.set_world_snapshot(&world);
        Ok(())
    }

    fn publish_runtime(&mut self, is_snapshot: bool) -> Result<Vec<u8>, SessionError> {
        let snapshot = self.runtime.snapshot()?.observer_snapshot();
        self.streams.publish(
            RUNTIME_STREAM_ID,
            snapshot.time,
            snapshot.physical_digest,
            snapshot.history_digest,
            is_snapshot,
            encode_observer_snapshot(&snapshot),
        )?;
        let envelope = self
            .streams
            .pop(RUNTIME_STREAM_ID)?
            .ok_or(SessionError::MissingStreamEnvelope)?;
        Ok(encode_stream_envelope(&envelope))
    }
}

fn session_config(seed: u64) -> RuntimeConfig {
    let mut config = RuntimeConfig::new(seed);
    config.actor_count = DEFAULT_ACTORS;
    config.sensor_count = DEFAULT_SENSORS;
    config.bootstrap_population = DEFAULT_POPULATION;
    config.mana_parameters.effect_threshold = 1;
    config.mana_parameters.effect_hysteresis = 0;
    // A chart with one dimension is a strip, not a map. The observer opts into
    // the square block; the default stays a line so no recorded fixture moves.
    config.active_chunk_shape = ActiveChunkShape::Area;
    config
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("advance tick count must be between 1 and {MAX_ADVANCE_TICKS}, got {0}")]
    InvalidAdvance(u64),
    #[error("runtime stream produced no envelope")]
    MissingStreamEnvelope,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Wire(#[from] WireError),
    #[error(transparent)]
    Stream(#[from] StreamError),
}

#[cfg(test)]
mod tests {
    use causafera_explanation::{
        ClaimEvidenceState, HISTORICAL_BOOTSTRAP_RECORD_SCHEMA, HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA,
        NumericClaimValue,
    };
    use causafera_runtime::BOOTSTRAP_STAGE_COUNT;

    use causafera_observer_api::{
        BOOTSTRAP_SUMMARY_SCHEMA_V1, FieldRasterKind, FieldRasterRequest,
        MATERIAL_SURFACE_DELTA_SCHEMA_V4, MAX_MATERIAL_SURFACE_DELTAS, OBSERVER_PROTOCOL_V1,
        ObserverQuery, QueryKind, QueryStatus,
    };
    use causafera_observer_wire::{
        ConnectRequest, decode_connect_response, decode_explanation_report, decode_field_raster,
        decode_observer_snapshot, decode_response, decode_stream_envelope, decode_world_snapshot,
        encode_connect_request, encode_field_raster_request, encode_query,
    };
    use causafera_types::SimulationTime;

    use super::*;

    #[test]
    fn session_negotiates_and_streams_real_runtime_snapshots() {
        let mut session = ObserverSession::new(9).unwrap();
        let connect = session
            .connect(&encode_connect_request(&ConnectRequest {
                supported_versions: vec![1],
                locale: "ru-RU".into(),
            }))
            .unwrap();
        let connect = decode_connect_response(&connect).unwrap();
        assert_eq!(connect.selected_version, 1);
        assert!(connect.capabilities.contains(&3));

        let initial = decode_stream_envelope(&session.open_runtime_stream().unwrap()).unwrap();
        assert!(initial.header.is_snapshot);
        let initial_summary = decode_observer_snapshot(&initial.payload).unwrap();
        assert_eq!(initial_summary.actor_count, u32::from(DEFAULT_ACTORS));
        assert_eq!(
            initial_summary.population_total + u64::from(initial_summary.actor_count),
            DEFAULT_POPULATION
        );

        let delta = decode_stream_envelope(&session.advance(4).unwrap()).unwrap();
        assert!(!delta.header.is_snapshot);
        assert_eq!(delta.header.sequence_number, 1);
        assert_eq!(delta.header.simulation_time, SimulationTime::new(4));
    }

    #[test]
    fn session_exposes_six_bootstrap_receipts() {
        // Given: a real observer session over a production runtime.
        let mut session = ObserverSession::new(9).unwrap();
        let initial = decode_stream_envelope(&session.open_runtime_stream().unwrap()).unwrap();
        let summary = decode_observer_snapshot(&initial.payload).unwrap();
        let bootstrap = &summary.bootstrap;

        // Then: it reports a validated six-stage record, bounded at six.
        assert_eq!(bootstrap.schema_version, BOOTSTRAP_SUMMARY_SCHEMA_V1);
        assert!(bootstrap.complete);
        assert_eq!(bootstrap.stage_count, 6);
        // Compared against the stage count the bootstrap actually runs, not
        // against the cap that produced the list — the cap would agree with a
        // truncated list.
        assert_eq!(bootstrap.receipts.len(), BOOTSTRAP_STAGE_COUNT);
        // The six-summary surface is frozen V1 contract. A session that ran the
        // appended hydrology stage reports it in the separately bounded optional
        // field, not as a seventh entry here — this session did not run one.
        assert_eq!(bootstrap.stage_count as usize, BOOTSTRAP_STAGE_COUNT);
        assert!(bootstrap.stage_seven.is_none());
        assert_eq!(bootstrap.world_seed, 9);
        assert_eq!(bootstrap.configured_population, DEFAULT_POPULATION);
        assert_eq!(
            bootstrap.configured_promotion_limit,
            u32::from(DEFAULT_ACTORS)
        );

        // And: the receipts are strictly ordered and chained through the
        // previous stage's completion trace.
        let mut previous: Option<causafera_types::TraceId> = None;
        for (ordinal, receipt) in bootstrap.receipts.iter().enumerate() {
            assert_eq!(receipt.stage, ordinal as u64 + 1);
            assert_eq!(
                receipt.completed_at,
                SimulationTime::new(ordinal as u64 + 1)
            );
            assert_eq!(
                receipt.dependency_traces,
                previous.into_iter().collect::<Vec<_>>()
            );
            previous = Some(receipt.completion_trace);
        }

        // And: every completion trace resolves in the session's own trace store.
        let data = session.runtime.export_snapshot().unwrap();
        for receipt in &bootstrap.receipts {
            assert!(
                data.traces
                    .events
                    .iter()
                    .any(|event| event.trace_id == receipt.completion_trace),
                "receipt trace {:?} must exist in the session trace store",
                receipt.completion_trace
            );
        }

        // And: the existing population/actor conservation still holds.
        assert_eq!(
            summary.population_total + u64::from(summary.actor_count),
            DEFAULT_POPULATION
        );
    }

    #[test]
    fn session_reset_and_headless_runtime_agree_on_the_bootstrap_record() {
        // Given: a session and a headless runtime over the same configuration.
        let mut session = ObserverSession::new(12).unwrap();
        let session_record = session.runtime.export_snapshot().unwrap().bootstrap;
        let headless = causafera_runtime::Runtime::new(session_config(12))
            .unwrap()
            .export_snapshot()
            .unwrap()
            .bootstrap;

        // Then: the session is not a separate bootstrap path.
        assert_eq!(session_record, headless);

        // And: a reset reproduces the same record rather than a new one, and a
        // different seed produces a different plan identity.
        session.open_runtime_stream().unwrap();
        session.advance(3).unwrap();
        session.reset(12).unwrap();
        assert_eq!(
            session.runtime.export_snapshot().unwrap().bootstrap,
            headless
        );
        session.reset(13).unwrap();
        assert_ne!(
            session.runtime.export_snapshot().unwrap().bootstrap.plan.id,
            headless.plan.id
        );
    }

    #[test]
    fn session_bootstrap_explanation_is_supported_and_carries_trace_anchors() {
        // Given: a real observer session over a production runtime.
        let session = ObserverSession::new(11).unwrap();

        // When: the bootstrap record is explained.
        let report = session
            .runtime
            .observer_bootstrap_record_explanation()
            .expect("a validated record must explain");

        // Then: it carries typed completeness and window claims backed by the
        // completion traces, with no rendered process name anywhere in the IR.
        let claims = &report.frames[0].claims;
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].schema, HISTORICAL_BOOTSTRAP_RECORD_SCHEMA);
        assert_eq!(claims[0].evidence_state, ClaimEvidenceState::Supported);
        assert_eq!(claims[0].value, NumericClaimValue::scalar(6));
        assert_eq!(claims[1].schema, HISTORICAL_BOOTSTRAP_WINDOW_SCHEMA);
        assert_eq!(
            claims[1].value,
            NumericClaimValue::range(0, 6).expect("a canonical six-stage window")
        );
        assert!(claims[0].evidence_traces.len() >= 6);
        assert!(
            claims[0]
                .evidence_traces
                .windows(2)
                .all(|pair| pair[0] < pair[1])
        );
    }

    #[test]
    fn world_query_contains_real_bounded_chunk_projection() {
        let mut session = ObserverSession::new(10).unwrap();
        let summary = decode_stream_envelope(&session.open_runtime_stream().unwrap()).unwrap();
        let summary = decode_observer_snapshot(&summary.payload).unwrap();
        let response = decode_response(
            &session
                .query(&encode_query(&ObserverQuery::world_chunks(2)))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(response.status, QueryStatus::Ok);
        let world = decode_world_snapshot(&response.payload).unwrap();
        assert!(!world.chunks.is_empty());
        assert!(world.chunks.len() <= 9);
        assert_eq!(
            world
                .chunks
                .iter()
                .map(|chunk| chunk.population_total)
                .sum::<u64>(),
            summary.population_total
        );
        assert_eq!(
            summary.population_total + u64::from(summary.actor_count),
            DEFAULT_POPULATION
        );
        assert_eq!(
            world.material_surface_delta_schema_version,
            MATERIAL_SURFACE_DELTA_SCHEMA_V4
        );
        assert!(!world.material_surface_deltas.is_empty());
        assert!(
            world.material_surface_deltas.len() <= MAX_MATERIAL_SURFACE_DELTAS,
            "material-surface observer deltas must remain bounded"
        );
        assert!(
            world
                .material_surface_deltas
                .iter()
                .all(|delta| delta.after_condition >= delta.before_condition)
        );
        assert!(
            world
                .material_surface_deltas
                .iter()
                .all(|delta| delta.contact_trace.is_none())
        );
        session.advance(1).unwrap();
        let contacted = decode_response(
            &session
                .query(&encode_query(&ObserverQuery::world_chunks(2)))
                .unwrap(),
        )
        .unwrap();
        let contacted = decode_world_snapshot(&contacted.payload).unwrap();
        assert!(
            contacted
                .material_surface_deltas
                .iter()
                .any(|delta| delta.contact_trace.is_some())
        );
    }

    /// INV-007 across every locale the observer offers, not only the first two it shipped with.
    ///
    /// The payload is compared as well as the digests: a locale must not change the bytes the
    /// session emits, or presentation would have leaked into the observer projection itself.
    #[test]
    fn locale_does_not_change_session_digests() {
        const LOCALES: [&str; 5] = ["en-US", "ru-RU", "zh-Hans", "de-DE", "es-ES"];

        // Given: one session per locale, all from the same seed.
        let mut envelopes = Vec::new();
        for locale in LOCALES {
            let mut session = ObserverSession::new(11).unwrap();
            session
                .connect(&encode_connect_request(&ConnectRequest {
                    supported_versions: vec![1],
                    locale: locale.into(),
                }))
                .unwrap();
            session.open_runtime_stream().unwrap();

            // When: each advances the same number of ticks.
            envelopes.push((
                locale,
                decode_stream_envelope(&session.advance(8).unwrap()).unwrap(),
            ));
        }

        // Then: digests and payload bytes are identical across all of them.
        let (first_locale, first) = &envelopes[0];
        for (locale, envelope) in envelopes.iter().skip(1) {
            assert_eq!(
                first.header.physical_digest, envelope.header.physical_digest,
                "physical digest diverged between {first_locale} and {locale}"
            );
            assert_eq!(
                first.header.history_digest, envelope.header.history_digest,
                "history digest diverged between {first_locale} and {locale}"
            );
            assert_eq!(
                first.payload, envelope.payload,
                "payload diverged between {first_locale} and {locale}"
            );
        }
    }

    fn raster_query(
        session: &mut ObserverSession,
        request_id: u64,
        request: FieldRasterRequest,
    ) -> causafera_observer_api::ObserverResponse {
        decode_response(
            &session
                .query(&encode_query(&ObserverQuery::field_raster(
                    request_id,
                    encode_field_raster_request(&request),
                )))
                .unwrap(),
        )
        .unwrap()
    }

    fn terrain_request(chunk_x: i32, chunk_y: i32, detail_level: u8) -> FieldRasterRequest {
        FieldRasterRequest {
            chart_id: 1,
            chunk_x,
            chunk_y,
            chunk_z: 0,
            field: FieldRasterKind::TerrainElevation,
            detail_level,
        }
    }

    /// The map's own claim: what it draws is the carrier's lattice, unreduced.
    #[test]
    fn terrain_raster_projects_the_carrier_lattice_and_its_block_means() {
        // Given: a live session with a two-dimensional chart.
        let mut session = ObserverSession::new(21).unwrap();
        session.open_runtime_stream().unwrap();

        // When: the same chunk is requested at every detail level.
        let full = raster_query(&mut session, 1, terrain_request(0, 0, 0));
        let half = raster_query(&mut session, 2, terrain_request(0, 0, 1));
        let quarter = raster_query(&mut session, 3, terrain_request(0, 0, 2));

        // Then: level 0 is the carrier's own 32 x 32 field, and each further
        // level is the block mean of the one above it.
        assert_eq!(full.status, QueryStatus::Ok);
        let full = decode_field_raster(&full.payload).unwrap();
        assert_eq!((full.edge, full.depth), (32, 1));
        assert_eq!(full.values.len(), 1_024);
        assert_eq!(full.auxiliary.len(), 1_024);
        assert!(full.generation_trace > 0);

        let half = decode_field_raster(&half.payload).unwrap();
        assert_eq!((half.edge, half.values.len()), (16, 256));
        for row in 0..16 {
            for column in 0..16 {
                let block = [
                    full.values[row * 2 * 32 + column * 2],
                    full.values[row * 2 * 32 + column * 2 + 1],
                    full.values[(row * 2 + 1) * 32 + column * 2],
                    full.values[(row * 2 + 1) * 32 + column * 2 + 1],
                ];
                assert_eq!(
                    half.values[row * 16 + column],
                    block.iter().sum::<i64>() / 4
                );
            }
        }

        let quarter = decode_field_raster(&quarter.payload).unwrap();
        assert_eq!((quarter.edge, quarter.values.len()), (8, 64));
    }

    #[test]
    fn mana_raster_projects_the_volume_with_per_cell_provenance() {
        let mut session = ObserverSession::new(22).unwrap();
        session.open_runtime_stream().unwrap();
        session.advance(8).unwrap();

        let response = raster_query(
            &mut session,
            4,
            FieldRasterRequest {
                field: FieldRasterKind::ManaIntensity,
                ..terrain_request(0, 0, 0)
            },
        );

        assert_eq!(response.status, QueryStatus::Ok);
        let raster = decode_field_raster(&response.payload).unwrap();
        // The configured lattice, projected whole rather than reduced.
        assert_eq!(raster.edge, raster.depth);
        let cells = raster
            .cell_count()
            .expect("a real raster declares a describable lattice");
        assert_eq!(raster.values.len(), cells);
        assert_eq!(raster.cell_traces.len(), cells);
        assert!(raster.values.iter().any(|value| *value > 0));
        assert!(raster.cell_traces.iter().any(|trace| *trace > 0));
    }

    #[test]
    fn raster_requests_are_bounded_by_the_active_set_and_the_detail_range() {
        let mut session = ObserverSession::new(23).unwrap();
        session.open_runtime_stream().unwrap();

        // Ground the session never activated is unavailable, not fabricated.
        assert_eq!(
            raster_query(&mut session, 5, terrain_request(48, 0, 0)).status,
            QueryStatus::NotAvailable
        );
        // A detail level outside the contract is a malformed request.
        assert_eq!(
            raster_query(&mut session, 6, terrain_request(0, 0, 7)).status,
            QueryStatus::InvalidRequest
        );
    }

    /// Two dimensions are what makes the chart a map; the shape is the session's
    /// choice, so the session is where it is asserted.
    #[test]
    fn the_observer_session_charts_an_area_rather_than_a_strip() {
        let mut session = ObserverSession::new(24).unwrap();
        session.open_runtime_stream().unwrap();

        let world = decode_world_snapshot(
            &decode_response(
                &session
                    .query(&encode_query(&ObserverQuery::world_chunks(7)))
                    .unwrap(),
            )
            .unwrap()
            .payload,
        )
        .unwrap();

        assert_eq!(world.chunks.len(), 9);
        assert!(world.chunks.iter().any(|chunk| chunk.chunk_y != 0));
        assert!(world.chunks.iter().all(|chunk| chunk.chunk_z == 0));
    }

    #[test]
    fn advance_is_explicitly_bounded() {
        let mut session = ObserverSession::new(12).unwrap();
        session.open_runtime_stream().unwrap();
        assert!(matches!(
            session.advance(MAX_ADVANCE_TICKS + 1),
            Err(SessionError::InvalidAdvance(_))
        ));
    }

    #[test]
    fn live_runtime_material_loop_explanation_distinguishes_absent_and_present_contact() {
        // Given: an observer session backed by the causally bootstrapped production runtime.
        let mut session = ObserverSession::new(13).unwrap();
        session.open_runtime_stream().unwrap();
        let query = ObserverQuery {
            request_id: 9,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::ExplanationIr,
            scope: None,
            payload: Vec::new(),
        };

        // When: the read-only Explanation query runs before and after actor contact.
        let before = decode_response(&session.analyze(&encode_query(&query)).unwrap()).unwrap();
        session.advance(4).unwrap();
        let after = decode_response(&session.analyze(&encode_query(&query)).unwrap()).unwrap();

        // Then: bootstrap claims no actor-contact evidence, while a later live query does.
        assert_eq!(before.status, QueryStatus::Ok);
        let before = decode_explanation_report(&before.payload).unwrap();
        assert!(
            before.frames[0]
                .claims
                .iter()
                .all(|claim| claim.evidence_traces.is_empty())
        );
        assert_eq!(after.status, QueryStatus::Ok);
        let after = decode_explanation_report(&after.payload).unwrap();
        assert_eq!(after.frames.len(), 1);
        assert_eq!(after.frames[0].checkpoint_time.raw(), 4);
        assert!(
            after.frames[0]
                .claims
                .iter()
                .any(|claim| claim.schema.raw() == 10 && claim.evidence_traces.len() >= 2)
        );
    }
}
