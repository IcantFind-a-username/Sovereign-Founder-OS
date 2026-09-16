//! Closed outcomes and release-excluded publish failpoints.

use std::cell::Cell;

use sovereign_authority::broker::store::StoreError;

thread_local! {
    static INJECT: Cell<u8> = const { Cell::new(0) };
}

/// In-process failpoints at every state and filesystem boundary.
/// Per-thread so parallel cargo tests cannot abort a sibling dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PublishFailpoint {
    BeforeDispatchingCommit = 1,
    AfterDispatchingCommit = 2,
    AfterFirstWrite = 3,
    AfterTempFlush = 4,
    BeforePublication = 5,
    AfterPublicationBeforeDirFlush = 6,
}

impl PublishFailpoint {
    pub const ALL: &'static [Self] = &[
        Self::BeforeDispatchingCommit,
        Self::AfterDispatchingCommit,
        Self::AfterFirstWrite,
        Self::AfterTempFlush,
        Self::BeforePublication,
        Self::AfterPublicationBeforeDirFlush,
    ];

    pub const FILESYSTEM: &'static [Self] = &[
        Self::AfterFirstWrite,
        Self::AfterTempFlush,
        Self::BeforePublication,
        Self::AfterPublicationBeforeDirFlush,
    ];
}

pub fn with_publish_failpoint<R>(stage: PublishFailpoint, body: impl FnOnce() -> R) -> R {
    INJECT.with(|cell| cell.set(stage as u8));
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            INJECT.with(|cell| cell.set(0));
        }
    }
    let _reset = Reset;
    body()
}

pub(crate) fn hit_publish(stage: PublishFailpoint) -> Result<(), PublishError> {
    if INJECT.with(Cell::get) == stage as u8 {
        return Err(PublishError::Failpoint(stage));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClosedOutcome {
    Succeeded,
    FailedBeforeDispatch,
    Indeterminate,
}

impl ClosedOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::FailedBeforeDispatch => "failed_before_dispatch",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishError {
    Unavailable,
    UnknownIntent,
    NotReserved,
    AlreadyTerminal,
    EpochMismatch,
    SessionMismatch,
    LogoutMismatch,
    GenerationMismatch,
    GuestImport,
    OutputRejected,
    GuestUnavailable,
    WriterIoObserved,
    AlreadyPublished,
    Failpoint(PublishFailpoint),
}

impl PublishError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unavailable => "E-PUBLISH-UNAVAILABLE",
            Self::UnknownIntent => "E-UNKNOWN-INTENT",
            Self::NotReserved => "E-NOT-RESERVED",
            Self::AlreadyTerminal => "E-ALREADY-TERMINAL",
            Self::EpochMismatch => "E-EPOCH-MISMATCH",
            Self::SessionMismatch => "E-SESSION-MISMATCH",
            Self::LogoutMismatch => "E-LOGOUT-MISMATCH",
            Self::GenerationMismatch => "E-GENERATION-MISMATCH",
            Self::GuestImport => "E-GUEST-IMPORT",
            Self::OutputRejected => "E-OUTPUT-REJECTED",
            Self::GuestUnavailable => "E-GUEST-UNAVAILABLE",
            Self::WriterIoObserved => "E-WRITER-IO-OBSERVED",
            Self::AlreadyPublished => "E-ALREADY-PUBLISHED",
            Self::Failpoint(_) => "E-PUBLISH-FAILPOINT",
        }
    }
}

impl From<StoreError> for PublishError {
    fn from(_: StoreError) -> Self {
        Self::Unavailable
    }
}
