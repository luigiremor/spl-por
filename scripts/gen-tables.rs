//! Generates the LaTeX tables of chapter 6 from results/ (single source: no
//! number is ever transcribed by hand).
//!   gen-tables [--measurements results/measurements-devnet.csv] [--matrix results/matrix-localnet.csv]
//!              [--thresholds scenarios/thresholds.toml] [--criteria results/criteria.tex]
//!              [--out-dir results/tables]
//! Numbers are typeset in pt-BR (decimal comma, thousands dot), as the thesis.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// (slug, id, description, expected outcome, threat vector of Quadro 4 or the
/// guarantee/premise exercised) of every scenario of the matrix, in the order
/// of the monograph (§5.3 / plan 06, Task 3 and its mapping to Quadro 4).
const MATRIX: &[(&str, &str, &str, &str, &str)] = &[
    (
        "s01_exact_coverage_ok",
        "S01",
        "Conforme exato ($R_{\\mathrm{norm}} = S$)",
        "\\texttt{OK}; verificador aprova",
        "(c)",
    ),
    (
        "s02_overcollateralized_ok",
        "S02",
        "Sobrecolateralizado ($R_{\\mathrm{norm}} > S$)",
        "\\texttt{OK}; verificador aprova",
        "(c)",
    ),
    (
        "s03_undercollateralized_divergent",
        "S03",
        "Subcolateralizado ($R_{\\mathrm{norm}} < S$)",
        "\\texttt{DIVERGENTE} com delta; verificador concorda",
        "(c)",
    ),
    (
        "s04_replay_rejected",
        "S04",
        "\\textit{Replay} de \\texttt{sequence} antiga",
        "transação rejeitada",
        "(b)",
    ),
    (
        "s05_sequence_gap_rejected",
        "S05",
        "\\texttt{sequence} com buraco",
        "transação rejeitada",
        "(b)",
    ),
    (
        "s06_wrong_mint_rejected",
        "S06",
        "\\textit{Mint} trocado",
        "transação rejeitada",
        "autenticidade",
    ),
    (
        "s07_wrong_decimals_rejected",
        "S07",
        "\\texttt{decimals} divergente do \\textit{mint}",
        "transação rejeitada",
        "autenticidade",
    ),
    (
        "s08_corrupt_signature_rejected",
        "S08",
        "Assinatura corrompida",
        "transação rejeitada (precompilado)",
        "autenticidade",
    ),
    (
        "s09_rotated_key_history_approved",
        "S09",
        "Histórico assinado por chave rotacionada",
        "aprovado (\\texttt{signer\\_pubkey} do registro)",
        "reprodutibilidade",
    ),
    (
        "s10_corrupt_inclusion_proof_rejected",
        "S10",
        "Prova de inclusão corrompida",
        "reprovado",
        "(a), (d)",
    ),
    (
        "s11_hu_subtotal_tamper_rejected",
        "S11",
        "Subtotal adulterado (Hu et al.)",
        "reprovado (binding por nó)",
        "(d)",
    ),
    (
        "s12_subtotal_overflow_aborts",
        "S12",
        "\\textit{Overflow} de subtotal",
        "abortado (\\textit{checked})",
        "(f)",
    ),
    (
        "s13_node_as_leaf_rejected",
        "S13",
        "Nó interno como folha",
        "reprovado (separação de domínio)",
        "(e)",
    ),
    (
        "s14_dataset_one_byte_tamper_rejected",
        "S14",
        "\\textit{Dataset} adulterado em 1 byte",
        "reprovado (\\texttt{sha256})",
        "disponibilidade",
    ),
    (
        "s15_dataset_unavailable_exit2",
        "S15",
        "\\textit{Dataset} indisponível",
        "erro operacional (exit 2)",
        "disponibilidade",
    ),
    (
        "s16_incoherent_status_rejected",
        "S16",
        "\\texttt{status} incoerente em \\textit{fixture}",
        "reprovado",
        "reprodutibilidade",
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

/// pt-BR number: decimal comma and a dot every three integer digits.
fn br(s: &str) -> String {
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push(',');
        out.push_str(frac);
    }
    out
}

fn fmt(v: Option<&f64>) -> String {
    match v {
        None => "--".into(),
        Some(v) if v.fract() == 0.0 => br(&format!("{v:.0}")),
        Some(v) if *v < 1.0 => br(&format!("{v:.3}")),
        Some(v) => br(&format!("{v:.2}")),
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
    let thresholds = get("--thresholds", "scenarios/thresholds.toml");
    let th: toml::Value = toml::from_str(
        &fs::read_to_string(&thresholds)
            .unwrap_or_else(|e| panic!("{}: {e}", thresholds.display())),
    )
    .unwrap();
    // declared ceiling of the complete verification for N, if any
    let verify_cap = |n: u64| {
        th.get("offchain")
            .and_then(|o| o.get("verify_time_ms"))
            .and_then(|t| t.get(format!("n{n}")))
            .and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64)))
    };
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
    // "cap" is the measured/ceiling ratio of the verification; e2e is a single
    // run (no median), shown for transparency as the thresholds file says
    let cols = [
        "cu_ingest",
        "tx_size",
        "build_time_ms",
        "verify_time_ms",
        "cap",
        "verify_e2e_ms",
        "proof_steps",
        "proof_bytes",
    ];
    let mut tex = format!(
        "% gerado por gen-tables a partir de {} (cluster {})\n\
         \\begin{{tabular}}{{r r r r r r r r r}}\n\\toprule\n\
         $N$ & CU & Tx (B) & \\makecell{{Constr. (ms)\\\\ {{[mín--máx]}}}} & \\makecell{{Verif. (ms)\\\\ {{[mín--máx]}}}} & \\makecell{{Verif./\\\\teto}} & \\makecell{{E2E (ms)\\\\ 1 exec.}} & \\makecell{{Prova\\\\ (passos)}} & \\makecell{{Prova\\\\ (B)}} \\\\\n\\midrule\n",
        meas.display(),
        clusters.into_iter().collect::<Vec<_>>().join("+")
    );
    for (n, m) in &by_n {
        tex.push_str(&fmt(Some(&(*n as f64))));
        for c in cols {
            if c == "cap" {
                let r = m
                    .get("verify_time_ms")
                    .zip(verify_cap(*n))
                    .map(|(v, l)| v / l);
                match r {
                    Some(r) => tex.push_str(&format!(" & {}\\%", br(&format!("{:.1}", r * 100.0)))),
                    None => tex.push_str(" & --"),
                }
                continue;
            }
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

    // tab-matriz: scenario x expected x vector/premise; a scenario that did not
    // behave as expected is flagged in place (the table never hides it)
    let observed: BTreeMap<String, f64> = rows(&matrix)
        .into_iter()
        .filter(|r| r[2] == "pass")
        .map(|r| (r[0].clone(), r[3].parse().unwrap()))
        .collect();
    let flag = |v: Option<&f64>| match v {
        Some(v) if *v == 1.0 => "",
        Some(_) => " (\\textbf{DIVERGIU do esperado})",
        None => " (\\textbf{não executado})",
    };
    let passed = MATRIX
        .iter()
        .filter(|(n, ..)| observed.get(*n) == Some(&1.0))
        .count();
    let mut tex = format!(
        "% gerado por gen-tables a partir de {}; {passed}/{} cenários conforme o esperado\n\
         \\begin{{tabular}}{{l p{{5.6cm}} p{{4.9cm}} l}}\n\\toprule\n\
         \\textbf{{\\#}} & \\textbf{{Cenário}} & \\textbf{{Esperado}} & \\textbf{{Vetor/premissa}} \\\\\n\\midrule\n",
        matrix.display(),
        MATRIX.len()
    );
    for (name, id, desc, expected, vector) in MATRIX {
        tex.push_str(&format!(
            "{id} & {desc} & {expected}{} & {vector} \\\\\n",
            flag(observed.get(*name))
        ));
    }
    for (name, v) in &observed {
        if !MATRIX.iter().any(|(n, ..)| n == name) {
            tex.push_str(&format!(
                "-- & \\texttt{{{}}} & --{} & -- \\\\\n",
                esc(name),
                flag(Some(v))
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
