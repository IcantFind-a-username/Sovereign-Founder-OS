//! Data-sovereignty boundary for Sovereign Founder OS (RFC 0004).
//!
//! The question this crate answers is not "how do we hide data from a model
//! that must read it" — that has no answer. It is "what is the least that has
//! to leave this device for the task, and can the owner see exactly that
//! before it goes".
//!
//! So the shape is:
//!
//! - values arrive opaque, and anything of unknown sensitivity is
//!   [`Sensitivity::Protected`];
//! - a **registered, versioned transform** names every field it may read and
//!   how each is dispositioned; a field it does not name is omitted, so
//!   growing the caller's data model cannot silently start disclosing;
//! - the **compiler** is the only thing that can build a public-compute job,
//!   so no caller can assemble outbound bytes by declaring its own data
//!   "green";
//! - a **preview** shows a person the exact outbound bytes first, and a
//!   pseudonymised field renders as a fixed label, so two records differing
//!   only in protected values compile to identical payloads;
//! - the **response** comes back untrusted and can widen nothing;
//! - [`PrivacyGateway`] is the closed high-level door: a `LocalOnly`
//!   workflow never constructs a public job and never observes a public or
//!   owned-node adapter.
//!
//! What this crate does not do, on purpose: it opens no socket, holds no
//! secret, persists nothing, and depends on neither the model gateway nor the
//! policy engine. Dispatching a compiled job is an external effect and is
//! authorized above this crate. The first projection consumer is a
//! process-local deterministic stand-in, labelled as on-device demonstration,
//! not “cloud-assisted” inference or real AI.
//!
//! Honest limit: this slice is **not** full RFC 0004, **not** a sandboxed
//! local model, **not** ActiveV2, **not** product Exact Effect / 1C0, and
//! **not** Secure Mesh / OwnedMesh executable. Pseudonymisation and previews
//! reduce what leaves; they do not make a public provider confidential.

mod broker;
mod compile;
mod gateway;
mod local;
mod response;
mod transform;
mod types;
mod value;

pub use broker::{Attempt, AttemptOutcome, ClosedProviderId, RouteEvidence};
pub use compile::{
    compile, require, CompileError, ExposureManifest, ExposurePreview, PreviewRow, PublicJob,
    JOB_TTL_SECONDS,
};
pub use gateway::{GatewayError, PrivacyGateway, WorkflowOutcome};
pub use local::{LocalError, LocalResult, STAND_IN_EXPLANATION, STAND_IN_KIND, STAND_IN_LABEL};
pub use response::{accept, RehydratedResponse, ResponseError, MAX_RESPONSE_CHARS};
pub use transform::{by_id, for_purpose, Disposition, FieldRule, Transform, REGISTRY};
pub use types::{
    activate_owned_mesh, ActivationError, ComputeUnavailable, Grant, LocalCapability, Placement,
    PlacementDecision, PolicySnapshot, Preset, Purpose, Recipient, Sensitivity,
    POLICY_SCHEMA_VERSION,
};
pub use value::{Provenance, SourceRecord, TrustedValue};
