//! Concise asynchronous API over the local entity directory.

use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};

use crate::{
    ActivationId, DirectoryConfig, DirectoryError, DispatchId, DrainFailure, DrainStage,
    EffectInterpreter, EntityId, LifecyclePhase, LocalDirectory, Refusal, RetirementMode,
    TransitionEvidence,
};

/// Exact-incarnation capabilities returned by transactional activation.
pub struct Activated<E, L> {
    /// Delivery-only capability.
    pub endpoint: E,
    /// Affine retirement capability.
    pub lease: L,
}

/// Stage at which an ordered fence operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FenceFailure {
    /// The fence was not enqueued.
    #[error("fence was not enqueued")]
    Enqueue,
    /// The fence was enqueued but not acknowledged.
    #[error("fence was enqueued but not acknowledged")]
    Acknowledgement,
}

/// Outcome of one [`EntityRuntime::passivate`] call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Passivation {
    /// Admission was closed at this call's lifecycle linearization point.
    Begun,
    /// No active incarnation exists to passivate.
    NotActive,
    /// A passivation was already in progress for the exact incarnation.
    AlreadyPassivating,
    /// The observed incarnation was superseded before the drain linearized.
    Superseded,
}

/// Runtime port implemented once by an actor runtime integration.
///
/// Actorpass can implement this port without exposing its addresses or
/// provisional activation types through the stable entity API.
pub trait LocalEntityRuntime<I, C>: Send + Sync + 'static {
    /// Cloneable exact-incarnation delivery capability.
    type Endpoint: Clone + Send + Sync + 'static;
    /// Affine exact-incarnation retirement capability.
    type Lease: Send + 'static;
    /// Transactional activation failure.
    type ActivationError: Send + 'static;

    /// Spawn work owned by the entity directory rather than a caller.
    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static);

    /// Prepare and transactionally activate an exact incarnation.
    fn activate(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
    ) -> impl Future<Output = Result<Activated<Self::Endpoint, Self::Lease>, Self::ActivationError>> + Send;

    /// Enqueue a command, returning ownership on failure.
    fn deliver(
        &self,
        endpoint: Self::Endpoint,
        command: C,
    ) -> impl Future<Output = Result<(), C>> + Send;

    /// Enqueue and await acknowledgement of an ordered processing fence.
    fn fence(
        &self,
        endpoint: Self::Endpoint,
    ) -> impl Future<Output = Result<(), FenceFailure>> + Send;

    /// Retire and await exact termination of one incarnation.
    fn retire(
        &self,
        lease: Self::Lease,
        retirement: RetirementMode,
    ) -> impl Future<Output = ()> + Send;
}

/// Failure from [`EntityRuntime::dispatch`] with command ownership preserved.
#[derive(Debug, thiserror::Error)]
pub enum DispatchFailure<C> {
    /// Lifecycle admission or delivery refused the command.
    #[error("lifecycle admission or delivery refused the command")]
    Refused {
        /// Original command.
        command: C,
        /// Exact refusal classification.
        reason: Refusal,
    },
    /// The non-reusable activation identity namespace is exhausted.
    #[error("activation identity namespace is exhausted")]
    ActivationIdsExhausted(C),
    /// The non-reusable dispatch identity namespace is exhausted.
    #[error("dispatch identity namespace is exhausted")]
    DispatchIdsExhausted(C),
}

struct PendingCommand<C> {
    command: C,
    completion: Arc<Completion<C>>,
}

struct Completion<C>(Mutex<CompletionState<C>>);

enum CompletionState<C> {
    Awaiting { waker: Option<Waker> },
    Ready(Result<(), DispatchFailure<C>>),
    Consumed,
}

impl<C> Completion<C> {
    fn new() -> Self {
        Self(Mutex::new(CompletionState::Awaiting { waker: None }))
    }

