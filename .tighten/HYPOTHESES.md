# Hypotheses (ordered by expected value; rotate domains per skill)

Each experiment: fill in Threatened invariant / Workload / Change / Falsifier / Measurements /
Rollback boundary BEFORE touching production code. One causal variable per experiment.

## P0 — contract/rule violations (safe, evidence already in hand)
- H01 errors: KEPT (E01). add thiserror `Error`+Display to DirectoryError, DispatchFailure, FenceFailure,
  LifecycleTopologyError (entity), TopologyError (transition, verify no_std support in
  thiserror 2.x), PoisonedInput (driver). Falsifier: any impl changes a public signature beyond
  adding trait impls, or breaks no_std build. Measure: gate green; check.sh score unchanged or better.
- H02 arithmetic: KEPT (E02). replace `saturating_add` in ReservationCount::reserve (lifecycle/mod.rs:156)
  and bare subs (:163, directory.rs:483) with checked_* + honest outcome. Threatened invariant:
  reservation counting exactness. Falsifier: any observable count differs on existing tests.
  Note: reserve overflow is unreachable at usize scale — first check whether the honest fix is a
  documented unreachable branch, NOT a new error variant (probe before choosing).
- H03 error discard: KEPT-MINIMAL (E03) — propagation rejected as finalized-algebra boundary. runtime.rs:347 `Err(_)` drops LocalEntityRuntime::ActivationError — wrap via
  #[source]/#from into the activation-failure path. Falsifier: changes which failures reach the
  dispatch caller.
- H04 atomics: KEPT (E04) — proof documented. directory.rs:499 `fetch_update(Relaxed, Relaxed)` — write the structural proof
  (shard mutex synchronizes all slot state; IDs are opaque tokens) as a doc comment, or switch
  to Acquire/Release if proof fails. Falsifier: proof requires happens-before the code lacks.

## P1 — type-state / boolean elimination (contract: no boolean state/results)
- H05 passivate bool: KEPT (E05) — Passivation enum, migration in commit. `EntityRuntime::passivate -> bool` (runtime.rs:244) is ambiguous AND racy
  (stale-read true, directory.rs:375). Replace with enum outcome (Begun / NoActivation /
  Ignored-stale). Public seam closure — document migration. Add race characterization test FIRST.
- H06 DispatchWait: KEPT (E06) — CompletionState sum type..completed bool (runtime.rs:266): derive from completion state; remove driftable
  flag. Falsifier: double-completion or missed wake in loom/test.
- H07 driver bool fields: KEPT (E07). SerializedExecution{running,poisoned} → single enum (removes illegal
  running+poisoned); armed/dispatching/acquired → guard-token or Option<MutexGuard> encoding.
  Falsifier: any loom/executor test changes outcome; poison semantics must stay identical.

## P2 — sentinel/duplication removal
- H08 DirectoryOutput: KEPT (E09)..dispatch_id Option → split DispatchOutput vs CallbackOutput types; deletes
  runtime.rs:222 expect. Falsifier: any callback path needs a dispatch id.
- H09 executor: REJECTED at analysis — see DEAD_ENDS. `machine: Option<M>` mid-step sentinel → StepInProgress-style encoding; deletes
  "executor machine missing" expects (driver:219, :305). Check reentrancy semantics first.
- H10 Choice: KEPT (E10) — alias removed, migration in commit.=Routed alias removal (transition machine.rs:292) — single name; document migration.
- H11 LifecycleModel: KEPT (E11) — folded into LIFECYCLE_TOPOLOGY + is_declared. newtype vs direct Topology use (machine.rs:222) — keep only if it carries
  validation evidence; else fold. Probe consumers first.
- H12 TransitionEvidence: REJECTED by contract — evidence taxonomy is finalized. SelfLoop vs Ignored (machine.rs:150) — probe whether the distinction is
  load-bearing in directory/runtime before touching; likely semantic, keep if so.

