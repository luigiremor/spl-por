/// Publishes a file to the local kubo repo under the FIXED CID profile and
/// returns the CID. Inherits IPFS_PATH from the environment (the E2E script
/// exports a throwaway repo). `ipfs add` works directly on the repo — no
/// daemon required. When the file fits a single raw block, cross-checks
/// kubo's CID against the offline computation and aborts on profile drift.
pub fn ipfs_add(path: &std::path::Path) -> anyhow::Result<String> {
    let out = std::process::Command::new("ipfs")
        .args([
            "add",
            "--cid-version=1",
            "--raw-leaves",
            "--hash=sha2-256",
            "--chunker=size-262144",
            "--pin=true",
            "-Q",
        ])
        .arg(path)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run `ipfs` (is kubo installed?): {e}"))?;
    anyhow::ensure!(
        out.status.success(),
        "ipfs add failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let cid = String::from_utf8(out.stdout)?.trim().to_string();
    anyhow::ensure!(
        !cid.is_empty() && cid.len() <= crate::attestation::MAX_CID_LEN,
        "unexpected CID from kubo: {cid:?}"
    );
    let bytes = std::fs::read(path)?;
    if let Some(expected) = crate::cid::compute_single_block_cid_v1_raw(&bytes) {
        anyhow::ensure!(
            cid == expected,
            "kubo returned {cid} but the fixed profile expects {expected}: CID profile drift"
        );
    }
    Ok(cid)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Integration test against a real kubo. Skips (with a visible note) when
    /// `ipfs` is not on PATH, so `cargo test` stays green on machines without
    /// kubo; the E2E script hard-requires it.
    #[test]
    fn ipfs_add_matches_offline_cid() {
        if std::process::Command::new("ipfs")
            .arg("version")
            .output()
            .is_err()
        {
            eprintln!("SKIP: kubo (ipfs) not on PATH");
            return;
        }
        let repo = std::env::temp_dir().join("por-attest-ipfs-test-repo");
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        // Only test that touches IPFS_PATH; kubo child processes inherit it.
        std::env::set_var("IPFS_PATH", &repo);
        let init = std::process::Command::new("ipfs")
            .args(["init", "--profile=test"])
            .output()
            .unwrap();
        assert!(
            init.status.success(),
            "ipfs init failed: {}",
            String::from_utf8_lossy(&init.stderr)
        );

        let dataset =
            crate::snapshot::serialize_dataset(&crate::snapshot::generate_snapshot(100, 42, 0));
        let file = repo.join("dataset.jsonl");
        std::fs::write(&file, &dataset).unwrap();

        let cid = ipfs_add(&file).unwrap();
        let expected = crate::cid::compute_single_block_cid_v1_raw(&dataset).unwrap();
        assert_eq!(
            cid, expected,
            "kubo CID must match the offline fixed-profile CID"
        );
        assert!(cid.len() <= 64);
    }
}
