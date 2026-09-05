use anchor_lang::prelude::*;

pub const SCHEMA_VERSION: u16 = 1;
pub const CURRENCY_BRL: [u8; 3] = *b"BRL";
pub const MAX_CID_LEN: usize = 64;
pub const MAX_PEAKS: usize = 32;
pub const STATUS_OK: u8 = 0;
pub const STATUS_DIVERGENTE: u8 = 1;

#[account]
pub struct PorConfig {
    pub mint: Pubkey,
    pub oracle_authority: Pubkey,
    pub admin: Pubkey,
    pub schema_version: u16,
    pub currency: [u8; 3],
    pub reserve_decimals: u8,
    pub max_value: u64,
    pub next_sequence: u64,
    pub log_state: Pubkey,
}

impl PorConfig {
    pub const SPACE: usize = 8 + 32 + 32 + 32 + 2 + 3 + 1 + 8 + 8 + 32;
}

/// `ipfs_cid` is bounded structurally by the fixed `SPACE`: a longer CID
/// fails account serialization (`AccountDidNotSerialize`).
#[account]
pub struct AuditRecord {
    pub sequence: u64,
    pub merkle_root: [u8; 32],
    pub total_reservas: u64,
    pub supply_at_ingestion: u64,
    pub ingestion_slot: u64,
    pub ipfs_cid: String,
    pub dataset_sha256: [u8; 32],
    pub signer_pubkey: Pubkey,
    pub status: u8,
}

impl AuditRecord {
    pub const SPACE: usize = 8 + 8 + 32 + 8 + 8 + 8 + (4 + MAX_CID_LEN) + 32 + 32 + 1;
}

#[account]
pub struct LogState {
    pub peaks: [[u8; 32]; MAX_PEAKS],
    pub peak_count: u8,
    pub leaf_count: u64,
}

impl LogState {
    pub const SPACE: usize = 8 + 32 * MAX_PEAKS + 1 + 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_space_matches_field_layout() {
        // disc(8) + 4x Pubkey(32) + u16(2) + [u8;3](3) + u8(1) + 2x u64(8)
        assert_eq!(PorConfig::SPACE, 158);
        // disc(8) + u64(8) + [32] + 3x u64(8) + (4+64) cid + [32] + Pubkey(32) + u8(1)
        assert_eq!(AuditRecord::SPACE, 205);
        // disc(8) + 32x[32](1024) + u8(1) + u64(8)
        assert_eq!(LogState::SPACE, 1041);
    }
}
