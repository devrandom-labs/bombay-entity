# Tighten Loop Progress

## State (2026-08-11, preparation phase)
- Branch: chore/bombay-tighten-loop, clean tree at 419a979.
- `.tighten/` ledger created: CONTRACT.md written; INVENTORY/SOURCES/HYPOTHESES/DEAD_ENDS pending scout results.
- Gate baseline: `nix flake check -L` GREEN (16.5s wall; fmt, doc, clippy, nextest, doctest, audit, license).
- Bench baseline: running (job bg_2).
- Scouts running: EntityScout, KernelScout, EvidenceScout (read-only inventory of all crates/tests/benches/docs).

## Baseline metrics (pre-loop)
- production_lines = 4468 (all crates src, incl. in-file tests)
- boolean_tokens = 14 (driver 6, entity 8) — see HYPOTHESES for classification
- custom_error_impls = 0 (no manual std/core Error impls)
- unsafe = 0
- pub item counts: entity 89 (lib 9, directory 19, lifecycle 47+0, protocol 7, runtime 7), transition 29, driver 16
- external deps: bombay-behavior 0.9.1 (+macros), bombay-communication 0.1.0, tokio 1.53.1, thiserror 2.0.20, pin-project-lite (transitive); dev: loom 0.7.2
- check.sh SCORE formula: 1000000 - lines - 100*bool - 100*error_impls → baseline = 1000000-4468-1400-0 = 994132

## Decisions
- /loop extension not found on disk; operator started the loop manually. This session performs prep per plugins/bombay-tighten-loop/prompts/bombay-tighten.md.
- Baseline check command: `plugins/bombay-tighten-loop/scripts/check.sh`.

## Next steps
1. Collect scout reports → write INVENTORY.md, HYPOTHESES.md, SOURCES.md, DEAD_ENDS.md (empty with header).
2. Record bench baseline into .tighten/BASELINE.md when bg_2 finishes.
3. Write GOAL.md + hand off `/loop goal … --check …` + `/loop run` invocation.

## Loop iterations (2026-08-11)
- E01 KEPT 633afba: thiserror::Error on all 6 error types (default-features=false, no_std safe).
- E02 KEPT be05f8e: checked arithmetic (shard mask precomputed, reserve/resolve checked).
- E03 KEPT-MIN 118af4d: named ActivationError discard + boundary comment; deeper propagation rejected (finalized algebra).
- E04 KEPT 118af4d: structural Relaxed proof documented on allocate().
- E05 KEPT 3d5db92: passivate -> Passivation enum (Begun/NotActive/AlreadyPassivating/Superseded); public seam closure, migration in commit.
- E06 KEPT 3e75854: CompletionState Awaiting/Ready/Consumed; completed flag removed.
- E07 KEPT f70ec7f: driver TurnState/DispatchState enums; Option-token guards; armed/acquired/running/poisoned/dispatching gone.
- Gate green after every experiment. SCORE 994132 -> 994701.
- Discovery: loom tests are NOT wired into nix gate (cfg loom never set there); driver loom tests run via RUSTFLAGS='--cfg loom' cargo test --lib. Verification-gap hypothesis material.
- Rule note: rs-parking-lot user rule recorded as H29 (test-harness-only migration candidate); rs-future-prelude rule applied (Future from 2024 prelude).

## Next
P2 sentinels: H08 (DirectoryOutput.dispatch_id Option split), H09 (machine: Option<M> mid-step), H10-H12. Then H26 bench-noise prerequisite before P3/P4 perf work. New: dispatch_pending -> bool is a boolean protocol result (convert to enum or justify).

## Convergence round 1 (2026-08-11)
- E18 62ee9e7: entity-loom gate check. E19 REVERTED 86f873a (operator dismissed parking_lot rule).
- E20 8f9be92: dispatch move-only control flow, 4 expects gone.
- Adversarial pass 1 FALSIFIED fixed point with 8 findings; handled:
  E21 6201a1a fix(executor): dispatch guard drop poison-abort (falsifier SIGABRT-verified).
  E22-E26 3c70501: must_use seam, export trim, doc diagram, len proof.
  E23 75a39e2: removable_activation duplicated authority removed.
  E24 356cf4a: absent-slot stack reduce (stale callbacks -70%, hot paths parity).
  H32 recorded contract-blocked. Loom panic falsifiers infeasible (generator TLS abort) — std-gate tests instead.
