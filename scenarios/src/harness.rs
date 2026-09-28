//! Shared mechanics of the scenario matrix and of the measurements: fund a
//! payer, create a mint, `initialize_config`, build the two-instruction
//! attestation transaction (with optional tampering), submit, read the
//! `AuditRecord`, run the public verifier and append results to a CSV with
//! fixed columns. Talks to a real validator (localnet or devnet) over RPC.
use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
use base64::Engine;
use solana_client::{client_error::ClientError, rpc_client::RpcClient};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use solana_system_interface::{instruction as system_instruction, program as system_program};
use std::{
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};
use sum_tree::{AttestationMessage, SumTree};
use verifier::chain::{AccountFetcher, FixtureFetcher, RpcFetcher};
use verifier::pipeline::{run, Outcome, PipelineParams};

pub const CSV_HEADER: &str = "scenario,N,metric,value,unit,commit,cluster,timestamp";

pub struct Ctx {
    pub rpc: RpcClient,
    pub rpc_url: String,
    pub payer: Keypair,
    pub mint: Pubkey,
    pub oracle: Keypair,
    pub admin: Keypair,
    pub decimals: u8,
    pub reserve_decimals: u8,
    pub max_value: u64,
}

/// `POR_RPC` or the local validator.
pub fn rpc_url() -> String {
    std::env::var("POR_RPC").unwrap_or_else(|_| "http://127.0.0.1:8899".into())
}

fn confirmed_client(url: &str) -> RpcClient {
    RpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed())
}

/// Payer: `POR_PAYER` (keypair file, e.g. the dedicated devnet payer) or a
/// fresh keypair funded by the localnet faucet.
fn payer(rpc: &RpcClient) -> Keypair {
    if let Ok(path) = std::env::var("POR_PAYER") {
        return read_keypair_file(&path).expect("POR_PAYER keypair");
    }
    let kp = Keypair::new();
    let sig = rpc
        .request_airdrop(&kp.pubkey(), 10_000_000_000)
        .expect("airdrop (localnet faucet)");
    wait_finalized(rpc, &sig);
    kp
}

/// `ClientError` is boxed: clippy's result_large_err (264-byte Err variant).
pub type SubmitResult = Result<Signature, Box<ClientError>>;

fn is_rate_limit(e: &ClientError) -> bool {
    let s = e.to_string();
    s.contains("429") || s.to_lowercase().contains("rate limit") || s.contains("Too Many Requests")
}

/// `send_and_confirm` tolerant to public-RPC rate limiting (HTTP 429): up to 5
/// attempts with exponential backoff (500 ms * 2^k) and a fixed 500 ms pause
/// after every accepted transaction. Program/precompile rejections return
/// immediately. The signed transaction is reused: the blockhash stays valid
/// for the whole retry window (~15 s).
pub fn submit_with_retry(rpc: &RpcClient, tx: &Transaction) -> SubmitResult {
    let mut delay = Duration::from_millis(500);
    for attempt in 0..5 {
        match rpc.send_and_confirm_transaction(tx) {
            Ok(sig) => {
                sleep(Duration::from_millis(500));
                return Ok(sig);
            }
            Err(e) if attempt < 4 && is_rate_limit(&e) => {
                eprintln!("rate limited (attempt {attempt}), backing off {delay:?}");
                sleep(delay);
                delay *= 2;
            }
            Err(e) => return Err(Box::new(e)),
        }
    }
    unreachable!()
}

fn send(ctx: &Ctx, ixs: &[Instruction], signers: &[&Keypair]) -> SubmitResult {
    let bh = ctx.rpc.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(ixs, Some(&ctx.payer.pubkey()), signers, bh);
    submit_with_retry(&ctx.rpc, &tx)
}

