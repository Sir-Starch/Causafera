//! The schema 9 history digest across how a session happens to be stored.
//!
//! The digest folds the canonical record stream, so it must not depend on the
//! durability mode, on when a group was forced, or on a rescan having been
//! run. See `plans/causal-history-storage.md` Todo 7.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use causafera_persistence::{DurabilityMode, HistoryStore};

use crate::{Runtime, RuntimeConfig};

struct Bundle(PathBuf);

impl Bundle {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(std::env::temp_dir().join(format!(
            "causafera-history-digest-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Bundle {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A session whose history lives in a bundle opened in `mode`.
fn session_in_mode(seed: u64, name: &str, mode: DurabilityMode) -> (Runtime, Bundle) {
    let runtime = Runtime::new(RuntimeConfig::new(seed)).expect("the session constructs");
    let bundle = Bundle::new(name);
    let store = HistoryStore::create(
        bundle.path(),
        crate::history_transaction::runtime_history_namespace(seed),
        mode,
    )
    .expect("the store creates");
    {
        let mut state = runtime.lock_state().expect("state");
        let hot_budget = state.traces.hot_budget();
        let (history, traces) =
            crate::history_transaction::RuntimeHistoryCoordinator::bootstrap_with_store(
                &state.traces,
                seed,
                hot_budget,
                store,
            )
            .expect("the coordinator installs");
        state.history = history;
        state.traces = traces;
    }
    (runtime, bundle)
}

#[test]
fn history_digest_is_independent_of_durability_mode_and_group_size() {
    // Given: one world stored with a forced catalog commit on every tick and
    // one stored in groups of up to 32 ticks.
    let (mut every_tick, _every_tick_bundle) =
        session_in_mode(11, "every-tick", DurabilityMode::EveryTick);
    let (mut grouped, _grouped_bundle) =
        session_in_mode(11, "grouped", DurabilityMode::GroupCommit);

    // When: both advance across the group boundary, compared at each step.
    let mut grouped_lag_seen = false;
    for ticks in [1, 30, 1, 1, 37] {
        every_tick.run_ticks(ticks).expect("every tick advances");
        grouped.run_ticks(ticks).expect("grouped advances");

        // Then: the durable generations differ in lag, and nothing else does.
        let every = every_tick.snapshot().expect("every tick snapshot");
        let group = grouped.snapshot().expect("grouped snapshot");
        assert_eq!(every.history_digest, group.history_digest);
        assert_eq!(every.physical_state_digest, group.physical_state_digest);
        assert_eq!(every.canonical_state, group.canonical_state);
        for runtime in [&every_tick, &grouped] {
            let state = runtime.lock_state().expect("state");
            assert_eq!(
                state.history_digest(),
                state.history_digest_full_rescan().expect("cold rescan")
            );
        }
        {
            let state = every_tick.lock_state().expect("state");
            assert_eq!(
                state.history.logical_generation(),
                state.history.durable_generation(),
                "EveryTick publishes every generation"
            );
        }

        // The durable frontier carries the same digest once the group is forced.
        let mut state = grouped.lock_state().expect("state");
        grouped_lag_seen |= state.history.logical_generation() > state.history.durable_generation();
        state.history.flush_durable().expect("force the group");
        let anchor = state.history.reader().anchor().expect("durable anchor");
        assert_eq!(anchor.logical_generation(), anchor.durable_generation());
        assert_eq!(*anchor.logical_digest(), state.history_digest().bytes());
        assert_eq!(
            state.history.durable_generation(),
            state.history.logical_generation()
        );
    }
    assert!(
        grouped_lag_seen,
        "the grouped session must have run behind its durable generation at least once"
    );
}

#[test]
fn reading_the_history_digest_never_scans_the_history() {
    // Given
    let mut runtime = Runtime::new(RuntimeConfig::new(21)).expect("the session constructs");
    crate::history_transaction::reset_whole_history_scans();

    // When: ticks run and every observer-facing digest is read.
    for _ in 0..24 {
        runtime.tick().expect("tick");
        let snapshot = runtime.snapshot().expect("snapshot");
        std::hint::black_box((snapshot.history_digest, snapshot.canonical_state));
    }

    // Then: no whole-history read happened until the oracle asked for one.
    assert_eq!(crate::history_transaction::whole_history_scans(), 0);
    runtime
        .lock_state()
        .expect("state")
        .history_digest_full_rescan()
        .expect("cold rescan");
    assert_eq!(crate::history_transaction::whole_history_scans(), 1);
}
