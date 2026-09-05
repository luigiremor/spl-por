use clap::{Parser, Subcommand};

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
