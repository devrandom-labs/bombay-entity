# GOAL: Research-driven tightening of the Bombay Entity workspace to a verified fixed point

Read first, every run: `.tighten/CONTRACT.md` (immutable), `.tighten/INVENTORY.md`,
`.tighten/HYPOTHESES.md`, `.tighten/BASELINE.md`, `.tighten/PROGRESS.md`, `.tighten/DEAD_ENDS.md`,
`plugins/bombay-tighten-loop/skills/bombay-tighten-loop/references/research-protocol.md`.
Never infer history from chat when these files exist.

## Objective
Iteratively tighten the finalized three-crate workspace (transition, driver, entity) —
semantics-preserving only — until the fixed-point protocol in the skill is satisfied.

## Scope
- Implementation tightening: error types (thiserror), arithmetic safety, boolean-state
  elimination, sentinel/duplication removal, public-seam minimization, dependency replacement
  with benchmark parity, concurrency hardening with proof, verification backfill.
- One independently reviewable experiment per iteration; one causal variable.
- Characterization/regression coverage BEFORE altering subtle invariants.

## Non-goals
- No redesign, reinterpretation, or semantic change to the algebra, lifecycle model, ordering,
  generation safety, affine ownership, cancellation, reclamation, reentrancy, or panic guarantees.
- No new features; no crate additions/removals; no weakening tests/benches/invariants to pass.
- No LOC-chasing as a proxy; no dependency swaps without benchmark parity on repo workloads.

## Measurable completion criteria
- `plugins/bombay-tighten-loop/scripts/check.sh` exits 0 (includes `nix flake check -L`).
- Fixed point per skill: two consecutive fresh whole-workspace audits find no untested
  high-value hypothesis + adversarial falsification pass + final metrics vs `.tighten/BASELINE.md`.

## Milestones
1. P0 rule violations (H01–H04) — safest, evidence in hand.
2. P5 verification backfill H26 (bench noise bounds) — prerequisite for perf experiments.
3. P1 booleans (H05–H07), each with characterization tests first.
4. P2 sentinels/duplication (H08–H12).
5. P3/P4 dependency + perf research (H13–H20), benchmark-gated, DEAD_ENDS recorded.
6. Remaining P5 backfill (H21–H25), P6 hygiene (H27–H28).
7. Convergence protocol per skill.

## Quality standards
- Every kept experiment: gate green, conventional commit, EXPERIMENTS.jsonl entry with raw
  commands/results, HYPOTHESES.md status update.
- Failed experiments: revert fully, record in DEAD_ENDS.md.
- Update PROGRESS.md at each iteration end (state, decision, next step).

## Assumptions
- `.tighten/BASELINE.md` numbers are the comparison floor (M4 Pro, 2026-08-11).
- Miri/fuzz absent by documented justification; do not add unless a counterexample path emerges.
- Scout artifacts agent://EntityScout, agent://KernelScout, agent://EvidenceScout hold the
  file:line evidence behind INVENTORY.md.

Check: `plugins/bombay-tighten-loop/scripts/check.sh`
