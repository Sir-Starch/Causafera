/// Cross-world transfer configuration.
pub struct TransferConfig {
    pub transfer_type: TransferType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferType {
    FullPhysical,
    IdentityPattern,
    PartialMemory,
    ReincarnationBinding,
    InformationalEcho,
    ArtifactTransfer,
    OverlappingIdentity,
}
