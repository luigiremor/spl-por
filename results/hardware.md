# Hardware e software de referência das medições off-chain

Máquina em que `verify_time_ms`, `build_time_ms` e `verify_e2e_ms` são medidos
(§5.4.2: teto declarado por faixa de N, em hardware de referência fixado antes
das medições).

- CPU: Apple M1 Pro (10 núcleos; `sysctl -n machdep.cpu.brand_string`)
- RAM: 16 GiB (`sysctl -n hw.memsize` = 17179869184)
- SO: Darwin 25.5.0 arm64 (macOS)
- rustc: 1.94.0 (4a4ef493e 2026-03-02), `cargo build --release`
  (`[profile.release]`: overflow-checks = true, lto = "fat", codegen-units = 1)
- solana CLI / solana-test-validator: 2.2.17 (Agave)
- anchor-cli: 0.32.1
- kubo (ipfs): 0.43.0
- Data da fixação: 2026-09-05
