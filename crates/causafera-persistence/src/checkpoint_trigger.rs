pub const CHECKPOINT_DURABLE_FRAME_BYTES: u64 = 64 * 1024 * 1024;
pub const CHECKPOINT_DURABLE_TICKS: u64 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointTrigger {
    ExplicitSave,
    DurableFrameBytes,
    DurableTicks,
}

pub const fn select_checkpoint_trigger(
    durable_ticks_since_checkpoint: u64,
    durable_frame_bytes_since_checkpoint: u64,
    explicit_save: bool,
) -> Option<CheckpointTrigger> {
    if explicit_save {
        return Some(CheckpointTrigger::ExplicitSave);
    }
    if durable_frame_bytes_since_checkpoint >= CHECKPOINT_DURABLE_FRAME_BYTES {
        return Some(CheckpointTrigger::DurableFrameBytes);
    }
    if durable_ticks_since_checkpoint >= CHECKPOINT_DURABLE_TICKS {
        return Some(CheckpointTrigger::DurableTicks);
    }
    None
}
