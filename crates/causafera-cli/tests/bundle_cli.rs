//! The CLI's durable-history lifecycle, driven through the real binary.
//!
//! See `plans/causal-history-storage.md` Todo 8.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use causafera_persistence::{DurabilityMode, write_snapshot_file};
use causafera_runtime::{BundleOptions, Runtime, RuntimeConfig, assemble_envelope};

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "causafera-cli-{name}-p{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary directory");
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn causafera(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_causafera"))
        .args(arguments)
        .output()
        .expect("the causafera binary runs")
}

fn text(output: &[u8]) -> String {
    String::from_utf8_lossy(output).into_owned()
}

fn succeeded(output: &Output) -> String {
    assert!(
        output.status.success(),
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        text(&output.stdout),
        text(&output.stderr)
    );
    text(&output.stdout)
}

fn failed(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "expected a failure, got:\n{}",
        text(&output.stdout)
    );
    text(&output.stderr)
}

/// The value of `key=` on the first line of `output` that has it.
fn field(output: &str, key: &str) -> String {
    output
        .lines()
        .flat_map(str::split_whitespace)
        .find_map(|token| token.strip_prefix(&format!("{key}=")))
        .unwrap_or_else(|| panic!("no `{key}=` in:\n{output}"))
        .to_owned()
}

