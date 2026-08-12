# Dead Ends

Reverted experiments and rejected candidates, so later contexts do not retry them blindly.
Each entry: hypothesis, what was tried, measurement/evidence, why reverted/rejected.

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

## H16 dashmap for directory shards (rejected at analysis, 2026-08-11)
Reentrant interpretation is a tested feature
(directory.rs::reentrant_delivery_resolution_appends_fence_to_current_interpreter):
an interpreter callback submits new facts to the same directory. dashmap shard
guards forbid reentrant access (documented deadlock hazard), so the reentrancy
contract cannot be preserved. The manual sharding is ~30 lines with a simple
lock-order proof. No benchmark run: semantic mismatch is disqualifying.

## H17 LinearizedExecutor poison asymmetry (rejected by contract, 2026-08-11)
"Panics after synchronization poison or a transition panic that consumed the
affine machine state" is documented public behavior; panic guarantees are
finalized contract. No change proposed.

## H18 endpoint clone per Deliver (rejected by contract, 2026-08-11)
SlotEffect::Deliver owns a clone of the delivery-only capability by design
("Clone of the delivery-only capability", lifecycle/mod.rs). Effects outlive
the slot state, so borrowing is impossible; Arc inside the effect would change
the finalized public algebra. Runtimes choose E (e.g. Arc-wrapped) to control
clone cost. No change.

## H19 LinearizedExecutor evidence double-store (rejected at analysis, 2026-08-11)
evidence() requires stored evidence; the submit-time clone is the single copy
into storage. The only in-workspace Evidence type is (TransitionEvidence,
Option<ActivationId>) — Copy. No measurable win without changing the public
evidence() accessor contract.

## H20 vec! per fence acknowledgement (rejected at analysis, 2026-08-11)
protocol.rs:139 allocates once per passivation (cold by design), and the
behavior-crate port takes ownership of a Vec<Delivery>. No alternative shape
without changing the external port.

## H32 retire_stale outstanding_reservations: 0 (contract-blocked, 2026-08-11)
Adversary finding: RetirementMode::Forced(DrainFailure{stage: Retirement,
outstanding_reservations: 0}) fabricates a count for an untracked stale
incarnation. The honest encoding (Option<usize> or a count-free variant)
crosses the finalized SlotEffect/RetirementMode algebra; not landable as
implementation tightening. Recorded, not actionable in this loop.

## Real-SUT loom models for entity (blocked upstream, 2026-08-11)
Adversary2 correctly found that entity/tests/loom_directory.rs models assert on
abstract protocol reimplementations, not LocalDirectory. The fix — cfg(loom)
sync swaps in directory.rs + [target.'cfg(loom)'.dependencies] loom — builds
only until bombay-behavior 0.9.1: it calls communication's Consumer::recv,
which bombay-communication gates #[cfg(not(loom))]. Any RUSTFLAGS=--cfg loom
build of bombay-entity therefore fails in an external crate. Prerequisite:
bombay-behavior must become loom-aware (upstream repo). Until then the abstract
  models stay, with docs corrected to describe them as protocol models (not
directory models). All edits reverted.

Superseded 2026-08-12 by E53: the actor protocol adapter is now a default
optional feature. A core-only `cfg(loom)` build does not compile the unrelated
upstream mailbox stack, so the real directory, slot executor, and synchronization
types are model-checked without weakening the default API.
