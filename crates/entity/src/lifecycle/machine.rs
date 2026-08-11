use std::sync::OnceLock;

use bombay_transition::{
    Base, Reducer, Topology, TopologyError, Transition, TriggerId, ValidatedTopology, Vertex,
    VertexId,
};

use super::{EntitySlot, SlotEffectBatch, SlotEvent, SlotReducer};

/// Stable lifecycle phase identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LifecyclePhase {
    /// No incarnation exists.
    Inactive,
    /// One activation is in flight.
    Activating,
    /// An incarnation admits commands.
    Active,
    /// Admission is closed while processing drains.
    Draining,
    /// Exact termination is awaited.
    Retiring,
}

impl LifecyclePhase {
    const fn id(self) -> VertexId {
        VertexId(self as u8)
    }
}

/// Stable identity of an input relevant to lifecycle topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LifecycleTrigger {
    /// Claim a fresh activation.
    ClaimActivation,
    /// Dispatch a command.
    Dispatch,
    /// Cancel an activation waiter.
    CancelWaiter,
    /// Activation committed.
    ActivationSucceeded,
    /// Activation failed.
    ActivationFailed,
    /// A reserved delivery resolved.
    DeliveryResolved,
    /// Close admission.
    BeginDrain,
    /// Processing fence acknowledged.
    FenceAcknowledged,
    /// Force bounded draining to finish.
    ForceDrain,
    /// Exact incarnation terminated.
    Terminated,
}

impl LifecycleTrigger {
    const fn id(self) -> TriggerId {
        TriggerId(self as u8)
    }
}

/// Authoritative identity of one phase-changing lifecycle edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleEdge {
    /// Inactive to activating.
    ClaimActivation,
    /// Activating to active.
    ActivationSucceeded,
    /// Activating to inactive.
    ActivationFailed,
    /// Active to draining.
    BeginDrain,
    /// Draining to retiring after the fence.
    FenceAcknowledged,
    /// Draining to retiring after a bounded failure.
    ForceDrain,
    /// Retiring to inactive after exact termination.
    Terminated,
}

impl LifecycleEdge {
    /// Every declared lifecycle edge in deterministic rendering order.
    pub const ALL: [Self; 7] = [
        Self::ClaimActivation,
        Self::ActivationSucceeded,
        Self::ActivationFailed,
        Self::BeginDrain,
        Self::FenceAcknowledged,
        Self::ForceDrain,
        Self::Terminated,
    ];

    /// Return the strongly typed source, trigger, and destination.
    #[must_use]
    pub const fn endpoints(self) -> (LifecyclePhase, LifecycleTrigger, LifecyclePhase) {
        match self {
            Self::ClaimActivation => (
                LifecyclePhase::Inactive,
                LifecycleTrigger::ClaimActivation,
                LifecyclePhase::Activating,
            ),
            Self::ActivationSucceeded => (
                LifecyclePhase::Activating,
                LifecycleTrigger::ActivationSucceeded,
                LifecyclePhase::Active,
            ),
            Self::ActivationFailed => (
                LifecyclePhase::Activating,
                LifecycleTrigger::ActivationFailed,
                LifecyclePhase::Inactive,
            ),
            Self::BeginDrain => (
                LifecyclePhase::Active,
                LifecycleTrigger::BeginDrain,
                LifecyclePhase::Draining,
            ),
            Self::FenceAcknowledged => (
                LifecyclePhase::Draining,
                LifecycleTrigger::FenceAcknowledged,
                LifecyclePhase::Retiring,
            ),
            Self::ForceDrain => (
                LifecyclePhase::Draining,
                LifecycleTrigger::ForceDrain,
                LifecyclePhase::Retiring,
            ),
            Self::Terminated => (
                LifecyclePhase::Retiring,
                LifecycleTrigger::Terminated,
                LifecyclePhase::Inactive,
            ),
        }
    }

    const fn transition(self) -> Transition {
        let (from, trigger, to) = self.endpoints();
        Transition {
            from: from.id(),
            trigger: trigger.id(),
            to: to.id(),
            label: trigger_label(trigger),
        }
    }
}

