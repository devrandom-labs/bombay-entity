# Research and falsification protocol

## Research ledger

For every design family under consideration, record:

- repository workload and invariant it addresses;
- current implementation cost and failure surface;
- best known standard-library facility;
- best maintained crate candidates;
- original papers or specifications when relevant;
- exact versions/commits and access date;
- semantic mismatches and unsupported assumptions;
- measurement design and raw-result location;
- keep/reject decision with evidence.

Prefer official documentation, upstream source, standards, original papers, and primary benchmark artifacts. Search broadly enough to find approaches not suggested by the operator or earlier agents.

## Whole-workspace lenses

Rotate through all lenses rather than repeating stylistic cleanup:

1. Semantic authority: duplicated state, transition, topology, protocol, or evidence representations.
2. Type design: boolean blindness, invalid product states, sentinel values, identity/generation confusion, capability leakage, and ownership encoded dynamically rather than statically.
3. Public seams: obligations callers may omit, reorder, duplicate, forge, or pair incorrectly.
   Inventory every `pub` item and prove its external necessity. Prefer private concrete machinery behind a minimal capability-oriented surface.
4. Concurrency: linearization, reentrancy, poison/panic paths, lost wakeups, lock scope/order, reclamation, stale work, and Loom visibility.
5. Effects: ordering, exactly-once ownership, batching, allocation, cancellation, completion, and interpreter boundaries.
6. Machinery: adapters, forwarding methods, compatibility aliases, wrapper outputs, repeated matches, clones, allocations, queues, locks, options, and `expect`-enforced states.
7. Dependencies: custom implementations replaceable by established crates and existing dependencies that cost more than they provide.
8. Performance: representative uncontended, contended, fan-out, stale, drain, failure, and reentrant workloads; latency distribution, throughput, allocations, code size, and compile time.
9. Verification: missing unit, property, exhaustive, stress, Loom, Miri, fuzz, doctest, coverage, and mutation evidence.
10. Documentation: claims not mechanically connected to executable behavior or benchmark evidence.
11. Rust expression: redundant type annotations, turbofish, wrapper types, bounds, conversions, forwarding methods, and annotations that inference can safely remove.
12. Patterns: current idiomatic Rust patterns and relevant Gang-of-Four patterns, accepted only when they measurably reduce state space, coupling, duplication, or public surface without changing semantics.

## Dependency decision

Do not accept a crate by popularity. Require:

- exact semantic and ownership fit;
- compatible stable Rust/MSRV and `no_std` requirements;
- acceptable transitive dependency, feature, license, audit, maintenance, and supply-chain profile;
- benchmark parity or improvement on repository workloads;
- acceptable code-size and compile-time impact;
- a smaller total maintenance and correctness burden.

Include `thiserror` in the baseline conventions for error types. Audit candidates such as small-vector storage, concurrent maps, futures/wakers, slab/generational identity, state-machine helpers, model checking, property testing, and error/reporting crates, but never assume any candidate belongs in the result.

## Experiment discipline

- Change one causal variable per experiment where possible.
- Record raw commands and results; retain reproducible benchmark inputs.
- Establish noise before accepting small performance changes.
- Add a regression detector before altering a subtle invariant.
- Revert regressions and record the dead end so later contexts do not retry it blindly.
- Never weaken a test, benchmark workload, invariant, or API guarantee to make an experiment pass.
- Do not use generated macros, traits, or generic frameworks unless they remove more conceptual machinery than they introduce.
- Do not force object-oriented pattern vocabulary onto algebraic Rust. Prefer enums, ownership, traits, generics, and modules when they express the same intent with fewer seams.
- Do not collapse semantically distinct types simply to reduce declarations or annotations.

## Fixed-point challenge

The final adversarial pass starts from source, contract, and measurements—not prior conclusions. It attempts to find:

- one remaining duplicated authority;
- one invalid representable state;
- one caller-misuse seam;
- one unnecessary allocation, clone, lock, queue, dependency, or abstraction;
- one unverified concurrency or panic interleaving;
- one maintained crate that might replace custom machinery;
- one benchmark that could reverse a previous decision.

Any credible finding reopens the loop.
