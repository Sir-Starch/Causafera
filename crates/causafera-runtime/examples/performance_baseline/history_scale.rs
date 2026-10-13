//! Persistent synthetic history-age probe. Measured append batches always contain one record.
//! Population batches are larger so a million prior records are reachable in a bounded run.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use causafera_core::{
    CausalEffect, CausalTarget, HistoryReader, HistoryRecord, OwnedCausalRecord, Phase,
    StateFingerprint,
};
use causafera_persistence::{DurabilityMode, HistoryStore};
use causafera_types::{
    EventId, EventKindId, SimulationTime, StateObjectKindId, StatePropertyId, TraceId,
};
use serde_json::{Value, json};

#[path = "history_scale_stats.rs"]
mod stats;
use stats::{distribution, grouped_ratio_upper_95, ratio_upper_95, regression};

const AGES: [u64; 5] = [100, 1_000, 10_000, 100_000, 1_000_000];
const SAMPLES: usize = 256;
const REPETITIONS: usize = 3;
const WARMUP_TICKS: u64 = 8;
const POPULATION_BATCH: u64 = 1_000;

struct OwnedBundle(PathBuf);

impl OwnedBundle {
    fn new(age: u64, repetition: usize) -> Result<Self, Box<dyn Error>> {
        let path = std::env::current_dir()?.join("target").join(format!(
            "causafera-history-scale-{}-{age}-{repetition}",
            std::process::id()
        ));
        if path.exists() {
            return Err(format!("benchmark bundle already exists: {}", path.display()).into());
        }
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for OwnedBundle {
    fn drop(&mut self) {
        if self.0.exists()
            && let Err(error) = fs::remove_dir_all(&self.0)
        {
            eprintln!(
                "could not remove owned benchmark bundle {}: {error}",
                self.0.display()
            );
        }
    }
}

fn append_batch(
    store: &mut HistoryStore,
    next_trace: &mut u64,
    count: u64,
) -> Result<(), Box<dyn Error>> {
    let generation = store
        .logical_generation()
        .checked_add(1)
        .ok_or("generation overflow")?;
    let mut records = Vec::with_capacity(usize::try_from(count)?);
    for _ in 0..count {
        let trace = *next_trace;
        let target = CausalTarget::new(StateObjectKindId::new(7), trace, StatePropertyId::new(9));
        let effect = CausalEffect::new(
            target,
            StateFingerprint::new([0; 32]),
            StateFingerprint::new([1; 32]),
        )?;
        let parent = if trace == 1 {
            Vec::new()
        } else {
            vec![TraceId::new(trace - 1)]
        };
        records.push(HistoryRecord::Causal(OwnedCausalRecord::new(
            generation,
            EventId::new(trace),
            TraceId::new(trace),
            SimulationTime::new(generation),
            Phase::Physics,
            EventKindId::new(13),
            parent,
            vec![effect],
        )?));
        *next_trace = next_trace.checked_add(1).ok_or("trace overflow")?;
    }
    store.append_tick(records)?;
    Ok(())
}

fn bytes_under(path: &Path) -> Result<u64, Box<dyn Error>> {
    if !path.exists() {
        return Ok(0);
    }
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        total = total
            .checked_add(bytes_under(&entry?.path())?)
            .ok_or("byte count overflow")?;
    }
    Ok(total)
}

fn thread_cpu_ns() -> Result<u64, Box<dyn Error>> {
    std::thread::yield_now();
    let schedstat = fs::read_to_string("/proc/thread-self/schedstat")?;
    Ok(schedstat
        .split_whitespace()
        .next()
        .ok_or("missing thread CPU time in schedstat")?
        .parse()?)
}

fn resident_bytes() -> Result<u64, Box<dyn Error>> {
    let status = fs::read_to_string("/proc/self/status")?;
    let kib = status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .ok_or("missing VmRSS in proc status")?
        .split_whitespace()
        .next()
        .ok_or("missing VmRSS value")?
        .parse::<u64>()?;
    Ok(kib.checked_mul(1_024).ok_or("VmRSS overflow")?)
}

fn hardware_metadata() -> Value {
    let cpu_model = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|contents| {
            contents
                .lines()
                .find_map(|line| line.strip_prefix("model name\t:"))
                .map(|value| value.trim().to_owned())
        });
    let rustc = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
    json!({
        "cpu_model": cpu_model,
        "logical_cpus": std::thread::available_parallelism().ok().map(|value| value.get()),
        "rustc": rustc,
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH
    })
}