/// Evidence produced by one lifecycle execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionEvidence {
    /// Execution traversed this exact declared edge.
    Traversed(LifecycleEdge),
    /// Input was handled without changing phase.
    SelfLoop {
        /// Phase retained by the execution.
        phase: LifecyclePhase,
        /// Handled input identity.
        trigger: LifecycleTrigger,
    },
    /// Input was irrelevant, invalid for the phase, or stale.
    Ignored {
        /// Phase retained by the execution.
        phase: LifecyclePhase,
        /// Ignored input identity.
        trigger: LifecycleTrigger,
    },
}

/// Output of one executable lifecycle-machine step.
#[derive(Debug)]
pub struct LifecycleOutput<C, E, L> {
    /// Ordered effects for the runtime interpreter.
    pub effects: SlotEffectBatch<C, E, L>,
    /// Checked phase-transition evidence.
    pub evidence: TransitionEvidence,
    pub(crate) activation_id: Option<super::ActivationId>,
}

const VERTICES: &[Vertex] = &[
    Vertex {
        id: LifecyclePhase::Inactive.id(),
        label: "inactive",
    },
    Vertex {
        id: LifecyclePhase::Activating.id(),
        label: "activating",
    },
    Vertex {
        id: LifecyclePhase::Active.id(),
        label: "active",
    },
    Vertex {
        id: LifecyclePhase::Draining.id(),
        label: "draining",
    },
    Vertex {
        id: LifecyclePhase::Retiring.id(),
        label: "retiring",
    },
];

const TRANSITIONS: &[Transition] = &[
    LifecycleEdge::ClaimActivation.transition(),
    LifecycleEdge::ActivationSucceeded.transition(),
    LifecycleEdge::ActivationFailed.transition(),
    LifecycleEdge::BeginDrain.transition(),
    LifecycleEdge::FenceAcknowledged.transition(),
    LifecycleEdge::ForceDrain.transition(),
    LifecycleEdge::Terminated.transition(),
];

/// Authoritative lifecycle topology.
pub const LIFECYCLE_TOPOLOGY: Topology = Topology {
    name: "entity_lifecycle",
    initial: LifecyclePhase::Inactive.id(),
    vertices: VERTICES,
    transitions: TRANSITIONS,
};

/// Property-test model obtained by structurally interpreting the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleModel(Topology);

impl LifecycleModel {
    /// Borrow the vertices in deterministic declaration order.
    #[must_use]
    pub const fn vertices(self) -> &'static [Vertex] {
        self.0.vertices
    }

    /// Borrow the edges in deterministic declaration order.
    #[must_use]
    pub const fn transitions(self) -> &'static [Transition] {
        self.0.transitions
    }

    /// Test whether exact typed edge evidence is declared by this model.
    #[must_use]
    pub fn contains(self, edge: LifecycleEdge) -> bool {
        self.0.transitions.contains(&edge.transition())
    }

    /// Render deterministic Mermaid into any formatting sink.
    ///
    /// # Errors
    ///
    /// Returns an error if the formatting sink fails or the model is invalid.
    pub fn write_mermaid(self, output: &mut impl core::fmt::Write) -> core::fmt::Result {
        self.0.write_mermaid(output)
    }
}

/// Return the authoritative lifecycle model used by execution and tests.
#[must_use]
pub const fn lifecycle_model() -> LifecycleModel {
    LifecycleModel(LIFECYCLE_TOPOLOGY)
}

/// Lifecycle-specific structural validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleTopologyError {
    /// Generic topology validation failed.
    Structure(TopologyError),
    /// One required lifecycle edge is absent.
    Missing(LifecycleEdge),
    /// Retirement could reopen admission.
    RetiringToActive,
}

