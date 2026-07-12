use std::error::Error;
use std::fmt::Write as _;

use clap::{Parser, Subcommand};
use ontopolis_core::StateFingerprint;
use ontopolis_lab::ExperimentRunner;
use ontopolis_runtime::Runtime;
use ontopolis_types::SimulationTime;

#[derive(Parser)]
#[command(name = "ontopolis")]
#[command(about = "Ontopolis deterministic headless simulation CLI")]
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
}

pub fn main() -> Result<(), Box<dyn Error>> {
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
                return Err(format!("unknown non-authoritative lab selector: {name}").into());
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
                fingerprint_hex(control.canonical_state),
            );
            println!(
                "intervention ticks={} traces={} mana_total={} resolution_level={} digest={}",
                intervention.time.raw(),
                intervention.causal_trace_count,
                intervention.mana_total,
                intervention.resolution_level,
                fingerprint_hex(intervention.canonical_state),
            );
            println!(
                "trajectories_diverged={} measured_replay_wall_ms_control={} measured_replay_wall_ms_intervention={}",
                report.trajectories_diverged,
                report.control.elapsed.as_millis(),
                report.intervention.elapsed.as_millis(),
            );
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
                fingerprint_hex(snapshot.canonical_state),
            );
        }
    }
    Ok(())
}

fn doctor() -> Result<(), Box<dyn Error>> {
    let mut first = Runtime::from_seed(0)?;
    let mut second = Runtime::from_seed(0)?;
    let first = first.run_ticks(8)?;
    let second = second.run_ticks(8)?;
    if first != second {
        return Err("strict replay mismatch".into());
    }
    println!("Ontopolis Doctor");
    println!(
        "status=ok runtime=true replay=true ticks={} traces={} digest={}",
        first.time.raw(),
        first.causal_trace_count,
        fingerprint_hex(first.canonical_state),
    );
    Ok(())
}

fn fingerprint_hex(fingerprint: StateFingerprint) -> String {
    let mut output = String::with_capacity(64);
    for byte in fingerprint.bytes() {
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}
