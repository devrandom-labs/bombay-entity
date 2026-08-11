use super::{
    ActivatingSlot, ActivationWaiter, EntitySlot, Refusal, SlotDecision, SlotEffect,
    SlotEffectBatch, SlotEvent, decision, reject, retire_stale,
};

pub(super) fn decide_inactive<C, E, L>(event: SlotEvent<C, E, L>) -> SlotDecision<C, E, L> {
    match event {
        SlotEvent::ClaimActivation {
            activation_id,
            dispatch_id,
            command,
            waiter_limit,
        } => decision(
            EntitySlot::Activating(ActivatingSlot {
                activation_id,
                waiters: vec![ActivationWaiter {
                    dispatch_id,
                    command,
                }],
                waiter_limit,
            }),
            SlotEffectBatch::one(SlotEffect::StartActivation { activation_id }),
        ),
        SlotEvent::Dispatch {
            dispatch_id,
            command,
        } => reject(
            EntitySlot::Inactive,
            dispatch_id,
            command,
            Refusal::Unavailable,
        ),
        SlotEvent::ActivationSucceeded {
            activation_id,
            lease,
            ..
        } => retire_stale(EntitySlot::Inactive, activation_id, lease),
        _ => decision(EntitySlot::Inactive, SlotEffectBatch::default()),
    }
}
