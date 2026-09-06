use clap::Parser;
use solana_sdk::pubkey::Pubkey;
use std::process::ExitCode;
use std::str::FromStr;
use verifier::chain::{AccountFetcher, FixtureFetcher, RpcFetcher};
use verifier::cli::{AttestationArgs, Cli, Command};
use verifier::pipeline::{run, Outcome, PipelineParams};

fn parse_account(hex_id: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_id).map_err(|e| format!("--account: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "--account must be 32 bytes (64 hex chars)".to_string())
}

fn execute(args: AttestationArgs) -> Result<Outcome, String> {
    let mint = Pubkey::from_str(&args.mint).map_err(|e| format!("--mint: {e}"))?;
    let program_id = match &args.program_id {
        Some(s) => Pubkey::from_str(s).map_err(|e| format!("--program-id: {e}"))?,
        None => por::ID,
    };
    let account = args.account.as_deref().map(parse_account).transpose()?;
    let fetcher: Box<dyn AccountFetcher> = match &args.fixtures {
        Some(path) => Box::new(FixtureFetcher::load(path).map_err(|e| e.to_string())?),
        None => Box::new(RpcFetcher::new(args.rpc.clone())),
    };
    let outcome = run(PipelineParams {
        fetcher: fetcher.as_ref(),
        program_id,
        mint,
        sequence: args.sequence,
        dataset_source: args.dataset.clone(),
        signature_path: args.signature.clone(),
        gateway: args.gateway.clone(),
        account,
    });
    // Printing lives here so `run` stays pure and testable.
    match &outcome {
        Outcome::Approved(r) | Outcome::Rejected(r) => {
            if args.json {
                println!("{}", r.to_json());
            } else {
                print!("{}", r.to_human());
            }
        }
        Outcome::OperationalFailure(msg) => eprintln!("por-verify: {msg}"),
    }
    Ok(outcome)
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Attestation(args) => match execute(args) {
            Ok(Outcome::Approved(_)) => ExitCode::from(0),
            Ok(Outcome::Rejected(_)) => ExitCode::from(1),
            Ok(Outcome::OperationalFailure(_)) => ExitCode::from(2),
            Err(msg) => {
                eprintln!("por-verify: operational error: {msg}");
                ExitCode::from(2)
            }
        },
    }
}
