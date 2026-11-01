# Verificação em ambiente limpo (evidência para o critério C3)

Objetivo: mostrar que um terceiro, sem o atestador, sem SOL e sem estado local, reexecuta a
verificação de uma atestação ancorada em devnet a partir de dados públicos (contas on-chain +
`results/devnet-datasets/`).

## Execução de 2026-10-18 (coordenação do autor; não substitui o relato de um terceiro)

- Ambiente: contêiner Debian 12 (bookworm) linux/arm64, `rust:1.94-bookworm`, apenas
  `pkg-config libssl-dev libudev-dev clang cmake` do apt; nenhum Anchor, Solana CLI, kubo ou
  validador. Fonte: este repositório em `b66293b`.
- Comando: `cargo build --release -p verifier` e depois
  `por-verify attestation --mint FsXE6uHbMiwhj8inCAEAVpN3CNbWd5vFwuBy5AyxE2rC --sequence 0
  --rpc https://api.devnet.solana.com --dataset results/devnet-datasets/n10 --json`.
- Resultado: `verdict = APPROVED`, `status_onchain = OK`, seis verificações aprovadas
  (integridade do conjunto, assinatura, inclusão, binding, cobertura, consistência do MMR),
  `EXIT=0`. Relatório integral: `cleanroom-2026-10-18-n10.json`. Slot da observação: 512592463
  (valor do campo `new_observation.slot` do próprio relatório).

Reproduzir: `docker build --platform linux/arm64 -t por-cleanroom . && docker run --rm por-cleanroom`
(a partir da raiz do repositório, com este `Dockerfile` copiado para lá).

## Execução de 2026-10-26 (terceiro independente, sem participação do autor)

- Relato integral: `relato.json`, no esquema `por-verify/1`, sem campos de identificação
  (ver nota de LGPD na Seção 6.5 da monografia).
- Resultado: `verdict = APPROVED`, `status_onchain = OK`, as seis verificações aprovadas.
- Observação própria do terceiro: slot 515381431 (2026-10-26 15:49 UTC), supply
  4241097430000, coberto.
- SHA-256: `5ba30bfee5220a7ac3a9ac8188aff3e4e973e5df473d09a24d7bd72b31fba7cd`.

Conferência independente deste relato, contra a cadeia e contra os dados versionados:
o supply do mint em `getTokenSupply` é 4241097430000; `results/devnet-datasets/n10/dataset.jsonl`
tem 1029 bytes e SHA-256 `a98f0a6a1edfd5177a18302b5f295b31e56ad922e01bd76ddcb7561223d36ff4`,
ambos idênticos ao que o relato afirma.

## Modelo de relato para um terceiro

Nome, vínculo e data. Máquina (SO, arquitetura, `rustc --version`). Commit do repositório
usado. Comando executado (acima) e o código de saída. `verdict` e `status_onchain` do JSON.
SHA-256 do relatório JSON (`shasum -a 256 <arquivo>`). Uma frase: "Executei o verificador a
partir do código-fonte, sem acesso ao atestador nem a chaves privadas, e obtive o veredito acima."

Limitação conhecida: o caminho completo do Apêndice B (validador local) exige x86_64 ou macOS,
porque a Agave não publica binários Linux arm64; o caminho acima não depende disso.
