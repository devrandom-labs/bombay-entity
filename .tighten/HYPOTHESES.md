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
- H54 pinned nightly Miri gate: STARTED (E52). Threatened invariant: stable-only
  verification cannot detect interpreter-visible undefined behavior or invalid
  aliasing in crate-owned machinery. Workload: all three crates' library tests;
  stable remains the sole toolchain for build, Clippy, tests, docs, coverage,
  audit, and benchmarks. Change: compose the Fenix-locked latest nightly cargo,
  rustc, rust-src, and Miri components into a separate Crane derivation and run
  `cargo miri test --workspace --lib`. Falsifier: nightly affects an existing
  check, the derivation is not reproducible/offline, Miri needs exclusions that
  hide crate-owned code, or any interpreted test fails. Measurement: separate
  Nix check plus the unchanged stable full gate. Rollback: flake, docs, and ledger
  commit only.
- H55 real-directory Loom instrumentation: STARTED (E53). Threatened invariant:
  abstract protocol models can drift from `LocalDirectory` lock, slot, and
  executor code. Workload: two first dispatches racing for one absent entity and
  exact activation completion. Change: make the actor protocol adapter a default
  optional feature so a core-only `cfg(loom)` build avoids the unrelated async
  mailbox dependency; swap directory synchronization imports to Loom under that
  cfg and execute the real directory in the model. Falsifier: default API/build
  changes, production synchronization changes, the real model cannot compile,
  more than one activation starts, command ownership is lost, or the state does
  not become active. Measurement: exact Loom check, default all-target Clippy,
  and full stable gate. Rollback: feature, cfg imports, model, flake, and ledger
  commit.
- H56 linearized machine sentinel: STARTED (E54). Threatened invariant: the
  linearized executor represents an unexplained `None` machine and enforces it
  with `expect`, although its only legitimate absence is permanent transition
  poison. Workload: submit, transition panic, later dispatch, and real-executor
  Loom submit/dispatch interleavings. Change: replace `Option<M>` with the sum
  `Ready(M) | Poisoned`, installing `Poisoned` before the consuming transition
  and `Ready(successor)` afterward. Falsifier: panic/recovery behavior changes,
  output/evidence ordering changes, Loom fails, or submit benchmark regresses
  over 5%. Rollback: one driver implementation and ledger commit.
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
- H41 prehashed shard entries: REJECTED AFTER FINAL VALIDATION (E39/E46). Threatened invariants: stable-key equality, pointer-
  exact removal, reentrancy, and representative performance. Workload: activating, active,
  independent, contended, and stale directory benches plus an exact Hash-call counter. Change:
  replace shard `std::HashMap` with `hashbrown` 0.17.1 raw entries and reuse the hash already
  computed for shard selection across lookup/insertion/removal. Candidate: default features off,
  `raw-entry` + `inline-more`; MIT/Apache-2.0; MSRV 1.85. Falsifier: active dispatch+resolution does
  not reduce identifier Hash calls from four to two, any directory/runtime/loom behavior changes,
  any retained benchmark regresses >5% on min, or audit/license/MSRV gates fail. Measurements:
  controlled min/median of seven before/after plus full gate. Active dispatch Hash calls fell from
  two to one (dispatch plus resolution is structurally four to two); alternating measurements show
  about 14-16% activating/active and 13-15% contended improvements in the initial run. A final
  same-run comparison found the shipped representation 7-11% slower for independent keys; E46's
  cached-hash refinement remained about 8% slower, so the dependency and implementation were
  rolled back. Rollback: completed in E46.
- H42 post-E39 documentation consistency: KEPT (E40). Threatened invariant: research and API
  documentation identify the implementation actually shipped. Workload: every current `HashMap`
  and public-surface claim in source docs, architecture, and loop ledgers. Change: remove residual
  standard-library table wording and clarify historical “removed” means removed from the public
  surface, not deleted internally. Falsifier: any current implementation claim still identifies
  `std::collections::HashMap`, generated docs fail, or history is rewritten rather than qualified.
  Measurement: source-first search plus workspace docs. Rollback: documentation-only commit.
- H43 transition decision consumption: KEPT (E41). Threatened invariant: every pure reduction's
  successor state and effect description are installed or deliberately inspected. Workload: direct
  calls through `Reducer::reduce`, including external reducers that bypass `Decision::new` at the
  call site. Change: mark the `Decision` type itself `must_use`; representation and behavior remain
  unchanged. Falsifier: the annotation changes code generation, introduces workspace warnings, or
  cannot warn on a discarded direct reducer result. Measurement: a rustc warning probe, workspace
  tests, and all-target Clippy. Rollback: one annotation/test-evidence commit.
- H44 prehashed callback lock scope: SUPERSEDED BY E46 ROLLBACK (E42). Threatened invariant: callers know which of
  their identifier callbacks may run while the non-reentrant shard mutex is held. Workload: every
  raw lookup/insertion/removal path after E39. Change: state the implemented split precisely:
  `Hash` runs once before locking, while `Eq` runs in the raw-entry closure under the lock.
  Falsifier: a Hash invocation remains inside a guard lifetime, equality is evaluated before the
  guard, or any current doc still groups both under the mutex. Measurement: source lock-lifetime
  audit, exact Hash counter, docs, and Clippy. Rollback: documentation-only commit.
