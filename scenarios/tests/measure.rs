use scenarios::gen::{generate, MAX_VALUE, SEED, SIZES};
use sum_tree::{build, prove};

#[test]
fn proof_growth_is_logarithmic() {
    for n in SIZES {
        let rows = generate(n, SEED);
        let tree = build(&rows, MAX_VALUE).unwrap();
        let steps = prove(&tree, n / 2).0.len();
        let lo = (n as f64).log2().floor() as usize;
        let hi = (n as f64).log2().ceil() as usize;
        assert!(
            steps >= lo && steps <= hi,
            "N={n}: proof has {steps} steps, expected in [{lo},{hi}]"
        );
    }
}

#[test]
fn measurement_csv_has_fixed_header() {
    // por-measure --dry-run writes only the header
    let out = std::env::temp_dir().join("por-measure-test.csv");
    let _ = std::fs::remove_file(&out);
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_por-measure"))
        .args(["--dry-run", "--out", out.to_str().unwrap()])
        .status()
        .expect("spawn por-measure");
    assert!(status.success());
    let head = std::fs::read_to_string(&out).unwrap();
    assert_eq!(
        head,
        "scenario,N,metric,value,unit,commit,cluster,timestamp\n"
    );
}
