//! Concurrent storage for local entity lifecycle machines.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, RandomState};
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bombay_machine_executor::{LinearizedExecutor, OutputEvidence};

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
#[derive(Debug, thiserror::Error)]
pub enum DirectoryError<C> {
    /// The shard count was not a power of two.
    #[error("shard count was not a power of two")]
    InvalidShardCount,
    /// The monotonically increasing activation namespace is exhausted.
    #[error("activation identity namespace is exhausted")]
    ActivationIdsExhausted(C),
    /// The monotonically increasing dispatch namespace is exhausted.
    #[error("dispatch identity namespace is exhausted")]
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
    pub(crate) activation_id: Option<ActivationId>,
    entity_id: EntityId<I>,
    slot: Arc<Slot<C, E, L>>,
}

struct Slot<C, E, L> {
    lifecycle: LinearizedExecutor<LifecycleMachine<C, E, L>>,
    removable_activation: Mutex<Option<ActivationId>>,
}

struct Installed {
    evidence: TransitionEvidence,
    activation_id: Option<ActivationId>,
}

impl<C, E: Clone, L> Slot<C, E, L> {
    fn new() -> Self {
        Self {
            lifecycle: LinearizedExecutor::new(lifecycle_machine()),
            removable_activation: Mutex::new(None),
        }
    }

    fn submit(&self, event: SlotEvent<C, E, L>) -> Installed {
        let (evidence, activation_id) = self.lifecycle.submit(event);
        Installed {
            evidence,
            activation_id,
        }
    }

    fn activation_id(&self) -> Option<ActivationId> {
        self.lifecycle.evidence().and_then(|evidence| evidence.1)
    }

    fn mark_removable(&self, activation_id: ActivationId) {
        *self
            .removable_activation
            .lock()
            .expect("slot lock poisoned") = Some(activation_id);
    }

    fn removable_as(&self, activation_id: ActivationId) -> bool {
        *self
            .removable_activation
            .lock()
            .expect("slot lock poisoned")
            == Some(activation_id)
    }
}

type Shard<I, C, E, L> = Mutex<HashMap<EntityId<I>, Arc<Slot<C, E, L>>>>;

/// Sharded local storage for authoritative per-entity lifecycle machines.
pub struct LocalDirectory<I, C, E, L, S = RandomState> {
    shards: Box<[Shard<I, C, E, L>]>,
    mask: u64,
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
        let mask = config
            .shards
            .get()
            .checked_sub(1)
            .and_then(|count| u64::try_from(count).ok())
            .ok_or(DirectoryError::InvalidShardCount)?;
        let shards = (0..config.shards.get())
            .map(|_| Mutex::new(HashMap::new()))
            .collect();
        Ok(Self {
            shards,
            mask,
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
                let slot = Arc::new(Slot::new());
                let installed = slot.submit(SlotEvent::ClaimActivation {
                    activation_id,
                    dispatch_id,
                    command: command.take().expect("command present"),
                    waiter_limit: self.waiter_limit,
                });
                entries.insert(entity_id.clone(), Arc::clone(&slot));
                (slot, Some(installed))
            }
        };
        let installed = output.unwrap_or_else(|| {
            slot.submit(SlotEvent::Dispatch {
                dispatch_id,
                command: command.take().expect("command present"),
            })
        });
        Ok(directory_output(
            entity_id,
            Some(dispatch_id),
            installed.evidence,
            installed.activation_id,
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

    pub(crate) fn current_activation(&self, entity_id: &EntityId<I>) -> Option<ActivationId> {
        self.shards[self.shard_index(entity_id)]
            .lock()
            .expect("directory shard lock poisoned")
            .get(entity_id)
            .cloned()
            .and_then(|slot| slot.activation_id())
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
        let (slot, installed) = if let Some(slot) = slot {
            let installed = slot.submit(event);
            (slot, installed)
        } else {
            let slot = Arc::new(Slot::new());
            let installed = slot.submit(event);
            (slot, installed)
        };
        directory_output(
            entity_id.clone(),
            None,
            installed.evidence,
            installed.activation_id,
            slot,
        )
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
        slot.lifecycle
            .dispatch_pending(&|output: LifecycleOutput<C, E, L>| {
                output.effects.for_each(|effect| match effect {
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
                });
            });
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
        // The mask derives from the shard count, so the masked hash always
        // fits the pointer width that allocated the shards.
        usize::try_from(self.hash_builder.hash_one(entity_id) & self.mask)
            .expect("masked hash fits usize")
    }
}

impl<C, E, L> OutputEvidence for LifecycleOutput<C, E, L> {
    type Evidence = (TransitionEvidence, Option<ActivationId>);

    fn evidence(&self) -> Self::Evidence {
        (self.evidence, self.activation_id)
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
    activation_id: Option<ActivationId>,
    slot: Arc<Slot<C, E, L>>,
) -> DirectoryOutput<I, C, E, L> {
    DirectoryOutput {
        dispatch_id,
        evidence,
        activation_id,
        entity_id,
        slot,
    }
}
