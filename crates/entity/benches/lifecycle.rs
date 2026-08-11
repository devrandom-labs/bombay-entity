use std::hint::black_box;
use std::num::{NonZeroU64, NonZeroUsize};
use std::time::{Duration, Instant};

use bombay_entity::{
    ActivationId, DispatchId, LifecycleEdge, SlotEvent, TransitionEvidence, lifecycle_machine,
};
use bombay_transition::Machine;

const ITERATIONS: u64 = 1_000_000;

fn activation(value: u64) -> ActivationId {
    ActivationId::new(NonZeroU64::new(value).unwrap())
}

fn measure(mut operation: impl FnMut()) -> Duration {
    let started = Instant::now();
    for _ in 0..ITERATIONS {
        operation();
    }
    started.elapsed()
}

fn main() {
    let ignored = measure(|| {
        let machine = lifecycle_machine::<u8, u8, u8>();
        let (output, machine) = machine.step(SlotEvent::Terminated {
            activation_id: activation(2),
        });
        black_box((output.evidence, machine));
    });

    let activation = measure(|| {
        let machine = lifecycle_machine::<u8, u8, u8>();
        let (output, machine) = machine.step(SlotEvent::ClaimActivation {
            activation_id: activation(1),
            dispatch_id: DispatchId(1),
            command: 1,
            waiter_limit: NonZeroUsize::MIN,
        });
        assert_eq!(
            output.evidence,
            TransitionEvidence::Traversed(LifecycleEdge::ClaimActivation)
        );
        black_box(machine);
    });

    println!("iterations={ITERATIONS}");
    println!("ignored_step={ignored:?}");
    println!("claim_activation_step={activation:?}");
}