/// Validate graph properties and required entity lifecycle invariants.
///
/// # Errors
///
/// Returns the first generic structural or lifecycle-specific defect.
pub fn validate_lifecycle_topology(topology: Topology) -> Result<(), LifecycleTopologyError> {
    topology
        .validate()
        .map_err(LifecycleTopologyError::Structure)?;
    for required in LifecycleEdge::ALL {
        if !topology.transitions.contains(&required.transition()) {
            return Err(LifecycleTopologyError::Missing(required));
        }
    }
    if topology.transitions.iter().any(|edge| {
        edge.from == LifecyclePhase::Retiring.id() && edge.to == LifecyclePhase::Active.id()
    }) {
        return Err(LifecycleTopologyError::RetiringToActive);
    }
    Ok(())
}

type TransitionFn<C, E, L> =
    fn(EntitySlot<C, E, L>, SlotEvent<C, E, L>) -> (LifecycleOutput<C, E, L>, EntitySlot<C, E, L>);

/// Representable executable lifecycle machine for one stable entity slot.
pub type LifecycleMachine<C, E, L> =
    Base<EntitySlot<C, E, L>, TransitionFn<C, E, L>, SlotEvent<C, E, L>, LifecycleOutput<C, E, L>>;

/// Construct an inactive entity lifecycle machine.
///
/// # Panics
///
/// Panics if the crate's statically declared lifecycle topology is invalid.
#[must_use]
pub fn lifecycle_machine<C, E: Clone, L>() -> LifecycleMachine<C, E, L> {
    static VALIDATED: OnceLock<ValidatedTopology> = OnceLock::new();
    Base::new(
        EntitySlot::Inactive,
        *VALIDATED.get_or_init(|| {
            LIFECYCLE_TOPOLOGY
                .validated()
                .expect("lifecycle topology is valid")
        }),
        transition::<C, E, L>,
    )
}

fn transition<C, E: Clone, L>(
    state: EntitySlot<C, E, L>,
    event: SlotEvent<C, E, L>,
) -> (LifecycleOutput<C, E, L>, EntitySlot<C, E, L>) {
    let from = state.phase();
    let trigger = event.trigger();
    let handled = state.handles(&event);
    let decision = SlotReducer.reduce(state, event);
    let to = decision.state.phase();
    let evidence = if from == to {
        if handled {
            TransitionEvidence::SelfLoop {
                phase: from,
                trigger,
            }
        } else {
            TransitionEvidence::Ignored {
                phase: from,
                trigger,
            }
        }
    } else {
        let edge = LifecycleEdge::ALL
            .into_iter()
            .find(|edge| edge.endpoints() == (from, trigger, to))
            .expect("every executable phase change must be declared");
        TransitionEvidence::Traversed(edge)
    };
    (
        LifecycleOutput {
            effects: decision.effects,
            evidence,
            activation_id: decision.state.activation_id(),
        },
        decision.state,
    )
}

const fn trigger_label(trigger: LifecycleTrigger) -> &'static str {
    match trigger {
        LifecycleTrigger::ClaimActivation => "claim_activation",
        LifecycleTrigger::Dispatch => "dispatch",
        LifecycleTrigger::CancelWaiter => "cancel_waiter",
        LifecycleTrigger::ActivationSucceeded => "activation_succeeded",
        LifecycleTrigger::ActivationFailed => "activation_failed",
        LifecycleTrigger::DeliveryResolved => "delivery_resolved",
        LifecycleTrigger::BeginDrain => "begin_drain",
        LifecycleTrigger::FenceAcknowledged => "fence_acknowledged",
        LifecycleTrigger::ForceDrain => "force_drain",
        LifecycleTrigger::Terminated => "terminated",
    }
}

#[cfg(test)]
mod tests {
    use core::num::{NonZeroU64, NonZeroUsize};

    use bombay_transition::Machine;

    use super::*;
    use crate::{ActivationId, DispatchId, DrainFailure, DrainStage, SlotEvent};

    fn activation(value: u64) -> ActivationId {
        ActivationId::new(NonZeroU64::new(value).unwrap())
    }

