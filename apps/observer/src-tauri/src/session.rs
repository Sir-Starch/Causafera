use causafera_observer_api::{
    DeliveryPolicy, ObserverStreamHub, StreamError, StreamKind, StreamScope,
};
use causafera_observer_wire::{
    ProtocolHandler, WireError, decode_connect_request, encode_connect_response,
    encode_observer_snapshot, encode_stream_envelope,
};
use causafera_runtime::{Runtime, RuntimeConfig, RuntimeError};
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
        Ok(self.protocol.handle_query(request)?)
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
    use causafera_observer_api::{
        MATERIAL_SURFACE_DELTA_SCHEMA_V3, MAX_MATERIAL_SURFACE_DELTAS, OBSERVER_PROTOCOL_V1,
        ObserverQuery, QueryKind, QueryStatus,
    };
    use causafera_observer_wire::{
        ConnectRequest, decode_connect_response, decode_explanation_report,
        decode_observer_snapshot, decode_response, decode_stream_envelope, decode_world_snapshot,
        encode_connect_request, encode_query,
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
            MATERIAL_SURFACE_DELTA_SCHEMA_V3
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
