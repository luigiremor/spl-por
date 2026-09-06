use clap::{Parser, Subcommand};
use solana_sdk::pubkey::Pubkey;

#[derive(Parser)]
#[command(
    name = "por-attest",
    about = "Reference attester for spl-por (exercises the interface; not part of the contribution)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a deterministic synthetic snapshot and write the canonical dataset
    Snapshot {
        #[arg(long)]
        n: u32,
        #[arg(long)]
        seed: u64,
        #[arg(long, default_value_t = 0)]
        haircut_bps: u16,
        #[arg(long)]
        out: std::path::PathBuf,
    },
    /// Create the PorConfig for a mint (payer == admin == oracle_authority in the PoC)
    InitConfig {
        #[arg(long)]
        mint: Pubkey,
        #[arg(long)]
        keypair: std::path::PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8899")]
        rpc: String,
    },
    /// Publish dataset to IPFS, sign and submit the two-instruction attestation
    Submit {
        #[arg(long)]
        dataset: std::path::PathBuf,
        /// Directory for manifest.json and signature.json
        #[arg(long)]
        out: std::path::PathBuf,
        #[arg(long)]
        mint: Pubkey,
        #[arg(long)]
        keypair: std::path::PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8899")]
        rpc: String,
        /// Defaults to PorConfig.next_sequence read from the chain
        #[arg(long)]
        sequence: Option<u64>,
    },
    /// Fetch and print an AuditRecord as JSON
    Show {
        #[arg(long)]
        mint: Pubkey,
        #[arg(long)]
        sequence: u64,
        #[arg(long, default_value = "http://127.0.0.1:8899")]
        rpc: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Snapshot {
            n,
            seed,
            haircut_bps,
            out,
        } => cmd_snapshot(n, seed, haircut_bps, &out),
        Command::InitConfig { mint, keypair, rpc } => {
            attester::tx::init_config(&rpc, &keypair, &mint)
        }
        Command::Submit {
            dataset,
            out,
            mint,
            keypair,
            rpc,
            sequence,
        } => attester::tx::submit(&rpc, &keypair, &mint, &dataset, &out, sequence),
        Command::Show {
            mint,
            sequence,
            rpc,
        } => attester::tx::show(&rpc, &mint, sequence),
    }
}

fn cmd_snapshot(n: u32, seed: u64, haircut_bps: u16, out: &std::path::Path) -> anyhow::Result<()> {
    use attester::snapshot::{dataset_sha256, generate_snapshot, serialize_dataset};
    let entries = generate_snapshot(n, seed, haircut_bps);
    let bytes = serialize_dataset(&entries);
    std::fs::create_dir_all(out)?;
    let path = out.join("dataset.jsonl");
    std::fs::write(&path, &bytes)?;
    let total: u64 = entries
        .iter()
        .try_fold(0u64, |acc, e| acc.checked_add(e.balance))
        .ok_or_else(|| anyhow::anyhow!("total overflow"))?;
    // Machine-readable summary on stdout (consumed by the E2E script via jq).
    println!(
        "{{\"n\":{},\"total\":\"{}\",\"dataset_sha256\":\"{}\",\"path\":\"{}\"}}",
        entries.len(),
        total,
        hex::encode(dataset_sha256(&bytes)),
        path.display()
    );
    Ok(())
}