    fn complete(&self, result: Result<(), DispatchFailure<C>>) {
        let waker = {
            let mut state = self.0.lock().expect("completion lock poisoned");
            match &mut *state {
                CompletionState::Awaiting { .. } => {
                    match mem::replace(&mut *state, CompletionState::Ready(result)) {
                        CompletionState::Awaiting { waker } => waker,
                        _ => None,
                    }
                }
                CompletionState::Ready(_) | CompletionState::Consumed => return,
            }
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Take the delivered result, or register the waker while still awaiting.
    fn poll_take(&self, waker: &Waker) -> Option<Result<(), DispatchFailure<C>>> {
        let mut state = self.0.lock().expect("completion lock poisoned");
        match &mut *state {
            CompletionState::Ready(_) => {
                match mem::replace(&mut *state, CompletionState::Consumed) {
                    CompletionState::Ready(result) => Some(result),
                    _ => None,
                }
            }
            CompletionState::Awaiting { waker: stored } => {
                *stored = Some(waker.clone());
                None
            }
            CompletionState::Consumed => None,
        }
    }

    /// Whether a delivered result was already consumed by the waiter.
    fn is_consumed(&self) -> bool {
        matches!(
            *self.0.lock().expect("completion lock poisoned"),
            CompletionState::Consumed
        )
    }
}

/// Local stable-routing facade used by applications.
pub struct EntityRuntime<I, C, R>
where
    R: LocalEntityRuntime<I, C>,
{
    inner: Arc<Runtime<I, C, R>>,
}

struct Runtime<I, C, R>
where
    R: LocalEntityRuntime<I, C>,
{
    directory: LocalDirectory<I, PendingCommand<C>, R::Endpoint, R::Lease>,
    actor_runtime: R,
}

impl<I, C, R> Clone for EntityRuntime<I, C, R>
where
    R: LocalEntityRuntime<I, C>,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<I, C, R> EntityRuntime<I, C, R>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
    /// Construct stable local entity routing over an actor-runtime port.
    ///
    /// # Errors
    ///
    /// Returns [`DirectoryError::InvalidShardCount`] for a shard count that is
    /// not a power of two.
    pub fn new(config: DirectoryConfig, actor_runtime: R) -> Result<Self, DirectoryError<C>> {
        let directory =
            LocalDirectory::new(config).map_err(|error: DirectoryError<PendingCommand<C>>| {
                match error {
                    DirectoryError::InvalidShardCount => DirectoryError::InvalidShardCount,
                    DirectoryError::ActivationIdsExhausted(pending) => {
                        DirectoryError::ActivationIdsExhausted(pending.command)
                    }
                    DirectoryError::DispatchIdsExhausted(pending) => {
                        DirectoryError::DispatchIdsExhausted(pending.command)
                    }
                }
            })?;
        Ok(Self {
            inner: Arc::new(Runtime {
                directory,
                actor_runtime,
            }),
        })
    }

    /// Deliver a command through stable entity routing.
    ///
    /// Dropping this future cancels only its bounded activation waiter. The
    /// shared activation task remains owned by the directory.
    ///
    /// # Errors
    ///
    /// Returns a typed refusal or exhausted identity namespace with the
    /// original command.
    ///
    /// # Panics
    ///
    /// Panics after synchronization poison or violation of an internal
    /// directory identity invariant.
    pub async fn dispatch(
        &self,
        entity_id: EntityId<I>,
        command: C,
    ) -> Result<(), DispatchFailure<C>> {
        let completion = Arc::new(Completion::new());
        let pending = PendingCommand {
            command,
            completion: Arc::clone(&completion),
        };
        let dispatched = self
            .inner
            .directory
            .dispatch(entity_id.clone(), pending)
            .map_err(|error| match error {
                DirectoryError::InvalidShardCount => unreachable!("configuration was validated"),
                DirectoryError::ActivationIdsExhausted(pending) => {
                    DispatchFailure::ActivationIdsExhausted(pending.command)
                }
                DirectoryError::DispatchIdsExhausted(pending) => {
                    DispatchFailure::DispatchIdsExhausted(pending.command)
                }
            })?;
        let dispatch_id = dispatched.dispatch_id;
        let activation_id = dispatched.output.activation_id;
        self.inner
            .directory
            .interpret(dispatched.output, &self.inner);
        DispatchWait {
            completion,
            entity_id,
            activation_id,
            dispatch_id,
            runtime: Arc::downgrade(&self.inner),
        }
        .await
    }

    /// Begin graceful passivation if the entity currently has an active incarnation.
    ///
    /// Admission closes at this call's lifecycle linearization point. Fence and
    /// retirement work continues in directory-owned tasks.
    ///
    /// # Panics
    ///
    /// Panics if directory synchronization was poisoned.
    pub fn passivate(&self, entity_id: &EntityId<I>) -> Passivation {
        let Some(activation_id) = self.inner.directory.current_activation(entity_id) else {
            return Passivation::NotActive;
        };
        let output = self.inner.directory.begin_drain(entity_id, activation_id);
        let passivation = match output.evidence {
            TransitionEvidence::Traversed(_) => Passivation::Begun,
            TransitionEvidence::SelfLoop { phase, .. }
            | TransitionEvidence::Ignored { phase, .. } => match phase {
                LifecyclePhase::Active => Passivation::Superseded,
                LifecyclePhase::Draining | LifecyclePhase::Retiring => {
                    Passivation::AlreadyPassivating
                }
                LifecyclePhase::Inactive | LifecyclePhase::Activating => Passivation::NotActive,
            },
        };
        self.inner.directory.interpret(output, &self.inner);
        passivation
    }
}

struct DispatchWait<I, C, R>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
    completion: Arc<Completion<C>>,
    entity_id: EntityId<I>,
    activation_id: Option<ActivationId>,
    dispatch_id: DispatchId,
    runtime: Weak<Runtime<I, C, R>>,
}

