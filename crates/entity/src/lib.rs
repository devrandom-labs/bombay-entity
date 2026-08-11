//! Local, in-memory activation and stable routing for entities.
//!
//! An entity is addressed by a stable [`EntityId`], while its actor incarnation
//! may be activated, drained, passivated, and later replaced. This crate owns
//! that lifecycle. Actor execution, business behavior, persistence, discovery,
//! and distributed consensus remain separate concerns.
//!
//! The initial runtime will implement this state machine:
//!
//! ```text
//! Inactive
//!   -> Activating(generation)
//!   -> Active(generation, actor reference)
//!   -> Draining(generation)
//!   -> Inactive
//! ```
//!
//! Generation checks prevent late activation or termination events from an old
//! incarnation from changing the binding for a newer incarnation.

#![deny(missing_docs)]

use core::fmt;
use core::hash::Hash;

mod lifecycle;
mod protocol;

pub use bombay_transition::{Decision, Reducer};
pub use lifecycle::{
    ActivatingSlot, ActivationId, ActivationWaiter, ActiveSlot, DispatchId, DrainFailure,
    DrainProgress, DrainStage, DrainingSlot, EntitySlot, Refusal, ReservationCount, RetirementMode,
    SlotDecision, SlotEffect, SlotEffectBatch, SlotEvent, SlotReducer,
};
pub use protocol::{DrainFenceAcknowledged, EntityBehavior, EntityEvent, EntityProtocol};

/// A stable, typed identifier for an entity.
///
/// The identifier names the logical entity rather than any particular actor
/// incarnation. The inner value determines equality, ordering, and hashing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId<T>(T);

impl<T> EntityId<T> {
    /// Wrap a domain identifier as an entity identifier.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the domain identifier.
    pub const fn get(&self) -> &T {
        &self.0
    }

    /// Return the wrapped domain identifier.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for EntityId<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("EntityId").field(&self.0).finish()
    }
}

impl<T: fmt::Display> fmt::Display for EntityId<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::EntityId;

    #[test]
    fn entity_id_preserves_domain_identity() {
        let id = EntityId::new(42_u64);

        assert_eq!(id.get(), &42);
        assert_eq!(id.to_string(), "42");
        assert_eq!(id.into_inner(), 42);
    }
}
