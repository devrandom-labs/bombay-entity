# Repository Guidelines

## Project Structure & Module Organization

The workspace contains the published library in `crates/entity/`; documentation lives in `docs/`. Put future benchmarks in `crates/entity/benches/` and focused integration tests in `crates/entity/tests/`.

## Build, Test, and Development Commands

Use Nix for every task. Enter with `nix develop`; run all Cargo commands there. `nix flake check -L` is the required build, format, Clippy, nextest, doctest, docs, audit, and license gate. `nix build .#coverage -L` produces HTML coverage. Targeted commands include `cargo test --workspace` and `cargo bench -p bombay-entity` once benchmarks exist.

## Engineering Principles

High performance is mandatory. Before choosing an algorithm or data structure, review current primary literature, including relevant arXiv papers; define the workload and invariants; and compare the best applicable approaches. Benchmark representative workloads and retain reproducible evidence.

Never assume correctness. Validate every behavior change with suitable unit, property, stress, Loom, Miri, fuzz, and benchmark coverage. Check current stable Rust patterns and standard-library capabilities before adding custom machinery; record non-obvious rationale.

After every pass, review the entire crate for unnecessary abstractions, duplication, allocations, dependencies, and code paths. Distill it to the smallest design preserving every feature, invariant, safety property, and measured performance characteristic.

## Coding Style & Naming Conventions

Use rustfmt defaults (four-space indentation) and idiomatic Rust naming: `snake_case` for modules, functions, and tests; `CamelCase` for types and traits; `SCREAMING_SNAKE_CASE` for constants. The workspace denies Clippy's `all` group and warns on `pedantic`; keep all targets warning-free. Document public APIs and preserve the lifecycle and generation-safety guarantees described in `crates/entity/src/lib.rs`.

## Testing Guidelines

Place focused integration tests in `crates/entity/tests/`; name tests after observable behavior, such as `concurrent_commands_start_one_activation`. Put future adversarial or long-running verification in a detached research crate. Concurrency changes require relevant ownership, reclamation, reentrancy, or interleaving regression coverage.

## Commit & Pull Request Guidelines

Use concise Conventional Commit subjects such as `feat(reclamation): ...` or `test(reclamation): ...`. Keep commits scoped and green. Pull requests must explain behavior, affected invariants, verification commands, and linked issues. Include benchmark or coverage results when relevant.
