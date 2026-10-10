//! The CLI's durable-history commands: `save`, `resume` and `run --history`.
//!
//! A saved session is a bundle directory (see `causafera_runtime::Runtime::
//! create_bundle`), created before the first tick and never over existing data.
//! `resume` opens one, says what a crash cost it, and carries on from its latest
//! checkpoint. See `plans/causal-history-storage.md` Todo 8.

use std::path::Path;

use causafera_persistence::DurabilityMode;
use causafera_runtime::{BundleOptions, Runtime, RuntimeConfig, RuntimeSnapshot};
use clap::ValueEnum;

use crate::{CliError, fingerprint_hex};

/// How a retained history is made durable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum DurabilityArg {
    /// Published in groups of at most 32 ticks or 250 ms; a crash can lose the
    /// last group. The default.
    #[default]
    Group,
    /// Published after every tick, so a crash loses nothing that was reported
    /// as done.
    EveryTick,
}

impl DurabilityArg {
    const fn mode(self) -> DurabilityMode {
        match self {
            Self::Group => DurabilityMode::GroupCommit,
            Self::EveryTick => DurabilityMode::EveryTick,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Group => "group",
            Self::EveryTick => "every-tick",
        }
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Create a bundle at `path`, run `ticks` ticks into it, save, and shut down.
///
/// Returns the final summary. `ticks` may be zero, which saves tick zero.
pub fn create_run_and_save(
    seed: u64,
    ticks: u64,
    path: &str,
    durability: DurabilityArg,
) -> Result<Option<RuntimeSnapshot>, CliError> {
    let path = Path::new(path);
    let mut runtime = Runtime::create_bundle(
        RuntimeConfig::new(seed),
        path,
        BundleOptions {
            durability: durability.mode(),
            ..BundleOptions::default()
        },
    )?;
    let created = runtime.history_status()?;
    println!(
        "bundle_status=created path={} generation={} durability={}",
        path.display(),
        created.durable_generation,
        durability.name(),
    );
    let summary = if ticks > 0 {
        Some(runtime.run_ticks(ticks)?)
    } else {
        None
    };
    let saved = runtime.save_bundle()?;
    let status = runtime.history_status()?;
    println!(
        "snapshot_status=ok path={} ticks={} generation={} checkpoint={} snapshot={} durable_lag={}",
        path.display(),
        saved.completed_time.raw(),
        saved.generation,
        hex(&saved.checkpoint_digest),
        saved.snapshot_path.display(),
        status.logical_generation - status.durable_generation,
    );
    runtime.shutdown()?;
    Ok(summary)
}

/// Open the bundle at `path`, say what the open found, run `ticks` more ticks,
/// save, and shut down.
pub fn open_run_and_save(path: &str, ticks: u64) -> Result<(), CliError> {
    let path = Path::new(path);
    if path.is_file() {
        return Err(CliError::LegacySnapshotFile {
            path: path.display().to_string(),
        });
    }
    let (mut runtime, recovery) = Runtime::open_bundle(path)?;
    let lost = recovery.lost.map_or_else(
        || "none".to_owned(),
        |range| format!("{}..{}", range.first_generation, range.last_generation),
    );
    println!(
        "resume_status=ok path={} resumed_generation={} checkpoint_generation={} replayed_ticks={} resumed_at={} lost_ticks={} lost_generations={} unaccounted_bytes={}",
        path.display(),
        recovery.generation,
        recovery.checkpoint_generation,
        recovery.replayed_ticks,
        recovery.completed_time.raw(),
        recovery.lost.map_or(0, |range| range.ticks),
        lost,
        recovery.unaccounted_bytes,
    );
    let summary = if ticks > 0 {
        runtime.run_ticks(ticks)?
    } else {
        runtime.snapshot()?
    };
    let saved = runtime.save_bundle()?;
    println!(
        "ticks={} traces={} physical_events={} mana_changes={} resolution_changes={} generation={} digest={}",
        summary.time.raw(),
        summary.causal_trace_count,
        summary.physical_events,
        summary.mana_cell_changes,
        summary.resolution_changes,
        saved.generation,
        fingerprint_hex(summary.canonical_state.fingerprint),
    );
    runtime.shutdown()?;
    Ok(())
}
