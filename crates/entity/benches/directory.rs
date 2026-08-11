use std::hint::black_box;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use bombay_entity::{
    ActivationId, DirectoryConfig, DispatchId, EffectInterpreter, EntityId, LocalDirectory,
    Refusal, RetirementMode,
};

const ITERATIONS: u64 = 1_000_000;
const INDEPENDENT_ITERATIONS: u64 = 100_000;
const THREADS: usize = 8;
const CONTENDED_ITERATIONS: u64 = 100_000;

type Directory = LocalDirectory<u64, u64, u64, u64>;

fn config(waiters: usize) -> DirectoryConfig {
    DirectoryConfig {
        shards: NonZeroUsize::new(64).unwrap(),
        activation_waiters: NonZeroUsize::new(waiters).unwrap(),
    }
}

struct Discard {
    activation: AtomicU64,
}

impl Discard {
    fn new() -> Self {
        Self {
            activation: AtomicU64::new(0),
        }
    }
}

impl EffectInterpreter<u64, u64, u64, u64> for Discard {
    fn start_activation(&self, _: EntityId<u64>, activation_id: ActivationId) {
        self.activation
            .store(activation_id.get().get(), Ordering::Relaxed);
    }
    fn deliver(&self, _: EntityId<u64>, _: ActivationId, _: DispatchId, _: u64, _: u64) {}
    fn reject(&self, _: DispatchId, _: u64, _: Refusal) {}
    fn enqueue_fence(&self, _: EntityId<u64>, _: ActivationId, _: u64) {}
    fn retire(&self, _: EntityId<u64>, _: ActivationId, _: u64, _: RetirementMode) {}
}

struct Resolve {
    directory: Arc<Directory>,
}

impl EffectInterpreter<u64, u64, u64, u64> for Resolve {
    fn start_activation(&self, _: EntityId<u64>, _: ActivationId) {}
    fn deliver(
        &self,
        entity_id: EntityId<u64>,
        activation_id: ActivationId,
        _: DispatchId,
        _: u64,
        _: u64,
    ) {
        let output = self
            .directory
            .delivery_resolved(&entity_id, activation_id, None);
        self.directory.interpret(output, self);
    }
    fn reject(&self, _: DispatchId, _: u64, _: Refusal) {}
    fn enqueue_fence(&self, _: EntityId<u64>, _: ActivationId, _: u64) {}
    fn retire(&self, _: EntityId<u64>, _: ActivationId, _: u64, _: RetirementMode) {}
}

fn activating_hot_key() -> Duration {
    let directory = Directory::new(config(64)).unwrap();
    let discard = Discard::new();
    let started = Instant::now();
    for command in 0..ITERATIONS {
        let output = directory
            .dispatch(EntityId::new(1), command)
            .unwrap()
            .output;
        directory.interpret(output, &discard);
    }
    started.elapsed()
}

fn active_hot_key() -> Duration {
    let directory = Arc::new(Directory::new(config(64)).unwrap());
    let discard = Discard::new();
    let entity_id = EntityId::new(1);
    directory.interpret(directory.dispatch(entity_id, 0).unwrap().output, &discard);
    let activation_id =
        ActivationId::new(NonZeroU64::new(discard.activation.load(Ordering::Relaxed)).unwrap());
    let resolve = Resolve {
        directory: Arc::clone(&directory),
    };
    directory.interpret(
        directory.activation_succeeded(&entity_id, activation_id, 1, 1),
        &resolve,
    );
    let started = Instant::now();
    for command in 0..ITERATIONS {
        let output = directory.dispatch(entity_id, command).unwrap().output;
        directory.interpret(output, &resolve);
    }
    started.elapsed()
}

fn independent_keys() -> Duration {
    let directory = Directory::new(config(1)).unwrap();
    let discard = Discard::new();
    let started = Instant::now();
    for command in 0..INDEPENDENT_ITERATIONS {
        let output = directory
            .dispatch(EntityId::new(command), command)
            .unwrap()
            .output;
        directory.interpret(output, &discard);
    }
    started.elapsed()
}

fn contended_active_key() -> Duration {
    let directory = Arc::new(Directory::new(config(THREADS)).unwrap());
    let discard = Discard::new();
    let entity_id = EntityId::new(1);
    directory.interpret(directory.dispatch(entity_id, 0).unwrap().output, &discard);
    let activation_id =
        ActivationId::new(NonZeroU64::new(discard.activation.load(Ordering::Relaxed)).unwrap());
    let resolve = Arc::new(Resolve {
        directory: Arc::clone(&directory),
    });
    directory.interpret(
        directory.activation_succeeded(&entity_id, activation_id, 1, 1),
        resolve.as_ref(),
    );
    let barrier = Arc::new(Barrier::new(THREADS + 1));
    let threads: Vec<_> = (0..THREADS)
        .map(|worker| {
            let directory = Arc::clone(&directory);
            let resolve = Arc::clone(&resolve);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for command in 0..CONTENDED_ITERATIONS {
                    let output = directory
                        .dispatch(EntityId::new(1), command + worker as u64)
                        .unwrap()
                        .output;
                    directory.interpret(output, resolve.as_ref());
                }
            })
        })
        .collect();
    barrier.wait();
    let started = Instant::now();
    for thread in threads {
        thread.join().unwrap();
    }
    started.elapsed()
}

fn main() {
    let activating = activating_hot_key();
    let active = active_hot_key();
    let independent = independent_keys();
    let contended = contended_active_key();
    black_box((&activating, &active, &independent, &contended));
    println!("iterations={ITERATIONS}");
    println!("activating_hot_key={activating:?}");
    println!("active_hot_key={active:?}");
    println!("independent_iterations={INDEPENDENT_ITERATIONS}");
    println!("independent_keys={independent:?}");
    println!("threads={THREADS}");
    println!("contended_iterations_per_thread={CONTENDED_ITERATIONS}");
    println!("contended_active_key={contended:?}");
}
