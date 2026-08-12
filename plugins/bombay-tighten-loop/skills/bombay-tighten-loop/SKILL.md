---
name: bombay-tighten-loop
description: Run an autonomous, research-driven OMP loop that discovers and implements semantics-preserving improvements to the Bombay Entity Rust workspace. Use when asked to fix, tighten, simplify, harden, compartmentalize, optimize, reduce custom machinery, improve interfaces, or converge the finalized three-crate design without losing features.
---

# Bombay Tighten Loop

Drive discovery; do not execute a prefabricated refactor plan. Treat prior suggestions as unproven hypotheses.

## Immutable contract

- Keep exactly the `entity`, `transition`, and `driver` crate design.
- Preserve the finalized algebra, mathematics, lifecycle states, events, transitions, effects, ordering, generation safety, affine ownership, cancellation, reclamation, reentrancy, and panic guarantees.
- Treat this exclusively as implementation tightening. Do not redesign, reinterpret, replace, weaken, or remove the underlying idea, algebra, lifecycle model, or semantics.
- Preserve every feature. Public compatibility may change only when the change deliberately closes an unsafe or unnecessary seam and migration is documented.
- Minimize the public API to the smallest useful capability surface. Keep implementation types and construction details private unless external use is a demonstrated requirement.
- Minimize explicit type specification where Rust can infer the same type clearly and robustly. Do not erase domain distinctions or capability types merely to shorten syntax.
- Maintain or improve representative runtime performance. Do not use source-line count as a substitute for performance or robustness.
- Model alternatives with sum types and simultaneous state with product types. Do not introduce boolean state, boolean protocol results, boolean mode flags, or boolean parameters.
- Use `thiserror` for error definitions. Verify `no_std`, MSRV, feature, build-time, and binary-size implications where applicable.
- Use Nix for all project commands. Keep `nix flake check -L` green.

Read [references/research-protocol.md](references/research-protocol.md) completely before the first mutation and after context recovery.

## Start or resume

1. Inspect `AGENTS.md`, the entire workspace, git state, public API, tests, benchmarks, docs, and prior durable loop files.
2. Create `.tighten/` and maintain `CONTRACT.md`, `INVENTORY.md`, `SOURCES.md`, `HYPOTHESES.md`, `EXPERIMENTS.jsonl`, `PROGRESS.md`, and `DEAD_ENDS.md` there. Never infer history from chat when these files exist.
3. Establish behavior, performance, allocation, code-size, compile-time, public-surface, and production-code baselines before refactoring.
4. Use current primary sources: official Rust and crate documentation/source, standards, original papers, and upstream issue/maintenance records. Record URLs, versions or commits, access dates, applicability, and limitations.
5. Inventory all handcrafted mechanisms and search maintained crates for replacements. Compare semantics, ownership, `no_std`, MSRV, dependencies, licenses, audit status, maintenance, performance, code size, and compile time. Benchmark viable candidates. Reject weak fits explicitly.
6. Review current idiomatic Rust design patterns and applicable Gang-of-Four patterns. Use a pattern only when it removes invalid states, public seams, duplication, or machinery; reject ceremonial pattern translation.
7. Form hypotheses from repository evidence. Include discoveries beyond naming, booleans, errors, dependency replacement, and known patterns.

## Iteration

Perform one independently reviewable experiment per loop turn:

1. Select the highest-value unresolved hypothesis; rotate domains to avoid local optimization.
2. State the threatened invariant, workload, proposed change, falsifier, measurements, and rollback boundary in `HYPOTHESES.md`.
3. Add characterization, property, stress, Loom, Miri, fuzz, or benchmark coverage needed to detect semantic drift before changing production code.
4. Implement the smallest coherent experiment.
5. Run targeted checks, then the relevant adversarial checks and benchmarks. Use repeated measurements and account for noise.
6. Keep only changes with evidence of equal semantics and a net improvement in robustness, compartmentalization, performance, surface area, or justified simplicity. Revert failed experiments without deleting their record.
7. Record the result and commit a kept experiment with a concise Conventional Commit message.
8. Re-audit the whole workspace after each kept pass; do not remain in one module indefinitely.

Never optimize a proxy alone. Reduced LOC is beneficial only when authority, state space, or machinery is actually removed. Do not replace local code with a dependency merely to move lines elsewhere.

## Convergence

Declare a candidate fixed point only after two consecutive whole-workspace audits, begun from fresh inventories, find no untested high-value hypothesis. Before declaring it:

- verify every contract item against tests or explicit reasoning;
- run the full Nix gate plus applicable Loom, Miri, fuzz, stress, coverage, docs, audit, license, and benchmark suites;
- compare final metrics to the recorded baseline;
- audit all public seams, allocations, clones, locks, queues, options, booleans, custom errors, unsafe code, dependencies, duplicated representations, and unreachable/expect paths;
- ask an adversarial review pass to falsify completion without showing it the proposed conclusions.

In endless loop mode, write `LOOP_DONE:` only after this protocol. Continue by reopening the weakest evidence or researching a new candidate until the operator stops the loop.

## OMP integration

Use the installed `pi-loop-mode` for unattended iteration and `pi-autoresearch` when a hypothesis has a measurable optimization target. The repository check command is:

```text
plugins/bombay-tighten-loop/scripts/check.sh
```

Do not let a passing check claim convergence; it is backpressure, not proof of a fixed point.