- Final A/B vs baseline: active parity, contended within band, stale -70%. SCORE 994132 -> 994895.
- Next: second fresh whole-workspace audit; if clean, verify contract items and declare candidate fixed point.

## Contract verification (convergence round 2, in progress)
- Crate trio/algebra/events/effects/ordering: unchanged; bounded trace enumeration + topology
  evidence tests green in gate. ReservationCount arithmetic (E02) is semantics-identical.
- Generation safety / affine ownership / cancellation / reentrancy: covered by stale-*,
  exact_termination, canceling_dispatch, reentrant_delivery tests — all green.
- Panic guarantees: E21 removed an undocumented abort, restoring the documented panic-only
  contract; falsifier aborts without the fix.
- Booleans: only predicate returns remain (5 tokens, all is_*/contains/handles-style).
- Errors: all six failure domains thiserror-derived; core::error::Error via default-features=false.
- no_std/MSRV: transition no_std, workspace 1.96 pinned, gate green.
- Public surface: LifecycleModel/lifecycle_model and Choice were removed;
  ActivationWaiter/DrainProgress/ReservationCount were unexported while remaining internal;
  DispatchOutput/Passivation/DispatchOutcome were added as honest types.
- Performance: repeated-harness A/B in E24 entry; hot paths at parity, stale callbacks -70%.

## Convergence round 2 (2026-08-11)
Adversary2 found 10 items; all handled:
- E25 744ab3c fix(lifecycle): late failed deliveries dropped in Retiring/Inactive/Activating
  violated delivered-or-returned ownership; both falsifier tests verified against pre-fix code.
- E26 BLOCKED-UPSTREAM: real-SUT entity loom models need bombay-behavior loom-awareness
  (communication gates recv cfg(not(loom))); swap reverted, dead end recorded, docs corrected.
- E27 3fc79e5: DispatchId hardening, DrainProgress NonZeroUsize, DuplicateTransition message,
  validator admission-reopening coverage (all four edge shapes, falsifier test).
- E28 c822663: docs aligned (diagrams, FenceFailure, DrainPolicy/telemetry forward-looking,
  verification counts, spawn run-to-completion, submit handler substitution).
- Operator guidance: full nix gate is expensive — use cargo directly (direnv) per experiment,
  gate once per convergence round.
- Audit counter: round 1 found 8, round 2 found 10 — fixed-point requires two consecutive
  clean fresh audits; audit 3 spawned.

## Convergence round 3 (2026-08-11)
Adversary3 verdict: correct — no HARD findings. Three P3s handled in E29 (bab167e+d401122):
doc overclaim on DispatchId forging, stale dual-removal doc clause, and a new pinned invariant
(ignored inputs emit only cleanup effects; the assertion surfaced sanctioned Retire-under-Ignored).
Audit counter: round 3 = clean #1 (P3-only). Round 4 spawned as clean #2 candidate.

## FIXED POINT REACHED (2026-08-11)
Convergence protocol satisfied:
- Two consecutive fresh whole-workspace audits clean: round 3 (P3-only, fixed in E29/E30),
  round 4 (clean at HARD level; falsification attempts on removal authority, transient path,
  poison recovery, completion protocol all failed to break the design).
- Contract verified item-by-item (see round 2 section).
- Full Nix gate green incl. entity-loom; 56/56 nextest; clippy/fmt/doc/doctest/audit/deny green.
- Final metrics: controlled same-conditions A/B vs baseline commit — all workloads equal or
  better; stale callbacks -57%. Recorded in BASELINE.md.
- 30 experiments: E01-E30; kept 22, reverted 2 (E16 smallvec benchmark regression, E19
  operator-rejected parking_lot), blocked-upstream 1 (E26 entity real-SUT loom needs
  bombay-behavior loom-awareness), analysis-rejected remainder with evidence in DEAD_ENDS.md.
- Residual known-accepted: see DEAD_ENDS.md (H32 algebra sentinel, E26 upstream block).

