mod common;

use verifier::chain::FixtureFetcher;
use verifier::pipeline::{run, Outcome, PipelineParams};

fn params<'a>(fetcher: &'a FixtureFetcher, sequence: u64, dataset: &str) -> PipelineParams<'a> {
    PipelineParams {
        fetcher,
        program_id: por::ID,
        mint: common::mint(),
        sequence,
        dataset_source: Some(dataset.to_string()),
        signature_path: None,
        gateway: "http://127.0.0.1:8080".to_string(),
        account: None,
    }
}

fn fixture(name: &str) -> FixtureFetcher {
    FixtureFetcher::load(&common::fixtures_dir().join(name)).unwrap()
}

fn dataset(name: &str) -> String {
    common::fixtures_dir()
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

#[test]
fn base_fixture_is_approved() {
    match run(params(&fixture("base.json"), 0, &dataset("dataset-0"))) {
        Outcome::Approved(report) => {
            assert_eq!(report.verdict, "APPROVED");
            assert_eq!(report.status_onchain, "OK");
            // integrity, signature, inclusion, binding, coverage, mmr
            let names: Vec<_> = report.checks.iter().map(|c| c.name).collect();
            assert_eq!(
                names,
                [
                    "dataset_integrity",
                    "signature",
                    "inclusion",
                    "binding",
                    "coverage",
                    "mmr_consistency"
                ]
            );
            assert!(report.new_observation.is_none(), "base fixture uses path A");
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

#[test]
fn base_nohist_fixture_is_approved_with_new_observation() {
    match run(params(
        &fixture("base_nohist.json"),
        0,
        &dataset("dataset-0"),
    )) {
        Outcome::Approved(report) => {
            let obs = report
                .new_observation
                .expect("path B emits a new observation");
            assert!(obs.covered);
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

#[test]
fn coherent_divergente_record_is_approved() {
    // sequence 1 was anchored DIVERGENTE (haircut) with a coherent status:
    // valid attestation, approved, status reported as DIVERGENTE.
    match run(params(&fixture("history3.json"), 1, &dataset("dataset-1"))) {
        Outcome::Approved(report) => {
            assert_eq!(report.status_onchain, "DIVERGENTE");
            assert_eq!(report.verdict, "APPROVED");
        }
        other => panic!("expected Approved, got {other:?}"),
    }
}

#[test]
fn every_sequence_of_history3_is_approved() {
    for seq in 0..3u64 {
        let out = run(params(
            &fixture("history3.json"),
            seq,
            &dataset(&format!("dataset-{seq}")),
        ));
        assert!(
            matches!(out, Outcome::Approved(_)),
            "sequence {seq}: {out:?}"
        );
    }
}

#[test]
fn unavailable_dataset_is_operational_failure_with_verification_message() {
    match run(params(&fixture("base.json"), 0, "/nonexistent/dataset")) {
        Outcome::OperationalFailure(msg) => {
            assert!(msg.contains("VERIFICATION FAILURE"), "msg: {msg}");
            assert!(msg.contains("unavailable"), "msg: {msg}");
        }
        other => panic!("expected OperationalFailure, got {other:?}"),
    }
}

#[test]
fn tampered_dataset_is_rejected_not_operational() {
    let tmp = common::tampered_dataset_dir("dataset-0");
    match run(params(
        &fixture("base.json"),
        0,
        tmp.path().to_str().unwrap(),
    )) {
        Outcome::Rejected(report) => {
            assert_eq!(report.verdict, "REJECTED");
            assert_eq!(report.checks.len(), 1);
            assert_eq!(report.checks[0].name, "dataset_integrity");
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn wrong_dataset_for_sequence_is_rejected() {
    // dataset-1 bytes do not hash to sequence 0's dataset_sha256.
    match run(params(&fixture("history3.json"), 0, &dataset("dataset-1"))) {
        Outcome::Rejected(_) => {}
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn incoherent_status_in_fixture_is_rejected() {
    // Local fixture with the AuditRecord's status flipped (OK -> DIVERGENTE)
    // while R_norm >= supply_at_ingestion: the verifier's own verdict contradicts
    // the recorded status (Cap. 6 scenario S16). Re-serialized through Anchor
    // (the account is zero-padded to AuditRecord::SPACE, so the status byte is
    // not simply the last byte).
    use anchor_lang::{AccountDeserialize, AccountSerialize};
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let path = common::fixtures_dir().join("base.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let audit = verifier::chain::audit_pda(&por::ID, &common::mint(), 0).to_string();
    let data = b64
        .decode(v["accounts"][&audit]["data_base64"].as_str().unwrap())
        .unwrap();
    let mut rec = por::state::AuditRecord::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(rec.status, por::state::STATUS_OK);
    rec.status = por::state::STATUS_DIVERGENTE;
    let mut out = Vec::new();
    rec.try_serialize(&mut out).unwrap();
    v["accounts"][&audit]["data_base64"] = serde_json::Value::String(b64.encode(&out));
    let f = FixtureFetcher::from_json(&v.to_string()).unwrap();
    match run(params(&f, 0, &dataset("dataset-0"))) {
        Outcome::Rejected(report) => {
            assert_eq!(report.status_onchain, "DIVERGENTE");
            let cov = report.checks.iter().find(|c| c.name == "coverage").unwrap();
            assert!(!cov.passed);
            assert!(cov.detail.contains("status tampering"), "{}", cov.detail);
            // the signature does not cover status, so it still passes
            assert!(
                report
                    .checks
                    .iter()
                    .find(|c| c.name == "signature")
                    .unwrap()
                    .passed
            );
        }
        other => panic!("expected Rejected, got {other:?}"),
    }
}

#[test]
fn golden_json_report_is_stable() {
    let Outcome::Approved(report) = run(params(&fixture("base.json"), 0, &dataset("dataset-0")))
    else {
        panic!("expected Approved");
    };
    let golden_path = common::fixtures_dir().join("golden_report.json");
    // Generate-and-freeze: on the first run (file absent) print the JSON for
    // manual freezing; afterwards compare byte by byte.
    if !golden_path.exists() {
        println!("GOLDEN CANDIDATE:\n{}", report.to_json());
        panic!("golden_report.json missing: freeze the candidate printed above");
    }
    let golden = std::fs::read_to_string(&golden_path).unwrap();
    assert_eq!(report.to_json().trim(), golden.trim());
}
