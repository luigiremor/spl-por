mod common;

use verifier::chain::{load_state, FixtureFetcher};
use verifier::dataset::{load_dataset, parse_jsonl, DatasetError};

const GW: &str = "http://127.0.0.1:8080";

fn expected_sha() -> [u8; 32] {
    let f = FixtureFetcher::load(&common::fixtures_dir().join("base.json")).unwrap();
    load_state(&f, &por::ID, &common::mint(), 0)
        .unwrap()
        .record
        .dataset_sha256
}

#[test]
fn intact_local_dataset_loads_and_parses_leaves() {
    let dir = common::fixtures_dir().join("dataset-0");
    let bundle = load_dataset(dir.to_str().unwrap(), GW, None, &expected_sha())
        .expect("intact dataset must load");
    assert_eq!(bundle.leaves.len(), 100);
    assert_eq!(
        bundle.leaves.iter().map(|(_, b)| *b).sum::<u64>(),
        4_660_446_420
    );
    assert_eq!(bundle.signature, common::signature("dataset-0"));
}

#[test]
fn single_flipped_bit_fails_sha256() {
    let tmp = common::tampered_dataset_dir("dataset-0");
    let err = load_dataset(tmp.path().to_str().unwrap(), GW, None, &expected_sha()).unwrap_err();
    assert!(
        matches!(err, DatasetError::HashMismatch { .. }),
        "got: {err}"
    );
}

#[test]
fn missing_local_dataset_is_unavailable() {
    let err = load_dataset("/nonexistent/dataset/dir", GW, None, &expected_sha()).unwrap_err();
    assert!(matches!(err, DatasetError::Unavailable(_)), "got: {err}");
}

#[test]
fn missing_signature_sidecar_is_unavailable() {
    let tmp = tempfile::tempdir().unwrap();
    let src = common::fixtures_dir().join("dataset-0");
    std::fs::copy(src.join("dataset.jsonl"), tmp.path().join("dataset.jsonl")).unwrap();
    let err = load_dataset(tmp.path().to_str().unwrap(), GW, None, &expected_sha()).unwrap_err();
    assert!(matches!(err, DatasetError::Unavailable(_)), "got: {err}");
    // An explicit --signature path fixes it.
    let sig = src.join("signature.json");
    load_dataset(
        tmp.path().to_str().unwrap(),
        GW,
        Some(&sig),
        &expected_sha(),
    )
    .unwrap();
}

#[test]
fn unreachable_gateway_for_cid_is_unavailable() {
    // Port 9 (discard) refuses the connection: the CID is never resolved.
    let err = load_dataset(
        "bafkreibunp26j4uaudlxycidydx3wtt32vg2hjh2dzrcmsgifxq37xf4ba",
        "http://127.0.0.1:9",
        None,
        &expected_sha(),
    )
    .unwrap_err();
    assert!(matches!(err, DatasetError::Unavailable(_)), "got: {err}");
}

#[test]
fn malformed_lines_are_rejected() {
    for bad in [
        "{\"account_id\":\"zz\",\"balance\":\"1\"}\n",
        "{\"account_id\":\"00\",\"balance\":\"1\"}\n",
        "{\"account_id\":\"0000000000000000000000000000000000000000000000000000000000000001\",\"balance\":\"-1\"}\n",
        "{\"account_id\":\"0000000000000000000000000000000000000000000000000000000000000001\",\"balance\":\"1\",\"x\":1}\n",
        "",
    ] {
        assert!(matches!(parse_jsonl(bad.as_bytes()), Err(DatasetError::Malformed(_))), "{bad:?}");
    }
    let ok = "{\"account_id\":\"0000000000000000000000000000000000000000000000000000000000000001\",\"balance\":\"7\"}\n";
    assert_eq!(
        parse_jsonl(ok.as_bytes()).unwrap(),
        vec![(
            [0u8; 31]
                .iter()
                .chain(&[1u8])
                .copied()
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
            7
        )]
    );
}
