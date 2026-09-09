//! What the fixture ceremony produces, and what it deliberately is not.

use uuid::Uuid;

/// How much a bootstrap is worth, and there is no third value.
///
/// Both variants are unqualified. The distinction is only which *kind* of
/// unqualified — a virtual authenticator measures the protocol, a real one
/// measures a mechanism on one platform — and neither is admission. There is
/// no `Product` variant, and adding one would be a change to this enum that a
/// reviewer would see rather than a flag someone could set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Qualification {
    /// A virtual authenticator: the protocol was exercised, nothing else.
    ProtocolFixtureOnly,
    /// A real authenticator on one recorded platform, and only that platform.
    MechanismQualifiedOnly,
}

impl Qualification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Qualification::ProtocolFixtureOnly => "protocol_fixture_only",
            Qualification::MechanismQualifiedOnly => "mechanism_qualified_only",
        }
    }
}

/// The outcome of a completed fixture ceremony.
///
/// Its name is the claim. Nothing converts this into a product admission,
/// because no such type exists — and it cannot be built outside this crate,
/// so a caller cannot manufacture one to hand to something that trusts it.
///
/// The winner of an enrolment is called exactly that. Not "the owner", not
/// "the founder": on an empty registry a hostile same-account process can be
/// the winner, and a name that implied otherwise would be the lie the whole
/// boundary exists to prevent.
#[derive(Debug, Clone)]
pub struct FixtureBootstrap {
    winner: Uuid,
    qualification: Qualification,
    // Private and unconstructible from outside: the only way to hold one of
    // these is to have been given it by this crate.
    _sealed: Sealed,
}

#[derive(Debug, Clone, Copy)]
struct Sealed;

impl FixtureBootstrap {
    /// Only this crate mints one, and only after a ceremony it ran itself.
    pub(crate) fn new(winner: Uuid, qualification: Qualification) -> Self {
        Self {
            winner,
            qualification,
            _sealed: Sealed,
        }
    }

    /// The handle that won the enrolment. Deliberately not named "owner".
    pub fn enrolment_winner(&self) -> Uuid {
        self.winner
    }

    pub fn qualification(&self) -> Qualification {
        self.qualification
    }

    /// One sentence a report can print without overstating what happened.
    pub fn honest_summary(&self) -> String {
        format!(
            "fixture enrolment winner {} ({}); not owner admission — a same-account process can win an empty registry",
            self.winner,
            self.qualification.as_str()
        )
    }
}
