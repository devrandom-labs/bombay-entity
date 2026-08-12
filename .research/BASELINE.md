# Baseline

## Environment

- Baseline commit: `dd5d05e7f34e43fba28f99ccadd4451f2ded4327`.
- Accessed: 2026-08-12.
- Host: Apple M4 Pro, `aarch64-apple-darwin`, Darwin 25.5.0.
- Nix shell Rust: rustc 1.96.0 (`ac68faa20`, LLVM 22.1.2), Cargo 1.96.0.
- Workspace MSRV: Rust 1.96, edition 2024.
- Pre-existing unrelated dirty paths preserved: `result`, `.serena/`.

## Verification

- Before the research ledger was activated, the draft passed `nix develop --command cargo test -p bombay-machine-executor --lib` (10 tests) and targeted all-target Clippy with `-D warnings`.
- This is provisional evidence only; the full flake gate and strengthened ownership tests remain required.

## Measurements

- Added reproducible `exclusive_executor` bench: 1,024 turns/batch, 2,000 batches, 9 repetitions, optimized build, Apple M4 Pro.
- The initial inlined benchmark was invalid for poison cost: whole-program optimization could prove the local executor was not observed after unwind and erase bookkeeping. It misleadingly reported direct payload 1.344–1.350 ns/turn and exclusive 1.337–1.339 ns/turn.
- The retained benchmark uses symmetric `#[inline(never)]` transition boundaries, preventing the caller from seeing through `turn` and preserving externally observable poison semantics.
- Five retained payload runs produced direct medians 3.351–3.366 ns/turn and exclusive medians 3.491–3.510 ns/turn: observed overhead 0.131–0.159 ns/turn, approximately 3.9–4.7% for this tiny workload.
- AArch64 disassembly confirms the semantic cost: the exclusive function adds a seat load, poison store, discriminant comparison/branch, and ready-successor store. Direct stepping has only state addition and output writes.
- DHAT measured zero blocks and zero bytes over 10,000 exclusive turns with an allocation-free counter machine.

## Rust surface

- Workspace crates: `bombay-transition`, `bombay-machine-executor`, `bombay-entity`.
- Driver dependencies: `bombay-transition`, `thiserror`; Loom is verification-only.
- Existing driver public policies: serialized run-to-completion and transition-linearized/split dispatch.
- Existing affine machine storage in concurrent policies uses explicit poison/turn states plus synchronization.
- No unsafe code is needed for the proposed exclusive adapter.