fn path_arg(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

#[test]
fn save_creates_the_complete_layout_and_resume_continues_the_run() {
    for durability in ["group", "every-tick"] {
        // Given
        let temp = TempDir::new(&format!("save-{durability}"));
        let bundle = temp.join("bundle");
        let control = succeeded(&causafera(&["run", "--seed", "3", "--ticks", "50"]));

        // When: 40 ticks are saved, and the bundle is resumed for ten more.
        let saved = succeeded(&causafera(&[
            "save",
            "--seed",
            "3",
            "--ticks",
            "40",
            "--path",
            path_arg(&bundle),
            "--durability",
            durability,
        ]));
        let resumed = succeeded(&causafera(&[
            "resume",
            "--path",
            path_arg(&bundle),
            "--ticks",
            "10",
        ]));

        // Then: the layout was complete, nothing was left behind in a group, and
        // the resumed run ends exactly where the uninterrupted one does.
        assert_eq!(field(&saved, "bundle_status"), "created");
        assert_eq!(field(&saved, "durable_lag"), "0");
        assert_eq!(field(&saved, "ticks"), "40");
        for entry in ["catalog.sqlite", "segments", "checkpoints", "snapshots"] {
            assert!(bundle.join(entry).exists(), "missing {entry}");
        }
        assert_eq!(field(&resumed, "resume_status"), "ok");
        assert_eq!(field(&resumed, "resumed_at"), "40");
        assert_eq!(field(&resumed, "lost_ticks"), "0");
        assert_eq!(field(&resumed, "digest"), field(&control, "digest"));
        println!(
            "MARKER:cli_save_resume durability={durability} digest={}",
            field(&resumed, "digest")
        );
    }
}

#[test]
fn a_second_resume_continues_from_the_first() {
    let temp = TempDir::new("chain");
    let bundle = temp.join("bundle");
    succeeded(&causafera(&[
        "save",
        "--seed",
        "4",
        "--ticks",
        "10",
        "--path",
        path_arg(&bundle),
    ]));
    for _ in 0..2 {
        succeeded(&causafera(&[
            "resume",
            "--path",
            path_arg(&bundle),
            "--ticks",
            "10",
        ]));
    }
    let third = succeeded(&causafera(&[
        "resume",
        "--path",
        path_arg(&bundle),
        "--ticks",
        "0",
    ]));
    let control = succeeded(&causafera(&["run", "--seed", "4", "--ticks", "30"]));
    assert_eq!(field(&third, "resumed_at"), "30");
    assert_eq!(field(&third, "digest"), field(&control, "digest"));
}

#[test]
fn save_never_overwrites_an_existing_path() {
    let temp = TempDir::new("overwrite");
    let bundle = temp.join("bundle");
    succeeded(&causafera(&[
        "save",
        "--ticks",
        "2",
        "--path",
        path_arg(&bundle),
    ]));
    let before = fs::read_dir(&bundle).expect("bundle").count();
    let file = temp.join("legacy.snapshot");
    fs::write(&file, b"data").expect("file");

    for target in [&bundle, &file] {
        let error = failed(&causafera(&[
            "save",
            "--ticks",
            "2",
            "--path",
            path_arg(target),
        ]));
        assert!(error.contains("already exists"), "{error}");
    }
    assert_eq!(fs::read_dir(&bundle).expect("bundle").count(), before);
    assert_eq!(fs::read(&file).expect("file"), b"data");
}

#[test]
fn a_legacy_single_file_snapshot_names_the_migration_target() {
    // Given: a snapshot in the old single-file format.
    let temp = TempDir::new("legacy");
    let file = temp.join("causafera.snapshot");
    let mut runtime = Runtime::from_seed(5).expect("session");
    runtime.run_ticks(3).expect("ticks");
    let data = runtime
        .export_snapshot_with_full_history()
        .expect("complete export");
    write_snapshot_file(&file, &assemble_envelope(&data).expect("envelope")).expect("write");

    // When / Then: resuming it in place is refused, naming `--migrate-to`.
    let error = failed(&causafera(&[
        "resume",
        "--path",
        path_arg(&file),
        "--ticks",
        "1",
    ]));
    assert!(error.contains("--migrate-to"), "{error}");
    assert!(error.contains("legacy"), "{error}");

    // And: the flag is accepted but the migration, which needs its proof, is
    // not available yet, so it refuses rather than guessing.
    let target = temp.join("migrated");
    let error = failed(&causafera(&[
        "resume",
        "--path",
        path_arg(&file),
        "--migrate-to",
        path_arg(&target),
    ]));
    assert!(error.contains("not available yet"), "{error}");
    assert!(!target.exists());
}

#[test]
fn resume_reports_the_ticks_a_crash_lost() {
    // Given: a group-commit session saved at tick 5 that ran seven more ticks
    // and died before its group was published.
    let temp = TempDir::new("crash");
    let bundle = temp.join("bundle");
    let mut session = Runtime::create_bundle(
        RuntimeConfig::new(6),
        &bundle,
        BundleOptions {
            durability: DurabilityMode::GroupCommit,
            ..BundleOptions::default()
        },
    )
    .expect("bundle");
    session.run_ticks(5).expect("first ticks");
    session.save_bundle().expect("save");
    session.run_ticks(7).expect("later ticks");
    session.simulate_crash();

    // When
    let resumed = succeeded(&causafera(&[
        "resume",
        "--path",
        path_arg(&bundle),
        "--ticks",
        "7",
    ]));

    // Then: the loss is named, and the lost ticks simulate again to the same end.
    assert_eq!(field(&resumed, "resumed_at"), "5");
    assert_eq!(field(&resumed, "lost_ticks"), "7");
    assert_ne!(field(&resumed, "lost_generations"), "none");
    let control = succeeded(&causafera(&["run", "--seed", "6", "--ticks", "12"]));
    assert_eq!(field(&resumed, "digest"), field(&control, "digest"));
    println!(
        "MARKER:cli_crash lost_ticks=7 lost_generations={}",
        field(&resumed, "lost_generations")
    );
}

#[test]
fn run_keeps_its_history_only_when_asked() {
    let temp = TempDir::new("run");
    let bundle = temp.join("bundle");

    let transient = succeeded(&causafera(&["run", "--seed", "7", "--ticks", "6"]));
    assert_eq!(field(&transient, "history_status"), "temporary");

    let retained = succeeded(&causafera(&[
        "run",
        "--seed",
        "7",
        "--ticks",
        "6",
        "--history",
        path_arg(&bundle),
    ]));
    assert_eq!(field(&retained, "history_status"), "retained");
    assert_eq!(field(&retained, "path"), path_arg(&bundle));
    assert_eq!(field(&retained, "digest"), field(&transient, "digest"));
    let resumed = succeeded(&causafera(&[
        "resume",
        "--path",
        path_arg(&bundle),
        "--ticks",
        "0",
    ]));
    assert_eq!(field(&resumed, "resumed_at"), "6");
}

#[test]
fn resume_refuses_a_damaged_bundle() {
    let temp = TempDir::new("damaged");
    let bundle = temp.join("bundle");
    succeeded(&causafera(&[
        "save",
        "--ticks",
        "3",
        "--path",
        path_arg(&bundle),
    ]));
    let checkpoint = fs::read_dir(bundle.join("checkpoints"))
        .expect("checkpoints")
        .next()
        .expect("a checkpoint")
        .expect("entry")
        .path();
    let mut bytes = fs::read(&checkpoint).expect("checkpoint");
    let middle = bytes.len() / 2;
    bytes[middle] ^= 0xFF;
    fs::write(&checkpoint, bytes).expect("damage");

    let error = failed(&causafera(&[
        "resume",
        "--path",
        path_arg(&bundle),
        "--ticks",
        "1",
    ]));
    assert!(!error.is_empty());
}
