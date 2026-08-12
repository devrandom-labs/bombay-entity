# Immutable contract

## Objective

Add an allocation-free `ExclusiveExecutor<M>` to `bombay-machine-executor` for callers that serialize affine machine turns through exclusive `&mut` access.

## Preserved behavior and guarantees

- `Machine::step(self, input)` remains the sole transition authority.
- One successful `turn` accepts exactly one input, returns exactly one output unchanged, and installs exactly the returned successor.
- The executor installs poison before entering `step`; unwind permanently withdraws the consumed machine.
- Only inputs offered after poison are recoverable; an accepted panicking input is consumed normally.
- No later transition executes after poison, and poisoned state exposes no machine.
- The poison boundary excludes later output consumption or actor/environment interpretation.
- Existing serialized and linearized execution laws and APIs remain unchanged.
- Safe reentrancy remains excluded by `turn(&mut self, ...)`.
- Structural auto-traits follow `M`; no lock or negative auto-trait implementation is added.

## Performance contract

- `turn` performs no executor-owned allocation, lock, queue, clone, receipt, or dynamic dispatch.
- Representative workload: 1,024 sequential turns for a no-op/state-update machine and owned output payload.
- Compare batched direct affine stepping with batched exclusive execution; establish noise before setting any numeric regression threshold.

## Operator constraints

- Use only `core` machinery for the new primitive; do not claim the existing crate is `no_std`.
- Use the existing `PoisonedInput<I>` rejection algebra.
- Model the private seat as `Ready(M) | Poisoned`; do not add a retained stepping state or unsafe recovery API.
- Preserve unrelated worktree changes, including `result` and `.serena/`.
- Use Nix for all Cargo and verification commands.

## Rust and Nix conventions

- Use Nix for development and verification.
- Prefer sum/product modeling over boolean or sentinel protocols.
- Use `thiserror` unless an evidenced constraint prevents it.
- Minimize public API and redundant type specification without erasing domain types.
- Preserve or improve representative performance.
