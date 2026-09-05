use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub schema_version: u16,
    pub mint: String,
    pub sequence: String,
    pub merkle_root: String,
    pub total_reservas: String,
    pub currency: String,
    pub decimals: u8,
    pub reserve_decimals: u8,
    pub ipfs_cid: String,
    pub dataset_sha256: String,
    pub supply_snapshot: String,
    pub snapshot_slot: String,
    pub hash_alg: String,
    pub issued_at: String,
}

impl Manifest {
    /// Canonical JSON for the restricted JCS profile used by the PoC.
    pub fn to_jcs(&self) -> serde_json::Result<String> {
        let value = serde_json::to_value(self)?;
        serde_json::to_string(&value)
    }
}

pub fn hex32(bytes: &[u8; 32]) -> String {
    use core::fmt::Write;
    let mut out = String::with_capacity(64);
    for byte in bytes {
        write!(out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}
