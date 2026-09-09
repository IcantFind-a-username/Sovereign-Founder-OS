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
//! - the **response** comes back untrusted and can widen nothing.
//!
//! What this crate does not do, on purpose: it opens no socket, holds no
//! secret, persists nothing, and depends on neither the model gateway nor the
//! policy engine. Dispatching a compiled job is an external effect and is
//! authorized above this crate.
//!
//! Honest limit: pseudonymisation and previews reduce what leaves; they do
//! not make a public provider confidential. A task that genuinely requires a
//! model to read raw customer text belongs on a local model or nowhere.

mod compile;
mod response;
mod transform;
mod types;
mod value;

pub use compile::{
    compile, require, CompileError, ExposureManifest, ExposurePreview, PreviewRow, PublicJob,
    JOB_TTL_SECONDS,
};
pub use response::{accept, RehydratedResponse, ResponseError, MAX_RESPONSE_CHARS};
pub use transform::{by_id, for_purpose, Disposition, FieldRule, Transform, REGISTRY};
pub use types::{
    Grant, Placement, PlacementDecision, PolicySnapshot, Preset, Purpose, Recipient, Sensitivity,
};
pub use value::{Provenance, SourceRecord, TrustedValue};
