use causafera_core::StateFingerprint;
use causafera_explanation::{
    ClaimEvidenceState, ComparisonContext, ExplanationReport, FrameAssessment, NumericClaimValue,
};
use causafera_lab::{ExperimentError, ExperimentRunner};
use causafera_persistence::{PersistenceError, read_snapshot_file, write_snapshot_file};
use causafera_runtime::{Runtime, RuntimeError, assemble_envelope, disassemble_envelope};
use causafera_types::SimulationTime;
use clap::{Parser, Subcommand};
use thiserror::Error;

#[derive(Parser)]
#[command(name = "causafera")]
#[command(about = "Causafera deterministic headless simulation CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Validate that a short strict replay can execute successfully.
    Doctor,
    /// Run a replay-verified long-run control/intervention experiment.
    Lab {
        /// Non-authoritative CLI experiment selector.
        #[arg(default_value = "long-run")]
        name: String,
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, default_value_t = 1_000)]
        ticks: u64,
        #[arg(long, default_value_t = 100)]
        checkpoint_interval: u64,
        #[arg(long, default_value_t = 400)]
        suppression_from: u64,
        #[arg(long, default_value_t = 600)]
        suppression_through: u64,
    },
    /// Run the causal simulation for a bounded number of ticks.
    Run {
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, default_value_t = 100)]
        ticks: u64,
    },
    /// Save a snapshot of the current runtime state.
    Save {
        #[arg(long, default_value_t = 0)]
        seed: u64,
        #[arg(long, default_value_t = 100)]
        ticks: u64,
        #[arg(long, default_value = "causafera.snapshot")]
        path: String,
    },
    /// Resume a simulation from a saved snapshot and continue running.
    Resume {
        #[arg(long, default_value = "causafera.snapshot")]
        path: String,
        #[arg(long, default_value_t = 100)]
        ticks: u64,
    },
}

pub fn main() -> Result<(), CliError> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor => doctor()?,
        Commands::Lab {
            name,
            seed,
            ticks,
            checkpoint_interval,
            suppression_from,
            suppression_through,
        } => {
            if name != "long-run" {
                return Err(CliError::UnknownLabSelector { name });
            }
            let report = ExperimentRunner::run_control_and_intervention(
                seed,
                ticks,
                checkpoint_interval,
                SimulationTime::new(suppression_from),
                SimulationTime::new(suppression_through),
            )?;
            let control = &report.control.result.final_snapshot;
            let intervention = &report.intervention.result.final_snapshot;
            println!("experiment=long-run status=ok replay_verified=true");
            println!(
                "control ticks={} traces={} mana_total={} resolution_level={} digest={}",
                control.time.raw(),
                control.causal_trace_count,
                control.mana_total,
                control.resolution_level,
                fingerprint_hex(control.canonical_state.fingerprint),
            );
            println!(
                "intervention ticks={} traces={} mana_total={} resolution_level={} digest={}",
                intervention.time.raw(),
                intervention.causal_trace_count,
                intervention.mana_total,
                intervention.resolution_level,
                fingerprint_hex(intervention.canonical_state.fingerprint),
            );
            println!(
                "trajectories_diverged={} measured_replay_wall_ms_control={} measured_replay_wall_ms_intervention={}",
                report.trajectories_diverged,
                report.control.elapsed.as_millis(),
                report.intervention.elapsed.as_millis(),
            );
            println!(
                "recovery baseline_distance={} perturbation_min_distance={} perturbation_max_distance={} final_distance={} time_to_recovery={}",
                report
                    .matched_checkpoint_analysis
                    .pre_intervention_baseline_distance,
                report
                    .matched_checkpoint_analysis
                    .perturbation_minimum_distance,
                report
                    .matched_checkpoint_analysis
                    .perturbation_maximum_distance,
                report.matched_checkpoint_analysis.final_recovery_distance,
                optional_u64(report.matched_checkpoint_analysis.time_to_recovery),
            );
            render_explanation_report(&report.explanation_report);
        }
        Commands::Run { seed, ticks } => {
            let mut runtime = Runtime::from_seed(seed)?;
            let snapshot = runtime.run_ticks(ticks)?;
            println!("simulation_status=ok strict_mode=true");
            println!(
                "ticks={} traces={} physical_events={} mana_changes={} resolution_changes={}",
                snapshot.time.raw(),
                snapshot.causal_trace_count,
                snapshot.physical_events,
                snapshot.mana_cell_changes,
                snapshot.resolution_changes,
            );
            println!(
                "mana_total={} mana_maximum={} resolution_relevance={} resolution_level={} digest={}",
                snapshot.mana_total,
                snapshot.mana_maximum,
                snapshot.resolution_relevance,
                snapshot.resolution_level,
                fingerprint_hex(snapshot.canonical_state.fingerprint),
            );
        }
        Commands::Save { seed, ticks, path } => {
            let mut runtime = Runtime::from_seed(seed)?;
            runtime.run_ticks(ticks)?;
            let data = runtime.export_snapshot()?;
            let envelope = assemble_envelope(&data)?;
            let path = std::path::Path::new(&path);
            write_snapshot_file(path, &envelope)?;
            println!("snapshot_status=ok path={} ticks={}", path.display(), ticks);
        }
        Commands::Resume { path, ticks } => {
            let path = std::path::Path::new(&path);
            let envelope = read_snapshot_file(path)?;
            let data = disassemble_envelope(&envelope)?;
            let mut runtime = Runtime::from_snapshot(data)?;
            let completed = runtime.current_time().raw();
            let snapshot = runtime.run_ticks(ticks)?;
            println!(
                "resume_status=ok path={} resumed_at={} new_ticks={} total_ticks={}",
                path.display(),
                completed,
                ticks,
                snapshot.time.raw(),
            );
            println!(
                "traces={} physical_events={} mana_changes={} resolution_changes={}",
                snapshot.causal_trace_count,
                snapshot.physical_events,
                snapshot.mana_cell_changes,
                snapshot.resolution_changes,
            );
        }
    }
    Ok(())
}

