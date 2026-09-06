# results/ — dados da avaliação (fonte única do Cap. 6)

Conteúdo versionado: `matrix-localnet.csv` (matriz S01–S16), `measurements-localnet.csv`
(rodada de medição em localnet), `measurements-devnet.csv` + `run-meta.toml` (rodada em
devnet, quando existente), `hardware.md`, `criteria.{md,tex}`, `tables/*.tex`.

Colunas fixas de todo CSV: `scenario,N,metric,value,unit,commit,cluster,timestamp`.
Métricas de `measure`: `build_time_ms`, `proof_steps`, `proof_bytes`, `cu_ingest`
(simulação), `cu_ingest_landed` (transação efetivada), `tx_size`, `verify_time_ms`
(pipeline do verificador in-process sobre snapshot das contas, mediana de 10 após 3
aquecimentos, `--release`), `verify_e2e_ms` (processo `por-verify` + RPC, só transparência).
Tetos: `scenarios/thresholds.toml`, commitado ISOLADAMENTE antes da primeira medição.

Regra: NENHUM número da monografia é transcrito à mão. O Cap. 6 importa as tabelas geradas:

1. No repo `spl-por`:
   `cargo run -p scripts --bin criteria-check [--measurements results/measurements-<cluster>.csv]`
   `cargo run -p scripts --bin gen-tables [--measurements results/measurements-<cluster>.csv]`
2. Copiar para o repo da tese: `cp results/tables/tab-*.tex <tese>/tables/cap6/`
3. No LaTeX do Cap. 6: `\input{tables/cap6/tab-medicoes.tex}` (idem `tab-matriz.tex` e
   `tab-criterios.tex`), dentro do ambiente `table`/`quadro` do template.
4. O texto cita a origem: commit (coluna `commit` / `run-meta.toml`), cluster e data da rodada.
   Regerar tabela = reexecutar 1–2 e recompilar a tese.

Reprodução completa da avaliação (validador local + programa implantado pelo script):

```
cargo run -p scenarios --bin por-gen                    # datasets (gitignored)
./scripts/eval-localnet.sh                              # matriz S01–S16 -> matrix-localnet.csv
cargo build --release -p verifier -p scenarios
./scripts/eval-localnet.sh target/release/por-measure   # -> measurements-localnet.csv
cargo run --release -p scenarios --bin por-devnet-run   # devnet: keys/devnet-payer.json financiado
cargo run -p scripts --bin criteria-check && cargo run -p scripts --bin gen-tables
```
