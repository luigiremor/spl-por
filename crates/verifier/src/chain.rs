//! On-chain state access behind the `AccountFetcher` trait: `RpcFetcher`
//! talks to a JSON-RPC node, `FixtureFetcher` serves accounts recorded from
//! localnet so no unit test depends on the network.
use anchor_lang::AccountDeserialize;
use base64::Engine;
use por::state::{AuditRecord, LogState, PorConfig};
use serde::Deserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use std::{collections::HashMap, path::Path, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("account not found: {0}")]
    NotFound(Pubkey),
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("cannot deserialize account {0}: {1}")]
    Deserialize(Pubkey, String),
    #[error("fixture error: {0}")]
    Fixture(String),
}

pub struct AccountSnapshot {
    pub data: Vec<u8>,
    pub slot: u64,
}

pub trait AccountFetcher {
    fn fetch(&self, pubkey: &Pubkey) -> Result<AccountSnapshot, FetchError>;
    /// `Some(supply)` only when the read provably corresponds to `slot`.
    fn supply_at_slot(&self, mint: &Pubkey, slot: u64) -> Result<Option<u64>, FetchError>;
}

pub struct MintInfo {
    pub supply: u64,
    pub decimals: u8,
}

pub fn parse_mint(data: &[u8]) -> Result<MintInfo, FetchError> {
    use solana_sdk::program_pack::Pack;
    let m = spl_token::state::Mint::unpack(data)
        .map_err(|e| FetchError::Rpc(format!("mint unpack: {e}")))?;
    Ok(MintInfo {
        supply: m.supply,
        decimals: m.decimals,
    })
}

// ---------- RPC ----------

pub struct RpcFetcher {
    client: RpcClient,
}

impl RpcFetcher {
    pub fn new(url: String) -> Self {
        Self {
            client: RpcClient::new_with_commitment(url, CommitmentConfig::finalized()),
        }
    }
}

impl AccountFetcher for RpcFetcher {
    fn fetch(&self, pubkey: &Pubkey) -> Result<AccountSnapshot, FetchError> {
        let resp = self
            .client
            .get_account_with_commitment(pubkey, CommitmentConfig::finalized())
            .map_err(|e| FetchError::Rpc(e.to_string()))?;
        let acc = resp.value.ok_or(FetchError::NotFound(*pubkey))?;
        Ok(AccountSnapshot {
            data: acc.data,
            slot: resp.context.slot,
        })
    }

    fn supply_at_slot(&self, mint: &Pubkey, slot: u64) -> Result<Option<u64>, FetchError> {
        // Standard JSON-RPC exposes no historical state per slot: the reread is
        // authoritative for `slot` only if the response context matches it
        // (§4.4.4); otherwise coverage falls back to path B.
        let snap = self.fetch(mint)?;
        if snap.slot == slot {
            Ok(Some(parse_mint(&snap.data)?.supply))
        } else {
            Ok(None)
        }
    }
}

// ---------- Fixtures ----------

#[derive(Deserialize)]
struct FixtureFile {
    slot: u64,
    accounts: HashMap<String, FixtureAccount>,
    #[serde(default)]
    historical_supplies: HashMap<String, HashMap<String, String>>,
}

#[derive(Deserialize)]
struct FixtureAccount {
    data_base64: String,
}

pub struct FixtureFetcher {
    slot: u64,
    accounts: HashMap<Pubkey, Vec<u8>>,
    historical: HashMap<(Pubkey, u64), u64>,
}

impl FixtureFetcher {
    pub fn from_json(json: &str) -> Result<Self, FetchError> {
        let fixture = |e: &dyn std::fmt::Display| FetchError::Fixture(e.to_string());
        let file: FixtureFile = serde_json::from_str(json).map_err(|e| fixture(&e))?;
        let mut accounts = HashMap::new();
        for (key, acc) in file.accounts {
            let pubkey = Pubkey::from_str(&key).map_err(|e| fixture(&e))?;
            let data = base64::engine::general_purpose::STANDARD
                .decode(&acc.data_base64)
                .map_err(|e| fixture(&e))?;
            accounts.insert(pubkey, data);
        }
        let mut historical = HashMap::new();
        for (mint, by_slot) in file.historical_supplies {
            let mint = Pubkey::from_str(&mint).map_err(|e| fixture(&e))?;
            for (slot, supply) in by_slot {
                let slot: u64 = slot.parse().map_err(|e| fixture(&e))?;
                let supply: u64 = supply.parse().map_err(|e| fixture(&e))?;
                historical.insert((mint, slot), supply);
            }
        }
        Ok(Self {
            slot: file.slot,
            accounts,
            historical,
        })
    }

