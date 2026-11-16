use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

pub mod decision;

/// Authoritative endpoint inputs, not an allocation or a recovery decision.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub id: String,
    pub region: String,
    pub model: String,
    pub budget_tpm: i64,
    pub reserved_tpm: i64,
    pub available: bool,
    pub revision: i64,
    pub run_epoch: String,
}

pub fn assess(endpoint: &Endpoint) -> Result<serde_json::Value> {
    ensure!(!endpoint.id.is_empty(), "missing endpoint ID");
    ensure!(!endpoint.run_epoch.is_empty(), "missing scenario epoch");
    ensure!(endpoint.revision > 0, "revision must be positive");
    ensure!(
        endpoint.budget_tpm >= 0 && endpoint.reserved_tpm >= 0,
        "capacity inputs must be nonnegative"
    );
    let spare = endpoint.budget_tpm - endpoint.reserved_tpm;
    Ok(serde_json::json!({
        "id": endpoint.id,
        "region": endpoint.region,
        "model": endpoint.model,
        "budget_tpm": endpoint.budget_tpm,
        "reserved_tpm": endpoint.reserved_tpm,
        "spare_tpm": if endpoint.available { spare.max(0) } else { 0 },
        "status": if !endpoint.available { "unavailable" } else if spare < 0 { "overcommitted" } else { "within_budget" },
        "input_revision": endpoint.revision,
        "run_epoch": endpoint.run_epoch,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint() -> Endpoint {
        Endpoint {
            id: "EU-A".into(),
            region: "EU".into(),
            model: "support-model".into(),
            budget_tpm: 100_000,
            reserved_tpm: 60_000,
            available: true,
            revision: 1,
            run_epoch: "test".into(),
        }
    }

    #[test]
    fn derives_headroom_without_claiming_recovery_or_policy() {
        let mut input = endpoint();
        assert_eq!(assess(&input).unwrap()["spare_tpm"], 40_000);
        input.available = false;
        assert_eq!(assess(&input).unwrap()["status"], "unavailable");
        assert_eq!(assess(&input).unwrap()["spare_tpm"], 0);
        input.available = true;
        input.budget_tpm = 50_000;
        assert_eq!(assess(&input).unwrap()["status"], "overcommitted");
        assert_eq!(assess(&input).unwrap()["spare_tpm"], 0);
        input.budget_tpm = -1;
        assert!(assess(&input).is_err());
    }
}
