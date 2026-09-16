//! Sensitivity, visibility, placement, and the preset that binds them
//! (RFC 0004, "Normative vocabulary").

use serde::Serialize;

/// How exposed a value may be. Deliberately **not** an ordered trust ladder:
/// each variant names a different obligation.
///
/// `NonDisclosableSecret` from the RFC has no variant here on purpose. Key
/// material, permanent credentials, and recovery secrets are never model or
/// public-compute inputs, so this crate must be unable to hold one. They stay
/// behind the vault, identity service, or effects broker as
/// operation-scoped handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Sensitivity {
    /// Raw or derived business secret, personal data, private content, or any
    /// unknown dynamic value.
    Protected,
    /// Transformed or externally produced content that may still reveal
    /// protected facts. Model output starts here.
    RestrictedDerived,
    /// Fixed or authoritative published content with trusted provenance.
    PublicContent,
}

/// Who a grant names. Not a linear order: a public projection is not "less
/// than" a named recipient, it is a different obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Recipient {
    ThisDevice,
    /// Reserved vocabulary. Not executable and not configurable until a
    /// Secure Mesh protocol RFC is accepted and reviewed.
    OwnedMesh,
    NamedRecipient,
    PublicComputeProjection,
}

/// Where a task may run. `Queued` is a decision, never a location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    Local,
    /// Reserved. Never executable in this version.
    OwnedNode,
    PublicProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlacementDecision {
    Run(Placement),
    /// No eligible placement now; the task waits rather than widening.
    Queued {
        reason: &'static str,
    },
    /// Nothing can run this task under the current policy.
    Unavailable {
        reason: &'static str,
    },
}

/// What a projection is allowed to be used for. Closed set: a purpose is part
/// of the authorization, so callers cannot invent one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    /// Turn discovery notes into problems, constraints, and questions.
    DraftDiscoverySummary,
    /// Draft a consulting proposal from an already-summarised brief.
    DraftProposal,
    /// Review a draft for gaps and contradictions.
    ReviewDraft,
}

impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Purpose::DraftDiscoverySummary => "draft_discovery_summary",
            Purpose::DraftProposal => "draft_proposal",
            Purpose::ReviewDraft => "review_draft",
        }
    }
}

/// One authorization tuple. A grant never widens its source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Grant {
    pub recipient: Recipient,
    pub purpose: Purpose,
    pub operation: &'static str,
    pub expires_at_unix: i64,
}

/// The founder's selected mode. Professional refinements may narrow this;
/// widening is a separate, previewed, signed transition and is not expressible
/// here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// Prefer local; permit public compute only for a compiler-built
    /// projection; otherwise queue or offer local alternatives.
    #[default]
    AutoProtect,
    /// No model dispatch leaves this device for the task.
    LocalOnly,
}

/// An immutable policy snapshot. A decision binds one; later edits affect new
/// decisions only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PolicySnapshot {
    pub preset: Preset,
    /// Bumped by any security-narrowing transition or emergency revocation.
    pub revocation_epoch: u64,
    pub created_at_unix: i64,
}

/// Canonicalization version bound into every snapshot digest. Timestamps and
/// random identifiers are not part of the digest (RFC 0004).
pub const POLICY_SCHEMA_VERSION: u32 = 1;

impl PolicySnapshot {
    pub fn new(preset: Preset, now_unix: i64) -> Self {
        Self {
            preset,
            revocation_epoch: 0,
            created_at_unix: now_unix,
        }
    }

    pub fn preset(&self) -> Preset {
        self.preset
    }

    pub fn revocation_epoch(&self) -> u64 {
        self.revocation_epoch
    }

    pub fn created_at_unix(&self) -> i64 {
        self.created_at_unix
    }

    /// `LocalOnly` never compiles or dispatches a public-compute job.
    pub fn forbids_public_compute(&self) -> bool {
        self.preset == Preset::LocalOnly
    }

    /// Deterministic digest of the security-relevant snapshot fields. The
    /// creation timestamp is excluded so two equivalent policies hash alike.
    pub fn digest(&self) -> String {
        use sha2::{Digest, Sha256};
        let preset = match self.preset {
            Preset::AutoProtect => "auto_protect",
            Preset::LocalOnly => "local_only",
        };
        let canonical = format!(
            "privacy-policy-v{POLICY_SCHEMA_VERSION}:{preset}:{}",
            self.revocation_epoch
        );
        hex::encode(Sha256::digest(canonical.as_bytes()))
    }

