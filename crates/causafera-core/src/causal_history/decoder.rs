use std::collections::BTreeMap;

use super::{
    Criticality, DomainEvidenceRecord, EvidenceSchemaId, EvidenceSchemaVersion, HistoryError,
};

pub trait EvidenceDecoder: std::fmt::Debug + Send + Sync {
    fn source_schema(&self) -> (EvidenceSchemaId, EvidenceSchemaVersion);
    fn target_version(&self) -> EvidenceSchemaVersion;
    fn decode_or_upcast(&self, payload: &[u8]) -> Result<Vec<u8>, HistoryError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedEvidence {
    pub schema_id: EvidenceSchemaId,
    pub version: EvidenceSchemaVersion,
    pub canonical_payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvidenceResolution {
    Decoded(DecodedEvidence),
    Raw(DomainEvidenceRecord),
}

#[derive(Debug, Default)]
pub struct EvidenceDecoderRegistry {
    decoders: BTreeMap<(EvidenceSchemaId, EvidenceSchemaVersion), Box<dyn EvidenceDecoder>>,
}

impl EvidenceDecoderRegistry {
    pub const fn new() -> Self {
        Self {
            decoders: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, decoder: Box<dyn EvidenceDecoder>) -> Result<(), HistoryError> {
        let key = decoder.source_schema();
        if self.decoders.contains_key(&key) {
            return Err(HistoryError::DuplicateDecoder {
                schema_id: key.0,
                version: key.1,
            });
        }
        self.decoders.insert(key, decoder);
        Ok(())
    }

    pub fn register_typed<D: EvidenceDecoder + 'static>(
        &mut self,
        decoder: D,
    ) -> Result<(), HistoryError> {
        self.register(Box::new(decoder))
    }

    pub fn resolve(
        &self,
        record: &DomainEvidenceRecord,
    ) -> Result<EvidenceResolution, HistoryError> {
        let key = (record.schema_id(), record.version());
        if let Some(decoder) = self.decoders.get(&key) {
            return Ok(EvidenceResolution::Decoded(DecodedEvidence {
                schema_id: record.schema_id(),
                version: decoder.target_version(),
                canonical_payload: decoder.decode_or_upcast(record.canonical_payload())?,
            }));
        }
        match record.criticality() {
            Criticality::ReplayRequired => Err(HistoryError::UnsupportedReplaySchema {
                schema_id: record.schema_id(),
                version: record.version(),
            }),
            Criticality::ExplanationOnly => Ok(EvidenceResolution::Raw(record.clone())),
        }
    }
}
