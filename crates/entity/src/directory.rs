//! Concurrent storage for local entity lifecycle machines.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, RandomState};
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bombay_transition::Machine;

use crate::{
    ActivationId, DispatchId, DrainFailure, EntityId, LifecycleMachine, LifecycleOutput,
    SlotEffect, SlotEvent, TransitionEvidence, lifecycle_machine,
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

/// One installed lifecycle decision and its ordered effects.
#[derive(Debug)]
pub struct DirectoryOutput<C, E, L> {
    /// Correlation identity allocated for a dispatched command, when applicable.
    pub dispatch_id: Option<DispatchId>,
    /// Checked evidence for the installed lifecycle decision.
    pub evidence: TransitionEvidence,
    /// Effects to interpret in their declared order after synchronization is released.
    pub effects: Vec<SlotEffect<C, E, L>>,
}

struct Slot<C, E, L> {
    machine: Mutex<Option<LifecycleMachine<C, E, L>>>,
}

impl<C, E: Clone, L> Slot<C, E, L> {
    fn new(machine: LifecycleMachine<C, E, L>) -> Self {
        Self {
            machine: Mutex::new(Some(machine)),
        }
    }

    fn submit(&self, event: SlotEvent<C, E, L>) -> LifecycleOutput<C, E, L> {
        let mut stored = self.machine.lock().expect("slot lock poisoned");
        let machine = stored.take().expect("slot machine missing");
        let (output, successor) = machine.step(event);
        *stored = Some(successor);
        output
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
    I: Eq + Hash,
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
    I: Eq + Hash,
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
    ) -> Result<DirectoryOutput<C, E, L>, DirectoryError<C>> {
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
                let slot = Arc::new(Slot::new(machine.1));
                entries.insert(entity_id, Arc::clone(&slot));
                (slot, Some(machine.0))
            }
        };
        let output = output.unwrap_or_else(|| {
            slot.submit(SlotEvent::Dispatch {
                dispatch_id,
                command: command.take().expect("command present"),
            })
        });
        Ok(directory_output(Some(dispatch_id), output))
    }

    /// Submit successful exact-incarnation activation to the represented slot.
    pub fn activation_succeeded(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        endpoint: E,
        lease: L,
    ) -> DirectoryOutput<C, E, L> {
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
    ) -> DirectoryOutput<C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::ActivationFailed { activation_id })
    }

    /// Cancel one bounded activation waiter.
    pub fn cancel_waiter(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        dispatch_id: DispatchId,
    ) -> DirectoryOutput<C, E, L> {
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
    ) -> DirectoryOutput<C, E, L> {
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
    ) -> DirectoryOutput<C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::BeginDrain { activation_id })
    }

    /// Submit acknowledgement of the ordered processing fence.
    pub fn fence_acknowledged(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
    ) -> DirectoryOutput<C, E, L> {
        self.submit_or_inactive(entity_id, SlotEvent::FenceAcknowledged { activation_id })
    }

    /// Force a bounded drain to retirement with its exact failure stage.
    pub fn force_drain(
        &self,
        entity_id: &EntityId<I>,
        activation_id: ActivationId,
        failure: DrainFailure,
    ) -> DirectoryOutput<C, E, L> {
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
    ) -> DirectoryOutput<C, E, L> {
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
    ) -> DirectoryOutput<C, E, L> {
        let slot = self.shards[self.shard_index(entity_id)]
            .lock()
            .expect("directory shard lock poisoned")
            .get(entity_id)
            .cloned();
        let output = match slot {
            Some(slot) => slot.submit(event),
            None => lifecycle_machine().step(event).0,
        };
        directory_output(None, output)
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

fn directory_output<C, E, L>(
    dispatch_id: Option<DispatchId>,
    output: LifecycleOutput<C, E, L>,
) -> DirectoryOutput<C, E, L> {
    DirectoryOutput {
        dispatch_id,
        evidence: output.evidence,
        effects: output.effects.into_vec(),
    }
}
