# Dead ends

Record rejected and reverted approaches, exact evidence, and conditions that would justify reopening them.

## Asymmetric direct-step benchmark

Rejected 2026-08-12. The first direct loop passed the whole machine through `black_box` on every turn while the exclusive loop did not, producing an invalid comparison in which the wrapper appeared faster. The corrected benchmark black-boxes inputs and outputs symmetrically. Reopen only if both paths preserve comparable optimizer barriers.

## Fully inlined poison benchmark

Rejected 2026-08-12 after adversarial review. Even with symmetric input/output barriers, whole-program optimization could see the executor's complete lifetime and erase poison behavior that remains observable to a real caller through `catch_unwind`. It misleadingly supported an indistinguishable-performance claim. Retained measurements place direct and exclusive transitions behind symmetric `#[inline(never)]` boundaries and verify the resulting instructions with disassembly.

## Third stepping state or unwind guard

Rejected 2026-08-12. Installing `Poisoned` with `core::mem::replace` before entering user transition code already supplies both the temporary safe seat and permanent unwind state. A retained `Stepping` variant or guard adds machinery without an observable state or cleanup action.
