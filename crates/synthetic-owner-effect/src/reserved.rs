//! Privately constructible handle proving one reservation committed.
//!
//! Only [`crate::reserve_exact_authority`] may construct this type. Fields and
//! the constructor are private. There is no reconstruction accessor and no
//! `Clone`, `Serialize`, or `Debug` implementation.
//!
//! **Maturity:** Developer Preview fixture. Design Accept ≠ product Current.

use crate::sealed::EffectIntentId;

/// Proof that every reservation fact for one intent committed together.
///
/// D06 consumes this by value. Nothing else can reach the writer.
pub struct AuthorityReservedEffect {
    intent_id: EffectIntentId,
}

impl AuthorityReservedEffect {
    pub(crate) fn from_committed(intent_id: EffectIntentId) -> Self {
        Self { intent_id }
    }

    pub(crate) fn intent_id(&self) -> EffectIntentId {
        self.intent_id
    }
}

#[cfg(test)]
mod proof_invariants {
    use super::AuthorityReservedEffect;

    static_assertions::assert_not_impl_any!(
        AuthorityReservedEffect: Clone,
        std::fmt::Debug,
        serde::Serialize
    );
}