impl<I, C, R> Unpin for DispatchWait<I, C, R>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
}

impl<I, C, R> Future for DispatchWait<I, C, R>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
    type Output = Result<(), DispatchFailure<C>>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        match self.completion.poll_take(context.waker()) {
            Some(result) => Poll::Ready(result),
            None => Poll::Pending,
        }
    }
}

impl<I, C, R> Drop for DispatchWait<I, C, R>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
    fn drop(&mut self) {
        if self.completion.is_consumed() {
            return;
        }
        if let Some(runtime) = self.runtime.upgrade()
            && let Some(activation_id) = self.activation_id
        {
            let output =
                runtime
                    .directory
                    .cancel_waiter(&self.entity_id, activation_id, self.dispatch_id);
            runtime.directory.interpret(output, &runtime);
        }
    }
}

impl<I, C, R> EffectInterpreter<I, PendingCommand<C>, R::Endpoint, R::Lease>
    for Arc<Runtime<I, C, R>>
where
    I: Clone + Eq + std::hash::Hash + Send + Sync + 'static,
    C: Send + 'static,
    R: LocalEntityRuntime<I, C>,
{
    fn start_activation(&self, entity_id: EntityId<I>, activation_id: ActivationId) {
        let runtime = Arc::clone(self);
        self.actor_runtime.spawn(async move {
            let result = runtime
                .actor_runtime
                .activate(entity_id.clone(), activation_id)
                .await;
            let output = match result {
                Ok(activated) => runtime.directory.activation_succeeded(
                    &entity_id,
                    activation_id,
                    activated.endpoint,
                    activated.lease,
                ),
                // The finalized lifecycle algebra consumes only the fact of
                // failure; the typed activation error is the actor runtime's
                // own diagnostic boundary and is deliberately not propagated.
                Err(_activation_error) => runtime
                    .directory
                    .activation_failed(&entity_id, activation_id),
            };
            runtime.directory.interpret(output, &runtime);
        });
    }

    fn deliver(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
        dispatch_id: DispatchId,
        endpoint: R::Endpoint,
        pending: PendingCommand<C>,
    ) {
        let runtime = Arc::clone(self);
        self.actor_runtime.spawn(async move {
            let completion = pending.completion;
            let failure = match runtime
                .actor_runtime
                .deliver(endpoint, pending.command)
                .await
            {
                Ok(()) => {
                    completion.complete(Ok(()));
                    None
                }
                Err(command) => Some((
                    dispatch_id,
                    PendingCommand {
                        command,
                        completion,
                    },
                )),
            };
            let output = runtime
                .directory
                .delivery_resolved(&entity_id, activation_id, failure);
            runtime.directory.interpret(output, &runtime);
        });
    }

    fn reject(&self, _: DispatchId, pending: PendingCommand<C>, reason: Refusal) {
        pending.completion.complete(Err(DispatchFailure::Refused {
            command: pending.command,
            reason,
        }));
    }

    fn enqueue_fence(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
        endpoint: R::Endpoint,
    ) {
        let runtime = Arc::clone(self);
        self.actor_runtime.spawn(async move {
            let output = match runtime.actor_runtime.fence(endpoint).await {
                Ok(()) => runtime
                    .directory
                    .fence_acknowledged(&entity_id, activation_id),
                Err(failure) => runtime.directory.force_drain(
                    &entity_id,
                    activation_id,
                    DrainFailure {
                        stage: match failure {
                            FenceFailure::Enqueue => DrainStage::FenceEnqueue,
                            FenceFailure::Acknowledgement => DrainStage::FenceAcknowledgement,
                        },
                        outstanding_reservations: 0,
                    },
                ),
            };
            runtime.directory.interpret(output, &runtime);
        });
    }

    fn retire(
        &self,
        entity_id: EntityId<I>,
        activation_id: ActivationId,
        lease: R::Lease,
        retirement: RetirementMode,
    ) {
        let runtime = Arc::clone(self);
        self.actor_runtime.spawn(async move {
            runtime.actor_runtime.retire(lease, retirement).await;
            let output = runtime.directory.terminated(&entity_id, activation_id);
            runtime.directory.interpret(output, &runtime);
        });
    }
}