/// Creates an SPL mint (authority = payer) with `decimals`.
pub fn create_mint(ctx: &Ctx, decimals: u8) -> Pubkey {
    let mint = Keypair::new();
    let len = spl_token::state::Mint::LEN;
    let rent = ctx.rpc.get_minimum_balance_for_rent_exemption(len).unwrap();
    let ixs = [
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &mint.pubkey(),
            rent,
            len as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::id(),
            &mint.pubkey(),
            &ctx.payer.pubkey(),
            None,
            decimals,
        )
        .unwrap(),
    ];
    send(ctx, &ixs, &[&ctx.payer, &mint]).expect("create mint");
    mint.pubkey()
}

/// Funds a payer, creates the mint and calls `initialize_config`
/// (oracle and admin are fresh keys distinct from the payer: relayer-friendly).
pub fn setup(rpc_url: &str, decimals: u8, reserve_decimals: u8, max_value: u64) -> Ctx {
    let rpc = confirmed_client(rpc_url);
    let payer = payer(&rpc);
    let mut ctx = Ctx {
        rpc,
        rpc_url: rpc_url.to_string(),
        payer,
        mint: Pubkey::default(),
        oracle: Keypair::new(),
        admin: Keypair::new(),
        decimals,
        reserve_decimals,
        max_value,
    };
    ctx.mint = create_mint(&ctx, decimals);
    let ix = Instruction {
        program_id: por::ID,
        accounts: por::accounts::InitializeConfig {
            config: config_pda(&ctx.mint),
            log_state: log_pda(&ctx.mint),
            mint: ctx.mint,
            payer: ctx.payer.pubkey(),
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: por::instruction::InitializeConfig {
            oracle_authority: ctx.oracle.pubkey(),
            admin: ctx.admin.pubkey(),
            reserve_decimals,
            max_value,
        }
        .data(),
    };
    send(&ctx, &[ix], &[&ctx.payer]).expect("initialize_config");
    ctx
}

/// Mints `amount` base units of `ctx.mint` into a fresh token account.
pub fn mint_supply_to(ctx: &Ctx, amount: u64) {
    let acct = Keypair::new();
    let len = spl_token::state::Account::LEN;
    let rent = ctx.rpc.get_minimum_balance_for_rent_exemption(len).unwrap();
    let ixs = [
        system_instruction::create_account(
            &ctx.payer.pubkey(),
            &acct.pubkey(),
            rent,
            len as u64,
            &spl_token::id(),
        ),
        spl_token::instruction::initialize_account3(
            &spl_token::id(),
            &acct.pubkey(),
            &ctx.mint,
            &ctx.payer.pubkey(),
        )
        .unwrap(),
        spl_token::instruction::mint_to(
            &spl_token::id(),
            &ctx.mint,
            &acct.pubkey(),
            &ctx.payer.pubkey(),
            &[],
            amount,
        )
        .unwrap(),
    ];
    send(ctx, &ixs, &[&ctx.payer, &acct]).expect("mint supply");
}

pub fn config_pda(mint: &Pubkey) -> Pubkey {
    verifier::chain::config_pda(&por::ID, mint)
}
pub fn audit_pda(mint: &Pubkey, seq: u64) -> Pubkey {
    verifier::chain::audit_pda(&por::ID, mint, seq)
}
pub fn log_pda(mint: &Pubkey) -> Pubkey {
    verifier::chain::log_pda(&por::ID, mint)
}

/// CID of the dataset under the attester's fixed profile: computed offline
/// when the file fits one raw block, otherwise through kubo (`IPFS_PATH`).
fn dataset_cid(dataset: &[u8]) -> String {
    attester::cid::compute_single_block_cid_v1_raw(dataset).unwrap_or_else(|| {
        let tmp = std::env::temp_dir().join(format!("por-scenarios-{}.jsonl", dataset.len()));
        std::fs::write(&tmp, dataset).unwrap();
        attester::ipfs::ipfs_add(&tmp).expect("ipfs add (multi-block dataset needs kubo)")
    })
}

/// The canonical message the oracle signs for `seq` over `tree`/`dataset`.
pub fn attestation_msg(ctx: &Ctx, seq: u64, tree: &SumTree, dataset: &[u8]) -> AttestationMessage {
    AttestationMessage {
        schema_version: por::state::SCHEMA_VERSION,
        mint: ctx.mint.to_bytes(),
        sequence: seq,
        merkle_root: tree.root_hash,
        total_reservas: tree.total,
        currency: por::state::CURRENCY_BRL,
        decimals: ctx.decimals,
        reserve_decimals: ctx.reserve_decimals,
        ipfs_cid: dataset_cid(dataset),
        dataset_sha256: attester::snapshot::dataset_sha256(dataset),
    }
}

/// Ed25519 signature over `canonical_bytes()` (Solana keypairs ARE Ed25519).
pub fn sign(signer: &Keypair, msg: &AttestationMessage) -> [u8; 64] {
    signer
        .sign_message(&msg.canonical_bytes())
        .as_ref()
        .try_into()
        .unwrap()
}

pub enum Tamper {
    None,
    /// ix#1 receives another mint account (config PDA still derived from the real one).
    WrongMint(Pubkey),
    /// Declared decimals diverge from the mint account.
    WrongDecimals(u8),
    /// Last byte of the signature flipped: the precompile must fail.
    CorruptSig,
}

/// Two-instruction transaction: #0 Ed25519Program (1 signature, self-referential)
/// + #1 `submit_attestation`, paid by `ctx.payer`.
pub fn build_submit_tx(
    ctx: &Ctx,
    msg: &AttestationMessage,
    signer: &Keypair,
    tamper: Tamper,
) -> Transaction {
    let mut msg = msg.clone();
    let mut mint_account = ctx.mint;
    match tamper {
        Tamper::WrongMint(m) => mint_account = m,
        Tamper::WrongDecimals(d) => msg.decimals = d,
        _ => {}
    }
    let payload = msg.canonical_bytes();
    let mut signature = sign(signer, &msg);
    if matches!(tamper, Tamper::CorruptSig) {
        signature[63] ^= 0x01;
    }
    let ed_ix = solana_ed25519_program::new_ed25519_instruction_with_signature(
        &payload,
        &signature,
        &signer.pubkey().to_bytes(),
    );
    let submit_ix = Instruction {
        program_id: por::ID,
        accounts: por::accounts::SubmitAttestation {
            config: config_pda(&ctx.mint),
            mint: mint_account,
            audit_record: audit_pda(&ctx.mint, msg.sequence),
            log_state: log_pda(&ctx.mint),
            payer: ctx.payer.pubkey(),
            instructions_sysvar: solana_sdk::sysvar::instructions::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: por::instruction::SubmitAttestation {
            args: por::message::SubmitAttestationArgs {
                schema_version: msg.schema_version,
                sequence: msg.sequence,
                merkle_root: msg.merkle_root,
                total_reservas: msg.total_reservas,
                decimals: msg.decimals,
                ipfs_cid: msg.ipfs_cid.clone(),
                dataset_sha256: msg.dataset_sha256,
            },
        }
        .data(),
    };
    let bh = ctx.rpc.get_latest_blockhash().unwrap();
    Transaction::new_signed_with_payer(
        &[ed_ix, submit_ix],
        Some(&ctx.payer.pubkey()),
        &[&ctx.payer],
        bh,
    )
}

/// Polls until `sig` is finalized (the verifier reads at `finalized`).
pub fn wait_finalized(rpc: &RpcClient, sig: &Signature) {
    let start = Instant::now();
    loop {
        if let Some(Ok(())) = rpc
            .get_signature_status_with_commitment(sig, CommitmentConfig::finalized())
            .expect("signature status")
        {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(120),
            "transaction {sig} not finalized after 120s"
        );
        sleep(Duration::from_millis(500));
    }
}

/// Sends the transaction; on success waits for finalization so that every
/// later read (verifier, fixtures) observes it. Rejections surface as `Err`
/// (preflight simulation) without waiting.
pub fn submit(ctx: &Ctx, tx: &Transaction) -> SubmitResult {
    let sig = submit_with_retry(&ctx.rpc, tx)?;
    wait_finalized(&ctx.rpc, &sig);
    Ok(sig)
}

fn fetch_finalized(ctx: &Ctx, key: &Pubkey) -> Vec<u8> {
    ctx.rpc
        .get_account_with_commitment(key, CommitmentConfig::finalized())
        .expect("rpc")
        .value
        .unwrap_or_else(|| panic!("account {key} not found (finalized)"))
        .data
}

pub fn read_audit(ctx: &Ctx, seq: u64) -> por::state::AuditRecord {
    let data = fetch_finalized(ctx, &audit_pda(&ctx.mint, seq));
    por::state::AuditRecord::try_deserialize(&mut data.as_slice()).unwrap()
}

pub fn read_config(ctx: &Ctx) -> por::state::PorConfig {
    let data = fetch_finalized(ctx, &config_pda(&ctx.mint));
    por::state::PorConfig::try_deserialize(&mut data.as_slice()).unwrap()
}

/// `set_oracle_authority(new)` signed by the admin (payer pays); finalized on return.
pub fn rotate_oracle_authority(ctx: &Ctx, new_authority: Pubkey) -> Signature {
    let ix = Instruction {
        program_id: por::ID,
        accounts: por::accounts::SetOracleAuthority {
            config: config_pda(&ctx.mint),
            admin: ctx.admin.pubkey(),
        }
        .to_account_metas(None),
        data: por::instruction::SetOracleAuthority { new_authority }.data(),
    };
    let sig = send(ctx, &[ix], &[&ctx.payer, &ctx.admin]).expect("set_oracle_authority");
    wait_finalized(&ctx.rpc, &sig);
    sig
}

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
}

/// Writes `scenarios/data/<name>/{dataset.jsonl,signature.json}` exactly as the
/// attester publishes them (the layout `por-verify --dataset DIR` expects).
pub fn write_dataset_dir(name: &str, dataset: &[u8], signature: &[u8; 64]) -> PathBuf {
    let dir = data_dir().join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("dataset.jsonl"), dataset).unwrap();
    std::fs::write(
        dir.join("signature.json"),
        format!("{{\"signature\":\"{}\"}}", hex::encode(signature)),
    )
    .unwrap();
    dir
}

