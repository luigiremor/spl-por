use por::state::{STATUS_DIVERGENTE, STATUS_OK};
use verifier::checks::coverage::check_coverage;

// total_reservas = 10_000 cents, decimals 6, reserve_decimals 2
// => R_norm = 10_000 * 10^4 = 100_000_000.

#[test]
fn via_a_historical_reread_consistent_status_passes() {
    let out = check_coverage(
        10_000,
        6,
        2,
        STATUS_OK,
        90_000_000,
        Some(90_000_000), // supply reread at ingestion_slot
        95_000_000,
        1000,
    );
    assert!(out.check.passed, "{}", out.check.detail);
    assert!(
        out.new_observation.is_none(),
        "path A emits no new observation"
    );
}

#[test]
fn via_a_status_inconsistent_with_historical_supply_fails() {
    // Historical reread says S = 120_000_000 > R_norm, but the record says OK
    // and supply_at_ingestion was tampered to look covered.
    let out = check_coverage(
        10_000,
        6,
        2,
        STATUS_OK,
        90_000_000,
        Some(120_000_000),
        120_000_000,
        1000,
    );
    assert!(!out.check.passed, "strict path A confrontation must reject");
}

#[test]
fn via_b_status_ok_but_recomputed_rnorm_below_supply_fails() {
    // KEY TEST (status tampering): recorded OK, but
    // R_norm = 100_000_000 < supply_at_ingestion = 200_000_000 => rejected.
    let out = check_coverage(
        10_000,
        6,
        2,
        STATUS_OK,
        200_000_000,
        None, // no historical state: path B
        200_000_000,
        1000,
    );
    assert!(!out.check.passed, "tampered status must reject");
    assert!(
        out.check.detail.contains("status"),
        "detail: {}",
        out.check.detail
    );
}

#[test]
fn via_b_divergent_snapshot_with_matching_status_is_valid() {
    // Coherent DIVERGENTE is a valid attestation, not a rejection (§4.4.4).
    let out = check_coverage(
        10_000,
        6,
        2,
        STATUS_DIVERGENTE,
        200_000_000,
        None,
        200_000_000,
        1000,
    );
    assert!(out.check.passed, "{}", out.check.detail);
}

#[test]
fn via_b_reports_current_read_as_new_observation_not_refutation() {
    // Current supply grew past R_norm; the historical snapshot stays OK.
    let out = check_coverage(10_000, 6, 2, STATUS_OK, 90_000_000, None, 150_000_000, 1234);
    assert!(
        out.check.passed,
        "current read does not refute the snapshot"
    );
    let obs = out.new_observation.expect("path B emits a new observation");
    assert_eq!(obs.slot, 1234);
    assert_eq!(obs.supply, 150_000_000);
    assert!(!obs.covered);
}

#[test]
fn exact_coverage_is_ok_never_strict_equality_required() {
    let out = check_coverage(10_000, 6, 2, STATUS_OK, 100_000_000, None, 100_000_000, 1);
    assert!(out.check.passed);
    let over = check_coverage(10_000, 6, 2, STATUS_OK, 1, None, 1, 1);
    assert!(over.check.passed, "over-collateralization conforms");
}

#[test]
fn normalization_overflow_is_rejected_by_checked_arithmetic() {
    let out = check_coverage(u64::MAX, 6, 2, STATUS_OK, 1, None, 1, 1);
    assert!(!out.check.passed, "normalization overflow must reject");
    let neg = check_coverage(1, 2, 6, STATUS_OK, 1, None, 1, 1);
    assert!(!neg.check.passed, "decimals < reserve_decimals must reject");
}
