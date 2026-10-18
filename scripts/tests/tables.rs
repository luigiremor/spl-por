use std::fs;

#[test]
fn gen_tables_emits_latex_from_csv() {
    let dir = std::env::temp_dir().join("gen-tables-test");
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("measurements.csv"),
        "scenario,N,metric,value,unit,commit,cluster,timestamp\n\
         measure,10,cu_ingest,42000,cu,abc,devnet,t\n\
         measure,10,verify_time_ms,80.25,ms,abc,devnet,t\n\
         measure,10,proof_steps,4,steps,abc,devnet,t\n\
         measure,100000,verify_time_ms,1725.5,ms,abc,devnet,t\n",
    )
    .unwrap();
    fs::write(
        dir.join("thresholds.toml"),
        "[offchain.verify_time_ms]\nn10 = 500\nn100000 = 30000\n",
    )
    .unwrap();
    fs::write(
        dir.join("matrix.csv"),
        "scenario,N,metric,value,unit,commit,cluster,timestamp\n\
         s01_exact_coverage_ok,100,pass,1,bool,abc,localnet,t\n\
         s99_unknown,1,pass,0,bool,abc,localnet,t\n",
    )
    .unwrap();
    fs::write(
        dir.join("criteria.tex"),
        "\\begin{tabular}{l}\nC1 \\\\\n\\end{tabular}\n",
    )
    .unwrap();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_gen-tables"))
        .args([
            "--measurements",
            dir.join("measurements.csv").to_str().unwrap(),
            "--matrix",
            dir.join("matrix.csv").to_str().unwrap(),
            "--thresholds",
            dir.join("thresholds.toml").to_str().unwrap(),
            "--criteria",
            dir.join("criteria.tex").to_str().unwrap(),
            "--out-dir",
            dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let med = fs::read_to_string(dir.join("tab-medicoes.tex")).unwrap();
    // pt-BR: thousands dot, decimal comma
    assert!(med.contains("42.000") && med.contains("80,25") && med.contains("\\toprule"));
    assert!(med.contains("100.000") && med.contains("1.725,50"), "{med}");
    // measured/ceiling ratio of the verification: 1725.5 / 30000 = 5.75%
    assert!(med.contains("5,8\\%") || med.contains("5,7\\%"), "{med}");
    assert!(
        !med.contains("NaN"),
        "missing metrics must render as a dash, not NaN"
    );
    let mat = fs::read_to_string(dir.join("tab-matriz.tex")).unwrap();
    assert!(mat.contains("S01") && mat.contains("Vetor/premissa") && mat.contains("& (c) \\\\"));
    assert!(mat.contains("s99\\_unknown") && mat.contains("DIVERGIU"));
    assert!(
        mat.contains("não executado"),
        "unexecuted scenarios are flagged"
    );
    assert!(
        mat.contains("R_{\\mathrm{norm}} = S"),
        "expected outcome column from the matrix"
    );
    let crit = fs::read_to_string(dir.join("tab-criterios.tex")).unwrap();
    assert!(crit.contains("C1"));
}