fn exit_code(outcome: Outcome) -> i32 {
    match outcome {
        Outcome::Approved(_) => 0,
        Outcome::Rejected(_) => 1,
        Outcome::OperationalFailure(msg) => {
            eprintln!("por-verify: {msg}");
            2
        }
    }
}

/// Runs the verifier pipeline (the function `por-verify` wraps) against
/// `fetcher`; returns the CLI exit code: 0 approved, 1 rejected, 2 operational.
pub fn verify_with(
    fetcher: &dyn AccountFetcher,
    mint: &Pubkey,
    seq: u64,
    dataset: Option<&Path>,
) -> i32 {
    exit_code(run(PipelineParams {
        fetcher,
        program_id: por::ID,
        mint: *mint,
        sequence: seq,
        dataset_source: dataset.map(|p| p.to_string_lossy().into_owned()),
        signature_path: None,
        gateway: "http://127.0.0.1:8080".into(),
        account: None,
    }))
}

/// Public verification over RPC (finalized reads), like `por-verify attestation`.
pub fn run_verify(ctx: &Ctx, seq: u64, dataset: Option<&Path>) -> i32 {
    let fetcher = RpcFetcher::new(ctx.rpc_url.clone());
    verify_with(&fetcher, &ctx.mint, seq, dataset)
}