SCORE: 994132 -> 994825 (formula-bounded; bool tokens 14 -> 5, all predicates).

## Endless-loop follow-ups (2026-08-12)
- E31: coverage made a first-class Nix check with a measured 93.2% line floor; workspace report
  is 93.22% (2160/2317). Added a Retiring-state characterization for dispatch rejection and stale
  activation cleanup discovered from the coverage report.
- E32: refreshed `docs/architecture.mdx` from the retained seven-repetition benches and linked the
  controlled baseline comparison; repaired SOURCES.md with versions, access dates, applicability,
  and limitations for the primary sources actually used by the loop.
- Remaining backlog item is E26's upstream-blocked real-SUT Loom integration; no actionable local
  hypothesis remains pending final gate and adversarial audit.

## Taken-over OMP loop (2026-08-12)
- E33-E38 pinned exhaustive protocol forwarding, the true dispatch-cancellation
  boundary, pointer-only map removal authority, activation-cancellation
  observation, the matching Loom model, and the shard-lock callback boundary.
- E39 retained prehashed `hashbrown` shard entries. Active dispatch hashes once
  instead of twice; alternating seven-repetition runs improved activating and
  active hot keys by about 14-16% and contention by about 13-15%, while the
  independent and stale minima stayed inside the 5% noise bound.
- E39 full `check.sh` gate green: build, fmt, strict Clippy, 61/61 nextest,
  Loom, doctests, docs, audit, deny, and the 93.2% coverage floor.
- E40 opened after the next source-first pass found residual standard-library
  `HashMap` wording and ambiguous removed-vs-unexported historical wording.
- E41 made `Decision<S, F>` itself `must_use`, closing the direct `Reducer::reduce`
  discard seam; an exact compile-fail doctest proves the warning contract.

## Fresh audit round 2 (post-E41, clean #1 candidate)
- Clean source-first structural pass across transition topology validation,
  no-std boundaries, protocol composition, Nix check wiring, and documentation-
  to-test claims; no actionable local defect or unsupported claim found.
- Focused evidence green: transition no-default-features check and five unit
  tests, transition compile-fail doctest, and all three protocol tests.
- An optional foreign-system `nix flake check --no-build --all-systems` probe
  could not evaluate Linux Fenix derivations from this Darwin store. It is not
  the required native gate and is not counted as either pass or failure; the
  native ten-check gate remains the portability evidence available here.
- Clean counter: 1. Next pass must use an independent adversarial lens.
- Adversarial lock-scope pass falsified the streak: E39 moved `Hash` before the
  shard lock but residual E38/E40 wording still grouped it with in-lock `Eq`.
  Clean counter reset to 0; E42 corrects the exact split.
- E43 pins the untested `with_hasher` collision seam introduced by E39: two
  constant-hash IDs retain distinct activations/deliveries, exact removal
  deletes only its captured slot, and an equality-blind mutation is killed.

## Fresh audit round 3 (post-E43, clean #1)
- Clean dependency/allocation pass: no duplicate normal dependencies;
  `hashbrown` resolves once with only `inline-more,raw-entry`; workspace package
  metadata retains Rust 1.96 and MIT OR Apache-2.0.
- The retained dhat regression observes exactly 0 blocks and 0 bytes across
  10,000 active dispatch-and-resolution operations on the E43 tree.
- No feature, dependency, allocation, or evidence-ledger mismatch was found.
  Clean counter: 1. A different whole-workspace lens is required next.
- The next lifecycle-evidence lens falsified that streak: E44 found the trace
  alphabet omitted cancellation and several stale/failure ownership classes.
  It now explores 19 equivalence classes through depth four in 0.09s. Clean
  counter reset to 0.
- The post-E44 synchronization pass found the intentional-panic serialized
  executor Loom model can abort during Loom generator teardown, matching the
  limitation already observed in E21. E45 moves queued-receipt poison proof to
  a deterministic real-executor std regression and retains two non-panicking
  real-SUT Loom models. Clean counter remains 0.

## Fresh audit round 4 (post-E45, clean #1)
- Clean public-contract pass across exports, type-level `must_use` boundaries,
  error/source ownership, cancellation promises, panic clauses, and current
  documentation claims. No actionable local mismatch found.
