#[test]
fn run_meta_records_environment() {
    // por-devnet-run --meta-only writes run-meta.toml without submitting anything.
    // Points at the local validator URL only to avoid network in unit tests; the
    // RPC version probe falls back to "unavailable" when nothing listens there.
    let dir = std::env::temp_dir().join("por-devnet-test");
    let _ = std::fs::remove_dir_all(&dir);
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_por-devnet-run"))
        .args([
            "--meta-only",
            "--rpc",
            "http://127.0.0.1:9",
            "--out-dir",
            dir.to_str().unwrap(),
        ])
        .status()
        .expect("spawn por-devnet-run");
    assert!(status.success());
    let meta = std::fs::read_to_string(dir.join("run-meta.toml")).unwrap();
    for key in [
        "cluster",
        "rpc_url",
        "validator_version",
        "solana_cli",
        "anchor",
        "program_id",
        "commit",
        "started_at",
    ] {
        assert!(meta.contains(key), "run-meta.toml must record {key}");
    }
}
