use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signer};

/// Loads a keypair from the solana-keygen JSON format: a JSON array of 64
/// bytes (32-byte secret seed followed by the 32-byte public key). Rejects
/// files whose stored public half does not match the derived one.
pub fn load_solana_keypair(path: &std::path::Path) -> anyhow::Result<Keypair> {
    let raw: Vec<u8> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    anyhow::ensure!(
        raw.len() == 64,
        "expected 64-byte solana keypair JSON, got {} bytes",
        raw.len()
    );
    let secret = SecretKey::from_bytes(&raw[..32])
        .map_err(|e| anyhow::anyhow!("invalid secret key: {e}"))?;
    let public: PublicKey = (&secret).into();
    anyhow::ensure!(
        public.as_bytes() == &raw[32..],
        "public key in file does not match the one derived from the secret"
    );
    Ok(Keypair { secret, public })
}

/// Signs arbitrary bytes (the attester always passes canonical_bytes()).
pub fn sign_message(keypair: &Keypair, message: &[u8]) -> [u8; 64] {
    keypair.sign(message).to_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{PublicKey, SecretKey};

    fn tmp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("por-attest-keys-{name}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Builds a solana-keygen-compatible JSON file (array of 64 u8:
    /// 32-byte secret followed by 32-byte public) from a fixed secret.
    fn write_fixture_keypair(dir: &std::path::Path) -> std::path::PathBuf {
        let secret = SecretKey::from_bytes(&[7u8; 32]).unwrap();
        let public: PublicKey = (&secret).into();
        let mut raw = secret.to_bytes().to_vec();
        raw.extend_from_slice(public.as_bytes());
        let path = dir.join("oracle.json");
        std::fs::write(&path, serde_json::to_string(&raw).unwrap()).unwrap();
        path
    }

    #[test]
    fn loads_solana_keygen_json_and_checks_pubkey() {
        let kp = load_solana_keypair(&write_fixture_keypair(&tmp("load"))).unwrap();
        let expected: PublicKey = (&SecretKey::from_bytes(&[7u8; 32]).unwrap()).into();
        assert_eq!(kp.public.as_bytes(), expected.as_bytes());
    }

    #[test]
    fn rejects_keypair_file_with_wrong_length() {
        let path = tmp("badlen").join("bad.json");
        std::fs::write(&path, serde_json::to_string(&vec![1u8; 63]).unwrap()).unwrap();
        assert!(load_solana_keypair(&path).is_err());
    }

    #[test]
    fn rejects_keypair_file_with_mismatched_pubkey() {
        let mut raw = SecretKey::from_bytes(&[7u8; 32])
            .unwrap()
            .to_bytes()
            .to_vec();
        raw.extend_from_slice(&[0u8; 32]); // wrong public half
        let path = tmp("mismatch").join("bad.json");
        std::fs::write(&path, serde_json::to_string(&raw).unwrap()).unwrap();
        assert!(load_solana_keypair(&path).is_err());
    }

    #[test]
    fn signature_over_canonical_bytes_verifies_strict_and_rejects_tamper() {
        let kp = load_solana_keypair(&write_fixture_keypair(&tmp("sign"))).unwrap();
        let entries = crate::snapshot::generate_snapshot(10, 42, 0);
        let msg = crate::attestation::build_message(
            &entries,
            [9u8; 32],
            0,
            2,
            "bafytestcid".into(),
            [7u8; 32],
        )
        .unwrap();
        let bytes = msg.canonical_bytes();
        let sig = sign_message(&kp, &bytes);
        let sig = ed25519_dalek::Signature::try_from(&sig[..]).unwrap();
        // Same canonical acceptance rule as the precompile and the verifier:
        // strict verification (rejects small-order points / non-canonical forms).
        kp.public.verify_strict(&bytes, &sig).unwrap();
        let mut tampered = bytes.clone();
        tampered[0] ^= 0x01;
        assert!(kp.public.verify_strict(&tampered, &sig).is_err());
    }

    #[test]
    fn signature_known_answer_is_frozen() {
        // Ed25519 is deterministic: same key + same message => same signature.
        // Frozen known-answer (generate-and-freeze): obtained once with
        // --nocapture and pasted here; freezes key loading + signing.
        let kp = load_solana_keypair(&write_fixture_keypair(&tmp("ka"))).unwrap();
        let sig = sign_message(&kp, b"spl-por attestation test vector");
        println!("signature = {}", hex::encode(sig));
        assert_eq!(
            hex::encode(sig),
            "9b921dae2d95719d49ef20463b31e1f67beeaf8191b8ae2ba6420465f25c10abf92dd1e5c2f9d74cd5fa2d103105bba67459475c5fd172f84d6a9c2ccfac930f"
        );
    }
}
