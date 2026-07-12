use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ontopolis")]
#[command(about = "Ontopolis simulation CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the simulation doctor
    Doctor,
    /// Run a lab experiment
    Lab { name: String },
    /// Start the runtime
    Run,
}

pub fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor => {
            println!("Ontopolis Doctor");
            println!("Status: OK");
        }
        Commands::Lab { name } => {
            println!("Running lab experiment: {}", name);
        }
        Commands::Run => {
            println!("Starting Ontopolis runtime...");
        }
    }
}
