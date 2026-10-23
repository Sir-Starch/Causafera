//! Saving a session the way a durable bundle is saved, and resuming it.
//!
//! Digest schema 9 names a session's history by the durable record stream it
//! owns, so "the resumed run equals the uninterrupted run" is a statement about
//! resuming over that same stream: the bounded snapshot plus the bundle it
//! anchors. Resuming from a complete causal-trace export instead rebuilds a
//! history from the traces it carries and starts its own lineage (see
//! `a_complete_trace_import_starts_its_own_history_lineage`), so its history
//! digest is not comparable with the original run's.
//!
//! Each test binary compiles this separately and uses a different part.
#![allow(dead_code)]

use causafera_runtime::Runtime;

/// Consume `runtime`, save its bounded snapshot at the current tick, and
/// resume a fresh session from that snapshot over the same durable history.
pub fn resume_through_bundle(runtime: Runtime) -> Runtime {
    let bytes = runtime
        .export_bounded_envelope()
        .expect("the session must export a bounded envelope")
        .encode()
        .expect("the bounded envelope must encode");
    let history = runtime
        .into_history_resume()
        .expect("the session must hand over its durable history");
    Runtime::from_bounded_snapshot_bytes(&bytes, history)
        .expect("the bounded snapshot must resume over its own history")
}
