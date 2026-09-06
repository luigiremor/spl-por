use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "por-verify",
    version,
    about = "Public proof-of-reserves verifier: re-executes the four chained checks + MMR consistency from public data"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Verify one anchored attestation: 4 chained checks + MMR consistency
    Attestation(AttestationArgs),
}

#[derive(clap::Args)]
pub struct AttestationArgs {
    /// Mint address (base58) the attestation refers to
    #[arg(long)]
    pub mint: String,
    /// Attestation sequence number
    #[arg(long)]
    pub sequence: u64,
    /// RPC endpoint URL
    #[arg(long, default_value = "http://127.0.0.1:8899")]
    pub rpc: String,
    /// Dataset source: local directory holding dataset.jsonl (+ signature.json)
    /// or an IPFS CID (default: AuditRecord.ipfs_cid via --gateway)
    #[arg(long)]
    pub dataset: Option<String>,
    /// Path to signature.json ({"pubkey","signature"}); default: <dataset dir>/signature.json
    #[arg(long)]
    pub signature: Option<PathBuf>,
    /// IPFS HTTP gateway used when the dataset source is a CID
    #[arg(long, default_value = "http://127.0.0.1:8080")]
    pub gateway: String,
    /// PoR program id (base58); default: the id compiled into the `por` crate
    #[arg(long)]
    pub program_id: Option<String>,
    /// Restrict inclusion check to one account_id (64 hex chars); default: all leaves
    #[arg(long)]
    pub account: Option<String>,
    /// Read accounts from a fixture file instead of the RPC (offline/testing)
    #[arg(long)]
    pub fixtures: Option<PathBuf>,
    /// Emit the machine-readable JSON report instead of the human one
    #[arg(long)]
    pub json: bool,
}
