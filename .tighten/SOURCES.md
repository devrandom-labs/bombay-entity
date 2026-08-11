# Research Sources

Every entry needs: URL, version/commit, access date, applicability, limitations. No blog posts as
primary evidence. Entries are added when consulted, not anticipated.

## Planned research areas (not yet consulted)
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
(none yet)
