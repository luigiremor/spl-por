Taxa de isencao de rent: 5080 lamports por byte de (128 + tamanho), igual em devnet, mainnet-beta, testnet.

| instrução | n | assinaturas | ix Ed25519 | taxa (lamports) | rent criado (lamports) | CU |
|---|---|---|---|---|---|---|
| `InitializeConfig` | 29 | 1 | 0 | 5000 | 7391400 | 15611–27611 |
| `SubmitAttestation` | 28 | 1 | 1 | 10000 | 1691640 | 25160–38660 |
| `SetOracleAuthority` | 1 | 2 | 0 | 10000 | 0 | 7905–7905 |
| `Deploy` | 1 | 2 | 0 | 10003 | 1830511960 | 2970–2970 |

Por atestação: taxa 10000 + rent 1691640 = 1701640 lamports; rent/taxa = 169×; a uma atestação por dia, 0.617 SOL/ano de rent imobilizado.
Todas as asserções passaram (59 transações).
