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
