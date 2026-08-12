# Baselines (2026-08-11, commit 419a979, aarch64-darwin M4 Pro)

## Gate
`nix flake check -L` GREEN in 16.5s wall. Checks: fmt, doc, clippy, nextest, doctest, audit, license.
Noise: 3 crane evaluation warnings (workspace Cargo.toml lacks `name`/`version`; placeholder used).

## Benchmarks (`nix develop -c cargo bench -p bombay-entity`)
Raw timing harness (not criterion); single run, noise unquantified — hypothesis: add repetition before accepting small deltas.

| workload | config | time |
|---|---|---|
| directory activating_hot_key | 1,000,000 iters | 69.98 ms |
| directory active_hot_key | 1,000,000 iters | 96.26 ms |
| directory independent_keys | 100,000 iters | 13.24 ms |
| directory contended_active_key | 8 threads × 100,000 | 190.22 ms |
| lifecycle ignored_step | 1,000,000 iters | 18.28 ms |
| lifecycle claim_activation_step | 1,000,000 iters | 21.24 ms |

## Static metrics
- production_lines = 4468 · boolean_tokens = 14 · custom_error_impls = 0 · unsafe = 0
- check.sh SCORE = 994132
- pub items: entity 89, transition 29, driver 16 (grep count, incl. pub use/const)
- external normal deps: bombay-behavior 0.9.1, bombay-communication 0.1.0, tokio 1.53.1, thiserror 2.0.20 (+ proc-macro support crates); dev: loom 0.7.2
- entity unit tests are `#[ignore]`d under bench harness (harness=false benches; 15 tests ignored in that run — expected, they run under nextest in the gate)

## Not yet measured (open)
- Allocation profile (no dhat/tracy harness present)
- Code size (rlib/text size) and clean compile time
- Public-surface diff tooling (cargo-public-api not installed)

## Post-E12 repeated-harness baselines (commit b115f38, min/median of 7)
| workload | min | median |
|---|---|---|
| activating_hot_key | 47.72 ms | 48.48 ms |
| active_hot_key | 90.81 ms | 91.23 ms |
| independent_keys | 10.18 ms | 10.71 ms |
| contended_active_key | 181.62 ms | 183.88 ms |
| ignored_step | 9.80 ms | 10.08 ms |
| claim_activation_step | 14.77 ms | 15.26 ms |

Decision rule: only deltas >5% on min are decision-grade.

## Final controlled comparison (2026-08-11, evening, same machine conditions)
Machine slowed ~1.45x globally during the session (thermal); absolute morning
numbers are not comparable. Definitive control: baseline commit 419a979 benched
in a worktree under identical current conditions vs final tree.

| workload | 419a979 (control) | final tree | delta |
|---|---|---|---|
| activating_hot_key | 69.61 ms | 71.96 ms | +3% (noise) |
| active_hot_key | 138.54 ms | 140.94 ms | +2% (noise) |
| independent_keys | 18.52 ms | 15.13 ms | -18% |
| contended_active_key | 248.79 ms | 238.98 ms | -4% |
| ignored_step | 15.17 ms | 14.42 ms | -5% |
| claim_activation_step | 22.21 ms | 21.46 ms | -3% |
| stale_absent_callbacks | ~84-88 ms (E24 pre-measure) | 36.98 ms | ~-57% |

Verdict: maintain-or-improve satisfied under controlled conditions.

## Allocation profile (2026-08-11, dhat)
- active_hot_key dispatch+interpret: 0 blocks, 0 bytes per operation (pinned by crates/entity/tests/allocations.rs).

## LLVM coverage (2026-08-12, Rust/LLVM 1.96.0/22.1.2)
- Workspace tests: 93.22% lines (2160/2317), 91.27% functions (230/252),
  91.70% regions (2619/2856).
- `checks.entity-coverage` enforces a 93.2% line floor in `nix flake check`.

## Pre-E39 tree benchmark refresh (2026-08-12, aarch64-darwin M4 Pro)
Seven repetitions; minimum and median shown. These are a dated pre-E39 reference,
not a replacement for the controlled baseline-vs-final comparison above.

| workload | min | median |
|---|---:|---:|
| activating_hot_key | 70.68 ms | 70.82 ms |
| active_hot_key | 138.63 ms | 138.80 ms |
| independent_keys | 15.00 ms | 15.49 ms |
| contended_active_key | 243.76 ms | 245.00 ms |
| stale_absent_callbacks | 38.08 ms | 38.11 ms |
| ignored_step | 14.41 ms | 14.44 ms |
| claim_activation_step | 21.53 ms | 21.62 ms |

## E39 prehashed shard lookup (2026-08-12, aarch64-darwin M4 Pro)
Two alternating executions of already-built baseline and candidate binaries,
seven repetitions per execution. Ranges below are the observed minima:

| workload | baseline range | candidate range | outcome |
|---|---:|---:|---:|
| activating_hot_key | 71.21–71.60 ms | 60.06–60.23 ms | about 16% faster |
| active_hot_key | 138.27–140.22 ms | 119.98–121.05 ms | about 13–14% faster |
| independent_keys | 15.17–15.63 ms | 15.12–15.52 ms | noise |
| contended_active_key | 238.61–239.93 ms | 203.98–206.87 ms | about 13–15% faster |
| stale_absent_callbacks | 36.82–37.88 ms | 38.07–38.36 ms | within 5% noise bound |

The exact counter test changes one active dispatch from two identifier `Hash`
calls to one. The candidate reuses that single caller-computed hash for shard
selection and the raw table lookup.

## E39 final validation and rollback (2026-08-12)
The initial candidate figures above did not reproduce for independent-key
insertion during the final alternating run. Same-run baseline minima were
14.97–15.45 ms versus 16.54–16.81 ms for the shipped E39 representation, a
7–12% regression. E46 cached each full hash in a lower-level `HashTable`, but
its independent minima were still 16.36–16.54 ms against 15.15–15.20 ms, about
8–9% slower. Both exceed the declared 5% falsifier, so the workspace returned
to the standard-library table and removed the added dependency. The earlier
hot-path gains are rejected tradeoff evidence, not the final baseline.
