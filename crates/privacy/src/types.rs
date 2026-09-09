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

impl PolicySnapshot {
    pub fn new(preset: Preset, now_unix: i64) -> Self {
        Self {
            preset,
            revocation_epoch: 0,
            created_at_unix: now_unix,
        }
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
