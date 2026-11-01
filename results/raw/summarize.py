#!/usr/bin/env python3
"""Resume as respostas RPC congeladas em results/raw/ e confere a formula do rent.
Offline: le apenas os JSONs deste diretorio. Sai com codigo 1 se alguma
afirmacao da monografia (Secao 6.4, 'Custo da ancoragem') nao se sustentar."""
import json, pathlib, sys, collections
HERE = pathlib.Path(__file__).parent
ED = "Ed25519SigVerify111111111111111111111111111"
SPACE = {"PorConfig": 158, "AuditRecord": 205, "LogState": 1041}  # programs/por/src/state.rs

# --- taxa de isencao por cluster/tamanho -------------------------------------
R = {}
for f in sorted((HERE / "rent-exemption").glob("*.json")):
    cl, sz = f.stem.rsplit("-", 1); sz = int(sz[:-1])
    R[(cl, sz)] = json.load(f.open())["result"] / (128 + sz)
rates = set(round(v) for v in R.values())
assert len(rates) == 1, f"taxa de isencao difere entre clusters/tamanhos: {R}"
RATE = rates.pop()

# --- transacoes ---------------------------------------------------------------
rows = collections.defaultdict(list)
for s in json.load((HERE / "devnet-tx/_signatures.json").open()):
    t = json.load((HERE / f"devnet-tx/{s['signature']}.json").open())["result"]
    m, msg = t["meta"], t["transaction"]["message"]
    logs = " ".join(m.get("logMessages") or [])
    ix = next((k for k in ("SubmitAttestation", "InitializeConfig", "SetOracleAuthority", "VerifyInclusion")
               if f"Instruction: {k}" in logs), "Deploy")
    keys = msg["accountKeys"]
    rows[ix].append(dict(
        fee=m["fee"], cu=m.get("computeUnitsConsumed"),
        nsig=msg["header"]["numRequiredSignatures"],
        ned=sum(1 for i in msg["instructions"] if keys[i["programIdIndex"]] == ED),
        rent=sum(po - pr for pr, po in zip(m["preBalances"], m["postBalances"]) if pr == 0 and po > 0),
        err=bool(s["err"])))

def one(vals, label):
    vs = sorted(set(vals)); assert len(vs) == 1, f"{label} nao e constante: {vs}"; return vs[0]

sub, ini = rows["SubmitAttestation"], rows["InitializeConfig"]
fee_sub  = one([r["fee"]  for r in sub], "taxa de SubmitAttestation")
rent_sub = one([r["rent"] for r in sub], "rent de SubmitAttestation")
fee_ini  = one([r["fee"]  for r in ini], "taxa de InitializeConfig")
rent_ini = one([r["rent"] for r in ini], "rent de InitializeConfig")

# --- as afirmacoes da monografia, como assercoes ------------------------------
assert rent_sub == RATE * (128 + SPACE["AuditRecord"]), "rent do AuditRecord != (128+205)*R"
assert rent_ini == RATE * (128 + SPACE["PorConfig"]) + RATE * (128 + SPACE["LogState"]), "rent de InitializeConfig != PorConfig+LogState"
assert one([r["nsig"] for r in sub], "assinaturas") == 1 and one([r["ned"] for r in sub], "ix ed25519") == 1
assert one([r["nsig"] for r in ini], "assinaturas") == 1 and one([r["ned"] for r in ini], "ix ed25519") == 0
assert fee_sub - fee_ini == fee_ini, "a verificacao do precompile nao custou exatamente uma taxa-base"
assert not any(r["err"] for v in rows.values() for r in v), "ha transacao com erro no historico"

# --- saida ---------------------------------------------------------------------
print(f"Taxa de isencao de rent: {RATE} lamports por byte de (128 + tamanho), igual em "
      f"{', '.join(sorted({c for c, _ in R}))}.\n")
print("| instrução | n | assinaturas | ix Ed25519 | taxa (lamports) | rent criado (lamports) | CU |")
print("|---|---|---|---|---|---|---|")
for ix, v in sorted(rows.items(), key=lambda kv: -len(kv[1])):
    cus = [r["cu"] for r in v if r["cu"]]
    f = lambda k: "/".join(str(x) for x in sorted({r[k] for r in v}))
    print(f"| `{ix}` | {len(v)} | {f('nsig')} | {f('ned')} | {f('fee')} | {f('rent')} | "
          f"{min(cus)}–{max(cus)} |" if cus else f"| `{ix}` | {len(v)} | {f('nsig')} | {f('ned')} | {f('fee')} | {f('rent')} | – |")
print(f"\nPor atestação: taxa {fee_sub} + rent {rent_sub} = {fee_sub+rent_sub} lamports; "
      f"rent/taxa = {rent_sub/fee_sub:.0f}×; a uma atestação por dia, {365*rent_sub/1e9:.3f} SOL/ano de rent imobilizado.")
print(f"Todas as asserções passaram ({sum(len(v) for v in rows.values())} transações).")