    /// Where a task may run under this snapshot.
    ///
    /// `LocalOnly` never reaches a public projection, and a local failure
    /// must not silently widen the route: availability is not a reason to
    /// disclose more.
    pub fn placement_for(&self, local_available: bool) -> PlacementDecision {
        match (self.preset, local_available) {
            (_, true) => PlacementDecision::Run(Placement::Local),
            (Preset::LocalOnly, false) => PlacementDecision::Queued {
                reason: "local_only_no_local_compute",
            },
            (Preset::AutoProtect, false) => PlacementDecision::Run(Placement::PublicProjection),
        }
    }
}

/// Observed local compute for this slice: the process-local deterministic
/// stand-in is either present or not. This is not a real model inventory
/// and not a sandbox capability probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalCapability {
    stand_in: bool,
}

impl LocalCapability {
    /// The in-process deterministic demonstration is available.
    pub fn stand_in_available() -> Self {
        Self { stand_in: true }
    }

    /// No local compute: a `LocalOnly` task must queue rather than widen.
    pub fn none() -> Self {
        Self { stand_in: false }
    }

    pub fn is_available(self) -> bool {
        self.stand_in
    }
}

/// Closed, value-free reasons a task cannot run now. Owned-node compute is
/// not an option in this version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeUnavailable {
    LocalOnlyNoLocalCompute,
    InstallOrConfigureLocalModel,
    ReduceTheTask,
    Wait,
}

impl ComputeUnavailable {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnlyNoLocalCompute => "local_only_no_local_compute",
            Self::InstallOrConfigureLocalModel => "install_or_configure_local_model",
            Self::ReduceTheTask => "reduce_the_task",
            Self::Wait => "wait",
        }
    }

    /// Safe alternatives offered when local compute is missing. Owned mesh
    /// is deliberately absent: it is not a configure action.
    pub const fn local_only_alternatives() -> &'static [Self] {
        &[
            Self::InstallOrConfigureLocalModel,
            Self::ReduceTheTask,
            Self::Wait,
        ]
    }
}

/// Why a reserved Owned Mesh / fake configuration action was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ActivationError {
    #[error(
        "owned mesh is not available in this version; this version will not connect another device"
    )]
    OwnedMeshNotAvailableInThisVersion,
    #[error("unknown preset")]
    UnknownPreset,
}

impl Preset {
    /// Parse a selectable v1 preset. `AutoProtect` and `LocalOnly` are the
    /// only accepted names. Owned Mesh labels are refused without echoing the
    /// caller string (value-free diagnostic).
    pub fn parse_selectable(label: &str) -> Result<Self, ActivationError> {
        match label {
            "auto_protect" | "auto-protect" | "AutoProtect" => Ok(Self::AutoProtect),
            "local_only" | "local-only" | "LocalOnly" => Ok(Self::LocalOnly),
            "owned_mesh"
            | "owned-mesh"
            | "OwnedMesh"
            | "my_devices"
            | "company_nodes"
            | "My Devices & Company Nodes" => {
                Err(ActivationError::OwnedMeshNotAvailableInThisVersion)
            }
            _ => Err(ActivationError::UnknownPreset),
        }
    }
}

/// Reserved Research activation. Always refuses: there is no configuration
/// CTA and no Secure Mesh in this version. Takes no configuration so a
/// caller cannot pretend to supply node identities.
pub fn activate_owned_mesh() -> Result<(), ActivationError> {
    Err(ActivationError::OwnedMeshNotAvailableInThisVersion)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_digest_ignores_timestamp_and_is_stable() {
        let early = PolicySnapshot::new(Preset::LocalOnly, 1);
        let late = PolicySnapshot::new(Preset::LocalOnly, 9_999_999);
        assert_eq!(early.digest(), late.digest());
        assert_ne!(
            PolicySnapshot::new(Preset::AutoProtect, 1).digest(),
            early.digest()
        );
        assert_eq!(early.digest().len(), 64);
    }

    #[test]
    fn default_preset_is_auto_protect() {
        assert_eq!(Preset::default(), Preset::AutoProtect);
    }
}
