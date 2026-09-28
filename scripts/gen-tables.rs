//! Generates the LaTeX tables of chapter 6 from results/ (single source: no
//! number is ever transcribed by hand).
//!   gen-tables [--measurements results/measurements-devnet.csv] [--matrix results/matrix-localnet.csv]
//!              [--criteria results/criteria.tex] [--out-dir results/tables]
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// (id, description, expected outcome) of every scenario of the matrix, in
/// the order of the monograph (§5.3 / plan 06, Task 3).
const MATRIX: &[(&str, &str, &str, &str)] = &[
    (
        "s01_exact_coverage_ok",
        "S01",
        "Conforme exato ($R_{\\mathrm{norm}} = S$)",
        "\\texttt{OK}; verificador aprova",
    ),
    (
        "s02_overcollateralized_ok",
        "S02",
        "Sobrecolateralizado ($R_{\\mathrm{norm}} > S$)",
        "\\texttt{OK}; verificador aprova",
    ),
    (
        "s03_undercollateralized_divergent",
        "S03",
        "Subcolateralizado ($R_{\\mathrm{norm}} < S$)",
        "\\texttt{DIVERGENTE} com delta; verificador concorda",
    ),
    (
        "s04_replay_rejected",
        "S04",
        "\\textit{Replay} de \\texttt{sequence} antiga",
        "transação rejeitada",
    ),
    (
        "s05_sequence_gap_rejected",
        "S05",
        "\\texttt{sequence} com buraco",
        "transação rejeitada",
    ),
    (
        "s06_wrong_mint_rejected",
        "S06",
        "\\textit{Mint} trocado",
        "transação rejeitada",
    ),
    (
        "s07_wrong_decimals_rejected",
        "S07",
        "\\texttt{decimals} divergente do \\textit{mint}",
        "transação rejeitada",
    ),
    (
        "s08_corrupt_signature_rejected",
        "S08",
        "Assinatura corrompida",
        "transação rejeitada (precompilado)",
    ),
    (
        "s09_rotated_key_history_approved",
        "S09",
        "Histórico assinado por chave rotacionada",
        "aprovado (\\texttt{signer\\_pubkey} do registro)",
    ),
    (
        "s10_corrupt_inclusion_proof_rejected",
        "S10",
        "Prova de inclusão corrompida",
        "reprovado",
    ),
    (
        "s11_hu_subtotal_tamper_rejected",
        "S11",
        "Subtotal adulterado (Hu et al.)",
        "reprovado (binding por nó)",
    ),
    (
        "s12_subtotal_overflow_aborts",
        "S12",
        "\\textit{Overflow} de subtotal",
        "abortado (\\textit{checked})",
    ),
    (
        "s13_node_as_leaf_rejected",
        "S13",
        "Nó interno como folha",
        "reprovado (separação de domínio)",
    ),
    (
        "s14_dataset_one_byte_tamper_rejected",
        "S14",
        "\\textit{Dataset} adulterado em 1 byte",
        "reprovado (\\texttt{sha256})",
    ),
    (
        "s15_dataset_unavailable_exit2",
        "S15",
        "\\textit{Dataset} indisponível",
        "erro operacional (exit 2)",
    ),
    (
        "s16_incoherent_status_rejected",
        "S16",
        "\\texttt{status} incoerente em \\textit{fixture}",
        "reprovado",
    ),
];

fn rows(path: &Path) -> Vec<Vec<String>> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(',').map(str::to_string).collect())
        .collect()
}

fn fmt(v: Option<&f64>) -> String {
    match v {
        None => "--".into(),
        Some(v) if v.fract() == 0.0 => format!("{v:.0}"),
        Some(v) if *v < 1.0 => format!("{v:.3}"),
        Some(v) => format!("{v:.2}"),
    }
}

