//! Concurrent storage for local entity lifecycle machines.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::{BuildHasher, Hash, RandomState};
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bombay_transition::Machine;

use crate::{
    ActivationId, DispatchId, DrainFailure, EntityId, LifecycleMachine, LifecycleOutput, Refusal,
    RetirementMode, SlotEffect, SlotEvent, TransitionEvidence, lifecycle_machine,
};

/// Fixed sizing and admission limits for a local directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectoryConfig {
    /// Number of independently locked map shards; must be a power of two.
    pub shards: NonZeroUsize,
    /// Maximum commands retained by one in-flight activation.
    pub activation_waiters: NonZeroUsize,
}

impl Default for DirectoryConfig {
    fn default() -> Self {
        Self {
            shards: NonZeroUsize::new(64).expect("64 is non-zero"),
            activation_waiters: NonZeroUsize::new(64).expect("64 is non-zero"),
        }
    }
}

/// Failure before a command can be submitted to a lifecycle machine.
#[derive(Debug)]
pub enum DirectoryError<C> {
    /// The shard count was not a power of two.
    InvalidShardCount,
    /// The monotonically increasing activation namespace is exhausted.
    ActivationIdsExhausted(C),
    /// The monotonically increasing dispatch namespace is exhausted.
    DispatchIdsExhausted(C),
}

/// Runtime capabilities used to interpret lifecycle effects.
///
/// Methods run without directory or slot synchronization held. Asynchronous
/// implementations should schedule owned work and later submit its typed fact
/// through the directory. Calls for one slot occur in declared effect order.
pub trait EffectInterpreter<I, C, E, L> {
    /// Start one directory-owned activation attempt.
    fn start_activation(&self, entity_id: EntityId<I>, activation_id: ActivationId);
    /// Start delivery to one exact-incarnation endpoint.
    fn deliver(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
        dispatch_id: DispatchId,
        endpoint: E,
        command: C,
    );
    /// Return a command that was not admitted or delivered.
    fn reject(&self, dispatch_id: DispatchId, command: C, reason: Refusal);
    /// Enqueue an ordered processing fence for one exact incarnation.
    fn enqueue_fence(&self, entity_id: EntityId<I>, activation_id: ActivationId, endpoint: E);
    /// Exercise exact-incarnation retirement authority.
    fn retire(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
        lease: L,
        retirement: RetirementMode,
    );
}

/// One installed lifecycle decision awaiting effect interpretation.
pub struct DirectoryOutput<I, C, E, L> {
    /// Correlation identity allocated for a dispatched command, when applicable.
    pub dispatch_id: Option<DispatchId>,
    /// Checked evidence for the installed lifecycle decision.
    pub evidence: TransitionEvidence,
    entity_id: EntityId<I>,
    slot: Arc<Slot<C, E, L>>,
}

struct Slot<C, E, L> {
    state: Mutex<SlotState<C, E, L>>,
}

struct SlotState<C, E, L> {
    machine: Option<LifecycleMachine<C, E, L>>,
    effects: VecDeque<SlotEffect<C, E, L>>,
    interpreting: bool,
    removable_activation: Option<ActivationId>,
}

impl<C, E: Clone, L> Slot<C, E, L> {
    fn new(machine: LifecycleMachine<C, E, L>, output: LifecycleOutput<C, E, L>) -> Self {
        Self {
            state: Mutex::new(SlotState {
                machine: Some(machine),
                effects: output.effects.into_vec().into(),
                interpreting: false,
                removable_activation: None,
            }),
        }
    }

    fn submit(&self, event: SlotEvent<C, E, L>) -> TransitionEvidence {
        let mut state = self.state.lock().expect("slot lock poisoned");
        let machine = state.machine.take().expect("slot machine missing");
        let (output, successor) = machine.step(event);
        state.machine = Some(successor);
        state.effects.extend(output.effects.into_vec());
        output.evidence
    }

    fn begin_interpretation(&self) -> bool {
        let mut state = self.state.lock().expect("slot lock poisoned");
        if state.interpreting {
            false
        } else {
            state.interpreting = true;
            true
        }
    }

    fn next_effect(&self) -> Option<SlotEffect<C, E, L>> {
        let mut state = self.state.lock().expect("slot lock poisoned");
        if let Some(effect) = state.effects.pop_front() {
            Some(effect)
        } else {
            state.interpreting = false;
            None
        }
    }

    fn mark_removable(&self, activation_id: ActivationId) {
        self.state
            .lock()
            .expect("slot lock poisoned")
            .removable_activation = Some(activation_id);
    }

    fn removable_as(&self, activation_id: ActivationId) -> bool {
        self.state
            .lock()
            .expect("slot lock poisoned")
            .removable_activation
            == Some(activation_id)
    }
}

type Shard<I, C, E, L> = Mutex<HashMap<EntityId<I>, Arc<Slot<C, E, L>>>>;

