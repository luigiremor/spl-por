use assert_cmd::Command;

#[test]
fn attestation_help_lists_contract_flags() {
    let mut cmd = Command::cargo_bin("por-verify").unwrap();
    let assert = cmd.args(["attestation", "--help"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    for flag in [
        "--mint",
        "--sequence",
        "--rpc",
        "--dataset",
        "--signature",
        "--gateway",
        "--program-id",
        "--account",
        "--fixtures",
        "--json",
    ] {
        assert!(stdout.contains(flag), "missing flag in help: {flag}");
    }
}

#[test]
fn missing_required_args_is_operational_error_exit_2() {
    let mut cmd = Command::cargo_bin("por-verify").unwrap();
    cmd.arg("attestation").assert().code(2);
}

mod common;

use predicates::prelude::*;

fn fixture(name: &str) -> String {
    common::fixtures_dir()
        .join(name)
        .to_str()
        .unwrap()
        .to_string()
}

fn verify(
    fixture_name: &str,
    sequence: &str,
    dataset: &str,
    extra: &[&str],
) -> assert_cmd::assert::Assert {
    let mut cmd = Command::cargo_bin("por-verify").unwrap();
    cmd.args([
        "attestation",
        "--mint",
        common::MINT,
        "--sequence",
        sequence,
        "--rpc",
        "http://unused.invalid",
        "--fixtures",
        &fixture(fixture_name),
        "--dataset",
        dataset,
    ]);
    cmd.args(extra);
    cmd.assert()
}

#[test]
fn approved_run_exits_0() {
    verify("base.json", "0", &fixture("dataset-0"), &[])
        .code(0)
        .stdout(predicate::str::contains("VERDICT: APPROVED"))
        .stdout(predicate::str::contains("[PASS] mmr_consistency"));
}

#[test]
fn json_flag_emits_report_with_contract_fields() {
    let assert = verify("history3.json", "1", &fixture("dataset-1"), &["--json"]).code(0);
    let out = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["schema"], "por-verify/1");
    assert_eq!(v["verdict"], "APPROVED");
    assert_eq!(v["status_onchain"], "DIVERGENTE");
    assert_eq!(v["sequence"], 1);
    assert_eq!(v["checks"].as_array().unwrap().len(), 6);
}

#[test]
fn account_flag_restricts_inclusion_and_absent_account_exits_1() {
    let first_line =
        std::fs::read_to_string(common::fixtures_dir().join("dataset-0/dataset.jsonl")).unwrap();
    let first: serde_json::Value =
        serde_json::from_str(first_line.lines().next().unwrap()).unwrap();
    let id = first["account_id"].as_str().unwrap();
    verify("base.json", "0", &fixture("dataset-0"), &["--account", id]).code(0);
    verify(
        "base.json",
        "0",
        &fixture("dataset-0"),
        &["--account", &"ff".repeat(32)],
    )
    .code(1)
    .stdout(predicate::str::contains("[FAIL] inclusion"));
    verify(
        "base.json",
        "0",
        &fixture("dataset-0"),
        &["--account", "zz"],
    )
    .code(2);
}

#[test]
fn tampered_dataset_run_exits_1() {
    let tmp = common::tampered_dataset_dir("dataset-0");
    verify("base.json", "0", tmp.path().to_str().unwrap(), &[])
        .code(1)
        .stdout(predicate::str::contains("VERDICT: REJECTED"))
        .stdout(predicate::str::contains("[FAIL] dataset_integrity"));
}

#[test]
fn unavailable_dataset_run_exits_2_with_verification_failure_message() {
    verify("base.json", "0", "/nonexistent/dataset", &[])
        .code(2)
        .stderr(predicate::str::contains("VERIFICATION FAILURE"));
}

#[test]
fn unknown_sequence_exits_2() {
    verify("base.json", "7", &fixture("dataset-0"), &[])
        .code(2)
        .stderr(predicate::str::contains("on-chain read"));
}

#[test]
fn unreachable_rpc_exits_2() {
    Command::cargo_bin("por-verify")
        .unwrap()
        .args([
            "attestation",
            "--mint",
            common::MINT,
            "--sequence",
            "0",
            "--rpc",
            "http://127.0.0.1:9",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("on-chain read"));
}
