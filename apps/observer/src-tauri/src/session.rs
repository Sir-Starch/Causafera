use causafera_lab::{ExperimentError, ExperimentRunner};
use causafera_observer_api::{
    DeliveryPolicy, ObserverStreamHub, StreamError, StreamKind, StreamScope,
};
use causafera_observer_wire::{
    ProtocolHandler, WireError, decode_connect_request, encode_connect_response,
    encode_observer_snapshot, encode_stream_envelope,
};
use causafera_runtime::{Runtime, RuntimeConfig, RuntimeError};
use causafera_types::SimulationTime;
use thiserror::Error;

const RUNTIME_STREAM_ID: u64 = 1;
const MAX_ADVANCE_TICKS: u64 = 64;
const DEFAULT_ACTORS: u8 = 8;
const DEFAULT_SENSORS: u8 = 2;
const DEFAULT_POPULATION: u64 = 512;
const ANALYSIS_POPULATION: u64 = 16;
const ANALYSIS_TICKS: u64 = 192;
const ANALYSIS_CHECKPOINT_INTERVAL: u64 = 24;
const ANALYSIS_SUPPRESSION_FROM: u64 = 72;
const ANALYSIS_SUPPRESSION_THROUGH: u64 = 120;

pub struct ObserverSession {
    runtime: Runtime,
    protocol: ProtocolHandler,
    streams: ObserverStreamHub,
    seed: u64,
}

impl ObserverSession {
    pub fn new(seed: u64) -> Result<Self, SessionError> {
        let runtime = Runtime::new(session_config(seed))?;
        let mut session = Self {
            runtime,
            protocol: ProtocolHandler::default(),
            streams: ObserverStreamHub::default(),
            seed,
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
        let report = ExperimentRunner::run_populated_control_and_intervention(
            self.seed,
            ANALYSIS_TICKS,
            ANALYSIS_CHECKPOINT_INTERVAL,
            SimulationTime::new(ANALYSIS_SUPPRESSION_FROM),
            SimulationTime::new(ANALYSIS_SUPPRESSION_THROUGH),
            ANALYSIS_POPULATION,
        )?;
        self.protocol
            .set_explanation_report(&report.explanation_report);
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
    #[error(transparent)]
    Experiment(#[from] ExperimentError),
}

#[cfg(test)]
mod tests {
    use causafera_observer_api::{OBSERVER_PROTOCOL_V1, ObserverQuery, QueryKind, QueryStatus};
    use causafera_observer_wire::{
        ConnectRequest, decode_connect_response, decode_observer_snapshot, decode_response,
        decode_stream_envelope, decode_world_snapshot, encode_connect_request, encode_query,
    };

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
        assert_eq!(initial_summary.population_total, DEFAULT_POPULATION);
        assert_eq!(initial_summary.actor_count, u32::from(DEFAULT_ACTORS));

        let delta = decode_stream_envelope(&session.advance(4).unwrap()).unwrap();
        assert!(!delta.header.is_snapshot);
        assert_eq!(delta.header.sequence_number, 1);
        assert_eq!(delta.header.simulation_time, SimulationTime::new(4));
    }

    #[test]
    fn world_query_contains_real_bounded_chunk_projection() {
        let mut session = ObserverSession::new(10).unwrap();
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
            DEFAULT_POPULATION
        );
    }

    #[test]
    fn locale_does_not_change_session_digests() {
        let mut russian = ObserverSession::new(11).unwrap();
        let mut english = ObserverSession::new(11).unwrap();
        for (session, locale) in [(&mut russian, "ru-RU"), (&mut english, "en-US")] {
            session
                .connect(&encode_connect_request(&ConnectRequest {
                    supported_versions: vec![1],
                    locale: locale.into(),
                }))
                .unwrap();
            session.open_runtime_stream().unwrap();
        }
        let russian = decode_stream_envelope(&russian.advance(8).unwrap()).unwrap();
        let english = decode_stream_envelope(&english.advance(8).unwrap()).unwrap();
        assert_eq!(
            russian.header.physical_digest,
            english.header.physical_digest
        );
        assert_eq!(russian.header.history_digest, english.header.history_digest);
        assert_eq!(russian.payload, english.payload);
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
    fn replay_verified_analysis_is_delivered_as_typed_observer_payload() {
        let mut session = ObserverSession::new(13).unwrap();
        let query = ObserverQuery {
            request_id: 9,
            protocol_version: OBSERVER_PROTOCOL_V1,
            kind: QueryKind::ExplanationIr,
            scope: None,
            payload: Vec::new(),
        };
        let response = decode_response(&session.analyze(&encode_query(&query)).unwrap()).unwrap();
        assert_eq!(response.status, QueryStatus::Ok);
        assert!(!response.payload.is_empty());
    }
}
