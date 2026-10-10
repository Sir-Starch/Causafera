#[cfg(test)]
static WHOLE_STATE_CAPTURE_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

use crate::RuntimeState;

#[cfg(test)]
pub(crate) fn reset_whole_state_capture_count() {
    WHOLE_STATE_CAPTURE_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
pub(crate) fn whole_state_capture_count() -> usize {
    WHOLE_STATE_CAPTURE_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

pub(crate) struct TickPreImage;

impl TickPreImage {
    pub(crate) const fn new() -> Self {
        Self
    }

    pub(crate) fn restore(self, state: &mut RuntimeState) {
        state.rollback_tick_undo();
        state.traces.rollback_transaction();
    }
}
