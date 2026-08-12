use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;
use std::time::Duration;

use bombay_entity::{
    Activated, ActivationId, DirectoryConfig, DispatchFailure, EntityId, EntityRuntime,
    FenceFailure, LocalEntityRuntime, Passivation, Refusal, RetirementMode,
};

struct ThreadWake(thread::Thread);

impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

#[derive(Clone)]
struct TestRuntime {
    state: Arc<TestRuntimeState>,
}

struct TestRuntimeState {
    activations: AtomicUsize,
    activation_completions: AtomicUsize,
    fail_delivery: AtomicBool,
    delivered: Mutex<Vec<u64>>,
    activation_gate: Mutex<Option<Arc<ActivationGate>>>,
    delivery_gate: Mutex<Option<Arc<ActivationGate>>>,
    fence_gate: Mutex<Option<Arc<ActivationGate>>>,
    fences: AtomicUsize,
    retirements: AtomicUsize,
}

impl TestRuntime {
    fn new() -> Self {
        Self {
            state: Arc::new(TestRuntimeState {
                activations: AtomicUsize::new(0),
                activation_completions: AtomicUsize::new(0),
                fail_delivery: AtomicBool::new(false),
                delivered: Mutex::new(Vec::new()),
                activation_gate: Mutex::new(None),
                delivery_gate: Mutex::new(None),
                fence_gate: Mutex::new(None),
                fences: AtomicUsize::new(0),
                retirements: AtomicUsize::new(0),
            }),
        }
    }
}

impl<I: Send + 'static> LocalEntityRuntime<I, u64> for TestRuntime {
    type Endpoint = u64;
    type Lease = u64;
    type ActivationError = ();

    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        thread::spawn(move || block_on(task));
    }

    async fn activate(
        &self,
        _: EntityId<I>,
        activation_id: ActivationId,
    ) -> Result<Activated<Self::Endpoint, Self::Lease>, Self::ActivationError> {
        self.state.activations.fetch_add(1, Ordering::Relaxed);
        let gate = self.state.activation_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        self.state
            .activation_completions
            .fetch_add(1, Ordering::Release);
        Ok(Activated {
            endpoint: activation_id.get().get(),
            lease: activation_id.get().get(),
        })
    }

    async fn deliver(&self, _: Self::Endpoint, command: u64) -> Result<(), u64> {
        let gate = self.state.delivery_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        if self.state.fail_delivery.load(Ordering::Relaxed) {
            Err(command)
        } else {
            self.state.delivered.lock().unwrap().push(command);
            Ok(())
        }
    }

    async fn fence(&self, _: Self::Endpoint) -> Result<(), FenceFailure> {
        let gate = self.state.fence_gate.lock().unwrap().clone();
        if let Some(gate) = gate {
            gate.wait().await;
        }
        self.state.fences.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn retire(&self, _: Self::Lease, _: RetirementMode) {
        self.state.retirements.fetch_add(1, Ordering::Release);
    }
}

struct HashGate {
    state: Mutex<HashGateState>,
    changed: Condvar,
}

#[derive(Default)]
struct HashGateState {
    blocked_thread: Option<thread::ThreadId>,
    calls: usize,
    blocked: bool,
    released: bool,
}

impl HashGate {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(HashGateState::default()),
            changed: Condvar::new(),
        })
    }

    fn block_third_hash_on_this_thread(&self) {
        self.state.lock().unwrap().blocked_thread = Some(thread::current().id());
    }

    fn wait_until_blocked(&self) {
        let mut state = self.state.lock().unwrap();
        while !state.blocked {
            state = self.changed.wait(state).unwrap();
        }
    }

    fn release(&self) {
        let mut state = self.state.lock().unwrap();
        state.released = true;
        self.changed.notify_all();
    }
}

#[derive(Clone)]
struct GatedId {
    value: u64,
    gate: Arc<HashGate>,
}

impl PartialEq for GatedId {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl Eq for GatedId {}

impl Hash for GatedId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        let mut gate = self.gate.state.lock().unwrap();
        if gate.blocked_thread == Some(thread::current().id()) {
            gate.calls += 1;
            if gate.calls == 3 {
                gate.blocked = true;
                self.gate.changed.notify_all();
                while !gate.released {
                    gate = self.gate.changed.wait(gate).unwrap();
                }
            }
        }
    }
}

struct ActivationGate {
    open: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl ActivationGate {
    fn closed() -> Arc<Self> {
        Arc::new(Self {
            open: AtomicBool::new(false),
            waker: Mutex::new(None),
        })
    }

    fn wait(self: &Arc<Self>) -> GateFuture {
        GateFuture(Arc::clone(self))
    }

    fn open(&self) {
        self.open.store(true, Ordering::Release);
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }
}

struct GateFuture(Arc<ActivationGate>);

impl Future for GateFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.open.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            *self.0.waker.lock().unwrap() = Some(context.waker().clone());
            if self.0.open.load(Ordering::Acquire) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}

#[test]
fn application_dispatch_is_one_asynchronous_operation() {
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();

    block_on(entities.dispatch(EntityId::new(42), 7)).unwrap();

    assert_eq!(observations.state.activations.load(Ordering::Relaxed), 1);
    assert_eq!(*observations.state.delivered.lock().unwrap(), [7]);
}

#[test]
fn failed_delivery_returns_the_original_command() {
    let actor_runtime = TestRuntime::new();
    actor_runtime
        .state
        .fail_delivery
        .store(true, Ordering::Relaxed);
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();

    let failure = block_on(entities.dispatch(EntityId::new(9), 77)).unwrap_err();

    assert!(matches!(
        failure,
        DispatchFailure::Refused {
            command: 77,
            reason: Refusal::Unavailable,
        }
    ));
}

