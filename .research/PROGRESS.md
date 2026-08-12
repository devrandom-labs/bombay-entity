# Progress

## Current state

H01 implemented and retained after targeted evidence, opaque-boundary benchmarks, generated-code inspection, zero-allocation measurement, full Nix verification, and adversarial audits.

Audit 1 challenged state modeling, panic boundaries, duplicated authority, dependency alternatives, allocation, benchmark symmetry, and architecture claims. It found and corrected stale two-policy documentation and rejected the asymmetric first benchmark.

Audit 2 restarted from the public API and ownership contract. It found no live machine in poison, no recoverable accepted input, no output-disposition coupling, no unnecessary bound or auto-trait override, no user-code panic point after successful `step`, and no smaller standard-library or maintained-crate replacement. The separate state and extraction-error types each represent externally distinct capabilities rather than duplicated storage authority.

Audit 3 challenged the performance evidence and falsified the earlier “indistinguishable” conclusion: whole-program inlining erased observable poison machinery. Symmetric opaque boundaries reveal stable 0.131–0.159 ns/turn overhead for the payload workload, matching the extra generated state instructions.

## Retained improvements

- `ExclusiveExecutor<M>` with explicit ready/poisoned ownership and direct output return.
- Persistent poison after transition unwind, exact later-input rejection, and successor inspection/extraction.
- Distinct ownership/drop tests, structural auto-trait evidence, DHAT allocation assertion, and comparative executor benchmark.
- Architecture documentation now distinguishes all three execution laws and poison boundaries.

## Active experiment

None.

## Exact next actions

1. Integrate the executor into actorpass in its own repository-scoped experiment.
2. Re-run the benchmark when changing machine representation or compiler/toolchain.
3. Investigate the unrelated one-shot `fence_failures_preserve_the_forced_retirement_stage` timing failure separately if it recurs.