fn run_age(age: u64, repetition: usize) -> Result<Value, Box<dyn Error>> {
    let bundle = OwnedBundle::new(age, repetition)?;
    let mut store = HistoryStore::create(bundle.path(), [0xA5; 32], DurabilityMode::GroupCommit)?;
    let mut next_trace = 1_u64;
    let population_end = age - WARMUP_TICKS;
    while next_trace <= population_end {
        let remaining = population_end - next_trace + 1;
        append_batch(&mut store, &mut next_trace, remaining.min(POPULATION_BATCH))?;
        if (next_trace - 1).is_multiple_of(100_000) {
            eprintln!(
                "history-scale populated age={age} records={}",
                next_trace - 1
            );
        }
    }
    for _ in 0..WARMUP_TICKS {
        append_batch(&mut store, &mut next_trace, 1)?;
    }
    store.flush_durable()?;
    let initial = store.status()?;
    if initial.logical_records != usize::try_from(age)? {
        return Err(format!("history population mismatch at age {age}").into());
    }
    let mut tick_wall_ns = Vec::with_capacity(SAMPLES);
    let mut append_wall_ns = Vec::with_capacity(SAMPLES);
    let mut append_cpu_ns = Vec::with_capacity(SAMPLES);
    let mut group_sync_wall_ns = Vec::new();
    for _ in 0..(SAMPLES * 4) {
        if tick_wall_ns.len() == SAMPLES {
            break;
        }
        let before = store.instrumentation();
        let cpu_before = thread_cpu_ns()?;
        let started = Instant::now();
        append_batch(&mut store, &mut next_trace, 1)?;
        if std::env::var_os("CAUSAFERA_BENCH_FORCE_FULL_RESCAN").is_some() {
            store.logical_digest_full_rescan()?;
        }
        let tick_ns = u64::try_from(started.elapsed().as_nanos())?;
        let cpu_ns = thread_cpu_ns()?
            .checked_sub(cpu_before)
            .ok_or("CPU time reversed")?;
        let after = store.instrumentation();
        if after.logical_digest_full_rescan_count != before.logical_digest_full_rescan_count
            || after.catalog_full_scan_count != before.catalog_full_scan_count
        {
            return Err("ordinary append executed a full-history rescan".into());
        }
        if after.durable_sync_count > before.durable_sync_count {
            group_sync_wall_ns.push(u64::try_from(
                (after.durable_sync_elapsed_wall_time - before.durable_sync_elapsed_wall_time)
                    .as_nanos(),
            )?);
        }
        // Group publication is a separate durability metric, not a logical-tick
        // append sample. This keeps the p95 storage-CPU comparison about the
        // same one-record operation at each prior-history age.
        if after.durable_sync_count == before.durable_sync_count {
            tick_wall_ns.push(tick_ns);
            append_cpu_ns.push(cpu_ns);
            append_wall_ns.push(u64::try_from(
                (after.append_elapsed_wall_time - before.append_elapsed_wall_time).as_nanos(),
            )?);
        }
    }
    if tick_wall_ns.len() != SAMPLES {
        return Err("could not collect enough append samples outside group commits".into());
    }
    store.flush_durable()?;
    let final_status = store.status()?;
    let reader_started = Instant::now();
    let reader = store.durable_reader()?;
    let reader_open_ns = u64::try_from(reader_started.elapsed().as_nanos())?;
    let lookup_started = Instant::now();
    let found = reader.event(TraceId::new(age))?;
    let indexed_event_ns = u64::try_from(lookup_started.elapsed().as_nanos())?;
    if found.is_none() {
        return Err("indexed event missing at measured age".into());
    }
    let cold_bytes = bytes_under(&bundle.path().join("segments"))?
        + bytes_under(&bundle.path().join("checkpoints"))?;
    let index_bytes = bytes_under(&bundle.path().join("catalog.sqlite"))?
        + bytes_under(&bundle.path().join("catalog.sqlite-wal"))?
        + bytes_under(&bundle.path().join("catalog.sqlite-shm"))?;
    let syncs = final_status.durable_sync_count - initial.durable_sync_count;
    let sync_ns =
        final_status.durable_sync_elapsed_wall_time - initial.durable_sync_elapsed_wall_time;
    let commits = final_status.catalog_commit_count - initial.catalog_commit_count;
    let rss_bytes = resident_bytes()?;
    drop(reader);
    drop(store);
    let recovery_started = Instant::now();
    let reopened = HistoryStore::open(bundle.path())?;
    let recovery_ns = u64::try_from(recovery_started.elapsed().as_nanos())?;
    if reopened.status()?.logical_records != final_status.logical_records {
        return Err("reopened record count differs from committed history".into());
    }
    Ok(json!({
        "prior_records": age,
        "repetition": repetition,
        "measured_records_per_tick": 1,
        "samples": SAMPLES,
        "warmup_ticks": WARMUP_TICKS,
        "tick_wall": distribution(&tick_wall_ns),
        "append_wall": distribution(&append_wall_ns),
        "append_cpu": distribution(&append_cpu_ns),
        "append_cpu_samples_ns": append_cpu_ns,
        "durable_sync_count": syncs,
        "durable_sync_wall_ns": u64::try_from(sync_ns.as_nanos())?,
        "group_sync_wall": distribution(&group_sync_wall_ns),
        "catalog_commit_count": commits,
        "cold_bytes": cold_bytes,
        "index_bytes": index_bytes,
        "rss_bytes": rss_bytes,
        "active_segment_bytes": final_status.active_extent,
        "segment_count": final_status.catalog_counts.segments,
        "checkpoint_count": final_status.catalog_counts.checkpoints,
        "reader_open_ns": reader_open_ns,
        "indexed_event_ns": indexed_event_ns,
        "recovery_ns": recovery_ns,
        "full_history_rescans": final_status.logical_digest_full_rescan_count,
        "catalog_full_scans": final_status.catalog_full_scan_count
    }))
}

