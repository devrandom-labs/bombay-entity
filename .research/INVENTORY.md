# Repository inventory

## Architecture and data flow

`Machine::step` consumes an input and the current machine, returning output plus a same-typed successor. `bombay-machine-executor` currently adapts this law only for concurrent/reentrant callers through `SerializedExecutor` and `LinearizedExecutor`.

## Public surface and consumers

- `SerializedExecutor`: shared submission, queue, handler, receipt.
- `LinearizedExecutor`: shared transition linearization and separately ordered disposition.
- Proposed `ExclusiveExecutor`: immediate returned output under caller-held exclusive access.
- Repository consumers currently use `LinearizedExecutor`; the exclusive primitive is intended for an actorpass integration outside this workspace.

## Dependencies and custom machinery

The standard library supplies the exact field-consumption operation through `core::mem::replace`. No maintained crate candidate improves semantic fit or reduces machinery for a two-variant private enum.

## Invariants and risk boundaries

- Affine machine ownership cannot be moved directly from `&mut self` without installing another valid field value.
- User transition code may unwind after consuming both machine and input.
- Poison must be installed before user code and remain observable if unwind is caught externally.
- Assignment of the successor is non-user-code; returned output handling occurs after the transition boundary.

## Existing verification and gaps

- Existing concurrency policies have unit and Loom coverage.
- The exclusive adapter needs ordinary ordering, successor transparency, panic, refusal, distinct drop ownership, output identity, `into_inner`, and positive auto-trait evidence.
- Loom is inapplicable because the adapter contains no shared concurrency primitive.
- Benchmark and allocation/code-generation evidence are absent.

## Nix gates and platform coverage

`nix flake check -L` is the required complete gate. Coverage is available through `nix build .#coverage -L`.

## Rust state modeling and public API

Private state is the sum `Ready(M) | Poisoned`. Public diagnostics use `ExclusiveState`; terminal extraction failure uses `ExclusivePoisoned`; already-poisoned admission reuses `PoisonedInput<I>`.
