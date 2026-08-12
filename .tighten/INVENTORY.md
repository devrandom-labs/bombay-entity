# Workspace Inventory (2026-08-11, commit 419a979)

Evidence: scout reports at agent://EntityScout, agent://KernelScout, agent://EvidenceScout
(full pub tables with file:line). This file distills; consult artifacts before editing.

## Crates and layering
- `bombay-transition` (715 src lines, `#![no_std]`, zero deps): `Decision`/`Reducer` algebra
  (lib.rs) + `Machine`/`Structure`/`Compose` with `Base`/`Then`/`Product`/`Routed`/`Either`
  composition, `Topology`/`ValidatedTopology`/`TopologyError`, mermaid rendering (machine.rs).
- `bombay-machine-executor` (629 src lines, std, dep: transition): `SerializedExecutor`
  (run-to-completion, Condvar `TurnReceipt`, explicit poison recovery) and
  `LinearizedExecutor` (transition under one mutex, queued output dispatch; NO poison recovery).
- `bombay-entity` (3124 src lines, deps: behavior 0.9.1, transition, driver): pure lifecycle
  algebra (`lifecycle/`: EntitySlot, SlotEvent/SlotEffect, SlotEffectBatch, SlotReducer,
  topology-validated LifecycleMachine/Model/Evidence) + sharded directory (directory.rs:
  `Box<[Mutex<HashMap<EntityId, Arc<Slot>>]>>`, 64 shards, AtomicU64 ID allocation) +
  async facade (runtime.rs: EntityRuntime, hand-rolled Completion one-shot) + behavior
  protocol wrapper (protocol.rs: EntityBehavior, forward_optional_event! macro).

## Handcrafted machinery (dependency-replacement candidates)
1. `SlotEffectBatch`/`EffectStorage` Empty/One/Many (lifecycle/mod.rs:339) — smallvec candidate.
2. `Completion<C>` one-shot Mutex+Option+Waker (runtime.rs:95) — oneshot channel candidate.
3. `TurnCompletion` Mutex+Condvar receipt (driver:55) — std mpsc / notify candidate.
4. Sharded `Mutex<HashMap>` directory (directory.rs) — dashmap candidate.
5. `forward_optional_event!` macro, 7 delegation impls (protocol.rs:75-90).
6. `LifecycleModel(Topology)` delegating newtype (lifecycle/machine.rs:222).
7. `Choice<A,B> = Routed<A,B>` compatibility alias (transition machine.rs:292).
8. `Decision`/`Reducer` parallel to `Machine` (transition lib.rs) — both currently used by entity.

## Booleans (contract bans boolean state/params/results)
- driver: `running`/`poisoned` (:115-116), `armed` (:200), `dispatching` (:275), `acquired` (:354).
- entity: `EntityRuntime::passivate -> bool` (runtime.rs:244, ambiguous + stale-read race),
  `DispatchWait.completed: bool` (runtime.rs:266).
- Query-shape bools (`is_empty`, `contains`, `handles`, `removable_as`, `dispatch_pending -> bool`)
  are predicate returns, lower priority but `dispatch_pending -> bool` is a protocol result.

## Sentinels / duplicated representations
- `DirectoryOutput.dispatch_id: Option<DispatchId>` — always Some from dispatch(), None from
  callbacks; `expect("dispatch has an identity")` at runtime.rs:222 (directory.rs:79).
- `machine: Option<M>` in both executors — None only mid-step (driver:109, :268).
- `Slot::removable_activation: Mutex<Option<ActivationId>>` (directory.rs:87).
- `evidence: Option<E>` stored AND returned by submit (driver:274, :308).
- `TransitionEvidence::SelfLoop` vs `Ignored` both carry {phase, trigger} (machine.rs:150).
- `LifecycleEdge` triple duplicates `LIFECYCLE_TOPOLOGY` data (machine.rs:66).

## Rule violations (AGENTS.md/CLAUDE.md)
- Arithmetic: `ReservationCount::reserve` saturating_add (lifecycle/mod.rs:156);
  `resolve` bare `- 1` (:163); `shard_index` bare `- 1` (directory.rs:483).
- Errors: DirectoryError, DispatchFailure, FenceFailure, LifecycleTopologyError, TopologyError,
  PoisonedInput — none implement Error/Display via thiserror.
- Error discard: `Err(_)` drops ActivationError (runtime.rs:347).
- Concurrency: `fetch_update(Relaxed, Relaxed)` ID allocator, no documented proof (directory.rs:499).
- Poison asymmetry: LinearizedExecutor panics all future calls after one transition panic.

## Concurrency map
- Lock order: shard mutex → (interpret) → Slot::removable_activation; never reversed; no AB/BA.
- Poison policy: every entity mutex `.expect("... poisoned")` = escalate; driver Serialized recovers.
- Completion wake: waker stored under mutex, woken after guard drop (correct).
- current_activation read outside shard lock → passivate stale-true race (directory.rs:375).
- Loom coverage: claim single-flight, fence-after-reservations, removal identity, drop-once.
  NOT modeled: executor poison paths, multi-reservation drain races, DispatchWait drop.

## Verification gaps (documented invariants without executable evidence)
- Init/preparation failure cleanup paths (doc invariant 3).
- Reservation-start vs BeginDrain race (invariant 4 partially).
- Graceful retirement blocked before fence ack (invariant 6 negative direction).
- Refusal::Busy bounded admission; FenceFailure variants; DrainPolicy::Bounded timeout.
- ActivationId exhaustion; telemetry claims; "bounded trace test" cited in docs does not exist.
- Miri absent (toolchain), fuzz absent (docs justify via finite alphabet + loom).
- LLVM source coverage is a gate check as of E31: 93.22% lines measured, with a 93.2% floor.