    #[test]
    fn topology_is_valid_and_renders_deterministically() {
        validate_lifecycle_topology(LIFECYCLE_TOPOLOGY).unwrap();
        let model = lifecycle_model();
        let mut mermaid = String::new();
        model.write_mermaid(&mut mermaid).unwrap();
        assert_eq!(model.vertices().len(), 5);
        assert_eq!(model.transitions().len(), 7);
        assert_eq!(
            mermaid,
            "stateDiagram-v2\n    [*] --> inactive\n    inactive --> activating: claim_activation\n    activating --> active: activation_succeeded\n    activating --> inactive: activation_failed\n    active --> draining: begin_drain\n    draining --> retiring: fence_acknowledged\n    draining --> retiring: force_drain\n    retiring --> inactive: terminated\n"
        );
    }

    #[test]
    fn every_observed_phase_change_carries_declared_edge_evidence() {
        let model = lifecycle_model();
        let machine = lifecycle_machine::<u8, u8, u8>();
        let (claim, machine) = machine.step(SlotEvent::ClaimActivation {
            activation_id: activation(1),
            dispatch_id: DispatchId(1),
            command: 1,
            waiter_limit: NonZeroUsize::MIN,
        });
        assert_eq!(
            claim.evidence,
            TransitionEvidence::Traversed(LifecycleEdge::ClaimActivation)
        );
        let (activated, machine) = machine.step(SlotEvent::ActivationSucceeded {
            activation_id: activation(1),
            endpoint: 2,
            lease: 3,
        });
        assert_eq!(
            activated.evidence,
            TransitionEvidence::Traversed(LifecycleEdge::ActivationSucceeded)
        );
        let (_, machine) = machine.step(SlotEvent::DeliveryResolved {
            activation_id: activation(1),
            failure: None,
        });
        let (drain, machine) = machine.step(SlotEvent::BeginDrain {
            activation_id: activation(1),
        });
        let (fence, machine) = machine.step(SlotEvent::FenceAcknowledged {
            activation_id: activation(1),
        });
        let (terminated, _machine) = machine.step(SlotEvent::Terminated {
            activation_id: activation(1),
        });
        for evidence in [
            claim.evidence,
            activated.evidence,
            drain.evidence,
            fence.evidence,
            terminated.evidence,
        ] {
            let TransitionEvidence::Traversed(edge) = evidence else {
                panic!("phase change lacked edge evidence")
            };
            assert!(model.contains(edge));
        }
        assert_eq!(lifecycle_model(), model);
    }

    #[test]
    fn stale_and_nonchanging_inputs_are_honestly_classified() {
        let machine = lifecycle_machine::<u8, u8, u8>();
        let (dispatch, machine) = machine.step(SlotEvent::Dispatch {
            dispatch_id: DispatchId(1),
            command: 1,
        });
        assert_eq!(
            dispatch.evidence,
            TransitionEvidence::SelfLoop {
                phase: LifecyclePhase::Inactive,
                trigger: LifecycleTrigger::Dispatch
            }
        );
        let (ignored, _) = machine.step(SlotEvent::Terminated {
            activation_id: activation(9),
        });
        assert_eq!(
            ignored.evidence,
            TransitionEvidence::Ignored {
                phase: LifecyclePhase::Inactive,
                trigger: LifecycleTrigger::Terminated
            }
        );
    }

    #[test]
    fn forced_drain_has_its_distinct_declared_edge() {
        let machine = lifecycle_machine::<u8, u8, u8>();
        let (_, machine) = machine.step(SlotEvent::ClaimActivation {
            activation_id: activation(1),
            dispatch_id: DispatchId(1),
            command: 1,
            waiter_limit: NonZeroUsize::MIN,
        });
        let (_, machine) = machine.step(SlotEvent::ActivationSucceeded {
            activation_id: activation(1),
            endpoint: 2,
            lease: 3,
        });
        let (_, machine) = machine.step(SlotEvent::DeliveryResolved {
            activation_id: activation(1),
            failure: None,
        });
        let (_, machine) = machine.step(SlotEvent::BeginDrain {
            activation_id: activation(1),
        });
        let (forced, _) = machine.step(SlotEvent::ForceDrain {
            activation_id: activation(1),
            failure: DrainFailure {
                stage: DrainStage::FenceAcknowledgement,
                outstanding_reservations: 0,
            },
        });
        assert_eq!(
            forced.evidence,
            TransitionEvidence::Traversed(LifecycleEdge::ForceDrain)
        );
    }

