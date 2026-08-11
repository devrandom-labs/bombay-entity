use std::hint::black_box;
use std::num::NonZeroUsize;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

use bombay_entity::{DirectoryConfig, EntityId, LocalDirectory};

const SEQUENTIAL_ITERATIONS: u64 = 1_000_000;
const THREADS: usize = 8;
const CONTENDED_ITERATIONS: u64 = 100_000;

type Directory = LocalDirectory<u64, u64, u64, u64>;

fn config(waiters: usize) -> DirectoryConfig {
    DirectoryConfig {
        shards: NonZeroUsize::new(64).unwrap(),
        activation_waiters: NonZeroUsize::new(waiters).unwrap(),
    }
}

fn sequential_hot_key() -> Duration {
    let directory = Directory::new(config(64)).unwrap();
    let started = Instant::now();
    for command in 0..SEQUENTIAL_ITERATIONS {
        black_box(directory.dispatch(EntityId::new(1), command).unwrap());
    }
    started.elapsed()
}

fn sequential_independent_keys() -> Duration {
    let directory = Directory::new(config(1)).unwrap();
    let started = Instant::now();
    for command in 0..SEQUENTIAL_ITERATIONS {
        black_box(directory.dispatch(EntityId::new(command), command).unwrap());
    }
    started.elapsed()
}

fn contended_hot_key() -> Duration {
    let directory = Arc::new(Directory::new(config(THREADS)).unwrap());
    let barrier = Arc::new(Barrier::new(THREADS + 1));
    let threads: Vec<_> = (0..THREADS)
        .map(|worker| {
            let directory = Arc::clone(&directory);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                for command in 0..CONTENDED_ITERATIONS {
                    black_box(
                        directory
                            .dispatch(EntityId::new(1), command + worker as u64)
                            .unwrap(),
                    );
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
    let hot = sequential_hot_key();
    let independent = sequential_independent_keys();
    let contended = contended_hot_key();
    println!("sequential_iterations={SEQUENTIAL_ITERATIONS}");
    println!("sequential_hot_key={hot:?}");
    println!("sequential_independent_keys={independent:?}");
    println!("threads={THREADS}");
    println!("contended_iterations_per_thread={CONTENDED_ITERATIONS}");
    println!("contended_hot_key={contended:?}");
}
