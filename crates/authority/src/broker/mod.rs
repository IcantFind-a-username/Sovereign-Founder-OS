//! The owner-effect fixture broker.
//!
//! RFC 0006 gives one process sole writable ownership of the fixture's store,
//! reached only over authenticated IPC. None of that exists yet: this module
//! is the scaffolding for it, so the boundary can be guarded before there is
//! anything behind it to guard.
//!
//! The whole module is behind the non-default `owner-effect-fixture` feature.
//! A default build contains no broker mode, no entry point, and not even the
//! name of one — `default_binary_has_no_hidden_broker_mode_or_symbols` in the
//! CLI's tests asserts that against the built binary, from this slice onward
//! rather than after the implementation lands.

/// The hidden subcommand the fixture re-execs itself as. Named here so the
/// CLI and the broker cannot drift apart, and absent from a default build.
pub const BROKER_SUBCOMMAND: &str = "__owner-effect-broker";

/// Scaffolding only. Every later slice of RFC 0006's broker — the bootstrap
/// frame, the five-second monotonic supervisor deadline, the process lock, the
/// redb open, the per-connection credentials — replaces this.
///
/// It fails closed and says so exactly, rather than pretending to be a broker
/// that owns nothing: a stub that returned success would be indistinguishable
/// from a working broker to any caller written against it.
pub fn run_owner_effect_fixture_broker() -> Result<(), BrokerNotImplemented> {
    Err(BrokerNotImplemented)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrokerNotImplemented;

impl std::fmt::Display for BrokerNotImplemented {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .write_str("owner-effect broker: not implemented — scaffolding only, no store is owned")
    }
}

impl std::error::Error for BrokerNotImplemented {}
