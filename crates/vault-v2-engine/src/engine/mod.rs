//! Process-owned engine state. Private to the binary; see `main.rs`.
//!
//! Until the dispatcher lands, these items are reached only by the unit tests
//! beside them, so the non-test build sees them as unused. That is the honest
//! state of the code rather than something to silence item by item.
// Task 2 exposes internal unlock APIs consumed by Task 3/4 tests and fixtures;
// the release dispatcher is still inert.
#![allow(dead_code, clippy::enum_variant_names, clippy::too_many_arguments)]

pub(crate) mod entropy;
pub(crate) mod ffi;
pub(crate) mod key_slots;
pub(crate) mod platform;
pub(crate) mod process;
pub(crate) mod recovery;
pub(crate) mod recovery_authorizer;
pub(crate) mod secret;
pub(crate) mod sqlcipher;
pub(crate) mod wrapper_golden_v1;
pub(crate) mod wrappers;
