# Improvements backlog (endless loop mode)

Concrete items only: file path + one-line acceptance criterion. Take the top open item.

## Open

1. **Evidence-classification pin** — `crates/entity/src/lifecycle/machine.rs` (`check_trace`):
   extend the bounded-trace assertions so `Traversed` evidence implies the phase actually
   changed and `SelfLoop`/`Ignored` imply it did not. Acceptance: assertion fails if the
   `handles()` classification and reducer behavior diverge on phase change.

2. **Allocation profile** — no allocation evidence exists (BASELINE.md gap). Add a dhat-based
   test, e.g. `crates/entity/tests/allocations.rs`: dispatch+interpret on an active key stays
   under a recorded allocation ceiling. Acceptance: test fails when per-dispatch allocations
   grow; ceiling recorded in `.tighten/BASELINE.md`.

3. **Coverage gate** — `flake.nix`: `packages.coverage` exists but is not a check. Add an
   `entity-coverage` check running `cargoLlvmCov` with a summary threshold. Acceptance:
   `nix flake check` fails when coverage regresses below the recorded baseline.

4. **Research-basis numbers** — `docs/architecture.mdx` "Research basis" cites benchmark
   numbers from before the tightening loop. Re-measure and refresh or mark with the
   measurement date. Acceptance: cited numbers match `.tighten/BASELINE.md` final comparison.

5. **Entity real-SUT loom** — blocked upstream (DEAD_ENDS.md, E26). If bombay-behavior becomes
   loom-aware, redo the cfg(loom) swap in `crates/entity/src/directory.rs` and replace the
   abstract models in `crates/entity/tests/loom_directory.rs` with real-directory models.
   Acceptance: loom closures construct `LocalDirectory`.

## Done

(none yet)
