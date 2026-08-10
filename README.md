# Bombay Entity

`bombay-entity` is the entity-runtime layer between a CQRS application and
actorpass. Applications send commands to a stable, typed `EntityId`; the
runtime manages the changing actor incarnation that currently serves it.

The first version is local and in-memory. Its responsibilities are:

- single-flight activation for concurrent commands;
- generation-safe commitment and retirement of actor incarnations;
- bounded admission with typed busy, draining, and unavailable refusals;
- routing admitted commands to the committed incarnation;
- passivation that closes admission before accepted work drains;
- rejection of stale completion and passivation events;
- typed replies and lifecycle telemetry without exposing runtime control.

The lifecycle is:

```text
Inactive
  -> Activating(generation)
  -> Active(generation, actor reference)
  -> Draining(generation)
  -> Inactive
```

Initialization failures return the entity to `Inactive` without installing a
broken actor. A later command may then activate a fresh incarnation.

The crate does not own journals, snapshots, aggregate business logic, event
serialization, projections, business idempotency, distributed consensus, or
discovery. A future distributed directory can replace the local directory
behind a narrow port without changing entity behavior.

Actorpass remains responsible for running exact actor incarnations.
Behaviorpass, Communication, and Observe provide the lower-level protocols,
communication, and observation facilities.

The detailed lifecycle, ownership, and correctness contract is documented in
[`docs/architecture.mdx`](docs/architecture.mdx).