#[test]
fn canceling_dispatch_does_not_cancel_shared_activation_or_deliver_command() {
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let gate = ActivationGate::closed();
    *actor_runtime.state.activation_gate.lock().unwrap() = Some(Arc::clone(&gate));
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();
    let mut dispatch = Box::pin(entities.dispatch(EntityId::new(5), 91));
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);

    assert!(dispatch.as_mut().poll(&mut context).is_pending());
    drop(dispatch);
    gate.open();
    for _ in 0..100 {
        if observations
            .state
            .activation_completions
            .load(Ordering::Acquire)
            == 1
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(observations.state.activations.load(Ordering::Relaxed), 1);
    assert_eq!(
        observations
            .state
            .activation_completions
            .load(Ordering::Acquire),
        1
    );
    block_on(entities.dispatch(EntityId::new(5), 92)).unwrap();
    assert_eq!(*observations.state.delivered.lock().unwrap(), [92]);
}

#[test]
fn dropping_active_dispatch_does_not_retract_owned_delivery() {
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime.clone()).unwrap();
    let entity_id = EntityId::new(8);
    block_on(entities.dispatch(entity_id, 1)).unwrap();

    let gate = ActivationGate::closed();
    *actor_runtime.state.delivery_gate.lock().unwrap() = Some(Arc::clone(&gate));
    let mut dispatch = Box::pin(entities.dispatch(entity_id, 2));
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut context = Context::from_waker(&waker);
    assert!(dispatch.as_mut().poll(&mut context).is_pending());

    drop(dispatch);
    gate.open();
    for _ in 0..100 {
        if observations.state.delivered.lock().unwrap().len() == 2 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }

    assert_eq!(*observations.state.delivered.lock().unwrap(), [1, 2]);
}

#[test]
fn passivation_reports_not_active_for_unknown_entity() {
    let entities = EntityRuntime::new(DirectoryConfig::default(), TestRuntime::new()).unwrap();

    assert_eq!(
        entities.passivate(&EntityId::new(99)),
        Passivation::NotActive
    );
}

#[test]
fn repeated_passivation_reports_already_passivating() {
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let fence_gate = ActivationGate::closed();
    *actor_runtime.state.fence_gate.lock().unwrap() = Some(Arc::clone(&fence_gate));
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();
    let entity_id = EntityId::new(7);
    block_on(entities.dispatch(entity_id, 1)).unwrap();

    assert_eq!(entities.passivate(&entity_id), Passivation::Begun);
    assert_eq!(
        entities.passivate(&entity_id),
        Passivation::AlreadyPassivating
    );

    fence_gate.open();
    for _ in 0..100 {
        if observations.state.retirements.load(Ordering::Acquire) == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(observations.state.fences.load(Ordering::Relaxed), 1);
    assert_eq!(observations.state.retirements.load(Ordering::Relaxed), 1);
}

#[test]
fn passivation_reports_superseded_after_incarnation_replacement() {
    let gate = HashGate::new();
    let entity_id = EntityId::new(GatedId {
        value: 8,
        gate: Arc::clone(&gate),
    });
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();
    block_on(entities.dispatch(entity_id.clone(), 1)).unwrap();

    let racing_runtime = entities.clone();
    let racing_id = entity_id.clone();
    let racing_gate = Arc::clone(&gate);
    let passivation = thread::spawn(move || {
        racing_gate.block_third_hash_on_this_thread();
        racing_runtime.passivate(&racing_id)
    });
    gate.wait_until_blocked();

    assert_eq!(entities.passivate(&entity_id), Passivation::Begun);
    for _ in 0..1_000 {
        if observations.state.retirements.load(Ordering::Acquire) != 0 {
            break;
        }
        thread::yield_now();
    }
    assert_eq!(observations.state.retirements.load(Ordering::Acquire), 1);
    let mut command = 2;
    let mut replacement_activated = false;
    for _ in 0..1_000 {
        match block_on(entities.dispatch(entity_id.clone(), command)) {
            Ok(()) => {
                replacement_activated = true;
                break;
            }
            Err(DispatchFailure::Refused {
                command: returned, ..
            }) => {
                command = returned;
                thread::yield_now();
            }
            Err(failure) => panic!("unexpected replacement dispatch failure: {failure}"),
        }
    }
    assert!(
        replacement_activated,
        "replacement activation did not complete"
    );

    gate.release();
    assert_eq!(passivation.join().unwrap(), Passivation::Superseded);
    assert_eq!(observations.state.activations.load(Ordering::Acquire), 2);
}

#[test]
fn passivation_fences_and_retires_the_exact_incarnation() {
    let actor_runtime = TestRuntime::new();
    let observations = actor_runtime.clone();
    let entities = EntityRuntime::new(DirectoryConfig::default(), actor_runtime).unwrap();
    let entity_id = EntityId::new(6);
    block_on(entities.dispatch(entity_id, 1)).unwrap();

    assert_eq!(entities.passivate(&entity_id), Passivation::Begun);
    for _ in 0..100 {
        if observations.state.retirements.load(Ordering::Acquire) == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(observations.state.fences.load(Ordering::Relaxed), 1);
    assert_eq!(observations.state.retirements.load(Ordering::Relaxed), 1);
    assert_eq!(observations.state.activations.load(Ordering::Relaxed), 1);
}
