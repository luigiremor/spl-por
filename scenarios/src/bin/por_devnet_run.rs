//! Full round on a public test network (monograph §5.4.2: "environment
//! parameters recorded next to the results"): writes results/run-meta.toml,
//! then runs the on-chain matrix (S01–S08) and por-measure against the
//! program deployed there. Uses a DEDICATED payer (never the CLI default).
//!   por-devnet-run [--rpc URL] [--out-dir results] [--payer keys/devnet-payer.json] [--meta-only]
use std::{fs, path::PathBuf, process::Command};

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).cloned())
}

fn cli_version(bin: &str) -> String {
    Command::new(bin)
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unavailable".into())
}

fn run(program: &str, args: &[&str], env: &[(&str, &str)]) {
    eprintln!("[devnet-run] {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .envs(env.iter().copied())
        .status()
        .expect("spawn");
    assert!(status.success(), "{program} {args:?} failed: {status}");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rpc_url =
        arg_value(&args, "--rpc").unwrap_or_else(|| "https://api.devnet.solana.com".into());
    let out_dir = PathBuf::from(arg_value(&args, "--out-dir").unwrap_or_else(|| "results".into()));
    let payer = arg_value(&args, "--payer").unwrap_or_else(|| "keys/devnet-payer.json".into());
    // absolute: `cargo test` children run with the crate dir as cwd, not the workspace root
    let payer = std::fs::canonicalize(&payer)
        .unwrap_or_else(|e| panic!("payer keypair {payer}: {e}"))
        .to_string_lossy()
        .into_owned();
    fs::create_dir_all(&out_dir).unwrap();

    let rpc = solana_client::rpc_client::RpcClient::new(rpc_url.clone());
    let validator_version = rpc
        .get_version()
        .map(|v| v.solana_core)
        .unwrap_or_else(|_| "unavailable".into());
    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let started = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let meta_path = out_dir.join("run-meta.toml");
    fs::write(
        &meta_path,
        format!(
            "cluster = \"devnet\"\nrpc_url = \"{rpc_url}\"\nvalidator_version = \"{validator_version}\"\n\
             solana_cli = \"{}\"\nanchor = \"{}\"\nprogram_id = \"{}\"\npayer = \"{payer}\"\n\
             commit = \"{commit}\"\nstarted_at = \"{started}\"\n",
            cli_version("solana"),
            cli_version("anchor"),
            por::ID,
        ),
    )
    .unwrap();
    if args.iter().any(|a| a == "--meta-only") {
        return;
    }

    // Isolated kubo repo for multi-block datasets (N >= 10k), as eval-localnet.sh does;
    // an explicit IPFS_PATH in the environment wins.
    let ipfs_path = std::env::var("IPFS_PATH").unwrap_or_else(|_| {
        let p = std::env::temp_dir().join("por-devnet-ipfs");
        if !p.join("config").exists() {
            run(
                "ipfs",
                &["init", "--profile=test"],
                &[("IPFS_PATH", p.to_str().unwrap())],
            );
        }
        p.to_string_lossy().into_owned()
    });
    let env = [
        ("POR_RPC", rpc_url.as_str()),
        ("POR_CLUSTER", "devnet"),
        ("POR_RECORD", "1"),
        ("POR_PAYER", payer.as_str()),
        ("IPFS_PATH", ipfs_path.as_str()),
    ];
    // On-chain matrix S01–S08 against devnet (append-only CSV: start clean).
    let _ = fs::remove_file(out_dir.join("matrix-devnet.csv"));
    run(
        "cargo",
        &[
            "test",
            "-p",
            "scenarios",
            "--test",
            "matrix_conformity",
            "--test",
            "matrix_onchain",
            "--",
            "--ignored",
            "--test-threads=1",
        ],
        &env,
    );
    // Measurements (release only).
    let csv = out_dir.join("measurements-devnet.csv");
    let _ = fs::remove_file(&csv);
    run(
        "cargo",
        &["build", "--release", "-p", "verifier", "-p", "scenarios"],
        &[],
    );
    run(
        "target/release/por-measure",
        &["--rpc", &rpc_url, "--out", csv.to_str().unwrap()],
        &env,
    );

    let finished = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    let mut meta = fs::read_to_string(&meta_path).unwrap();
    meta.push_str(&format!("finished_at = \"{finished}\"\n"));
    fs::write(&meta_path, meta).unwrap();
    println!("devnet round complete: {}", meta_path.display());
}