- E45's current-tree evidence covers seven real std executor tests, two
  non-panicking real-SUT Loom models, and strict driver Clippy. A separate fresh
  rustdoc/runtime command was not counted because an unrelated Cargo research
  build left build scripts sleeping; only this session's processes were stopped.
- Clean counter: 1. Full native Nix docs/doctests/runtime coverage remains a
  mandatory convergence gate rather than inferred from this audit.

## Fresh audit round 5 (post-E45, clean #2)
- Clean minimality/performance pass across every production clone/allocation,
  custom collection, public/internal representation, dependency, allow-list,
  and retained rejected alternative. No actionable simplification or
  unsupported performance claim found.
- `SharedBuildHasher`'s one construction-time Arc preserves arbitrary non-Clone
  `BuildHasher` support; active dispatch remains allocation-free; endpoint,
  entity-ID, and waker clones cross documented ownership boundaries.
- Consecutive clean counter: 2. This is a convergence candidate only; a fresh
  adversarial pass, final benchmarks, and the complete native Nix gate remain.
- The loop remains active. These are completed experiments, not a convergence
  declaration; fresh whole-workspace clean-audit counting restarts after E39.

## Prior fixed-point declaration (2026-08-12; later reopened)
- Full `plugins/bombay-tighten-loop/scripts/check.sh` green: all 10 flake checks, 58/58 nextest,
  driver real-SUT Loom, entity protocol Loom models, fmt, strict Clippy, docs/doctests, audit,
  license/deny, build, and 93.2% coverage floor.
- Retained benches re-measured over seven repetitions; values are recorded in BASELINE.md and the
  architecture research basis. Hot-path results remain within the prior controlled comparison's
  5% decision band.
- Final source-first audit rechecked public seams, booleans, options/expect paths, allocations,
  clones, locks, queues, dependencies, unsafe code, and documentation. It found one stale inventory
  coverage statement, corrected in E32, and no untested actionable high-value hypothesis.
- The only open-looking backlog entry is E26, explicitly blocked by bombay-behavior 0.9.1's lack of
  Loom support and recorded in DEAD_ENDS.md. Completion does not claim that upstream work is done.

## Fresh audit round 1 (2026-08-12, active goal takeover)
- Prior LOOP_DONE is historical evidence only; convergence counter reset.
- Effects/panic candidate rejected by contract: interpreter panic dropping its current owned output
  is the documented LinearizedExecutor guarantee, so retrying remaining effects would reinterpret
  finalized panic semantics.
- E33 kept: all seven macro-generated optional behavior-system event lanes now have field-preserving
  characterization coverage. Targeted test and all-target entity Clippy are green; production code
  is unchanged.
- Next lens: concurrency and completion/cancellation interleavings, starting from real runtime code
  rather than the abstract entity Loom protocol models.
- E34 kept: a real-runtime gated-delivery test located the cancellation linearization boundary.
  Dropping during activation cancels the waiter; dropping after active delivery owns the command
  cannot retract delivery. Corrected the architecture/API overclaim; production unchanged.
- E35 kept: fresh public-authority audit found the architecture still described a removed dual
  removal condition. It now states the implemented split: slot pointer identity authorizes map
  removal; ActivationId rejects stale lifecycle facts within the captured slot. Regression and
  docs green.
- E36 kept: the activation-cancellation regression previously asserted before the gated activation
  result necessarily reached the directory. It now observes activation return and completes a
  later valid witness dispatch before proving only that witness was delivered. Five isolated
  repetitions, all runtime tests, and Clippy green. The isolated target avoided interference from
  an unrelated concurrent Cargo build in the shared target directory.
- E37 kept: the delayed-removal abstract Loom model encoded the obsolete `(slot, activation)`
  conjunction. It now isolates pointer identity, while separate stale-activation and stale-
  termination regressions prove ActivationId authority. Invariant 8 was split accordingly; targeted
  tests and Clippy green.
- E38 kept: before E39, invariant 10 overclaimed that no application code ran under directory
  locks. Standard HashMap lookup/insertion invoked user-defined EntityId Hash/Eq under the shard
  mutex; only effect and runtime callbacks were guaranteed outside it. E42 records the narrower
  post-E39 split. Public API and architecture state the applicable non-reentrancy obligation.