    #[derive(Clone, Copy)]
    enum Input {
        ClaimCurrent,
        Dispatch,
        ActivationCurrent,
        ActivationStale,
        ActivationFailure,
        DeliveryCurrent,
        BeginDrainCurrent,
        BeginDrainStale,
        FenceCurrent,
        ForceCurrent,
        TerminatedCurrent,
        TerminatedStale,
    }

    const INPUTS: [Input; 12] = [
        Input::ClaimCurrent,
        Input::Dispatch,
        Input::ActivationCurrent,
        Input::ActivationStale,
        Input::ActivationFailure,
        Input::DeliveryCurrent,
        Input::BeginDrainCurrent,
        Input::BeginDrainStale,
        Input::FenceCurrent,
        Input::ForceCurrent,
        Input::TerminatedCurrent,
        Input::TerminatedStale,
    ];

    fn event(input: Input) -> SlotEvent<u8, u8, u8> {
        match input {
            Input::ClaimCurrent => SlotEvent::ClaimActivation {
                activation_id: activation(1),
                dispatch_id: DispatchId(1),
                command: 1,
                waiter_limit: NonZeroUsize::new(2).unwrap(),
            },
            Input::Dispatch => SlotEvent::Dispatch {
                dispatch_id: DispatchId(2),
                command: 2,
            },
            Input::ActivationCurrent => SlotEvent::ActivationSucceeded {
                activation_id: activation(1),
                endpoint: 1,
                lease: 1,
            },
            Input::ActivationStale => SlotEvent::ActivationSucceeded {
                activation_id: activation(2),
                endpoint: 2,
                lease: 2,
            },
            Input::ActivationFailure => SlotEvent::ActivationFailed {
                activation_id: activation(1),
            },
            Input::DeliveryCurrent => SlotEvent::DeliveryResolved {
                activation_id: activation(1),
                failure: None,
            },
            Input::BeginDrainCurrent => SlotEvent::BeginDrain {
                activation_id: activation(1),
            },
            Input::BeginDrainStale => SlotEvent::BeginDrain {
                activation_id: activation(2),
            },
            Input::FenceCurrent => SlotEvent::FenceAcknowledged {
                activation_id: activation(1),
            },
            Input::ForceCurrent => SlotEvent::ForceDrain {
                activation_id: activation(1),
                failure: DrainFailure {
                    stage: DrainStage::FenceAcknowledgement,
                    outstanding_reservations: 0,
                },
            },
            Input::TerminatedCurrent => SlotEvent::Terminated {
                activation_id: activation(1),
            },
            Input::TerminatedStale => SlotEvent::Terminated {
                activation_id: activation(2),
            },
        }
    }

    fn check_trace(trace: &[Input], model: LifecycleModel) {
        let mut machine = lifecycle_machine::<u8, u8, u8>();
        for input in trace {
            let before = lifecycle_model();
            let (output, successor) = machine.step(event(*input));
            if let TransitionEvidence::Traversed(edge) = output.evidence {
                assert!(model.contains(edge));
            }
            assert_eq!(lifecycle_model(), before);
            machine = successor;
        }
    }

    fn enumerate(prefix: &mut Vec<Input>, remaining: usize, model: LifecycleModel) {
        check_trace(prefix, model);
        if remaining == 0 {
            return;
        }
        for input in INPUTS {
            prefix.push(input);
            enumerate(prefix, remaining - 1, model);
            prefix.pop();
        }
    }

    #[test]
    fn bounded_event_traces_preserve_topology_evidence_and_structure() {
        let model = lifecycle_model();
        enumerate(&mut Vec::new(), 4, model);
    }
}
