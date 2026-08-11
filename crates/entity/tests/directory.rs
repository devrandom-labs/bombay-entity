use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::{Arc, Barrier};
use std::thread;

use bombay_entity::{ActivationId, DirectoryConfig, EntityId, LocalDirectory, Refusal, SlotEffect};

type Directory = LocalDirectory<u64, u64, u64, u64>;

fn activation(value: u64) -> ActivationId {
    ActivationId::new(NonZeroU64::new(value).unwrap())
}

#[test]
fn concurrent_first_dispatches_share_one_bounded_activation() {
    const CALLERS: usize = 16;
    let directory = Arc::new(
        Directory::new(DirectoryConfig {
            shards: NonZeroUsize::new(8).unwrap(),
            activation_waiters: NonZeroUsize::new(CALLERS).unwrap(),
        })
        .unwrap(),
    );
    let barrier = Arc::new(Barrier::new(CALLERS));
    let threads: Vec<_> = (0..CALLERS)
        .map(|command| {
            let directory = Arc::clone(&directory);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                directory
                    .dispatch(EntityId::new(7), command as u64)
                    .unwrap()
            })
        })
        .collect();
    let outputs: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();

    assert_eq!(directory.len(), 1);
    let starts: Vec<_> = outputs
        .iter()
        .flat_map(|output| &output.effects)
        .filter_map(|effect| match effect {
            SlotEffect::StartActivation { activation_id } => Some(*activation_id),
            _ => None,
        })
        .collect();
    assert_eq!(starts.len(), 1);

    let activated = directory.activation_succeeded(&EntityId::new(7), starts[0], 11, 12);
    assert_eq!(
        activated
            .effects
            .iter()
            .filter(|effect| matches!(effect, SlotEffect::Deliver { .. }))
            .count(),
        CALLERS
    );
}

#[test]
fn draining_closes_admission_before_fence_execution() {
    let directory = Directory::new(DirectoryConfig::default()).unwrap();
    let entity_id = EntityId::new(9);
    let claimed = directory.dispatch(entity_id, 1).unwrap();
    let activation_id = claimed
        .effects
        .iter()
        .find_map(|effect| match effect {
            SlotEffect::StartActivation { activation_id } => Some(*activation_id),
            _ => None,
        })
        .unwrap();
    let activated = directory.activation_succeeded(&entity_id, activation_id, 2, 3);
    let dispatch_id = activated
        .effects
        .iter()
        .find_map(|effect| match effect {
            SlotEffect::Deliver { dispatch_id, .. } => Some(*dispatch_id),
            _ => None,
        })
        .unwrap();
    directory.delivery_resolved(&entity_id, activation_id, None);

    let drain = directory.begin_drain(&entity_id, activation_id);
    assert!(matches!(
        drain.effects.as_slice(),
        [SlotEffect::EnqueueFence { .. }]
    ));
    let refused = directory.dispatch(entity_id, 4).unwrap();
    assert!(matches!(
        refused.effects.as_slice(),
        [SlotEffect::Reject {
            reason: Refusal::Draining,
            command: 4,
            ..
        }]
    ));
    assert_eq!(dispatch_id.0, 1);
}

#[test]
fn stale_successful_activation_returns_its_exact_lease_for_retirement() {
    let directory = Directory::new(DirectoryConfig::default()).unwrap();
    let output = directory.activation_succeeded(&EntityId::new(1), activation(99), 2, 7);

    assert!(matches!(
        output.effects.as_slice(),
        [SlotEffect::Retire {
            activation_id,
            lease: 7,
            ..
        }] if *activation_id == activation(99)
    ));
}