- H45 custom-hasher collision safety: KEPT (E43). Threatened invariants: stable-key equality,
  distinct activation ownership, and pointer-exact removal when `with_hasher` produces identical
  hashes. Workload: two unequal IDs forced into one hash bucket, activation and delivery for both,
  then exact retirement/removal of only one. Change: test-only constant-hasher regression for the
  directory table paths. Falsifier: IDs alias, either delivery is lost/misrouted, removing one deletes
  the other, or the test does not fail against an intentionally equality-blind lookup. Measurement:
  targeted test plus directory suite and Clippy. Rollback: test-and-ledger commit.
- H46 bounded-trace alphabet completeness: KEPT (E44). Threatened invariants: cancellation never
  mutates the wrong generation, failed deliveries retain command ownership, and stale drain facts
  cannot traverse lifecycle edges. Workload: all lifecycle event variants through depth four,
  split into current/stale and success/failure equivalence classes where behavior differs. Change:
  add the missing cancellation, stale-failure, failed-delivery, stale-fence, and stale-force inputs
  to the independent bounded trace enumeration. Falsifier: evidence/structure assertions fail,
  command-return cleanup violates the ignored-effect constraint, or runtime grows impractically.
  Measurement: exact trace test duration, lifecycle suite, coverage, and Clippy. Rollback:
  test-and-ledger commit.
- H47 poison verification determinism: KEPT (E45). Threatened invariant: a serialized handler
  panic resolves the active and every queued receipt as poisoned without making the verification
  process itself abort. Workload: reentrant submission queues a second receipt during the first
  handler, then that handler panics. Change: replace the intentional-panic Loom model (which can
  abort in generator teardown) with a deterministic real-executor std regression; retain the two
  non-panicking real-SUT Loom models. Falsifier: the queued receipt is unresolved/non-poisoned,
  later input is accepted, the std regression does not kill a drain-cleanup mutation, or either
  retained Loom model fails. Measurement: repeated exact std test, cfg-Loom suite, full gate.
  Rollback: test/documentation/ledger commit.
- H48 cached-hash table refinement: REJECTED (E46). Threatened invariants: collision-safe identity,
  pointer-exact removal, one identifier hash per operation, and representative performance.
  Workload: the E39 collision/hash regressions and all five retained directory benchmarks under an
  alternating baseline/candidate schedule. Change: replace the shard `HashMap` plus shared build
  hasher with `HashTable<(u64, EntityId, Arc<Slot>)>` so growth reuses each entry's cached full
  hash. Falsifier: any semantic/hash-count test fails, any retained minimum is more than 5% slower
  than the pre-E39 baseline, or the hot-path gain disappears. Measurement: targeted tests,
  alternating seven-repetition benchmark binaries, Clippy, and full gate. Rollback: one experiment
  commit.
- H49 superseded-passivation classification: KEPT (E47). Threatened invariant: a passivation
  caller must not claim it drained an incarnation that was replaced after observation. Workload:
  pause the call between `current_activation` and `begin_drain`, fully retire the observed
  incarnation, activate a replacement, then resume. Change: deterministic runtime regression for
  the public `Passivation::Superseded` branch. Falsifier: the call reports `Begun`, drains the
  replacement, deadlocks, or the race cannot be deterministically reproduced. Measurement: exact
  repeated test, runtime suite, Clippy, and full gate. Rollback: test-and-ledger commit.
- H50 fence-failure runtime mapping: KEPT (E48). Threatened invariant: forced retirement must
  honestly preserve whether an ordered fence failed before enqueue or while awaiting
  acknowledgement. Workload: activate, begin passivation, inject each `FenceFailure`, and observe
  the exact `RetirementMode` delivered to the runtime port. Change: record retirement modes in the
  runtime test double and cover both mappings. Falsifier: either stage collapses, the outstanding
  count is nonzero, graceful retirement occurs, or the task fails to complete. Measurement: exact
  runtime test, runtime suite, mapping mutation, Clippy, and full gate. Rollback: test-and-ledger
  commit.
- H51 activation-failure facade ownership: KEPT (E49). Threatened invariants: transactional
  activation failure returns the original command exactly once and removes the failed slot so a
  later dispatch can activate afresh. Workload: inject activation failure through the runtime port,
  await the public dispatch result, then clear the failure and retry the same entity. Change:
  focused runtime regression. Falsifier: command ownership is lost/changed, refusal is not
  `Unavailable`, retry wedges/reuses the failed activation, or the failed command is delivered.
  Measurement: exact test, runtime suite, cleanup mutation, Clippy, and full gate. Rollback:
  test-and-ledger commit.
- H52 post-rollback benchmark documentation: KEPT (E50). Threatened invariant: published
  current-tree performance evidence must describe the implementation actually shipped. Workload:
  retained directory benchmark built from exact HEAD and alternated with the pre-E39 standard-map
  binary. Change: replace rejected E39 absolute figures and derived per-operation costs in the
  architecture, and record the controlled ranges in the baseline ledger. Falsifier: any number is
  not present in raw output, any retained minimum regresses beyond 5%, or docs still describe the
  rejected candidate as current. Measurement: two alternating seven-repetition pairs plus docs
  gate. Rollback: documentation-and-ledger commit.
- H53 terminal-ledger consistency: KEPT (E51). Threatened invariant: fresh readers and automation
  must not mistake a superseded fixed-point marker or an initially validated experiment for the
  current outcome. Workload: completion markers, latest per-ID experiment status, dead-end
  preamble, and chronological reopening text. Change: qualify the old completion heading, append
  E39's final rejected-after-validation status, and remove the stale empty-list marker. Falsifier:
  an unqualified `LOOP_DONE` remains, E39's latest status is successful, or the dead-end ledger
  claims emptiness. Measurement: exact ledger searches. Rollback: ledger-only commit.