- Final alternating benchmark validation falsified E39's shipped representation: hot-key and
  contention gains held, but independent-key minima were 7-11% slower than the same-run pre-E39
  baseline, beyond the declared 5% decision band. Convergence is revoked and the clean counter is
  reset to zero. E46's cached-full-hash `HashTable` refinement remained about
  8-9% slower on independent keys, so both E39 and E46 were rolled back to the
  standard-library table and the added dependency was removed.

## Fresh audit round 5 (post-E46 rollback, clean #1)
- Re-read the full three-crate production surface and every manifest from the
  standard-library-table tree. Public items remain documented and scoped to the
  transition algebra, executor ownership protocol, lifecycle facts, directory
  effects, and runtime facade; no new boolean/sentinel/error seam or unsafe code
  was found.
- Re-audited every production `expect`, clone, allocation, mutex, atomic, and
  callback boundary. Panics remain poison/impossible-state assertions, clones
  transfer documented ownership, active dispatch remains allocation-free, and
  relaxed counters require uniqueness only. The rollback restores the already
  characterized two-hash standard-map path without changing lifecycle logic.
- Reconciled manifests, lockfile, inventory, API docs, architecture, hypothesis
  status, and benchmark ledger after removing `hashbrown`; remaining mentions
  are explicitly historical rejected-experiment evidence.
- No untested actionable high-value hypothesis emerged. Consecutive clean
  counter: 1. The next audit must use an independent verification-coverage lens.
- The verification-coverage lens falsified that streak: the public
  `Passivation::Superseded` branch had no direct runtime regression. E47 now
  pauses a passivation after activation observation, replaces the incarnation,
  resumes the stale drain request, and proves `Superseded` without draining the
  replacement. Five exact runs, all eight runtime tests, strict entity Clippy,
  and an effective classification mutation are green. Clean counter reset to 0.
- The continued runtime-port matrix found neither `FenceFailure` was exercised
  through `EntityRuntime`. E48 records the retirement mode and proves enqueue
  and acknowledgement failures preserve their distinct forced-drain stages;
  collapsing the mapping is killed. All nine runtime tests and strict entity
  Clippy are green. Clean counter remains 0.
- E49 closes the activation-error facade seam: injected transactional failure
  returns the exact command as `Unavailable`, delivers nothing, removes the
  failed slot, and a retry performs a fresh activation. Removing the cleanup
  effect is killed by the retry assertion. All ten runtime tests and strict
  entity Clippy are green. Clean counter remains 0.

## Fresh audit round 6 (post-E49, clean #1)
- Completed the independent verification matrix across every lifecycle event
  variant and behavior-distinct generation class, all six effects, all four
  passivation outcomes, both fence failures, activation/delivery failure and
  cancellation ownership, every protocol forwarding family, topology defects,
  executor ownership outcomes, and composition forms.
- Direct runtime tests now cover every public passivation and runtime-port
  outcome; the 19-class bounded trace and directory regressions cover stale and
  phase-specific lifecycle equivalence classes without duplicating them at the
  facade. Exhaustion errors are structurally private counter terminal states and
  retain direct formatting/ownership variants; no counterexample path warrants
  a test-only production seam.
- No further untested actionable high-value hypothesis emerged. Consecutive
  clean counter: 1. The next audit switches to dependency, allocation, and
  synchronization minimality.
- The minimality pass found no duplicate dependency versions (`cargo tree
  --duplicates` is empty), unnecessary production ownership path, or new
  allocation. Exact current-tree benchmarks stayed within 5% of the retained
  standard-map binary, but the architecture still printed rejected E39
  absolute figures. E50 corrects that evidence; clean counter resets to 0.

## Fresh audit round 7 (post-E50, clean #1)
- Repeated the dependency, allocation, and synchronization-minimality pass on
  exact HEAD. `cargo tree --workspace --duplicates` is empty; production has no
  unsafe or lint suppression; every queue, mutex, atomic, `Arc`, clone, boxed
  transient output, and multi-effect vector still carries a documented
  ownership, ordering, or representation requirement.
