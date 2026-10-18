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
  `EXIT=0`. Relatório integral: `cleanroom-2026-10-18-n10.json`. Slot da observação: 512592251.

Reproduzir: `docker build --platform linux/arm64 -t por-cleanroom . && docker run --rm por-cleanroom`
(a partir da raiz do repositório, com este `Dockerfile` copiado para lá).

## Modelo de relato para um terceiro (Apêndice D da monografia)

Nome, vínculo e data. Máquina (SO, arquitetura, `rustc --version`). Commit do repositório
usado. Comando executado (acima) e o código de saída. `verdict` e `status_onchain` do JSON.
SHA-256 do relatório JSON (`shasum -a 256 <arquivo>`). Uma frase: "Executei o verificador a
partir do código-fonte, sem acesso ao atestador nem a chaves privadas, e obtive o veredito acima."

Limitação conhecida: o caminho completo do Apêndice B (validador local) exige x86_64 ou macOS,
porque a Agave não publica binários Linux arm64; o caminho acima não depende disso.
