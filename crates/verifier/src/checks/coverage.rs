use crate::report::CheckResult;
use por::state::{STATUS_DIVERGENTE, STATUS_OK};

#[derive(Debug, Clone, serde::Serialize)]
pub struct NewObservation {
    pub slot: u64,
    pub supply: u64,
    pub covered: bool,
}

pub struct CoverageOutcome {
    pub check: CheckResult,
    pub new_observation: Option<NewObservation>,
}

/// R_norm = total_reservas * 10^(decimals - reserve_decimals), checked
/// arithmetic (same rule as the program: overflow aborts, never wraps).
fn normalize(total_reservas: u64, decimals: u8, reserve_decimals: u8) -> Option<u64> {
    let exp = decimals.checked_sub(reserve_decimals)?;
    let factor = 10u64.checked_pow(u32::from(exp))?;
    total_reservas.checked_mul(factor)
}

fn expected_status(r_norm: u64, supply: u64) -> u8 {
    // Inequality, never strict equality: over-collateralization conforms.
    if r_norm >= supply {
        STATUS_OK
    } else {
        STATUS_DIVERGENTE
    }
}

pub fn status_name(status: u8) -> &'static str {
    match status {
        STATUS_OK => "OK",
        STATUS_DIVERGENTE => "DIVERGENTE",
        _ => "UNKNOWN",
    }
}

/// Check 4 (§4.4.4): recomputes the coverage classification and confronts it
/// with the recorded status, through one of two paths:
///  - Path A (historical_supply = Some): supply reread at the very
///    ingestion_slot -> STRICT confrontation; any disagreement rejects.
///  - Path B (None): checks status vs the recorded supply_at_ingestion; the
///    current read becomes a NEW OBSERVATION, not a refutation of the snapshot.
///
/// In both: DIVERGENTE with a coherent status is a valid attestation.
#[allow(clippy::too_many_arguments)]
pub fn check_coverage(
    total_reservas: u64,
    decimals: u8,
    reserve_decimals: u8,
    status: u8,
    supply_at_ingestion: u64,
    historical_supply: Option<u64>,
    current_supply: u64,
    supply_slot: u64,
) -> CoverageOutcome {
    let Some(r_norm) = normalize(total_reservas, decimals, reserve_decimals) else {
        return CoverageOutcome {
            check: CheckResult::fail(
                "coverage",
                "normalization overflowed u64 (checked arithmetic)",
            ),
            new_observation: None,
        };
    };
    let tampering = |supply: u64, label: &str, expected: u8| {
        CheckResult::fail(
            "coverage",
            format!(
                "recorded status {} contradicts R_norm={r_norm} vs {label}={supply} (expected {}): status tampering",
                status_name(status),
                status_name(expected)
            ),
        )
    };
    match historical_supply {
        Some(supply_hist) => {
            if supply_hist != supply_at_ingestion {
                return CoverageOutcome {
                    check: CheckResult::fail(
                        "coverage",
                        format!(
                            "historical supply reread ({supply_hist}) differs from recorded supply_at_ingestion ({supply_at_ingestion})"
                        ),
                    ),
                    new_observation: None,
                };
            }
            let expected = expected_status(r_norm, supply_hist);
            if expected != status {
                return CoverageOutcome {
                    check: tampering(supply_hist, "S(ingestion_slot)", expected),
                    new_observation: None,
                };
            }
            CoverageOutcome {
                check: CheckResult::pass(
                    "coverage",
                    format!(
                        "historical path: R_norm={r_norm} vs S={supply_hist} at ingestion_slot -> {}, status consistent",
                        status_name(status)
                    ),
                ),
                new_observation: None,
            }
        }
        None => {
            let observation = NewObservation {
                slot: supply_slot,
                supply: current_supply,
                covered: r_norm >= current_supply,
            };
            let expected = expected_status(r_norm, supply_at_ingestion);
            if expected != status {
                return CoverageOutcome {
                    check: tampering(supply_at_ingestion, "supply_at_ingestion", expected),
                    new_observation: Some(observation),
                };
            }
            CoverageOutcome {
                check: CheckResult::pass(
                    "coverage",
                    format!(
                        "current path: R_norm={r_norm} vs supply_at_ingestion={supply_at_ingestion} -> {}, status consistent; current supply is a new observation",
                        status_name(status)
                    ),
                ),
                new_observation: Some(observation),
            }
        }
    }
}
