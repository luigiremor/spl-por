//! Measurement harness (monograph §5.4.2): for every N of the scalability
//! axis records, in a CSV with fixed columns, the tree build time, the
//! inclusion-proof size, the ingestion cost (compute units, transaction
//! bytes) and the complete off-chain verification time. Always run in
//! --release against a validator with `por` deployed:
//!   por-measure [--rpc URL] [--out results/measurements-<cluster>.csv] [--dry-run]
use scenarios::{gen::*, harness::*};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use verifier::chain::FixtureFetcher;

fn arg_value(args: &[String], key: &str) -> Option<String> {
    args.iter()
        .position(|a| a == key)
        .and_then(|i| args.get(i + 1).cloned())
}

/// Median of 10 timed runs after 3 discarded warm-ups, in milliseconds.
fn median_ms(mut f: impl FnMut()) -> f64 {
    for _ in 0..3 {
        f();
    }
    let mut samples: Vec<f64> = (0..10)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1_000.0
        })
        .collect();
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    (samples[4] + samples[5]) / 2.0
}

/// `por-verify` next to this binary (same cargo profile) or `POR_VERIFY_BIN`.
fn por_verify_bin() -> PathBuf {
    std::env::var_os("POR_VERIFY_BIN")
        .map(Into::into)
        .unwrap_or_else(|| {
            let exe = std::env::current_exe().unwrap();
            exe.with_file_name("por-verify")
        })
}

/// Compute units consumed by the LANDED transaction (cross-check of the
/// simulation), through raw getTransaction.
fn landed_cu(ctx: &Ctx, sig: &solana_sdk::signature::Signature) -> f64 {
    let v: serde_json::Value = ctx
        .rpc
        .send(
            solana_client::rpc_request::RpcRequest::GetTransaction,
            serde_json::json!([sig.to_string(), {"encoding": "json", "commitment": "finalized",
                "maxSupportedTransactionVersion": 0}]),
        )
        .expect("getTransaction");
    v["meta"]["computeUnitsConsumed"]
        .as_f64()
        .expect("computeUnitsConsumed")
}

fn measure_n(n: usize, rpc_url: &str, out: &Path) {
    let rec = |metric: &str, value: f64, unit: &str| record(out, "measure", n, metric, value, unit);
    let rows = generate(n, SEED);

    // 1) tree construction
    rec(
        "build_time_ms",
        median_ms(|| {
            sum_tree::build(&rows, MAX_VALUE).unwrap();
        }),
        "ms",
    );
    let tree = sum_tree::build(&rows, MAX_VALUE).unwrap();

    // 2) inclusion proof for leaf N/2: steps and bytes (32 hash + 8 sum + 1 side)
    let proof = sum_tree::prove(&tree, n / 2);
    rec("proof_steps", proof.0.len() as f64, "steps");
    rec("proof_bytes", (proof.0.len() * 41) as f64, "bytes");

    // 3) on-chain ingestion: CU by simulation + serialized size, then landed
    let ctx = setup(rpc_url, 6, 2, MAX_VALUE);
    mint_supply_to(&ctx, tree.total * 10u64.pow(4));
    let ds = serialize_dataset(&rows);
    let msg = attestation_msg(&ctx, 0, &tree, &ds);
    let tx = build_submit_tx(&ctx, &msg, &ctx.oracle, Tamper::None);
    let sim = ctx.rpc.simulate_transaction(&tx).expect("simulate");
    assert!(
        sim.value.err.is_none(),
        "simulation failed: {:?}",
        sim.value
    );
    rec(
        "cu_ingest",
        sim.value.units_consumed.expect("units_consumed") as f64,
        "cu",
    );
    rec(
        "tx_size",
        bincode::serialize(&tx).unwrap().len() as f64,
        "bytes",
    );
    let sig = submit(&ctx, &tx).expect("ingestion must land");
    rec("cu_ingest_landed", landed_cu(&ctx, &sig), "cu");

    // 4) complete verification in-process over a snapshot of the accounts
    //    (algorithmic cost only: no RPC latency, no process start-up)
    let dir = write_dataset_dir(&format!("tmp-measure-n{n}"), &ds, &sign(&ctx.oracle, &msg));
    let fixture = FixtureFetcher::from_json(&snapshot_fixture(&ctx).to_string()).unwrap();
    rec(
        "verify_time_ms",
        median_ms(|| assert_eq!(verify_with(&fixture, &ctx.mint, 0, Some(&dir)), 0)),
        "ms",
    );

    // 5) end to end: por-verify process + RPC, for transparency
    let t = Instant::now();
    let status = std::process::Command::new(por_verify_bin())
        .args([
            "attestation",
            "--mint",
            &ctx.mint.to_string(),
            "--sequence",
            "0",
            "--rpc",
            rpc_url,
            "--dataset",
            dir.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .status()
        .expect("spawn por-verify (build -p verifier in the same profile)");
    assert_eq!(status.code(), Some(0), "por-verify must approve");
    rec("verify_e2e_ms", t.elapsed().as_secs_f64() * 1_000.0, "ms");
    eprintln!("[measure] N={n} done (mint {})", ctx.mint);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cluster = std::env::var("POR_CLUSTER").unwrap_or_else(|_| "localnet".into());
    let out = arg_value(&args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| results_path(&format!("measurements-{cluster}.csv")));
    record_header_only(&out);
    if args.iter().any(|a| a == "--dry-run") {
        return;
    }
    if cfg!(debug_assertions) {
        eprintln!("por-measure: refusing to measure in a debug build; use --release");
        std::process::exit(2);
    }
    let rpc = arg_value(&args, "--rpc").unwrap_or_else(rpc_url);
    for n in SIZES {
        measure_n(n, &rpc, &out);
    }
    println!("measurements written to {}", out.display());
}
