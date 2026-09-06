#![allow(dead_code)]
//! Constants shared by the tests: every value below was produced by
//! `scripts/e2e-localnet.sh` with `RECORD_FIXTURES=1` (fixed keys under
//! `tests/fixtures/keys`, snapshots with fixed seeds) and frozen.

pub fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Pubkey of `tests/fixtures/keys/mint.json`.
pub const MINT: &str = "3d7g5XnaMpGfi91LEzU7q3kqjSJoF6qVPEWF1AwbYiXb";
/// Pubkey of `tests/fixtures/keys/oracle.json` (payer == admin == oracle_authority).
pub const ORACLE: &str = "3hqct4aiLBtncACxFEWQ12LMXC1TknD9Qtp2g7p5xeZF";

pub fn mint() -> Pubkey {
    Pubkey::from_str(MINT).unwrap()
}

/// Ed25519 signature from `tests/fixtures/<dataset dir>/signature.json`.
pub fn signature(dataset_dir: &str) -> [u8; 64] {
    #[derive(serde::Deserialize)]
    struct Sidecar {
        signature: String,
    }
    let raw =
        std::fs::read_to_string(fixtures_dir().join(dataset_dir).join("signature.json")).unwrap();
    let sidecar: Sidecar = serde_json::from_str(&raw).unwrap();
    hex::decode(sidecar.signature).unwrap().try_into().unwrap()
}

/// Copies a dataset fixture dir into `tmp`, flipping one bit in the MIDDLE of dataset.jsonl.
pub fn tampered_dataset_dir(dataset_dir: &str) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let src = fixtures_dir().join(dataset_dir);
    let mut bytes = std::fs::read(src.join("dataset.jsonl")).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0x01;
    std::fs::write(tmp.path().join("dataset.jsonl"), &bytes).unwrap();
    std::fs::copy(
        src.join("signature.json"),
        tmp.path().join("signature.json"),
    )
    .unwrap();
    tmp
}
