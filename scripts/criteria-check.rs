//! Confronts the recorded evidence with the four acceptance criteria of the
//! monograph (§1.3), as §5.5 prescribes: one row per criterion, verdict
//! APROVADO/REPROVADO and the exceedance recorded (never hidden).
//!   criteria-check [--thresholds scenarios/thresholds.toml] [--matrix results/matrix-localnet.csv]
//!                  [--measurements results/measurements-devnet.csv] [--out-dir results]
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Deserialize)]
struct Thresholds {
    onchain: Onchain,
    offchain: Offchain,
}
#[derive(Deserialize)]
struct Onchain {
    cu_ingest_max: f64,
    tx_size_max: f64,
}
#[derive(Deserialize)]
struct Offchain {
    verify_time_ms: HashMap<String, f64>,
}

struct Row {
    scenario: String,
    n: u64,
    metric: String,
    value: f64,
    cluster: String,
}

/// Fixed 8-column CSV written by `scenarios::harness::record` (no quoting).
fn read_rows(path: &PathBuf) -> Vec<Row> {
    let text = fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let c: Vec<&str> = l.split(',').collect();
            assert!(c.len() >= 8, "malformed row: {l}");
            Row {
                scenario: c[0].into(),
                n: c[1].parse().unwrap(),
                metric: c[2].into(),
                value: c[3].parse().unwrap(),
                cluster: c[6].into(),
            }
        })
        .collect()
}

fn scenarios_pass(matrix: &[Row], names: &[&str]) -> (bool, String) {
    let missing: Vec<&str> = names
        .iter()
        .copied()
        .filter(|s| {
            !matrix
                .iter()
                .any(|r| r.scenario == *s && r.metric == "pass" && r.value == 1.0)
        })
        .collect();
    if missing.is_empty() {
        (
            true,
            format!(
                "{} cenários com resultado igual ao esperado ({})",
                names.len(),
                names.join(", ")
            ),
        )
    } else {
        (false, format!("faltou/reprovou: {}", missing.join(", ")))
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .map(|i| PathBuf::from(&args[i + 1]))
    };
    let th_path = get("--thresholds").unwrap_or_else(|| "scenarios/thresholds.toml".into());
    let th: Thresholds = toml::from_str(&fs::read_to_string(&th_path).unwrap()).unwrap();
    let matrix =
        read_rows(&get("--matrix").unwrap_or_else(|| "results/matrix-localnet.csv".into()));
    let meas = read_rows(
        &get("--measurements").unwrap_or_else(|| "results/measurements-devnet.csv".into()),
    );
    let out_dir = get("--out-dir").unwrap_or_else(|| "results".into());

    let c1 = scenarios_pass(
        &matrix,
        &[
            "s01_exact_coverage_ok",
            "s02_overcollateralized_ok",
            "s04_replay_rejected",
            "s05_sequence_gap_rejected",
            "s06_wrong_mint_rejected",
            "s07_wrong_decimals_rejected",
            "s08_corrupt_signature_rejected",
            "s10_corrupt_inclusion_proof_rejected",
            "s11_hu_subtotal_tamper_rejected",
            "s13_node_as_leaf_rejected",
            "s14_dataset_one_byte_tamper_rejected",
        ],
    );
    let c2 = scenarios_pass(&matrix, &["s03_undercollateralized_divergent"]);
    let c3 = scenarios_pass(
        &matrix,
        &[
            "s09_rotated_key_history_approved",
            "s16_incoherent_status_rejected",
            "s01_exact_coverage_ok",
        ],
    );

    // C4: every measurement against its ceiling; exceedance = value - limit.
    let mut c4_fail = vec![];
    let mut checked = 0usize;
    for r in meas.iter().filter(|r| r.scenario == "measure") {
        let limit = match r.metric.as_str() {
            "cu_ingest" | "cu_ingest_landed" => Some(th.onchain.cu_ingest_max),
            "tx_size" => Some(th.onchain.tx_size_max),
            "verify_time_ms" => {
                let key = format!("n{}", r.n);
                let l = th.offchain.verify_time_ms.get(&key).copied();
                if l.is_none() {
                    c4_fail.push(format!(
                        "verify_time_ms N={}: sem teto declarado ({key})",
                        r.n
                    ));
                }
                l
            }
            "proof_steps" => {
                checked += 1;
                let (lo, hi) = ((r.n as f64).log2().floor(), (r.n as f64).log2().ceil());
                if r.value < lo || r.value > hi {
                    c4_fail.push(format!(
                        "proof_steps N={}: {} fora de [{lo},{hi}]",
                        r.n, r.value
                    ));
                }
                None
            }
            _ => None,
        };
        if let Some(l) = limit {
            checked += 1;
            if r.value > l {
                c4_fail.push(format!(
                    "{} N={}: {} > {} (+{})",
                    r.metric,
                    r.n,
                    r.value,
                    l,
                    r.value - l
                ));
            }
        }
    }
    let clusters: std::collections::BTreeSet<&str> =
        meas.iter().map(|r| r.cluster.as_str()).collect();
    let cluster = clusters.into_iter().collect::<Vec<_>>().join("+");
    let c4 = if checked == 0 {
        (false, "nenhuma medição encontrada".to_string())
    } else if c4_fail.is_empty() {
        (
            true,
            format!("{checked} medições dentro dos tetos declarados (cluster {cluster})"),
        )
    } else {
        (
            false,
            format!("cluster {cluster}; excedentes: {}", c4_fail.join("; ")),
        )
    };

    let rows = [
        ("C1: aceita válida / rejeita adulterada", c1),
        ("C2: detecta subcolateralização injetada", c2),
        ("C3: reproduzível por terceiro independente", c3),
        ("C4: desempenho e escalabilidade nos tetos declarados", c4),
    ];
    let verdict = |ok: bool| if ok { "APROVADO" } else { "REPROVADO" };

    let mut md =
        String::from("| Critério (§1.3) | Resultado | Evidência / excedente |\n|---|---|---|\n");
    let mut tex = String::from(
        "\\begin{tabular}{p{5.5cm} l p{7cm}}\n\\toprule\n\
         \\textbf{Critério (\\S 1.3)} & \\textbf{Resultado} & \\textbf{Evidência / excedente} \\\\\n\\midrule\n",
    );
    for (name, (ok, detail)) in rows {
        md.push_str(&format!("| {name} | {} | {detail} |\n", verdict(ok)));
        tex.push_str(&format!(
            "{} & {} & {} \\\\\n",
            name,
            verdict(ok),
            detail.replace('_', "\\_").replace('%', "\\%")
        ));
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(out_dir.join("criteria.md"), &md).unwrap();
    fs::write(out_dir.join("criteria.tex"), tex).unwrap();
    print!("{md}");
}