- Rechecked manifests, lockfile, inventory, architecture, and current benchmark
  claims after E50. `hashbrown` remains absent from dependency state and appears
  only in explicitly rejected historical evidence; no rejected absolute figure
  remains in current documentation.
- No untested actionable high-value hypothesis emerged. Consecutive clean
  counter: 1. The next pass uses a fresh public-contract and failure-surface
  lens.

## Fresh audit round 8 (post-E50, clean #2)
- Re-read every exported type, constructor, operation, error, panic contract,
  lifecycle invariant, and README/architecture guarantee against its current
  implementation and direct verification. The public surface still exposes no
  provisional actor capability, hidden retry promise, or stale-generation
  authority; command and lease ownership claims match all success/failure paths.
- All four typed-error integration tests, the `Decision` compile-fail doctest,
  workspace doctests, and workspace documentation with `-D warnings` are green.
  An initial `cargo rustdoc --workspace` probe used an unsupported Cargo flag;
  the supported `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps`
  command supplied the intended evidence and passed.
- No untested actionable high-value hypothesis emerged. Consecutive clean
  counter: 2. This is only a convergence candidate; a separate final
  adversarial falsification pass and complete native gate remain mandatory.
- The final bookkeeping adversary found the older `LOOP_DONE` heading and
  E39's last same-ID experiment status could still be read as current success
  despite the later rollback. E51 makes the historical heading explicit, adds
  an append-only terminal E39 rejection status, and removes the stale
  `DEAD_ENDS` “none yet” placeholder. Clean counter resets to 0.

## Fresh audit round 9 (post-E51, clean #1)
- Rechecked the chronological experiment ledger, hypothesis terminal states,
  dead-end prerequisites, completion headings, rollback commit, dependency
  state, and benchmark evidence as one consistency graph. Every started
  experiment has a later terminal record; E39's latest same-ID status is the
  post-validation rejection; E46 records the failed refinement; E50 records the
  restored final tree; no current completion heading exists.
- The only blocked work remains real-`LocalDirectory` Loom instrumentation,
  whose upstream `bombay-behavior`/`bombay-communication` cfg incompatibility is
  unchanged and cannot be resolved inside this semantics-preserving workspace.
- No untested actionable high-value hypothesis emerged. Consecutive clean
  counter: 1. The next pass independently rechecks test machinery and production
  invariants rather than ledger state.

## Fresh audit round 10 (post-E51, clean #2)
- Rebuilt the verification inventory from the test binaries rather than prior
  ledger conclusions: 17 entity unit tests, 15 real-directory integration tests,
  10 runtime-facade tests, five protocol Loom models, two real-directory stress
  tests, the allocation ceiling, four typed-error tests, seven executor tests,
  five transition tests, and the `Decision` compile-fail doctest.
- Cross-checked that inventory against every documented lifecycle phase, all 19
  behavior-distinct event classes in the bounded trace, effect ordering and
  ownership, generation and stable-binding authority, cancellation boundaries,
  passivation and fence outcomes, reentrancy, poison recovery, and public error
  ownership. Mutation evidence retained in E43 and E47-E49 demonstrates that the
  newest facade classifications and cleanup assertions are effective.
- The five entity Loom tests honestly model the concurrency protocols rather than
  claiming production instrumentation. Real `LocalDirectory` substitution remains
  blocked by the recorded upstream cfg/type incompatibility; real-directory stress
  tests and deterministic race regressions supply the applicable in-workspace
  evidence without weakening that limitation.
- No untested actionable high-value hypothesis emerged. Consecutive clean counter:
  2. The candidate fixed point now proceeds to a separate adversarial falsification
  pass starting from source, contract, and measurements.

## Final adversarial falsification pass (post-E51)
- Started from the immutable contract, production source, public surface, and
  current measurements rather than the clean-audit conclusions. Challenged one
  duplicated authority, invalid representable state, caller-misuse seam,
  unnecessary allocation/clone/lock/queue/dependency, unverified panic or
  concurrency interleaving, replacement crate, and benchmark reversal.
- `cargo tree --workspace --duplicates` remains empty. The complete production
  scan has no unsafe code or lint suppression; every remaining option, vector,
  queue, mutex, atomic, `Arc`, clone, box, and `expect` is tied to the documented
  lifecycle algebra, affine ownership, bounded admission, executor poison
  protocol, callback lock boundary, or measured transient-output tradeoff.
