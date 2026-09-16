//! Closed high-level privacy gateway.
//!
//! This is the only supported way to run a privacy-bound workflow. Attempt
//! transitions, adapters, and request bytes stay inside the crate. Callers
//! receive a local result or a queue decision, never a borrowable public
//! request.

use crate::broker::{Broker, BrokerError, RouteEvidence};
use crate::compile::CompileError;
use crate::local::{process, LocalError, LocalResult};
use crate::types::{
    activate_owned_mesh, ComputeUnavailable, LocalCapability, Placement, PlacementDecision,
    PolicySnapshot, Purpose,
};
use crate::value::SourceRecord;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GatewayError {
    #[error("local deterministic processing failed: {0}")]
    LocalComputeFailed(LocalError),
    #[error(transparent)]
    Compile(#[from] CompileError),
    #[error(transparent)]
    Broker(#[from] BrokerError),
    #[error(
        "owned mesh is not available in this version; this version will not connect another device"
    )]
    OwnedMeshNotAvailableInThisVersion,
}

/// Outcome of a closed workflow. No variant carries a public-compute job.
#[derive(Debug, Clone)]
pub enum WorkflowOutcome {
    Local {
        result: LocalResult,
        evidence: RouteEvidence,
    },
    /// AutoProtect compiled a projection and ran the in-process stand-in.
    /// Still no network; labelled as an on-device demonstration.
    Projection {
        result: LocalResult,
        evidence: RouteEvidence,
    },
    Queued {
        reason: ComputeUnavailable,
        alternatives: &'static [ComputeUnavailable],
        evidence: RouteEvidence,
    },
}

/// Process-local closed gateway. `sovereign-model` may re-export this type
/// and nothing else from the broker/attempt surface.
#[derive(Debug)]
pub struct PrivacyGateway {
    broker: Broker,
}

impl Default for PrivacyGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyGateway {
    pub fn new() -> Self {
        Self {
            broker: Broker::new(),
        }
    }

    pub fn public_adapter_observations(&self) -> usize {
        self.broker.public_adapter_observations()
    }

    pub fn owned_node_observations(&self) -> usize {
        self.broker.owned_node_observations()
    }

    pub fn public_jobs_created(&self) -> usize {
        self.broker.public_jobs_created()
    }

    /// Run one workflow under an immutable policy snapshot.
    ///
    /// `LocalOnly` never compiles a public job and never observes a public
    /// or owned-node adapter, including when local compute is missing or
    /// the stand-in fails.
    pub fn run(
        &self,
        record: &SourceRecord,
        purpose: Purpose,
        policy: PolicySnapshot,
        now_unix: i64,
        local: LocalCapability,
    ) -> Result<WorkflowOutcome, GatewayError> {
        match policy.placement_for(local.is_available()) {
            PlacementDecision::Run(Placement::Local) => {
                match process(record, purpose) {
                    Ok(result) => Ok(WorkflowOutcome::Local {
                        evidence: RouteEvidence::local_success(policy, purpose),
                        result,
                    }),
                    Err(error) => {
                        // Failure stays local: no compile, no public job, no
                        // adapter observation. The error is returned so the
                        // caller can offer local alternatives; the route
                        // does not widen.
                        let _evidence = RouteEvidence::failed_before_dispatch(policy, purpose);
                        Err(GatewayError::LocalComputeFailed(error))
                    }
                }
            }
            PlacementDecision::Run(Placement::PublicProjection) => {
                self.broker.note_public_job_created();
                let (job, _preview) = crate::compile::compile(record, purpose, policy, now_unix)?;
                let result = self.broker.dispatch_projection_stand_in(&job, now_unix)?;
                Ok(WorkflowOutcome::Projection {
                    evidence: RouteEvidence::projection_stand_in(&job),
                    result,
                })
            }
            PlacementDecision::Run(Placement::OwnedNode) => {
                let _ = activate_owned_mesh();
                Err(GatewayError::OwnedMeshNotAvailableInThisVersion)
            }
            PlacementDecision::Queued { .. } => Ok(WorkflowOutcome::Queued {
                reason: ComputeUnavailable::LocalOnlyNoLocalCompute,
                alternatives: ComputeUnavailable::local_only_alternatives(),
                evidence: RouteEvidence::queued(policy, purpose),
            }),
            PlacementDecision::Unavailable { .. } => Ok(WorkflowOutcome::Queued {
                reason: ComputeUnavailable::LocalOnlyNoLocalCompute,
                alternatives: ComputeUnavailable::local_only_alternatives(),
                evidence: RouteEvidence::queued(policy, purpose),
            }),
        }
    }
}
