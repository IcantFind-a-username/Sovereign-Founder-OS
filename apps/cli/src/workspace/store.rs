use super::util::storage;
use super::*;

/// One audit event of a state-changing operation: the action, the resource it
/// touched, and a human-meaningful summary payload (hashed onto the chain).
pub(super) struct AuditEntry {
    pub action: String,
    pub resource: String,
    pub payload: serde_json::Value,
}
use std::path::Path;

use sovereign_audit_ledger::{hash_bytes, AppendInput, AuditLedger};
use sovereign_contracts::{ActionRequest, AutomationLevel, DataClass};
use sovereign_identity::DeviceIdentity;
use sovereign_policy::PolicyEngine;
use sovereign_vault::Vault;

impl Store {
    pub fn open(root: &Path) -> Result<Self, WorkspaceError> {
        std::fs::create_dir_all(root).map_err(storage)?;
        let device_path = root.join("device.json");
        let device = if device_path.exists() {
            DeviceIdentity::load(&device_path).map_err(storage)?
        } else {
            let device = DeviceIdentity::generate();
            device.save(&device_path).map_err(storage)?;
            device
        };
        Ok(Self {
            root: root.to_path_buf(),
            device,
            policy: PolicyEngine::new(),
        })
    }

    pub fn load(&self) -> Result<Workspace, WorkspaceError> {
        let vault = Vault::init(self.root.join("vault")).map_err(storage)?;
        match vault.get(WORKSPACE_VAULT_ENTRY) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| WorkspaceError::Storage(format!("corrupt workspace: {error}"))),
            Err(sovereign_vault::VaultError::NotFound(_)) => Ok(Workspace {
                version: WORKSPACE_VERSION,
                ..Workspace::default()
            }),
            Err(error) => Err(storage(error)),
        }
    }

    pub(super) fn save(&self, workspace: &Workspace) -> Result<(), WorkspaceError> {
        let mut vault = Vault::init(self.root.join("vault")).map_err(storage)?;
        let bytes = serde_json::to_vec(workspace).map_err(storage)?;
        vault.put(WORKSPACE_VAULT_ENTRY, &bytes).map_err(storage)?;
        Ok(())
    }

    /// Evaluate policy for a workflow action; deny fails closed, and both
    /// allowed and denied evaluations can be audited by the caller.
    pub(super) fn check_policy(
        &self,
        tool: &str,
        operation: &str,
        resource: &str,
        data_class: DataClass,
        automation_level: AutomationLevel,
    ) -> (bool, bool, String) {
        let decision = self.policy.evaluate(ActionRequest {
            actor_id: "founder".into(),
            venture_id: "workspace".into(),
            tool: tool.into(),
            operation: operation.into(),
            resource: resource.into(),
            data_class,
            automation_level,
        });
        (
            decision.allowed,
            decision.requires_approval,
            decision.reason,
        )
    }

    /// Commit one logical operation **audit-first**: every event of the
    /// operation is appended to the signed chain in a single durable ledger
    /// write, and only then is the state written. A crash between the two
    /// leaves the chain ahead of the state — an interrupted operation the
    /// integrity self-audit reports as a warning and a retry heals — and
    /// never committed state without its signed evidence.
    pub(super) fn commit(
        &self,
        workspace: &Workspace,
        events: Vec<AuditEntry>,
    ) -> Result<(), WorkspaceError> {
        self.append_events(events)?;
        self.save(workspace)
    }

    /// Append audit events without a state change (exports, disclosures of
    /// read-only actions). State-changing operations go through [`commit`].
    pub(super) fn record(
        &self,
        action: &str,
        resource: &str,
        payload: serde_json::Value,
    ) -> Result<(), WorkspaceError> {
        self.append_events(vec![AuditEntry {
            action: action.into(),
            resource: resource.into(),
            payload,
        }])
    }

    /// Append every event to the signed hash chain and persist the chain in
    /// one atomic write, so a multi-event operation can never leave a
    /// half-recorded ledger behind.
    fn append_events(&self, events: Vec<AuditEntry>) -> Result<(), WorkspaceError> {
        let ledger_path = self.root.join("ledger.json");
        let mut ledger = if ledger_path.exists() {
            AuditLedger::load(&ledger_path, self.device.public_key_b64()).map_err(storage)?
        } else {
            AuditLedger::new()
        };
        for event in events {
            let payload_digest = hash_bytes(&serde_json::to_vec(&event.payload).map_err(storage)?);
            ledger
                .append(
                    AppendInput {
                        venture_id: "workspace".into(),
                        actor_id: "founder".into(),
                        action: event.action,
                        resource: event.resource,
                        capability_id: None,
                        payload: serde_json::json!({
                            "summary": event.payload,
                            "payload_digest": payload_digest,
                        }),
                        policy_decision_hash: None,
                    },
                    &self.device,
                )
                .map_err(storage)?;
        }
        ledger.save(&ledger_path).map_err(storage)?;
        Ok(())
    }
}
