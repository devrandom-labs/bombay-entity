# Research sources

| Topic | Primary source/version | Accessed | Applicability | Limitations |
|---|---|---|---|---|
| Field consumption | https://doc.rust-lang.org/std/mem/fn.replace.html, Rust 1.97 docs | 2026-08-12 | Moves a field value through `&mut` while immediately installing a valid replacement; neither exchanged value is implicitly dropped. | Documentation version is one release newer than workspace MSRV; API is stable since 1.0 and const since 1.83. |
| Exception safety | https://doc.rust-lang.org/stable/nomicon/exception-safety.html | 2026-08-12 | Recommends leaving a safe state before invoking caller-controlled panicking code or using cleanup guards. Poison-first replacement provides the safe state without a guard. | Nomicon guidance, not a formal semantic specification. |
| Panic/unwind cleanup | https://doc.rust-lang.org/stable/reference/panic.html | 2026-08-12 | Defines unwind cleanup and distinguishes abort, supporting external `catch_unwind` tests without catching inside the crate. | Abort builds cannot observe persistent poison after panic. |
| Unwind safety/poison | https://doc.rust-lang.org/std/panic/trait.UnwindSafe.html | 2026-08-12 | Poison is an established speed bump for logical invariants; `&mut T` requires explicit `AssertUnwindSafe` at test catch boundaries. | `UnwindSafe` is advisory, not an unsafe contract. |
| Auto traits | https://doc.rust-lang.org/nomicon/send-and-sync.html | 2026-08-12 | Structural `Send`/`Sync` derivation is correct for a wrapper with no interior mutability. | Nomicon explanation; compiler behavior remains authoritative. |
| Affine ownership context | https://arxiv.org/abs/2301.02308 | 2026-08-12 | Confirms Rust ownership as affine type discipline; supports treating consuming `step` as an ownership law. | Does not prescribe this executor API or measure its performance. |
