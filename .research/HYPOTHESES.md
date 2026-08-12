# Hypotheses

Order by expected value and rotate repository domains.

## H01

- Status: retained
- Threatened invariant or cost: actor-style callers already holding exclusive ownership must currently adopt locks, queues, handlers, and receipts to drive an affine `Machine` safely across unwind.
- Workload: successful sequential turns, a panicking turn followed by refusal, non-Clone owned payloads, and batches of 1,024 sequential turns.
- Proposed change: add `ExclusiveExecutor<M>` backed by private `Ready(M) | Poisoned`, poisoning via `core::mem::replace` before calling `step`.
- Falsifier: payload duplication/leak/double-drop; successor not installed exactly; accepted input claimed recoverable; post-poison step execution; public API requiring synchronization/allocation; material unexplained overhead beyond the observable poison branch/state write; any existing gate regression.
- Measurements: targeted tests/Clippy, full `nix flake check -L`, batched direct-versus-wrapper timings with noise characterization, executor-attributable allocation inspection, and generated-code inspection if timing differs.
- Rollback boundary: only `crates/driver` exclusive-executor changes and their dedicated evidence; preserve unrelated worktree paths.
- Evidence: 12 driver unit tests including distinct ownership counters; Miri, Clippy, docs, doctests, Loom, coverage, audit, deny, build and Nextest passed through `nix flake check -L`; DHAT reports zero wrapper allocations; opaque-boundary payload benchmarks measure stable 0.131–0.159 ns/turn overhead, explained by the generated poison-state instructions.
