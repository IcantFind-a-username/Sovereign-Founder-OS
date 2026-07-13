use chrono::{DateTime, Duration, Utc};
use sovereign_contracts::{CapabilityToken, PolicyDecision};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CapabilityError {
    #[error("policy denied the request")]
    PolicyDenied,
    #[error("human approval required before issuing token")]
    ApprovalRequired,
    #[error("token expired")]
    Expired,
    #[error("token exhausted")]
    Exhausted,
    #[error("token scope mismatch")]
    ScopeMismatch,
    #[error("venture mismatch")]
    VentureMismatch,
}

#[derive(Debug, Clone)]
pub struct IssueOptions {
    pub ttl: Duration,
    pub max_uses: u32,
}

impl Default for IssueOptions {
    fn default() -> Self {
        Self {
            ttl: Duration::minutes(5),
            max_uses: 1,
        }
    }
}

#[derive(Debug, Default)]
pub struct CapabilityIssuer;

impl CapabilityIssuer {
    pub fn new() -> Self {
        Self
    }

    pub fn issue(
        &self,
        decision: &PolicyDecision,
        options: IssueOptions,
        approved: bool,
    ) -> Result<CapabilityToken, CapabilityError> {
        if !decision.allowed {
            return Err(CapabilityError::PolicyDenied);
        }
        if decision.requires_approval && !approved {
            return Err(CapabilityError::ApprovalRequired);
        }

        let now = Utc::now();
        Ok(CapabilityToken {
            token_id: Uuid::new_v4(),
            venture_id: decision.request.venture_id.clone(),
            actor_id: decision.request.actor_id.clone(),
            tool: decision.request.tool.clone(),
            operation: decision.request.operation.clone(),
            resource: decision.request.resource.clone(),
            max_uses: options.max_uses,
            issued_at: now,
            expires_at: now + options.ttl,
            policy_decision_id: decision.decision_id,
        })
    }
}

#[derive(Debug, Default)]
pub struct CapabilityValidator {
    uses: std::collections::HashMap<Uuid, u32>,
}

impl CapabilityValidator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate(
        &mut self,
        token: &CapabilityToken,
        venture_id: &str,
        tool: &str,
        operation: &str,
        resource: &str,
        now: DateTime<Utc>,
    ) -> Result<(), CapabilityError> {
        if token.venture_id != venture_id {
            return Err(CapabilityError::VentureMismatch);
        }
        if token.tool != tool || token.operation != operation || token.resource != resource {
            return Err(CapabilityError::ScopeMismatch);
        }
        if now > token.expires_at {
            return Err(CapabilityError::Expired);
        }

        let count = self.uses.entry(token.token_id).or_insert(0);
        if *count >= token.max_uses {
            return Err(CapabilityError::Exhausted);
        }
        *count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sovereign_contracts::{ActionRequest, AutomationLevel, DataClass};
    use sovereign_policy::PolicyEngine;

    #[test]
    fn issue_and_consume_token() {
        let engine = PolicyEngine::new();
        let request = ActionRequest {
            actor_id: "agent".into(),
            venture_id: "ven_1".into(),
            tool: "email".into(),
            operation: "draft".into(),
            resource: "customer:1".into(),
            data_class: DataClass::Amber,
            automation_level: AutomationLevel::L1Draft,
        };
        let decision = engine.evaluate(request);
        let issuer = CapabilityIssuer::new();
        let token = issuer.issue(&decision, IssueOptions::default(), false).unwrap();

        let mut validator = CapabilityValidator::new();
        validator
            .validate(&token, "ven_1", "email", "draft", "customer:1", Utc::now())
            .unwrap();
        assert_eq!(
            validator.validate(&token, "ven_1", "email", "draft", "customer:1", Utc::now()),
            Err(CapabilityError::Exhausted)
        );
    }

    #[test]
    fn rejects_scope_mismatch() {
        let engine = PolicyEngine::new();
        let request = ActionRequest {
            actor_id: "agent".into(),
            venture_id: "ven_1".into(),
            tool: "email".into(),
            operation: "draft".into(),
            resource: "customer:1".into(),
            data_class: DataClass::Green,
            automation_level: AutomationLevel::L1Draft,
        };
        let decision = engine.evaluate(request);
        let token = CapabilityIssuer::new()
            .issue(&decision, IssueOptions::default(), false)
            .unwrap();
        let mut validator = CapabilityValidator::new();
        assert_eq!(
            validator.validate(&token, "ven_1", "email", "send", "customer:1", Utc::now()),
            Err(CapabilityError::ScopeMismatch)
        );
    }
}
