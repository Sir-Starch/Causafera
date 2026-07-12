use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("help");

    match cmd {
        "ci" => {
            println!("Running CI checks...");
            let status = Command::new("cargo")
                .args(["fmt", "--check"])
                .status()
                .expect("cargo fmt failed");
            if !status.success() {
                std::process::exit(1);
            }
            let status = Command::new("cargo")
                .args([
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--",
                    "-D",
                    "warnings",
                ])
                .status()
                .expect("cargo clippy failed");
            if !status.success() {
                std::process::exit(1);
            }
            let status = Command::new("cargo")
                .args(["test", "--workspace", "--all-features"])
                .status()
                .expect("cargo test failed");
            if !status.success() {
                std::process::exit(1);
            }
            let status = Command::new("cargo")
                .args(["test", "--workspace", "--no-default-features"])
                .status()
                .expect("cargo test no-default-features failed");
            if !status.success() {
                std::process::exit(1);
            }
            println!("CI checks passed.");
        }
        _ => {
            println!("Usage: cargo xtask <command>");
            println!("Commands:");
            println!("  ci    Run full CI checks");
        }
    }
}
