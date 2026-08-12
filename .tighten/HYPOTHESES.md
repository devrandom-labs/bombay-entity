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
- H31 absent-slot: KEPT (E24) — stack reduce + boxed Transient; stale callbacks -70%. callback allocation (adversary finding): submit_or_inactive allocates a fresh
  Arc<Slot> per stale callback. Requires a stale-callback bench + stack-reduce redesign of
  DirectoryOutput internals. Scheduled after adversary-batch experiments.
- H32 retire_stale outstanding_reservations: 0 sentinel (adversary finding): honest encoding
  crosses the finalized SlotEffect/RetirementMode algebra — recorded in DEAD_ENDS, not actionable.

## Endless-loop evidence follow-ups (2026-08-12)
- H33 coverage gate: KEPT (E31). Threatened invariant: verification coverage can regress while
  the ordinary test gate remains green. Workload: all workspace tests under source-based LLVM
  coverage. Change: make coverage a flake check at the measured 93.2% line floor and pin the
  uncovered Retiring stale-activation cleanup path. Falsifier: the check accepts a report below
  93.2%, the characterization test observes a different Reject/Retire sequence, or the full gate
  fails. Measurement: 93.22% lines (2160/2317). Rollback: the flake check and test-only commit.
- H34 research-basis measurements: KEPT (E32). Threatened invariant: published performance
  evidence describes the final implementation. Workload: the retained directory and lifecycle
  benches, seven repetitions. Change: replace pre-loop absolute numbers with dated final-tree
  min/median results and link the controlled comparison. Falsifier: documentation differs from
  raw benchmark output or BASELINE.md. Rollback: documentation-only commit.
- H35 protocol forwarding: KEPT (E33). Threatened invariant: wrapping a behavior preserves each
  optional system-event lane without reclassification or field loss. Workload: all seven
  `forward_optional_event!` expansions. Change: add one exhaustive characterization test over a
  field-preserving inner event type; production remains untouched. Falsifier: any lane returns
  `None`, selects the wrong `EntityEvent` variant, or changes an event field. Measurement: targeted
  protocol unit tests and coverage report. Rollback: test-and-ledger commit.
- H36 dispatch cancellation authority: KEPT (E34). Threatened invariant: documentation must not
  promise cancellation after command ownership has crossed the active-delivery linearization
  point. Workload: drop a polled dispatch future while its spawned delivery is gated. Change:
  characterize the real runtime race, then scope the architecture claim to bounded activation
  waiters if delivery continues. Falsifier: the gated command is dropped rather than delivered, or
  the existing activation-waiter cancellation behavior changes. Measurement: both cancellation
  tests plus all-target entity Clippy. Rollback: test-and-documentation commit.
- H37 removal authority documentation: KEPT (E35). Threatened invariant: exactly one mechanism
  authorizes removal of the stable map binding. Workload: delayed removal racing a replacement
  binding. Change: align the architecture identity section with `remove_matching`: activation IDs
  classify lifecycle facts inside a slot; `Arc::ptr_eq` alone authorizes map removal because a slot
  allocation is never reused. Falsifier: source contains an activation-ID removal check, or the
  delayed-removal regression permits removing a replacement. Measurement: targeted regression and
  documentation build. Rollback: documentation-and-ledger commit.
- H38 activation-cancellation observation: KEPT (E36). Threatened invariant: a command canceled
  while waiting for activation is never delivered after that activation completes. Workload: real
  runtime with activation held behind a gate. Change: distinguish activation start from completion
  in the test runtime, wait for completion, then complete a subsequent valid dispatch as a witness
  that the activation result and delivery effects were processed. Falsifier: the canceled command
  appears beside the witness command, or the witness cannot complete. Measurement: repeated
  targeted test plus all runtime tests. Rollback:
  test-and-ledger commit.
- H39 removal-model authority: KEPT (E37). Threatened invariant: each regression model proves one
  implemented authority rather than a stronger obsolete conjunction. Workload: delayed removal
  racing replacement. Change: make the abstract Loom binding model compare captured slot identity
  only and correct invariant 8; exact ActivationId rejection remains independently covered by stale
  lifecycle/directory tests. Falsifier: replacement slot can be removed, or no independent stale-ID
  regression exists. Measurement: targeted Loom protocol model plus stale activation/termination
  tests. Rollback: test-documentation-and-ledger commit.
- H40 shard-lock callback boundary: KEPT (E38). Threatened invariant: callers know exactly which
  of their code may execute under a directory mutex. Workload: shard lookup/insertion/removal for a
  user-defined `EntityId<I>`. Change: narrow invariant 10 to effect/runtime callbacks and document
  that `I::Hash`/`I::Eq` execute inside `HashMap` operations under the shard lock and must not
  reenter the directory. Falsifier: every Hash/Eq call occurs before lock acquisition, or an effect
  interpreter callback runs with a shard guard live. Measurement: source lock-scope audit, docs,
  Clippy. Rollback: documentation-and-ledger commit.