fn run_every_tick_sync() -> Result<Value, Box<dyn Error>> {
    const SYNC_SAMPLES: usize = 128;
    let bundle = OwnedBundle::new(1_000, 0)?;
    let mut store = HistoryStore::create(bundle.path(), [0xA5; 32], DurabilityMode::EveryTick)?;
    let mut next_trace = 1_u64;
    append_batch(&mut store, &mut next_trace, 1_000)?;
    for _ in 0..WARMUP_TICKS {
        append_batch(&mut store, &mut next_trace, 1)?;
    }
    let mut sync_wall_ns = Vec::with_capacity(SYNC_SAMPLES);
    for _ in 0..SYNC_SAMPLES {
        let before = store.instrumentation();
        append_batch(&mut store, &mut next_trace, 1)?;
        let after = store.instrumentation();
        if after.durable_sync_count != before.durable_sync_count + 1 {
            return Err("EveryTick failed to sync exactly once per tick".into());
        }
        sync_wall_ns.push(u64::try_from(
            (after.durable_sync_elapsed_wall_time - before.durable_sync_elapsed_wall_time)
                .as_nanos(),
        )?);
    }
    let status = store.status()?;
    if status.logical_generation != status.durable_generation || status.pending_frames != 0 {
        return Err("EveryTick has durability lag".into());
    }
    Ok(json!({
        "prior_records": 1_000,
        "warmup_ticks": WARMUP_TICKS,
        "sync_wall": distribution(&sync_wall_ns),
        "durability_lag_frames": status.pending_frames
    }))
}

