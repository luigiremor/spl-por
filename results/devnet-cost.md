# Custo observado da rodada em devnet (28/09/2026)

Leituras de `solana balance` do pagador dedicado `Ead6W3JhvHfGncuhVBjvyoCdkGeVCWYUPcB4tbR63MyW`
(`-u devnet`), tomadas pelo autor entre as etapas; não são extraídas por script.

| etapa | saldo antes (SOL) | saldo depois (SOL) | consumo (SOL) |
|---|---|---|---|
| implantação do programa (`--max-len 360000`; rent 1,82968 + taxas) | 5,000000 | 3,167672 | 1,832328 |
| matriz S01–S08 (execução abortada antes das medições) | 3,167672 | 3,028217 | 0,139455 |
| rodada `por-devnet-run` completa: S01–S08 + medições N=10..1e5 | 3,028217 | 2,875388 | 0,152829 |
| cenários do verificador S09, S14–S16 (leituras + 4 atestações) | 2,875388 | 2,828685 | 0,046703 |

Uma rodada completa (matriz 16 cenários + 5 cardinalidades de medição), excluída a implantação,
custou cerca de 0,20 SOL em taxas, rent das contas de estado e dos mints de teste. A matriz
on-chain sozinha (S01–S08) fica em torno de 0,14 SOL. O rent do programa (1,83 SOL) é recuperável
ao fechá-lo.
