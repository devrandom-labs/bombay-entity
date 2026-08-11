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
