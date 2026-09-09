//! The owner-effect fixture broker.
//!
//! RFC 0006 gives one process sole writable ownership of the fixture's store,
//! reached only over authenticated IPC. This slice implements the front of
//! that: read one bounded bootstrap frame, classify the root it names, and
//! stop. Later slices continue from here with the loopback bind, the
//! five-second monotonic supervisor deadline, the process lock, the redb
//! open, and per-connection credentials.
//!
//! The whole module is behind the non-default `owner-effect-fixture` feature.
//! A default build contains no broker mode, no entry point, and not even the
//! name of one — `default_binary_has_no_hidden_broker_mode_or_symbols` in the
//! CLI's tests asserts that against the built binary.

pub mod bootstrap;
pub mod connections;
pub mod corpus;
pub mod exact_fixture;
pub mod fixture_root;
pub mod listener;
pub mod process_lock;
pub mod protocol;
pub mod store;
pub mod supervisor;

/// The hidden subcommand the fixture re-execs itself as. Named here so the
/// CLI and the broker cannot drift apart, and absent from a default build.
pub const BROKER_SUBCOMMAND: &str = "__owner-effect-broker";

/// Read one bootstrap frame and classify the root it names, then stop.
///
/// The ordering is the contract, not an implementation detail. Anything wrong
/// with the input ends this *before* the root is examined; a root that does
/// not classify ends it before anything is bound, locked, or opened.
///
/// A same-account caller can supply a syntactically valid frame. RFC 0006
/// states that plainly and calls it unqualified fixture control, never
/// product admission — so the thing that must not follow is that such a
/// caller can aim the broker at the owner's real Vault or workspace. That is
/// why the root is classified here rather than by the CLI: delegating it
/// would mean a direct caller skipped it.
///
/// Returns `Infallible` on the success side because there is no success yet:
/// until the later slices land, a fully valid input still ends in
/// `NotImplemented` — after the checks have run, so they are exercised rather
/// than merely written.
pub fn run_owner_effect_fixture_broker(
    input: &mut impl std::io::Read,
    address_out: &mut impl std::io::Write,
    diagnostics: &mut impl std::io::Write,
) -> Result<std::convert::Infallible, BrokerExit> {
    let frame = match protocol::read_frame(input) {
        Ok(frame) => frame,
        Err(diagnostic) => return Err(emit(diagnostics, diagnostic)),
    };
    let root = match bootstrap::classify(&frame.root) {
        Ok(root) => root,
        Err(diagnostic) => return Err(emit(diagnostics, diagnostic)),
    };
    // Bind, capture the one deadline, publish the address. Nothing is locked
    // or opened yet, and nothing may be until a supervisor authenticates
    // within that deadline.
    let established = match listener::bind_and_publish(&frame.nonce, address_out) {
        Ok(established) => established,
        Err(diagnostic) => return Err(emit(diagnostics, diagnostic)),
    };
    #[cfg(feature = "fault-injection")]
    crate::fault_injection::reach(
        crate::fault_injection::Barrier::AfterAddressBeforeSupervisorHello,
    );

    let mut supervisor = match listener::accept_by_deadline(&established) {
        Ok(stream) => stream,
        Err(diagnostic) => return Err(emit(diagnostics, diagnostic)),
    };
    // The port admits every process on this machine, so the port is not the
    // boundary — the launch key is. Nothing is locked or opened until this
    // connection proves it holds it.
    if let Err(diagnostic) =
        supervisor::authenticate(&mut supervisor, &frame.launch_key, &frame.nonce)
    {
        return Err(emit(diagnostics, diagnostic));
    }
    #[cfg(feature = "fault-injection")]
    crate::fault_injection::reach(
        crate::fault_injection::Barrier::AfterAuthenticatedHelloBeforeLock,
    );

    // Only now: a broker that has not proved its parent must not claim the
    // store, and a broker that cannot prove it is alone must not either.
    let _lock = match process_lock::acquire(root.path()) {
        Ok(lock) => lock,
        Err(process_lock::LockError::BrokerAlreadyRunning) => {
            return Err(emit(
                diagnostics,
                protocol::Diagnostic::BrokerAlreadyRunning,
            ))
        }
        Err(process_lock::LockError::Unavailable) => {
            return Err(emit(diagnostics, protocol::Diagnostic::LockUnavailable))
        }
    };
    #[cfg(feature = "fault-injection")]
    crate::fault_injection::reach(crate::fault_injection::Barrier::AfterLockBeforeRedbOpen);

    // Only with the lock in hand. `OwnedStore::open` takes a `&HeldLock` and
    // has no other constructor, so this ordering cannot be skipped by a later
    // caller who did not read the comment.
    let _store = match store::OwnedStore::open(root.path(), &_lock) {
        Ok(store) => store,
        Err(store::StoreError::AlreadyOpen) => {
            return Err(emit(
                diagnostics,
                protocol::Diagnostic::BrokerAlreadyRunning,
            ))
        }
        Err(store::StoreError::Unavailable) => {
            return Err(emit(diagnostics, protocol::Diagnostic::StoreUnavailable))
        }
    };
    #[cfg(feature = "fault-injection")]
    crate::fault_injection::reach(crate::fault_injection::Barrier::AfterRedbOpenBeforeBrokerReady);

    Err(BrokerExit::NotImplemented)
}

/// Fixed codes only: an error channel that echoed its input would be a
/// disclosure channel.
fn emit(diagnostics: &mut impl std::io::Write, diagnostic: protocol::Diagnostic) -> BrokerExit {
    let _ = writeln!(diagnostics, "{diagnostic}");
    BrokerExit::Refused(diagnostic)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerExit {
    /// The input or the root was refused; the code is already emitted.
    Refused(protocol::Diagnostic),
    /// Everything this slice checks passed, and there is nothing further yet.
    NotImplemented,
}

impl std::fmt::Display for BrokerExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerExit::Refused(diagnostic) => {
                write!(formatter, "owner-effect broker: {diagnostic}")
            }
            BrokerExit::NotImplemented => formatter.write_str(
                "owner-effect broker: not implemented — bootstrap and root accepted, no store is owned yet",
            ),
        }
    }
}

impl std::error::Error for BrokerExit {}
