use bombay_transition::{Base, Reducer, Topology, Transition};

use super::{EntitySlot, SlotEffectBatch, SlotEvent, SlotReducer};

const TRANSITIONS: &[Transition] = &[
    Transition {
        from: "inactive",
        input: "claim_activation",
        to: "activating",
    },
    Transition {
        from: "activating",
        input: "activation_succeeded",
        to: "active",
    },
    Transition {
        from: "activating",
        input: "activation_failed",
        to: "inactive",
    },
    Transition {
        from: "active",
        input: "begin_drain",
        to: "draining",
    },
    Transition {
        from: "draining",
        input: "fence_acknowledged",
        to: "retiring",
    },
    Transition {
        from: "draining",
        input: "force_drain",
        to: "retiring",
    },
    Transition {
        from: "retiring",
        input: "terminated",
        to: "inactive",
    },
];

const TOPOLOGY: Topology = Topology {
    name: "entity_lifecycle",
    transitions: TRANSITIONS,
};

type TransitionFn<C, E, L> =
    fn(EntitySlot<C, E, L>, SlotEvent<C, E, L>) -> (SlotEffectBatch<C, E, L>, EntitySlot<C, E, L>);

/// Representable executable lifecycle machine for one stable entity slot.
pub type LifecycleMachine<C, E, L> = Base<EntitySlot<C, E, L>, TransitionFn<C, E, L>>;

/// Construct an inactive entity lifecycle machine.
#[must_use]
pub fn lifecycle_machine<C, E: Clone, L>() -> LifecycleMachine<C, E, L> {
    Base::new(EntitySlot::Inactive, TOPOLOGY, transition::<C, E, L>)
}

fn transition<C, E: Clone, L>(
    state: EntitySlot<C, E, L>,
    event: SlotEvent<C, E, L>,
) -> (SlotEffectBatch<C, E, L>, EntitySlot<C, E, L>) {
    let decision = SlotReducer.reduce(state, event);
    (decision.effects, decision.state)
}

#[cfg(test)]
mod tests {
    use core::num::{NonZeroU64, NonZeroUsize};

    use bombay_transition::{Machine, Structure, Topology};

    use super::lifecycle_machine;
    use crate::{ActivationId, DispatchId, SlotEffect, SlotEvent};

    struct TopologyOnly;

    impl Structure for TopologyOnly {
        type Output = Topology;

        fn base(&mut self, topology: Topology) -> Self::Output {
            topology
        }

        fn then(&mut self, _first: Self::Output, _second: Self::Output) -> Self::Output {
            panic!("entity lifecycle is one base machine")
        }

        fn product(&mut self, _left: Self::Output, _right: Self::Output) -> Self::Output {
            panic!("entity lifecycle is one base machine")
        }

        fn choice(&mut self, _left: Self::Output, _right: Self::Output) -> Self::Output {
            panic!("entity lifecycle is one base machine")
        }
    }

    #[test]
    fn execution_and_topology_come_from_the_same_machine() {
        let machine = lifecycle_machine::<u8, u8, u8>();
        let topology = machine.describe(&mut TopologyOnly);
        assert_eq!(topology.name, "entity_lifecycle");
        assert_eq!(topology.transitions.len(), 7);

        let (effects, successor) = machine.step(SlotEvent::ClaimActivation {
            activation_id: ActivationId::new(NonZeroU64::MIN),
            dispatch_id: DispatchId(1),
            command: 9,
            waiter_limit: NonZeroUsize::MIN,
        });

        assert!(matches!(
            effects.as_slice(),
            [SlotEffect::StartActivation { .. }]
        ));
        assert_eq!(
            successor.describe(&mut TopologyOnly),
            topology,
            "execution must preserve its structural representation"
        );
    }
}
