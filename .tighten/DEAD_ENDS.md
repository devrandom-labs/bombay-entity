# Dead Ends

Reverted experiments and rejected candidates, so later contexts do not retry them blindly.
Each entry: hypothesis, what was tried, measurement/evidence, why reverted/rejected.

(none yet)

## H03 deep propagation (rejected at analysis, 2026-08-11)
Threading `R::ActivationError` into the directory would change the finalized
`SlotEvent::ActivationFailed { activation_id }` algebra. The typed error is the
actor runtime's diagnostic boundary; only the fact of failure is consumed.
Resolution: named binding + boundary comment (E03).
