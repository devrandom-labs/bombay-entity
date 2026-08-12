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
- Public surface: LifecycleModel/lifecycle_model, Choice, ActivationWaiter/DrainProgress/
  ReservationCount removed; DispatchOutput/Passivation/DispatchOutcome added as honest types.
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

## LOOP_DONE: evidence follow-up fixed point (2026-08-12)
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
