| Critério (§1.3) | Resultado | Evidência / excedente |
|---|---|---|
| C1: aceita válida / rejeita adulterada | APROVADO | 11 cenários com resultado igual ao esperado (s01_exact_coverage_ok, s02_overcollateralized_ok, s04_replay_rejected, s05_sequence_gap_rejected, s06_wrong_mint_rejected, s07_wrong_decimals_rejected, s08_corrupt_signature_rejected, s10_corrupt_inclusion_proof_rejected, s11_hu_subtotal_tamper_rejected, s13_node_as_leaf_rejected, s14_dataset_one_byte_tamper_rejected) |
| C2: detecta subcolateralização injetada | APROVADO | 1 cenários com resultado igual ao esperado (s03_undercollateralized_divergent) |
| C3: reproduzível por terceiro independente | APROVADO | 3 cenários com resultado igual ao esperado (s09_rotated_key_history_approved, s16_incoherent_status_rejected, s01_exact_coverage_ok) |
| C4: desempenho e escalabilidade nos tetos declarados | APROVADO | 25 medições dentro dos tetos declarados (cluster localnet) |