/// Sharded local storage for authoritative per-entity lifecycle machines.
pub struct LocalDirectory<I, C, E, L, S = RandomState> {
    shards: Box<[Shard<I, C, E, L>]>,
    hash_builder: S,
    waiter_limit: NonZeroUsize,
    next_activation: AtomicU64,
    next_dispatch: AtomicU64,
}

impl<I, C, E, L> LocalDirectory<I, C, E, L>
where
    I: Eq + Hash + Clone,
    E: Clone,
{
    /// Construct a directory using the standard randomized hash builder.
    ///
    /// # Errors
    ///
    /// Returns [`DirectoryError::InvalidShardCount`] unless the configured
    /// shard count is a power of two.
    pub fn new(config: DirectoryConfig) -> Result<Self, DirectoryError<C>> {
        Self::with_hasher(config, RandomState::new())
    }
}

impl<I, C, E, L, S> LocalDirectory<I, C, E, L, S>
where
    I: Eq + Hash + Clone,
    E: Clone,
    S: BuildHasher,
{
    /// Construct a directory with an explicit hash builder.
    ///
    /// # Errors
    ///
    /// Returns [`DirectoryError::InvalidShardCount`] unless the configured
    /// shard count is a power of two.
    pub fn with_hasher(
        config: DirectoryConfig,
        hash_builder: S,
    ) -> Result<Self, DirectoryError<C>> {
        if !config.shards.get().is_power_of_two() {
            return Err(DirectoryError::InvalidShardCount);
        }
        let shards = (0..config.shards.get())
            .map(|_| Mutex::new(HashMap::new()))
            .collect();
        Ok(Self {
            shards,
            hash_builder,
            waiter_limit: config.activation_waiters,
            next_activation: AtomicU64::new(1),
            next_dispatch: AtomicU64::new(1),
        })
    }

    /// Dispatch a command through the stable slot for `entity_id`.
    ///
    /// The first caller atomically installs an activating machine. Concurrent
    /// callers use that same slot and bounded activation attempt.
    ///
    /// # Errors
    ///
    /// Returns the original command if either identity namespace is exhausted.
    ///
    /// # Panics
    ///
    /// Panics after a synchronization poison or an internal ownership invariant
    /// violation. Neither can be recovered without risking lifecycle corruption.
    pub fn dispatch(
        &self,
        entity_id: EntityId<I>,
        command: C,
    ) -> Result<DirectoryOutput<I, C, E, L>, DirectoryError<C>> {
        let mut command = Some(command);
        let dispatch_id = match allocate(&self.next_dispatch) {
            Some(value) => DispatchId(value.get()),
            None => {
                return Err(DirectoryError::DispatchIdsExhausted(
                    command.take().expect("command present"),
                ));
            }
        };
        let shard = &self.shards[self.shard_index(&entity_id)];
        let (slot, output) = {
            let mut entries = shard.lock().expect("directory shard lock poisoned");
            if let Some(slot) = entries.get(&entity_id) {
                (Arc::clone(slot), None)
            } else {
                let Some(sequence) = allocate(&self.next_activation) else {
                    return Err(DirectoryError::ActivationIdsExhausted(
                        command.take().expect("command present"),
                    ));
                };
                let activation_id = ActivationId::new(sequence);
                let machine = lifecycle_machine().step(SlotEvent::ClaimActivation {
                    activation_id,
                    dispatch_id,
                    command: command.take().expect("command present"),
                    waiter_limit: self.waiter_limit,
                });
                let evidence = machine.0.evidence;
                let slot = Arc::new(Slot::new(machine.1, machine.0));
                entries.insert(entity_id.clone(), Arc::clone(&slot));
                (slot, Some(evidence))
            }
        };
        let evidence = output.unwrap_or_else(|| {
            slot.submit(SlotEvent::Dispatch {
                dispatch_id,
                command: command.take().expect("command present"),
            })
        });
        Ok(directory_output(
            entity_id,
            Some(dispatch_id),
            evidence,
            slot,
        ))
    }

    /// Submit successful exact-incarnation activation to the represented slot.
    pub fn activation_succeeded(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        endpoint: E,
        lease: L,
    ) -> DirectoryOutput<I, C, E, L> {
        let event = SlotEvent::ActivationSucceeded {
            activation_id,
            endpoint,
            lease,
        };
        self.submit_or_inactive(entity_id, event)
    }

    /// Submit a failed activation attempt to the represented slot.
    pub fn activation_failed(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::ActivationFailed { activation_id })
    }

    /// Cancel one bounded activation waiter.
    pub fn cancel_waiter(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        dispatch_id: DispatchId,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(
            entity_id,
            SlotEvent::CancelWaiter {
                activation_id,
                dispatch_id,
            },
        )
    }

    /// Resolve one previously admitted exact-incarnation delivery.
    pub fn delivery_resolved(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        failure: Option<(DispatchId, C)>,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(
            entity_id,
            SlotEvent::DeliveryResolved {
                activation_id,
                failure,
            },
        )
    }

    /// Atomically close admission for an exact active incarnation.
    pub fn begin_drain(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::BeginDrain { activation_id })
    }

    /// Submit acknowledgement of the ordered processing fence.
    pub fn fence_acknowledged(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::FenceAcknowledged { activation_id })
    }

    /// Force a bounded drain to retirement with its exact failure stage.
    pub fn force_drain(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        failure: DrainFailure,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(
            entity_id,
            SlotEvent::ForceDrain {
                activation_id,
                failure,
            },
        )
    }

    /// Submit an exact-incarnation termination observation.
    pub fn terminated(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
    ) -> DirectoryOutput<I, C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::Terminated { activation_id })
    }

    /// Return the number of represented stable routing slots.
    ///
    /// # Panics
    ///
    /// Panics if a directory shard lock was poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        self.shards
            .iter()
            .map(|shard| shard.lock().expect("directory shard lock poisoned").len())
            .sum()
    }

    /// Return whether no stable routing slots are represented.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn submit_or_inactive(
        &self,
        entity_id: &EntityId<I>,
        event: SlotEvent<C, E, L>,
    ) -> DirectoryOutput<I, C, E, L> {
        let slot = self.shards[self.shard_index(entity_id)]
            .lock()
            .expect("directory shard lock poisoned")
            .get(entity_id)
            .cloned();
        let (slot, evidence) = if let Some(slot) = slot {
            let evidence = slot.submit(event);
            (slot, evidence)
        } else {
            let (output, machine) = lifecycle_machine().step(event);
            let evidence = output.evidence;
            (Arc::new(Slot::new(machine, output)), evidence)
        };
        directory_output(entity_id.clone(), None, evidence, slot)
    }

    /// Interpret all effects queued for the output's stable slot.
    ///
    /// A concurrent or reentrant call only contributes its already-queued
    /// effects; the current interpreter retains ownership until the queue is
    /// empty. Runtime callbacks execute without directory synchronization held.
    pub fn interpret<R>(&self, output: DirectoryOutput<I, C, E, L>, runtime: &R)
    where
        R: EffectInterpreter<I, C, E, L>,
    {
        let DirectoryOutput {
            entity_id, slot, ..
        } = output;
        if !slot.begin_interpretation() {
            return;
        }
        while let Some(effect) = slot.next_effect() {
            match effect {
                SlotEffect::StartActivation { activation_id } => {
                    runtime.start_activation(entity_id.clone(), activation_id);
                }
                SlotEffect::Deliver {
                    activation_id,
                    dispatch_id,
                    endpoint,
                    command,
                } => runtime.deliver(
                    entity_id.clone(),
                    activation_id,
                    dispatch_id,
                    endpoint,
                    command,
                ),
                SlotEffect::Reject {
                    dispatch_id,
                    command,
                    reason,
                } => runtime.reject(dispatch_id, command, reason),
                SlotEffect::EnqueueFence {
                    activation_id,
                    endpoint,
                } => runtime.enqueue_fence(entity_id.clone(), activation_id, endpoint),
                SlotEffect::Retire {
                    activation_id,
                    lease,
                    retirement,
                } => runtime.retire(entity_id.clone(), activation_id, lease, retirement),
                SlotEffect::Remove { activation_id } => {
                    slot.mark_removable(activation_id);
                    self.remove_matching(&entity_id, &slot, activation_id);
                }
            }
        }
    }

    fn remove_matching(
        &self,
        entity_id: &EntityId<I>,
        slot: &Arc<Slot<C, E, L>>,
        activation_id: ActivationId,
    ) -> bool {
        let mut entries = self.shards[self.shard_index(entity_id)]
            .lock()
            .expect("directory shard lock poisoned");
        let matches = entries
            .get(entity_id)
            .is_some_and(|stored| Arc::ptr_eq(stored, slot))
            && slot.removable_as(activation_id);
        if matches {
            entries.remove(entity_id);
        }
        matches
    }

    fn shard_index(&self, entity_id: &EntityId<I>) -> usize {
        let mask = u64::try_from(self.shards.len() - 1).expect("shard mask fits u64");
        usize::try_from(self.hash_builder.hash_one(entity_id) & mask)
            .expect("masked hash fits usize")
    }
}

fn allocate(sequence: &AtomicU64) -> Option<NonZeroU64> {
    sequence
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .ok()
        .and_then(NonZeroU64::new)
}

fn directory_output<I, C, E, L>(
    entity_id: EntityId<I>,
    dispatch_id: Option<DispatchId>,
    evidence: TransitionEvidence,
    slot: Arc<Slot<C, E, L>>,
) -> DirectoryOutput<I, C, E, L> {
    DirectoryOutput {
        dispatch_id,
        evidence,
        entity_id,
        slot,
    }
}
