use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! define_id {
    ($name:ident, $inner:ty) => {
        #[derive(
            Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name($inner);

        impl $name {
            pub const fn new(id: $inner) -> Self {
                Self(id)
            }

            pub const fn raw(self) -> $inner {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

define_id!(AgentId, u64);
define_id!(BodyId, u64);
define_id!(BodySegmentId, u64);
define_id!(EventId, u64);
define_id!(TraceId, u64);
define_id!(EventKindId, u64);
define_id!(StateObjectKindId, u64);
define_id!(StatePropertyId, u64);
define_id!(FeatureId, u64);
define_id!(PerceptId, u64);
define_id!(SensorId, u64);
define_id!(SignalChannelId, u64);
define_id!(AcquisitionId, u64);
define_id!(AttentionTargetId, u64);
define_id!(PerceivedObjectId, u64);
define_id!(SubjectiveBodyPartId, u64);
define_id!(SelfAssociationId, u64);
define_id!(WorkingItemId, u64);
define_id!(WorkingItemKindId, u64);
define_id!(EpisodeId, u64);
define_id!(PredictionId, u64);
define_id!(ActionPatternId, u64);
define_id!(OutcomePatternId, u64);
define_id!(ConceptId, u64);
define_id!(BeliefId, u64);
define_id!(EvidenceId, u64);
define_id!(SubjectiveSourceId, u64);
define_id!(CausalHypothesisId, u64);
define_id!(SubjectivePatternId, u64);
define_id!(LexemeId, u64);
define_id!(LanguageId, u64);
define_id!(PhonologicalUnitId, u64);
define_id!(LexemeUseId, u64);
define_id!(SpeechActId, u64);
define_id!(UtteranceId, u64);
define_id!(TransmissionId, u64);
define_id!(PracticeId, u64);
define_id!(PracticeExecutionId, u64);
define_id!(PracticeConditionId, u64);
define_id!(PracticeOperationId, u64);
define_id!(QuantitySchemaId, u64);
define_id!(UnitId, u64);
define_id!(CalibrationId, u64);
define_id!(MeasurementId, u64);
define_id!(DocumentId, u64);
define_id!(WritingSystemId, u64);
define_id!(GlyphId, u64);
define_id!(DocumentMediumId, u64);
define_id!(DocumentTransformationId, u64);
define_id!(ManaFieldId, u64);
define_id!(PhysicalPatternId, u64);
define_id!(ResolutionFieldId, u64);
define_id!(ResolutionChannelId, u64);
define_id!(OrganizationId, u64);
define_id!(SocialRelationId, u64);
define_id!(SocialRelationSchemaId, u64);
define_id!(RoleAssignmentId, u64);
define_id!(RoleSchemaId, u64);
define_id!(CommunicationLinkId, u64);
define_id!(CommunicationChannelId, u64);
define_id!(AuthorityGrantId, u64);
define_id!(AuthorityScopeId, u64);
define_id!(InstitutionalRuleId, u64);
define_id!(PropertyClaimId, u64);
define_id!(AgreementId, u64);
define_id!(AgreementPartyId, u64);
define_id!(InventoryLotId, u64);
define_id!(MaterialTransferId, u64);
define_id!(MaterialTransformationId, u64);
define_id!(LabourContributionId, u64);
define_id!(ParcelRecordId, u64);
define_id!(BuildingRecordId, u64);
define_id!(InfrastructureNetworkId, u64);
define_id!(InfrastructureSchemaId, u64);
define_id!(InfrastructureNodeId, u64);
define_id!(InfrastructureLinkId, u64);
define_id!(PlaceId, u64);
define_id!(ChunkId, u64);
define_id!(FormationId, u64);
define_id!(MaterialId, u64);
define_id!(PathogenId, u64);
define_id!(PopulationLineageId, u64);
define_id!(AggregateId, u64);
define_id!(EntityId, u64);
define_id!(SubstanceId, u64);