/// Verification against a local fixture file (`por-verify --fixtures`).
pub fn run_verify_fixture(fixture: &Path, mint: &Pubkey, seq: u64, dataset: Option<&Path>) -> i32 {
    let fetcher = FixtureFetcher::load(fixture).expect("fixture");
    verify_with(&fetcher, mint, seq, dataset)
}

/// Snapshot of the on-chain accounts the verifier needs (config, every
/// AuditRecord, LogState, mint) in the `FixtureFetcher` JSON layout, read at
/// `finalized`. No `historical_supplies`: coverage takes path B.
pub fn snapshot_fixture(ctx: &Ctx) -> serde_json::Value {
    let b64 = base64::engine::general_purpose::STANDARD;
    let config = read_config(ctx);
    let mut keys = vec![config_pda(&ctx.mint), log_pda(&ctx.mint), ctx.mint];
    keys.extend((0..config.next_sequence).map(|s| audit_pda(&ctx.mint, s)));
    let accounts: serde_json::Map<String, serde_json::Value> = keys
        .iter()
        .map(|k| {
            let data = b64.encode(fetch_finalized(ctx, k));
            (k.to_string(), serde_json::json!({ "data_base64": data }))
        })
        .collect();
    let slot = ctx
        .rpc
        .get_slot_with_commitment(CommitmentConfig::finalized())
        .unwrap();
    serde_json::json!({ "slot": slot, "accounts": accounts })
}

