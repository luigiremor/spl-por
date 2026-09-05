use data_encoding::BASE32_NOPAD;
use sha2::{Digest, Sha256};

/// kubo default chunk size (chunker=size-262144).
pub const KUBO_CHUNK_SIZE: usize = 262_144;

/// Offline CIDv1 for a SINGLE-BLOCK file under the fixed profile:
/// multibase 'b' (lowercase base32, RFC 4648 no padding) over
/// [0x01 version | 0x55 codec raw | 0x12 sha2-256 | 0x20 length | 32-byte digest].
/// Returns None when the file would span multiple chunks (kubo then builds a
/// UnixFS dag-pb root and the CID is no longer the hash of the file bytes;
/// integrity rests on dataset_sha256 in that case).
pub fn compute_single_block_cid_v1_raw(bytes: &[u8]) -> Option<String> {
    if bytes.len() > KUBO_CHUNK_SIZE {
        return None;
    }
    let digest = Sha256::digest(bytes);
    let mut cid_bytes = vec![0x01, 0x55, 0x12, 0x20]; // v1 | raw | sha2-256 | len 32
    cid_bytes.extend_from_slice(&digest);
    Some(format!(
        "b{}",
        BASE32_NOPAD.encode(&cid_bytes).to_ascii_lowercase()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cid_has_fixed_profile_shape() {
        let cid = compute_single_block_cid_v1_raw(b"hello world").unwrap();
        assert!(cid.starts_with('b'), "multibase base32 lowercase prefix");
        assert_eq!(cid.len(), 59, "CIDv1 raw + sha2-256 in base32 is 59 chars");
        assert!(cid.len() <= 64, "must fit the on-chain ipfs_cid limit");
        assert!(cid
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn cid_known_answer_hello_world() {
        // Frozen known-answer (generate-and-freeze): obtained once with
        // --nocapture AND cross-checked against real kubo. Freezes the
        // profile (version 1, codec raw 0x55, multihash sha2-256 0x12/0x20, base32).
        let cid = compute_single_block_cid_v1_raw(b"hello world").unwrap();
        println!("cid(hello world) = {cid}");
        assert_eq!(cid, "bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e");
    }

    #[test]
    fn multi_chunk_input_returns_none() {
        let big = vec![0u8; KUBO_CHUNK_SIZE + 1];
        assert!(compute_single_block_cid_v1_raw(&big).is_none());
        let exact = vec![0u8; KUBO_CHUNK_SIZE];
        assert!(compute_single_block_cid_v1_raw(&exact).is_some());
    }

    #[test]
    fn different_bytes_yield_different_cids() {
        assert_ne!(
            compute_single_block_cid_v1_raw(b"a").unwrap(),
            compute_single_block_cid_v1_raw(b"b").unwrap()
        );
    }
}
