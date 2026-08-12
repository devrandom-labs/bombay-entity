# Tightening Contract (immutable)

Source: `plugins/bombay-tighten-loop/skills/bombay-tighten-loop/SKILL.md` (read 2026-08-11).

- Keep exactly the `entity`, `transition`, `driver` crate design.
- Preserve the finalized algebra, mathematics, lifecycle states, events, transitions, effects,
  ordering, generation safety, affine ownership, cancellation, reclamation, reentrancy, and
  panic guarantees.
- Implementation tightening only: no redesign, reinterpretation, replacement, weakening, or
  removal of the idea, algebra, lifecycle model, or semantics.
- Preserve every feature. Public compatibility may change only to close an unsafe or
  unnecessary seam, with migration documented.
- Minimize public API to the smallest useful capability surface; keep implementation types
  and construction details private unless external use is demonstrated.
- Minimize explicit type specification where inference is clear and robust; do not erase
  domain distinctions or capability types to shorten syntax.
- Maintain or improve representative runtime performance. LOC is not a proxy for performance
  or robustness.
- Sum types for alternatives, product types for simultaneous state. No boolean state, boolean
  protocol results, boolean mode flags, or boolean parameters.
- `thiserror` for error definitions. Verify `no_std`, MSRV, feature, build-time, and
  binary-size implications where applicable.
- Nix for all project commands. `nix flake check -L` stays green.

## Loop discipline (from references/research-protocol.md)

- One independently reviewable experiment per loop turn; one causal variable.
- Characterization/regression coverage before altering subtle invariants.
- Record falsifier, measurements, rollback boundary in HYPOTHESES.md before each experiment.
- Revert failed experiments without deleting their record (DEAD_ENDS.md).
- Rotate whole-workspace lenses: semantic authority, type design, public seams, concurrency,
  effects, machinery, dependencies, performance, verification, documentation, Rust expression,
  patterns.
- Fixed point: two consecutive fresh whole-workspace audits find no untested high-value
  hypothesis, plus adversarial falsification pass, plus full gate + Loom/Miri/fuzz/stress/
  coverage/docs/audit/license/benchmarks, plus final-vs-baseline metric comparison.
