//! Deterministic verification pipeline: on-chain read -> dataset retrieval ->
//! integrity + the four chained checks + MMR consistency -> report. Pure
//! (no printing): `main` maps `Outcome` to the exit-code contract 0/1/2.
use crate::chain::{self, AccountFetcher, OnChainState};
use crate::checks;
use crate::checks::coverage::{status_name, NewObservation};
use crate::dataset::{self, DatasetError};
use crate::report::{CheckResult, VerificationReport};
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;

#[derive(Debug)]
pub enum Outcome {
    /// Every check passed (exit 0).
    Approved(VerificationReport),
    /// At least one check failed (exit 1).
    Rejected(VerificationReport),
    /// Operational error (exit 2): the verification could not be completed.
    /// Includes dataset unavailability, flagged as VERIFICATION FAILURE (§4.2.5).
    OperationalFailure(String),
}

pub struct PipelineParams<'a> {
    pub fetcher: &'a dyn AccountFetcher,
    pub program_id: Pubkey,
    pub mint: Pubkey,
    pub sequence: u64,
    /// Local directory or CID; None => AuditRecord.ipfs_cid through the gateway.
    pub dataset_source: Option<String>,
    /// signature.json path; None => `<dataset dir>/signature.json`.
    pub signature_path: Option<PathBuf>,
    pub gateway: String,
    pub account: Option<[u8; 32]>,
}

fn build_report(
    p: &PipelineParams,
    st: &OnChainState,
    checks: Vec<CheckResult>,
    new_observation: Option<NewObservation>,
) -> VerificationReport {
    let verdict = if checks.iter().all(|c| c.passed) {
        "APPROVED"
    } else {
        "REJECTED"
    };
    VerificationReport {
        schema: "por-verify/1",
        program_id: p.program_id.to_string(),
        mint: p.mint.to_string(),
        sequence: p.sequence,
        verdict,
        status_onchain: status_name(st.record.status),
        checks,
        new_observation,
    }
}

pub fn run(p: PipelineParams) -> Outcome {
    // On-chain read: PorConfig, every AuditRecord, LogState, mint.
    let st = match chain::load_state(p.fetcher, &p.program_id, &p.mint, p.sequence) {
        Ok(st) => st,
        Err(e) => return Outcome::OperationalFailure(format!("on-chain read: {e}")),
    };

    // Published dataset (local directory or IPFS gateway) + signature sidecar.
    let source = p
        .dataset_source
        .clone()
        .unwrap_or_else(|| st.record.ipfs_cid.clone());
    let bundle = match dataset::load_dataset(
        &source,
        &p.gateway,
        p.signature_path.as_deref(),
        &st.record.dataset_sha256,
    ) {
        Ok(b) => b,
        Err(DatasetError::Unavailable(msg)) => {
            // §4.2.5: without the full leaf set the anti-Hu binding cannot be
            // re-executed; unavailability IS a verification failure, signalled
            // as an operational error (exit 2) with an explicit message.
            return Outcome::OperationalFailure(format!(
                "VERIFICATION FAILURE: dataset unavailable ({msg}); without the full leaf set \
                 the binding check cannot be re-executed, and unavailability is a verification \
                 failure, never tacit approval"
            ));
        }
        Err(e) => {
            // Hash mismatch or malformed dataset: tampering -> rejection (exit 1).
            let checks = vec![CheckResult::fail("dataset_integrity", e.to_string())];
            return Outcome::Rejected(build_report(&p, &st, checks, None));
        }
    };

    let mut results = vec![CheckResult::pass(
        "dataset_integrity",
        format!(
            "sha256 of {} published bytes matches anchored dataset_sha256 {}",
            bundle.dataset_len,
            hex::encode(st.record.dataset_sha256)
        ),
    )];

    // Check 1: strict signature under the recorded signer_pubkey.
    results.push(checks::signature::check_signature(
        &st.config,
        &st.record,
        st.mint_decimals,
        &bundle.signature,
    ));

    // Checks 2 + 3: per-account inclusion and per-node binding over the whole tree.
    let tree = checks::tree::check_tree(
        &bundle.leaves,
        &st.record.merkle_root,
        st.record.total_reservas,
        st.config.max_value,
        p.account,
    );
    results.push(tree.inclusion);
    results.push(tree.binding);

    // Check 4: coverage with the verifier's own supply read (path A or B);
    // this is where the verdict is confronted with the recorded status.
    let cov = checks::coverage::check_coverage(
        st.record.total_reservas,
        st.mint_decimals,
        st.config.reserve_decimals,
        st.record.status,
        st.record.supply_at_ingestion,
        st.historical_supply,
        st.current_supply,
        st.supply_slot,
    );
    results.push(cov.check);

    // MMR history consistency.
    results.push(checks::mmr::check_mmr(&st.history_roots, &st.log));

    let report = build_report(&p, &st, results, cov.new_observation);
    if report.approved() {
        Outcome::Approved(report)
    } else {
        Outcome::Rejected(report)
    }
}
