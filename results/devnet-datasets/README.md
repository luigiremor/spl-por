# Conjuntos de dados das atestações ancoradas em devnet (2026-09-28)

Programa `PoR4VVWSnwt1AAu6ph5S8WtsbtLPgFY2iYbNV1kMDDU`, RPC `https://api.devnet.solana.com`,
rodada `results/run-meta.toml`. Cada diretório traz o `dataset.jsonl` publicado (JSONL canônico)
e o `signature.json` da autoridade do oráculo; o `AuditRecord` on-chain guarda o SHA-256 e o CID
desse arquivo, que o verificador recomputa. Sequência 0 em todos.

| N | mint |
|---|---|
| 10 | FsXE6uHbMiwhj8inCAEAVpN3CNbWd5vFwuBy5AyxE2rC |
| 100 | B8zLUXPAhEH9JWqpbeJcTiQHe37jA3Kn9X8iymk79Xdy |
| 1000 | HZ2RBpVfHu8sbtHTtXsmmTUjBZS3j74iZs16rfPJwP9X |
| 10000 | 4jwKAv4md81HvJ8BUdMrA4zToPLq9L5eZWTBgj4PKGWd |
| 100000 | 3tVBnhERfnZNbCxqgUJy5DMBKDjYhW21nigXw9z9ZKzz |

Verificação por terceiro, sem SOL e sem o atestador:

    cargo build --release -p verifier
    ./target/release/por-verify attestation --mint FsXE6uHbMiwhj8inCAEAVpN3CNbWd5vFwuBy5AyxE2rC \
      --sequence 0 --rpc https://api.devnet.solana.com --dataset results/devnet-datasets/n10 --json