fn esc(s: &str) -> String {
    s.replace('_', "\\_")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: &str| {
        args.iter()
            .position(|a| a == k)
            .map(|i| PathBuf::from(&args[i + 1]))
            .unwrap_or_else(|| PathBuf::from(d))
    };
    let meas = get("--measurements", "results/measurements-devnet.csv");
    let matrix = get("--matrix", "results/matrix-localnet.csv");
    let criteria = get("--criteria", "results/criteria.tex");
    let out = get("--out-dir", "results/tables");
    fs::create_dir_all(&out).unwrap();

    // tab-medicoes: one row per N, one column per metric (pivot of the CSV)
    let mut by_n: BTreeMap<u64, BTreeMap<String, f64>> = BTreeMap::new();
    let mut clusters = std::collections::BTreeSet::new();
    for r in rows(&meas) {
        by_n.entry(r[1].parse().unwrap())
            .or_default()
            .insert(r[2].clone(), r[3].parse().unwrap());
        clusters.insert(r[6].clone());
    }
    let cols = [
        "cu_ingest",
        "tx_size",
        "build_time_ms",
        "verify_time_ms",
        "proof_steps",
        "proof_bytes",
    ];
    let mut tex = format!(
        "% gerado por gen-tables a partir de {} (cluster {})\n\
         \\begin{{tabular}}{{r r r r r r r}}\n\\toprule\n\
         $N$ & CU & Tx (B) & Constr. (ms) [mín--máx] & Verif. (ms) [mín--máx] & Prova (passos) & Prova (B) \\\\\n\\midrule\n",
        meas.display(),
        clusters.into_iter().collect::<Vec<_>>().join("+")
    );
    for (n, m) in &by_n {
        tex.push_str(&format!("{n}"));
        for c in cols {
            // timed metrics carry the spread of the 10 runs when recorded
            let (lo, hi) = (m.get(&format!("{c}_min")), m.get(&format!("{c}_max")));
            match (m.get(c), lo, hi) {
                (Some(_), Some(_), Some(_)) => {
                    tex.push_str(&format!(" & {} [{}--{}]", fmt(m.get(c)), fmt(lo), fmt(hi)))
                }
                _ => tex.push_str(&format!(" & {}", fmt(m.get(c)))),
            }
        }
        tex.push_str(" \\\\\n");
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    fs::write(out.join("tab-medicoes.tex"), tex).unwrap();

    // tab-matriz: scenario x expected x observed
    let observed: BTreeMap<String, f64> = rows(&matrix)
        .into_iter()
        .filter(|r| r[2] == "pass")
        .map(|r| (r[0].clone(), r[3].parse().unwrap()))
        .collect();
    let verdict = |v: Option<&f64>| match v {
        Some(v) if *v == 1.0 => "conforme o esperado",
        Some(_) => "DIVERGIU do esperado",
        None => "não executado",
    };
    let mut tex = format!(
        "% gerado por gen-tables a partir de {}\n\
         \\begin{{tabular}}{{l p{{5.2cm}} p{{4.6cm}} l}}\n\\toprule\n\
         \\textbf{{\\#}} & \\textbf{{Cenário}} & \\textbf{{Esperado}} & \\textbf{{Observado}} \\\\\n\\midrule\n",
        matrix.display()
    );
    for (name, id, desc, expected) in MATRIX {
        tex.push_str(&format!(
            "{id} & {desc} & {expected} & {} \\\\\n",
            verdict(observed.get(*name))
        ));
    }
    for (name, v) in &observed {
        if !MATRIX.iter().any(|(n, ..)| n == name) {
            tex.push_str(&format!(
                "-- & \\texttt{{{}}} & -- & {} \\\\\n",
                esc(name),
                verdict(Some(v))
            ));
        }
    }
    tex.push_str("\\bottomrule\n\\end{tabular}\n");
    fs::write(out.join("tab-matriz.tex"), tex).unwrap();

    // tab-criterios: copy of criteria.tex (single source)
    let crit = fs::read_to_string(&criteria)
        .unwrap_or_else(|e| panic!("{} (run criteria-check first): {e}", criteria.display()));
    fs::write(out.join("tab-criterios.tex"), crit).unwrap();
    println!("tables written to {}", out.display());
}