    pub fn load(path: &Path) -> Result<Self, FetchError> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| FetchError::Fixture(format!("{}: {e}", path.display())))?;
        Self::from_json(&json)
    }
}

impl AccountFetcher for FixtureFetcher {
    fn fetch(&self, pubkey: &Pubkey) -> Result<AccountSnapshot, FetchError> {
        self.accounts
            .get(pubkey)
            .map(|data| AccountSnapshot {
                data: data.clone(),
                slot: self.slot,
            })
            .ok_or(FetchError::NotFound(*pubkey))
    }

    fn supply_at_slot(&self, mint: &Pubkey, slot: u64) -> Result<Option<u64>, FetchError> {
        Ok(self.historical.get(&(*mint, slot)).copied())
    }
}

// ---------- PDAs (same seeds as the program) ----------

pub fn config_pda(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"config", mint.as_ref()], program_id).0
}

pub fn audit_pda(program_id: &Pubkey, mint: &Pubkey, sequence: u64) -> Pubkey {
    Pubkey::find_program_address(
        &[b"audit", mint.as_ref(), &sequence.to_le_bytes()],
        program_id,
    )
    .0
}

pub fn log_pda(program_id: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"log", mint.as_ref()], program_id).0
}

// ---------- Consolidated state ----------

pub struct OnChainState {
    pub config: PorConfig,
    pub record: AuditRecord,
    pub log: LogState,
    pub mint_decimals: u8,
    pub current_supply: u64,
    pub supply_slot: u64,
    /// Supply reread at the record's ingestion_slot, when available (path A, §4.4.4).
    pub historical_supply: Option<u64>,
    /// merkle_root of EVERY anchored attestation (0..config.next_sequence), in
    /// order: the MMR in LogState accumulates the whole history, so consistency
    /// is checked against all of it, whatever `sequence` is being verified.
    pub history_roots: Vec<[u8; 32]>,
}

fn deser<T: AccountDeserialize>(pda: Pubkey, snap: &AccountSnapshot) -> Result<T, FetchError> {
    T::try_deserialize(&mut snap.data.as_slice())
        .map_err(|e| FetchError::Deserialize(pda, e.to_string()))
}

pub fn load_state(
    fetcher: &dyn AccountFetcher,
    program_id: &Pubkey,
    mint: &Pubkey,
    sequence: u64,
) -> Result<OnChainState, FetchError> {
    let cfg_pda = config_pda(program_id, mint);
    let config: PorConfig = deser(cfg_pda, &fetcher.fetch(&cfg_pda)?)?;
    if sequence >= config.next_sequence {
        return Err(FetchError::NotFound(audit_pda(program_id, mint, sequence)));
    }

    let mut history_roots = Vec::with_capacity(config.next_sequence as usize);
    let mut record: Option<AuditRecord> = None;
    for seq in 0..config.next_sequence {
        let pda = audit_pda(program_id, mint, seq);
        let rec: AuditRecord = deser(pda, &fetcher.fetch(&pda)?)?;
        history_roots.push(rec.merkle_root);
        if seq == sequence {
            record = Some(rec);
        }
    }
    let record = record.expect("sequence < next_sequence is covered by the loop");

    let l_pda = log_pda(program_id, mint);
    let log: LogState = deser(l_pda, &fetcher.fetch(&l_pda)?)?;

    let mint_snap = fetcher.fetch(mint)?;
    let mint_info = parse_mint(&mint_snap.data)?;
    let historical_supply = fetcher.supply_at_slot(mint, record.ingestion_slot)?;

    Ok(OnChainState {
        config,
        record,
        log,
        mint_decimals: mint_info.decimals,
        current_supply: mint_info.supply,
        supply_slot: mint_snap.slot,
        historical_supply,
        history_roots,
    })
}
