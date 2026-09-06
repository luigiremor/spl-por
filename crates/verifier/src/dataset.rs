//! Retrieval of the published dataset (§4.2.5 / §4.3.2). The attester
//! publishes `dataset.jsonl` (canonical JSONL, one `{"account_id","balance"}`
//! per line, bytes covered by `dataset_sha256`; the IPFS CID addresses this
//! single file) and, next to it, a `signature.json` sidecar carrying the
//! Ed25519 signature OUTSIDE the object covered by the hash (no circular
//! dependency). Three error classes: unavailable (not retrieved) -> exit 2
//! with an explicit VERIFICATION FAILURE message; hash mismatch or malformed
//! (retrieved but tampered) -> rejection, exit 1.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DatasetError {
    /// Could not RETRIEVE the data (IO/HTTP/missing input). §4.2.5:
    /// unavailability is a verification failure, never tacit approval.
    #[error("dataset unavailable: {0}")]
    Unavailable(String),
    /// Retrieved, but the bytes differ from the anchored dataset_sha256: tampering.
    #[error("dataset_sha256 mismatch: expected {expected}, got {got}")]
    HashMismatch { expected: String, got: String },
    /// Retrieved, but not in the canonical format: tampering or attester bug.
    #[error("malformed dataset: {0}")]
    Malformed(String),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetLine {
    account_id: String,
    balance: String,
}

#[derive(Debug, Deserialize)]
struct SignatureSidecar {
    signature: String,
}

#[derive(Debug)]
pub struct DatasetBundle {
    /// Leaves exactly as published (ordering/range rules are enforced by
    /// `sum_tree::build`, the single source of those rules).
    pub leaves: Vec<([u8; 32], u64)>,
    /// Ed25519 signature over `AttestationMessage::canonical_bytes()`.
    pub signature: [u8; 64],
    /// Size of the published dataset bytes (reported only).
    pub dataset_len: usize,
}

fn looks_like_path(source: &str) -> bool {
    source.contains('/') || source.contains('\\') || source.starts_with('.')
}

fn fetch_dataset_bytes(source: &str, gateway: &str) -> Result<Vec<u8>, DatasetError> {
    let dir = Path::new(source);
    if dir.is_dir() {
        let file = dir.join("dataset.jsonl");
        return std::fs::read(&file)
            .map_err(|e| DatasetError::Unavailable(format!("{}: {e}", file.display())));
    }
    if looks_like_path(source) {
        return Err(DatasetError::Unavailable(format!(
            "local dataset dir not found: {source}"
        )));
    }
    // Otherwise a CID: the fixed-profile CID addresses the raw dataset bytes.
    let url = format!("{}/ipfs/{}", gateway.trim_end_matches('/'), source);
    let resp = ureq::get(&url)
        .call()
        .map_err(|e| DatasetError::Unavailable(format!("{url}: {e}")))?;
    let mut buf = Vec::new();
    resp.into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| DatasetError::Unavailable(format!("{url}: {e}")))?;
    Ok(buf)
}

pub fn parse_jsonl(bytes: &[u8]) -> Result<Vec<([u8; 32], u64)>, DatasetError> {
    let text =
        std::str::from_utf8(bytes).map_err(|e| DatasetError::Malformed(format!("utf-8: {e}")))?;
    let mut leaves = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let n = idx + 1;
        let l: DatasetLine = serde_json::from_str(line)
            .map_err(|e| DatasetError::Malformed(format!("line {n}: {e}")))?;
        let id: [u8; 32] = hex::decode(&l.account_id)
            .map_err(|e| DatasetError::Malformed(format!("line {n}: account_id hex: {e}")))?
            .try_into()
            .map_err(|_| {
                DatasetError::Malformed(format!("line {n}: account_id must be 32 bytes"))
            })?;
        let balance: u64 = l
            .balance
            .parse()
            .map_err(|e| DatasetError::Malformed(format!("line {n}: balance: {e}")))?;
        leaves.push((id, balance));
    }
    if leaves.is_empty() {
        return Err(DatasetError::Malformed("dataset has no leaves".into()));
    }
    Ok(leaves)
}

fn read_signature(path: &Path) -> Result<[u8; 64], DatasetError> {
    let raw = std::fs::read(path)
        .map_err(|e| DatasetError::Unavailable(format!("{}: {e}", path.display())))?;
    let sidecar: SignatureSidecar = serde_json::from_slice(&raw)
        .map_err(|e| DatasetError::Malformed(format!("signature.json: {e}")))?;
    hex::decode(&sidecar.signature)
        .map_err(|e| DatasetError::Malformed(format!("signature hex: {e}")))?
        .try_into()
        .map_err(|_| DatasetError::Malformed("signature must be 64 bytes".into()))
}

/// `source`: local directory (dataset.jsonl [+ signature.json]) or IPFS CID
/// fetched from `gateway`. `signature_path` overrides / supplies the sidecar
/// (mandatory for a CID source: the fixed-profile CID covers only the dataset).
pub fn load_dataset(
    source: &str,
    gateway: &str,
    signature_path: Option<&Path>,
    expected_sha256: &[u8; 32],
) -> Result<DatasetBundle, DatasetError> {
    let raw = fetch_dataset_bytes(source, gateway)?;
    // Integrity over the EXACT published bytes: independent of host and CID.
    let got: [u8; 32] = Sha256::digest(&raw).into();
    if &got != expected_sha256 {
        return Err(DatasetError::HashMismatch {
            expected: hex::encode(expected_sha256),
            got: hex::encode(got),
        });
    }
    let leaves = parse_jsonl(&raw)?;
    let sig_path: PathBuf = match signature_path {
        Some(p) => p.to_path_buf(),
        None if Path::new(source).is_dir() => Path::new(source).join("signature.json"),
        None => {
            return Err(DatasetError::Unavailable(
                "signature.json is not addressed by the dataset CID: pass --signature <PATH>"
                    .into(),
            ))
        }
    };
    let signature = read_signature(&sig_path)?;
    Ok(DatasetBundle {
        leaves,
        signature,
        dataset_len: raw.len(),
    })
}