pub(super) fn run(max_age: Option<&str>) -> Result<Value, Box<dyn Error>> {
    let max_age = max_age
        .map(str::parse::<u64>)
        .transpose()?
        .unwrap_or(u64::MAX);
    let mut ages = Vec::with_capacity(AGES.len() * REPETITIONS);
    for age in AGES.into_iter().filter(|age| *age <= max_age) {
        for repetition in 1..=REPETITIONS {
            eprintln!("history-scale starting age={age} repetition={repetition}/{REPETITIONS}");
            ages.push(run_age(age, repetition)?);
        }
    }
    if ages.is_empty() {
        return Err("maximum age must be at least 100".into());
    }
    let sample_groups_at = |age| -> Result<Vec<Vec<u64>>, Box<dyn Error>> {
        ages.iter()
            .filter(|row| row["prior_records"] == age)
            .map(|row| {
                row["append_cpu_samples_ns"]
                    .as_array()
                    .ok_or("missing CPU samples")?
                    .iter()
                    .map(|value| value.as_u64().ok_or("invalid CPU sample"))
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    };
    let scaling = if max_age >= 1_000_000 {
        let prior_groups = sample_groups_at(1_000)?;
        let mature_groups = sample_groups_at(1_000_000)?;
        let prior_samples = prior_groups.iter().flatten().copied().collect::<Vec<_>>();
        let mature_samples = mature_groups.iter().flatten().copied().collect::<Vec<_>>();
        let prior_p95 = distribution(&prior_samples)["p95_ns"]
            .as_u64()
            .ok_or("missing 1k p95")?;
        let mature_p95 = distribution(&mature_samples)["p95_ns"]
            .as_u64()
            .ok_or("missing 1m p95")?;
        let observed_ratio = Duration::from_nanos(mature_p95).as_secs_f64()
            / Duration::from_nanos(prior_p95.max(1)).as_secs_f64();
        let iid_tick_upper_ratio = ratio_upper_95(&prior_samples, &mature_samples);
        let upper_ratio = grouped_ratio_upper_95(&prior_groups, &mature_groups);
        let regression_x = ages
            .iter()
            .map(|row| {
                row["prior_records"]
                    .as_u64()
                    .map(|value| value as f64)
                    .ok_or("missing regression age")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let regression_y = ages
            .iter()
            .map(|row| {
                row["append_cpu"]["p95_ns"]
                    .as_u64()
                    .map(|value| value as f64)
                    .ok_or("missing regression p95")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let regression_log_x = regression_x
            .iter()
            .map(|value| value.log2())
            .collect::<Vec<_>>();
        json!({
            "independent_repetitions": REPETITIONS,
            "samples_per_age": prior_samples.len(),
            "observed_p95_ratio_1m_to_1k": observed_ratio,
            "grouped_bootstrap_upper_95_ci_ratio": upper_ratio,
            "iid_tick_bootstrap_upper_95_ci_ratio_diagnostic": iid_tick_upper_ratio,
                "threshold": 1.50,
                "passes_upper_ci_threshold": upper_ratio <= 1.50,
                "p95_regression_against_log2_records": regression(&regression_log_x, &regression_y),
                "p95_regression_against_linear_records": regression(&regression_x, &regression_y)
        })
    } else {
        Value::Null
    };
    Ok(json!({
        "schema": "causafera.history-scale.v2",
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "population_batch_records": POPULATION_BATCH,
        "measured_records_per_tick": 1,
        "samples_per_age": SAMPLES,
        "independent_repetitions_per_age": REPETITIONS,
        "warmup_ticks_per_age": WARMUP_TICKS,
        "cpu_source": "/proc/thread-self/schedstat (nanoseconds; includes proc read overhead)",
        "storage_root": "repository_target_filesystem",
        "hardware": hardware_metadata(),
        "ages": ages,
        "every_tick_sync_at_1k": run_every_tick_sync()?,
        "scaling": scaling
    }))
}
