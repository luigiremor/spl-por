use std::fs;

#[test]
fn criteria_check_flags_exceedance_and_records_it() {
    let dir = std::env::temp_dir().join("criteria-check-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("thresholds.toml"),
        "schema = 1\n[onchain]\ncu_ingest_max = 200000\ntx_size_max = 1232\n\
         [offchain.verify_time_ms]\nn10 = 500\n[proof]\nstep_bytes = 41\n",
    )
    .unwrap();
    fs::write(
        dir.join("matrix.csv"),
        "scenario,N,metric,value,unit,commit,cluster,timestamp\n\
         s01_exact_coverage_ok,100,pass,1,bool,abc,localnet,t\n\
         s03_undercollateralized_divergent,100,pass,1,bool,abc,localnet,t\n",
    )
    .unwrap();
    fs::write(
        dir.join("measurements.csv"),
        "scenario,N,metric,value,unit,commit,cluster,timestamp\n\
         measure,10,cu_ingest,250000,cu,abc,devnet,t\n\
         measure,10,tx_size,900,bytes,abc,devnet,t\n\
         measure,10,verify_time_ms,80,ms,abc,devnet,t\n\
         measure,10,proof_steps,4,steps,abc,devnet,t\n",
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_criteria-check"))
        .args([
            "--thresholds",
            dir.join("thresholds.toml").to_str().unwrap(),
            "--matrix",
            dir.join("matrix.csv").to_str().unwrap(),
            "--measurements",
            dir.join("measurements.csv").to_str().unwrap(),
            "--out-dir",
            dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "a failed criterion is a result, not an error"
    );
    let md = fs::read_to_string(dir.join("criteria.md")).unwrap();
    // C2 approved (s03 passed); C1/C3 fail on missing scenarios; C4 fails on
    // cu_ingest 250000 > 200000 WITH the exceedance recorded, not hidden.
    assert!(md.contains("C2") && md.contains("APROVADO"));
    assert!(md.contains("REPROVADO"));
    assert!(md.contains("+50000"), "exceedance must be recorded: {md}");
    assert!(
        md.contains("s02_overcollateralized_ok"),
        "missing scenarios named"
    );
    // singular for one scenario, and the short ids of the monograph's tables
    assert!(
        md.contains("1 cenário com resultado igual ao esperado (S03)"),
        "{md}"
    );
    let tex = fs::read_to_string(dir.join("criteria.tex")).unwrap();
    assert!(tex.contains("\\bottomrule") && tex.contains("+50000"));
}