fn render_explanation_report(report: &ExplanationReport) {
    println!(
        "explanation_report experiment={} assessment={}",
        report.experiment.raw(),
        assessment_code(report.overall_assessment),
    );
    for frame in &report.frames {
        println!(
            "explanation_frame checkpoint={} assessment={} claims={}",
            frame.checkpoint_time.raw(),
            assessment_code(frame.overall_assessment),
            frame.claims.len(),
        );
        for claim in &frame.claims {
            println!(
                "explanation_claim schema={} value={} confidence={:.6} evidence_state={} comparison={} traces={}",
                claim.schema.raw(),
                numeric_value_code(claim.value),
                claim.confidence.raw(),
                evidence_state_code(claim.evidence_state),
                comparison_code(claim.comparison),
                trace_list_code(&claim.evidence_traces),
            );
        }
    }
}

fn numeric_value_code(value: NumericClaimValue) -> String {
    match value {
        NumericClaimValue::Scalar { value } => format!("scalar:{value}"),
        NumericClaimValue::Range { start, end } => format!("range:{start}..{end}"),
        NumericClaimValue::Ratio {
            numerator,
            denominator,
        } => format!("ratio:{numerator}/{denominator}"),
    }
}

fn comparison_code(comparison: ComparisonContext) -> String {
    match comparison {
        ComparisonContext::None => "none".to_owned(),
        ComparisonContext::MatchedCohort { cohort } => format!("matched:{}", cohort.raw()),
        ComparisonContext::Counterfactual { cohort } => format!("counterfactual:{}", cohort.raw()),
    }
}

fn trace_list_code(traces: &[causafera_types::TraceId]) -> String {
    let mut output = String::new();
    for (index, trace) in traces.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&trace.raw().to_string());
    }
    output
}

fn assessment_code(assessment: FrameAssessment) -> &'static str {
    match assessment {
        FrameAssessment::Supported => "supported",
        FrameAssessment::Partial => "partial",
        FrameAssessment::Unsupported => "unsupported",
        FrameAssessment::Unknown => "unknown",
    }
}

fn evidence_state_code(state: ClaimEvidenceState) -> &'static str {
    match state {
        ClaimEvidenceState::Supported => "supported",
        ClaimEvidenceState::Unsupported => "unsupported",
        ClaimEvidenceState::Unknown => "unknown",
    }
}

fn optional_u64(value: Option<u64>) -> String {
    match value {
        Some(value) => value.to_string(),
        None => "unknown".to_owned(),
    }
}

fn doctor() -> Result<(), CliError> {
    let mut first = Runtime::from_seed(0)?;
    let mut second = Runtime::from_seed(0)?;
    let first = first.run_ticks(8)?;
    let second = second.run_ticks(8)?;
    if first != second {
        return Err(CliError::StrictReplayMismatch);
    }
    println!("Causafera Doctor");
    println!(
        "status=ok runtime=true replay=true ticks={} traces={} digest={}",
        first.time.raw(),
        first.causal_trace_count,
        fingerprint_hex(first.canonical_state.fingerprint),
    );
    Ok(())
}

fn fingerprint_hex(fingerprint: StateFingerprint) -> String {
    const HEX: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut output = String::with_capacity(64);
    for byte in fingerprint.bytes() {
        output.push(HEX[usize::from(byte >> 4)]);
        output.push(HEX[usize::from(byte & 0x0f)]);
    }
    output
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("unknown non-authoritative lab selector: {name}")]
    UnknownLabSelector { name: String },
    #[error("strict replay mismatch")]
    StrictReplayMismatch,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Experiment(#[from] ExperimentError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
}
