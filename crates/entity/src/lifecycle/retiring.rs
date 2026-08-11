use super::{
    ActivationId, EntitySlot, Generation, Refusal, SlotDecision, SlotEffect, SlotEffectBatch,
    SlotEvent, decision, reject, retire_stale,
};

pub(super) fn decide_retiring<C, E, L>(
    activation_id: ActivationId,
    event: SlotEvent<C, E, L>,
) -> SlotDecision<C, E, L> {
    match event {
        SlotEvent::Terminated {
            activation_id: observed,
        } => match activation_id.classify(observed, ()) {
            Generation::Current(()) => decision(
                EntitySlot::Inactive,
                SlotEffectBatch::one(SlotEffect::Remove { activation_id }),
            ),
            Generation::Stale(()) => decision(
                EntitySlot::Retiring { activation_id },
                SlotEffectBatch::default(),
            ),
        },
        SlotEvent::Dispatch {
            dispatch_id,
            command,
        }
        | SlotEvent::ClaimActivation {
            dispatch_id,
            command,
            ..
        } => reject(
            EntitySlot::Retiring { activation_id },
            dispatch_id,
            command,
            Refusal::Draining,
        ),
        SlotEvent::ActivationSucceeded {
            activation_id: stale_id,
            lease,
            ..
        } => retire_stale(EntitySlot::Retiring { activation_id }, stale_id, lease),
        _ => decision(
            EntitySlot::Retiring { activation_id },
            SlotEffectBatch::default(),
        ),
    }
}