## P3 — machinery/dependency research (benchmark-gated)
- H13 SlotEffectBatch: REVERTED (E16) — 5-9% bench regression; see DEAD_ENDS. → smallvec SmallVec<[SlotEffect;1]>? Requires: no_std? (entity is std — ok),
  MSRV 1.96, license, bench parity on lifecycle benches + directory hot key. Reject explicitly if
  parity fails. Note contract: dependency must reduce total burden, not just move lines.
- H14 runtime Completion: REJECTED at analysis — see DEAD_ENDS. → oneshot channel (tokio already in tree). Verify wake semantics +
  cancel-on-drop parity with runtime.rs tests.
- H15 driver TurnCompletion: KEPT (E15). notify_all → notify_one (unique waiter per receipt). Trivial, loom-gated.
- H16 dashmap: REJECTED — reentrancy conflict, see DEAD_ENDS. vs sharded std Mutex<HashMap>: contended bench (190ms baseline) is the decider.
  High bar: dashmap semantic fit (no remove-if with slot identity check? we need
  compare-and-remove on (Arc ptr, ActivationId)) — verify entry API supports it atomically.

## P4 — concurrency/perf deep probes (research first)
- H17 LinearizedExecutor poison: REJECTED by contract — documented semantics. asymmetry vs SerializedExecutor recovery. WARNING: panic
  guarantees are finalized contract — research whether propagation is documented intent before
  proposing anything. Possibly doc-only outcome.
- H18 endpoint clone: REJECTED by contract — finalized algebra. per Deliver effect (lifecycle/mod.rs:650,704,785,887) — E: Clone bound on
  SlotReducer. Measure clone cost in benches first; consider Arc-based capability sharing only
  if measurable. Threatened invariant: affine ownership — lease must NOT be shared; endpoint may.
- H19 LinearizedExecutor evidence: REJECTED — see DEAD_ENDS. double-store + clone (driver:274,308,328) — return-only or
  shared-ref accessor. Bench submit path.
- H20 protocol: REJECTED — cold path, port-mandated..rs:139 vec![] per fence ack — rare path; likely reject after measurement.

## P5 — verification backfill (tests only, semantics untouched)
- H21: KEPT (E17).init/preparation failure → Inactive cleanup test (doc invariant 3).
- H22: KEPT (E17).reservation-start vs BeginDrain race loom test (invariant 4).
- H23: KEPT (E17).graceful retirement blocked pre-ack negative test (invariant 6).
- H24: KEPT (E17).Refusal::Busy bounded admission test; FenceFailure variant tests.
- H25: KEPT (E17).executor poison-path loom models.
- H26 bench methodology: KEPT (E12).: benches are single-run raw timing; add repetition/noise bounds before
  accepting ANY perf hypothesis (prerequisite for P3/P4).

## P6 — hygiene
- H27 workspace: KEPT (E13). Cargo.toml missing name/version → 3 crane eval warnings in gate. Add
  workspace.package.version. Zero-risk.
- H28 redundant: KEPT (E14). type annotation runtime.rs:165 (turbofish suffices); driver redundant where-clause
  repetition (:320-322). Contract: minimize explicit type specification where inference is clear.

## Explicitly deferred / rejected at intake
- Fuzz target: docs justify absence (finite alphabet enumerated + loom). Reopen only if H21-H25
  reveal counterexample paths.
- Miri: unavailable on pinned stable 1.96 toolchain. Not actionable.
- EntityBehavior/forward_optional_event! folding: blocked on behavior-crate capabilities
  (external dep) — research only, no change in this workspace.
- H29 parking_lot: user rule prefers parking_lot::Mutex where lock results are immediately
  unwrapped. Production driver relies on std poison semantics (SerializedExecutor recovery) and
  entity escalates poison deliberately — do NOT migrate those. Candidate scope: test harnesses
  only, as one wholesale convention migration (never mixed). Research loom interaction first:
  loom tests require loom::sync types under cfg(loom).
- H30 loom gate wiring: KEPT (E18) — entity-loom check runs driver loom tests in nix flake check.
