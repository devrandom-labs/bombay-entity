//! Minimal algebra for deterministic, effect-describing state machines.
//!
//! A reducer is the total function `State × Event → State × Effects`. It does
//! not execute effects, choose a transport, own locks, or prescribe storage.
//! Those omissions are the useful boundary: interpreters may be concurrent,
//! asynchronous, simulated, or model checked without changing the reducer.
//!
//! Entity lifecycle management is one application. Actor supervision,
//! protocol sessions, retry policies, and resource ownership can use the same
//! kernel when their decisions are deterministic and their effects can be
//! represented as data. It is not appropriate for algorithms whose state is
//! intrinsically external or whose correctness depends on hidden I/O.

#![no_std]
#![deny(missing_docs)]

/// The value produced by one deterministic reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision<S, F> {
    /// State to install after the reduction.
    pub state: S,
    /// Description of effects for a separate interpreter.
    pub effects: F,
}

impl<S, F> Decision<S, F> {
    /// Pair the next state with its effect description.
    #[must_use]
    pub const fn new(state: S, effects: F) -> Self {
        Self { state, effects }
    }

    /// Transform only the effect description.
    #[must_use]
    pub fn map_effects<G>(self, map: impl FnOnce(F) -> G) -> Decision<S, G> {
        Decision::new(self.state, map(self.effects))
    }

    /// Transform only the next state.
    #[must_use]
    pub fn map_state<T>(self, map: impl FnOnce(S) -> T) -> Decision<T, F> {
        Decision::new(map(self.state), self.effects)
    }

    /// Transform the next state and effect description independently.
    #[must_use]
    pub fn map<T, G>(
        self,
        state: impl FnOnce(S) -> T,
        effects: impl FnOnce(F) -> G,
    ) -> Decision<T, G> {
        Decision::new(state(self.state), effects(self.effects))
    }
}

/// A deterministic transition algebra.
pub trait Reducer<S, E> {
    /// Effect description returned to an interpreter.
    type Effects;

    /// Reduce one state and event into the next state and effects.
    fn reduce(&self, state: S, event: E) -> Decision<S, Self::Effects>;

    /// Fold an ordered event stream through the reducer.
    ///
    /// Effects retain event order through their associative operation. Empty
    /// input returns the initial state and the effect identity.
    fn fold<I>(&self, initial: S, events: I) -> Decision<S, Self::Effects>
    where
        I: IntoIterator<Item = E>,
        Self::Effects: Default + core::ops::Add<Output = Self::Effects>,
    {
        events.into_iter().fold(
            Decision::new(initial, Self::Effects::default()),
            |accumulator, event| {
                self.reduce(accumulator.state, event)
                    .map_effects(|effects| accumulator.effects + effects)
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Add;

    use super::{Decision, Reducer};

    #[derive(Default, PartialEq, Eq, Debug)]
    struct Effects(u32);

    impl Add for Effects {
        type Output = Self;

        fn add(self, other: Self) -> Self {
            Self(self.0 * 10 + other.0)
        }
    }

    struct Sum;

    impl Reducer<u32, u32> for Sum {
        type Effects = Effects;

        fn reduce(&self, state: u32, event: u32) -> Decision<u32, Self::Effects> {
            Decision::new(state + event, Effects(event))
        }
    }

    #[test]
    fn fold_preserves_state_and_effect_order() {
        assert_eq!(Sum.fold(1, [2, 4, 8]), Decision::new(15, Effects(248)));
    }
}