- Seven fresh benchmark repetitions did not reverse a retained decision:
  activating/active/independent/contended/stale minima were 54.01/102.97/13.14/
  172.69/28.11 ms, and lifecycle ignored/claim minima were 10.72/16.16 ms.
  Active dispatch remains guarded by the zero-allocation regression. The current
  score is 994595 (5005 production-source lines, four boolean tokens, no custom
  error implementations), above the 994132 intake baseline; line growth is
  predominantly invariant verification colocated under `cfg(test)` and is not
  treated as an optimization proxy.
- No credible falsifier emerged. Miri remains unavailable on the pinned stable
  toolchain; the bounded finite lifecycle alphabet plus exhaustive traces makes
  a separate fuzz target non-actionable; real-directory Loom remains the
  explicitly recorded upstream-cfg dead end. Proceeding to the complete Nix gate.

## LOOP_DONE: fixed point restored after E51 (2026-08-12)
- Two consecutive fresh whole-workspace audits (rounds 9 and 10) found no
  untested actionable high-value hypothesis. A separate source-first adversarial
  pass failed to falsify completion.
- Fresh `cargo bench -p bombay-entity` seven-repetition measurements preserve the
  maintain-or-improve requirement, and the zero-allocation active-dispatch test
  remains green. Final score: 994595 versus the 994132 intake baseline.
- `plugins/bombay-tighten-loop/scripts/check.sh` is green on exact final source:
  release build, rustfmt, strict workspace/all-target Clippy, 66/66 nextest,
  two real-executor Loom models, compile-fail doctest, docs, 93.2% LLVM line
  coverage floor, RustSec audit, and cargo-deny dependency/license/source policy.
- Applicable stress, allocation, exhaustive bounded traces, mutation evidence,
  benchmarks, docs, audit, and license checks are complete. Miri, fuzz, and real
  `LocalDirectory` Loom limitations remain explicitly justified above and in the
  durable dead-end ledger rather than being represented as executed evidence.
- The implementation-tightening loop is complete. Reopen only for a new contract,
  changed dependency/toolchain evidence, a reproducible counterexample, or a new
  representative benchmark that invalidates a retained decision.

## Operator-requested verification reopening (post-E51)
- E52 adds an isolated, reproducible nightly Miri check while every shipped and
  ordinary verification target remains on stable Rust 1.96. Miri passes 27
  interpreted library tests with leak detection enabled; native verification
  retains the computationally exhaustive and synthetic-static-fixture tests.
- E53 supersedes the real-directory Loom dead end. The actor protocol adapter is
  now a default optional feature, allowing a core-only model to drive the actual
  `LocalDirectory`, shard mutex, slot executor, identity allocation, activation
  commitment, and delivery effects. All schedules produce one activation and
  deliver all three commands.
- E54 replaces the linearized executor's unexplained `Option<M>` sentinel with
  `LinearizedMachine::Ready(M) | Poisoned`. The serialized executor's option is
  retained because its drain guard genuinely owns the affine machine outside
  shared storage; replacing it would rename rather than remove that state.
- Fresh directory/lifecycle minima are 50.12/98.83/11.03/182.64/26.58 and
  9.87/15.42 ms. All remain within or better than retained evidence. Strict
  Clippy, seven driver tests, two real-executor Loom models, the real-directory
  Loom model, and nightly Miri are green. Full combined gate follows.

## Reopened verification pass complete (2026-08-12)
- `plugins/bombay-tighten-loop/scripts/check.sh` is green with score 994570.
  All eleven built checks plus the cached RustSec audit pass: stable release
  build, fmt, strict all-target Clippy, 66/66 nextest, doctest, docs, 93.2%
  coverage floor, cargo-deny, two real-executor Loom models, the real-directory
  Loom model, and the isolated nightly Miri suite.
- The requested limitations are closed. Default public compatibility is
  preserved; core-only consumers gain an opt-out from the protocol adapter.
  The sole retained machine option represents real affine ownership outside the
  mutex and is not an invalid state. No further corrective hypothesis remains
  from this reopening.
