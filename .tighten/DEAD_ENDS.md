# Dead Ends

Reverted experiments and rejected candidates, so later contexts do not retry them blindly.
Each entry: hypothesis, what was tried, measurement/evidence, why reverted/rejected.

(none yet)

## H03 deep propagation (rejected at analysis, 2026-08-11)
Threading `R::ActivationError` into the directory would change the finalized
`SlotEvent::ActivationFailed { activation_id }` algebra. The typed error is the
actor runtime's diagnostic boundary; only the fact of failure is consumed.
Resolution: named binding + boundary comment (E03).

## H09 machine: Option<M> sentinel (rejected at analysis, 2026-08-11)
After E07, turn/dispatch ownership exclusivity is enforced by TurnState/
DispatchState, so the None window is observable only by the owner that moved
the affine machine out. Option<M> is the canonical Rust encoding of state
temporarily moved out of shared storage; renaming to an enum adds names
without removing states or expect paths. No experiment run.

## H14 Completion -> oneshot channel (rejected at analysis, 2026-08-11)
DispatchWait::drop must distinguish result-consumed from result-pending to
decide cancel_waiter. tokio/futures oneshot receivers cannot report "consumed"
after poll-to-Ready, so the state E06 removed would have to be re-added as a
wrapper. Entity core is deliberately runtime-free (tests use std threads); a
tokio/futures dependency adds burden and removes nothing.

## H13 SlotEffectBatch -> smallvec (rejected by benchmark, 2026-08-11)
SmallVec<[SlotEffect;1]> replaces Empty/One/Many machinery but carries an
explicit capacity field, growing the per-decision Decision move. Reproducible
min-of-7 regressions: activating_hot_key +5.9%, active_hot_key +8.9%,
independent_keys +9.0% (two runs). The hand-rolled enum is smaller and faster;
the custom machinery stays. Reverted.
