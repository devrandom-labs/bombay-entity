# Research Sources

Every entry needs: URL, version/commit, access date, applicability, limitations. No blog posts as
primary evidence. Entries are added when consulted, not anticipated.

## Research areas
1. Rust std: `Vec` small-size optimization status, `NonZero*` niche guarantees, `Mutex` poisoning
   semantics — for SlotEffectBatch::EffectStorage vs smallvec, and lock `.expect` paths.
2. smallvec / arrayvec / tinyvec: replacing `EffectStorage` Empty/One/Many custom enum.
3. dashmap vs std sharded `Mutex<HashMap>`: directory shard machinery (crates/entity/src/directory.rs).
4. thiserror 2.x: error definitions for DirectoryError / LifecycleTopologyError / DispatchFailure /
   FenceFailure — check current manual Display impls (check.sh counts Error impls = 0; verify how
   errors are currently defined).
5. loom 0.7: current model-checking coverage vs what loom_directory.rs exercises.
6. Boolean-state elimination candidates (type-state or enum encoding) for driver executor flags
   (running/poisoned/armed/dispatching/acquired) — verify each is genuinely binary protocol state.
7. Bench methodology: criterion vs raw timing; noise quantification for accepting small deltas.

## Consulted

- Rust standard library 1.96.0, atomic module, `AtomicU64`, `HashMap`, and `Mutex` documentation:
  <https://doc.rust-lang.org/1.96.0/std/sync/atomic/>,
  <https://doc.rust-lang.org/1.96.0/std/sync/atomic/struct.AtomicU64.html>,
  <https://doc.rust-lang.org/1.96.0/std/collections/struct.HashMap.html>, and
  <https://doc.rust-lang.org/1.96.0/std/sync/struct.Mutex.html>; accessed 2026-08-12.
  Applicability: atomic portability/orderings and the lock/hash primitives used by the directory.
  Limitation: API and memory-model authority, not comparative workload performance.
- `smallvec` 1.15.1 upstream documentation and source:
  <https://docs.rs/smallvec/1.15.1/smallvec/>; accessed 2026-08-12. Applicability: inline-storage,
  `no_std`, feature, and representation claims for E16. Limitation: upstream docs do not predict
  this repository's `SlotEffectBatch` move cost; the retained min-of-seven benchmark decided E16.
- `dashmap` 6.1.0 upstream documentation and source:
  <https://docs.rs/dashmap/6.1.0/dashmap/>; accessed 2026-08-12. Applicability: guard-based map API
  and documented same-thread locking hazards considered in H16. Limitation: no benchmark was run
  because the API's guard lifetime conflicts with the repository's required reentrant callbacks.
- Ori Shalev and Nir Shavit, “Split-Ordered Lists: Lock-Free Extensible Hash Tables,” JACM 53(3),
  DOI <https://doi.org/10.1145/872035.872049>; journal version 2006, accessed 2026-08-12.
  Applicability: lock-free dynamically extensible hash-table alternative. Limitation: requires
  substantially different resizing and reclamation machinery from the Arc-owned local directory.
- Tobias Maier, Peter Sanders, and Roman Dementiev, “Concurrent Hash Tables: Fast and General?(!),”
  arXiv:1601.04017 v1, <https://arxiv.org/abs/1601.04017>; accessed 2026-08-12. Applicability:
  evidence that contention, layout, key restrictions, and resizing change comparative results.
  Limitation: C++ algorithms and workloads are not direct performance evidence for this Rust API.
- Philip A. Bernstein et al., “Orleans: Distributed Virtual Actors for Programmability and
  Scalability,” MSR-TR-2014-41, 2014,
  <https://www.microsoft.com/en-us/research/publication/orleans-distributed-virtual-actors-for-programmability-and-scalability/>;
  accessed 2026-08-12. Applicability: virtual-actor activation and directory context. Limitation:
  distributed Orleans architecture does not establish this local directory's synchronization law.
- Clovis Eberhart and Tom Hirschowitz, “What is a Machine? An Essay on the Composable
  Representable Executable Machines,” arXiv:2307.09090 v2,
  <https://arxiv.org/abs/2307.09090>; accessed 2026-08-12. Applicability: compositional machine
  framing. Limitation: conceptual semantics, not evidence for executor or directory performance.
- `hashbrown` 0.17.1 upstream crate metadata, documentation, and raw-entry source:
  <https://docs.rs/hashbrown/0.17.1/hashbrown/hash_map/enum.RawEntryMut.html> and
  <https://github.com/rust-lang/hashbrown>; accessed 2026-08-12. Applicability: raw lookup and
  insertion with a caller-computed hash can reuse the directory's shard-selection hash; crate
  metadata reports MIT/Apache-2.0 and MSRV 1.85. Limitation: upstream API evidence does not prove
  a win on this repository's ownership-heavy and contended workloads; E39 benchmarks decide it.
