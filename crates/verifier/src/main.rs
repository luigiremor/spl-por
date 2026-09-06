use clap::Parser;
use std::process::ExitCode;
use verifier::cli::{Cli, Command};

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Attestation(_args) => {
            eprintln!("por-verify: operational error: verification pipeline not wired yet");
            ExitCode::from(2)
        }
    }
}
