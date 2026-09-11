//! Process-owned engine state. Private to the binary; see `main.rs`.
//!
//! Until the dispatcher lands, these items are reached only by the unit tests
//! beside them, so the non-test build sees them as unused. That is the honest
//! state of the code rather than something to silence item by item.
#![cfg_attr(not(test), allow(dead_code))]

pub(crate) mod ffi;
pub(crate) mod secret;
pub(crate) mod sqlcipher;
