//! Process-local public-projection broker (RFC 0004 first implementation).
//!
//! Request bytes become visible to an adapter only inside a dispatch that
//! already recorded an attempt. There is no public adapter trait, no public
//! transition method, and no way to mint route evidence from a caller string.
//! This broker has no network capability.

use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use uuid::Uuid;

use crate::compile::PublicJob;
use crate::local::{LocalResult, STAND_IN_KIND};
use crate::types::{Placement, PolicySnapshot, Purpose};

/// Closed provider identity used in evidence. Not a caller-supplied string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClosedProviderId {
    OnDeviceDeterministic,
}

/// Attempt outcome recorded by the broker, never by a caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Succeeded,
    FailedBeforeDispatch,
    Queued,
}

/// One-use attempt state. Fields are private and there is no public
/// constructor or transition method: callers cannot manufacture completion.
#[allow(dead_code)]
pub struct Attempt {
    job_id: Uuid,
    outcome: AttemptOutcome,
}

/// Value-free route evidence derived from broker state. Identifiers are
/// closed; this type is not built from caller strings.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct RouteEvidence {
    placement: Placement,
    provider: ClosedProviderId,
    outcome: AttemptOutcome,
    policy_digest: String,
    purpose: Purpose,
    stand_in_kind: &'static str,
}

impl RouteEvidence {
    pub fn placement(&self) -> Placement {
        self.placement
    }

    pub fn provider(&self) -> ClosedProviderId {
        self.provider
    }

    pub fn outcome(&self) -> AttemptOutcome {
        self.outcome
    }

    pub fn policy_digest(&self) -> &str {
        &self.policy_digest
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn stand_in_kind(&self) -> &'static str {
        self.stand_in_kind
    }

    pub(crate) fn local_success(policy: PolicySnapshot, purpose: Purpose) -> Self {
        Self {
            placement: Placement::Local,
            provider: ClosedProviderId::OnDeviceDeterministic,
            outcome: AttemptOutcome::Succeeded,
            policy_digest: policy.digest(),
            purpose,
            stand_in_kind: STAND_IN_KIND,
        }
    }

    pub(crate) fn queued(policy: PolicySnapshot, purpose: Purpose) -> Self {
        Self {
            placement: Placement::Local,
            provider: ClosedProviderId::OnDeviceDeterministic,
            outcome: AttemptOutcome::Queued,
            policy_digest: policy.digest(),
            purpose,
            stand_in_kind: STAND_IN_KIND,
        }
    }

    pub(crate) fn failed_before_dispatch(policy: PolicySnapshot, purpose: Purpose) -> Self {
        Self {
            placement: Placement::Local,
            provider: ClosedProviderId::OnDeviceDeterministic,
            outcome: AttemptOutcome::FailedBeforeDispatch,
            policy_digest: policy.digest(),
            purpose,
            stand_in_kind: STAND_IN_KIND,
        }
    }

    pub(crate) fn projection_stand_in(job: &PublicJob) -> Self {
        Self {
            placement: Placement::PublicProjection,
            provider: ClosedProviderId::OnDeviceDeterministic,
            outcome: AttemptOutcome::Succeeded,
            policy_digest: job.policy().digest(),
            purpose: job.purpose(),
            stand_in_kind: STAND_IN_KIND,
        }
    }
}

impl std::fmt::Debug for RouteEvidence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RouteEvidence")
            .field("placement", &self.placement)
            .field("provider", &self.provider)
            .field("outcome", &self.outcome)
            .field("policy_digest", &self.policy_digest)
            .field("purpose", &self.purpose)
            .field("stand_in_kind", &self.stand_in_kind)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BrokerError {
    #[error("the job expired before dispatch")]
    Expired,
    #[error("this job already has an attempt")]
    AlreadyAttempted,
}

pub(crate) struct Broker {
    public_adapter_observations: AtomicUsize,
    owned_node_observations: AtomicUsize,
    public_jobs_created: AtomicUsize,
    attempts: Mutex<BTreeSet<Uuid>>,
}

impl Broker {
    pub(crate) fn new() -> Self {
        Self {
            public_adapter_observations: AtomicUsize::new(0),
            owned_node_observations: AtomicUsize::new(0),
            public_jobs_created: AtomicUsize::new(0),
            attempts: Mutex::new(BTreeSet::new()),
        }
    }

    pub(crate) fn public_adapter_observations(&self) -> usize {
        self.public_adapter_observations.load(Ordering::SeqCst)
    }

    pub(crate) fn owned_node_observations(&self) -> usize {
        self.owned_node_observations.load(Ordering::SeqCst)
    }

    pub(crate) fn public_jobs_created(&self) -> usize {
        self.public_jobs_created.load(Ordering::SeqCst)
    }

    pub(crate) fn note_public_job_created(&self) {
        self.public_jobs_created.fetch_add(1, Ordering::SeqCst);
    }

    /// Dispatch compiled projection bytes to the one fixed no-network
    /// stand-in. The stand-in is process-local and labelled as an on-device
    /// demonstration; counting the observation records that a public
    /// projection was handed to the projection consumer.
    pub(crate) fn dispatch_projection_stand_in(
        &self,
        job: &PublicJob,
        now_unix: i64,
    ) -> Result<LocalResult, BrokerError> {
        if !job.is_live_at(now_unix) {
            return Err(BrokerError::Expired);
        }
        let mut attempts = self
            .attempts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !attempts.insert(job.id()) {
            return Err(BrokerError::AlreadyAttempted);
        }
        // Bytes become visible to the adapter only after the attempt exists.
        self.public_adapter_observations
            .fetch_add(1, Ordering::SeqCst);
        let outbound = job.outbound_text();
        Ok(LocalResult::from_projection_bytes(job.purpose(), &outbound))
    }
}

impl std::fmt::Debug for Broker {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Broker")
            .field(
                "public_adapter_observations",
                &self.public_adapter_observations(),
            )
            .field("owned_node_observations", &self.owned_node_observations())
            .field("public_jobs_created", &self.public_jobs_created())
            .finish()
    }
}
