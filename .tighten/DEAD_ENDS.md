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
