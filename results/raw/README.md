# Respostas RPC congeladas (evidência da Seção 6.4, "Custo da ancoragem")

O histórico de assinaturas da devnet é efêmero: a rede é zerada periodicamente e o
endpoint público retém pouco. Para que os valores de taxa e rent citados na monografia
continuem reproduzíveis depois disso, as respostas brutas estão versionadas aqui.

- `devnet-tx/_signatures.json` — `getSignaturesForAddress` do programa
  `PoR4VVWSnwt1AAu6ph5S8WtsbtLPgFY2iYbNV1kMDDU`, 59 transações, 2026-09-28 00:20–00:39 UTC.
- `devnet-tx/<assinatura>.json` — `getTransaction` de cada uma (`encoding: json`,
  `maxSupportedTransactionVersion: 0`), sem edição.
- `rent-exemption/<cluster>-<tamanho>B.json` — `getMinimumBalanceForRentExemption` para
  0, 158, 205 e 1041 bytes em devnet, testnet e mainnet-beta.
- `summarize.py` — resume tudo **offline** e falha (exit 1) se qualquer afirmação da
  monografia deixar de valer: taxa de isenção igual nos três clusters; rent do
  `AuditRecord` = (128+205)×R; rent de `InitializeConfig` = PorConfig + LogState; uma
  assinatura e uma instrução Ed25519 por ingestão; a verificação do precompile custa
  exatamente uma taxa-base; nenhuma transação com erro.
- `summary.md` — saída de `summarize.py`.

Capturado em 2026-11-01. Para recapturar (exige rede e, na devnet, que o histórico ainda
exista): repetir as chamadas acima; `summarize.py` não acessa a rede.

Observação: o histórico contém 29 `InitializeConfig` de execuções repetidas e **nenhuma
transação com erro** — os cenários de rejeição (S04–S08) falharam no *preflight* e nunca
foram submetidos. Os números aqui são **custo unitário por instrução**, não "custo da
rodada"; esse está em `../devnet-cost.md` (leituras de saldo).