/// Rewrites the AuditRecord of `seq` inside a fixture with `status` (S16:
/// an incoherent record the verifier must contradict).
pub fn override_fixture_status(fixture: &mut serde_json::Value, ctx: &Ctx, seq: u64, status: u8) {
    use anchor_lang::AccountSerialize;
    let b64 = base64::engine::general_purpose::STANDARD;
    let key = audit_pda(&ctx.mint, seq).to_string();
    let data = b64
        .decode(fixture["accounts"][&key]["data_base64"].as_str().unwrap())
        .unwrap();
    let mut rec = por::state::AuditRecord::try_deserialize(&mut data.as_slice()).unwrap();
    rec.status = status;
    let mut out = Vec::new();
    rec.try_serialize(&mut out).unwrap();
    out.resize(por::state::AuditRecord::SPACE, 0);
    fixture["accounts"][&key]["data_base64"] = serde_json::Value::String(b64.encode(out));
}

pub fn write_fixture(name: &str, fixture: &serde_json::Value) -> PathBuf {
    let path = data_dir().join(format!("{name}.fixture.json"));
    std::fs::create_dir_all(data_dir()).unwrap();
    std::fs::write(&path, fixture.to_string()).unwrap();
    path
}

/// Creates the CSV with the fixed header when it does not exist yet.
pub fn record_header_only(csv: &Path) {
    if !csv.exists() {
        if let Some(dir) = csv.parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(csv, format!("{CSV_HEADER}\n")).unwrap();
    }
}

/// Appends one observation: `scenario,N,metric,value,unit,commit,cluster,timestamp`.
/// No-op unless `POR_RECORD=1`: `results/` is the monograph's data source, so
/// only deliberate runs (eval-localnet.sh, por-devnet-run) may append to it;
/// a plain `cargo test` must never touch it.
pub fn record(csv: &Path, scenario: &str, n: usize, metric: &str, value: f64, unit: &str) {
    use std::io::Write;
    if std::env::var("POR_RECORD").as_deref() != Ok("1") {
        return;
    }
    record_header_only(csv);
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let cluster = std::env::var("POR_CLUSTER").unwrap_or_else(|_| "localnet".into());
    let ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap();
    // one write_all per row: O_APPEND + a single write keeps rows whole when
    // several test threads/processes append concurrently (writeln! may split)
    let line = format!("{scenario},{n},{metric},{value},{unit},{commit},{cluster},{ts}\n");
    let mut f = std::fs::OpenOptions::new().append(true).open(csv).unwrap();
    f.write_all(line.as_bytes()).unwrap();
}

/// `results/matrix-<cluster>.csv` (POR_CLUSTER, default localnet).
pub fn matrix_csv() -> PathBuf {
    let cluster = std::env::var("POR_CLUSTER").unwrap_or_else(|_| "localnet".into());
    results_path(&format!("matrix-{cluster}.csv"))
}

/// Results CSV path relative to the workspace root (tests run with the crate
/// dir as cwd, binaries with the workspace root).
pub fn results_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.join("results").join(name)
}
