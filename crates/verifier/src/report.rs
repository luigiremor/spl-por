use crate::checks::coverage::NewObservation;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

impl CheckResult {
    pub fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            passed: true,
            detail: detail.into(),
        }
    }
    pub fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            passed: false,
            detail: detail.into(),
        }
    }
}

/// Machine-readable report (`--json`), schema `por-verify/1`. Field order
/// and names are a contract (golden test): the evaluation (Cap. 6) reads it.
#[derive(Debug, serde::Serialize)]
pub struct VerificationReport {
    pub schema: &'static str,
    pub program_id: String,
    pub mint: String,
    pub sequence: u64,
    /// "APPROVED" | "REJECTED": verdict recomputed by the verifier.
    pub verdict: &'static str,
    /// "OK" | "DIVERGENTE": status anchored in the AuditRecord (confronted by
    /// the coverage check; a coherent DIVERGENTE IS approved).
    pub status_onchain: &'static str,
    pub checks: Vec<CheckResult>,
    pub new_observation: Option<NewObservation>,
}

impl VerificationReport {
    pub fn approved(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("report is serializable")
    }

    pub fn to_human(&self) -> String {
        let mut out = format!(
            "por-verify: attestation mint={} sequence={} program={}\n",
            self.mint, self.sequence, self.program_id
        );
        for c in &self.checks {
            let mark = if c.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!("  [{mark}] {:<17} {}\n", c.name, c.detail));
        }
        out.push_str(&format!("  on-chain status: {}\n", self.status_onchain));
        if let Some(obs) = &self.new_observation {
            out.push_str(&format!(
                "  new observation: supply={} at slot={} (covered={})\n",
                obs.supply, obs.slot, obs.covered
            ));
        }
        out.push_str(&format!("VERDICT: {}\n", self.verdict));
        out
    }
}
