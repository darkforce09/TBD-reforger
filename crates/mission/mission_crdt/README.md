# Mission CRDT

The `mission_crdt` crate: the building blocks of the [mission](/documentation/glossary/g_to_m.md#mission)
document below its rows. A squad's `slotIds` and a layer's `entityIds` held as native `yrs` arrays
so two peers appending at once both keep their id; `SlotSoa`, the row-aligned
[slot](/documentation/glossary/n_to_z.md#slot) columns the map draws and picks from; and the
clocks, window and cap that group local edits into undo steps.

## Contents

```text
crates/mission/mission_crdt/
├── Cargo.toml  the package: `time_source`, `yrs`, layout tier 1
└── src/        the id arrays, the slot columns and interner, the undo grouping clocks
```

## How it works

`MissionDocCore` (crate `mission_document`) runs its row writes through `id_arrays`, fills a
`SlotSoa` on every materialise with an `Interner` per dictionary, and builds its `yrs` undo manager
from `undo_groups::undo_options`. The undo timestamp is a `GroupingClock` over a host
`time_source::Clock`: the platform wall clock by default, any injected clock otherwise. The
grouping clock floors every reading to 1 ms and freezes while an explicit group is open, so a
batch is one undo step however long it runs. The [source README](src/README.md) describes the
columns; the module READMEs describe the id list forms and the undo grouping.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_crdt   # the id array forms, the grouping clock and the cap math
```

## Configuration

No features and no environment variables. The host clock is the only seam: a caller passes a
`time_source::Clock` to `mission_document`'s `MissionDocCore::with_undo_clock`.

## Public surface

- `id_arrays`: `SLOT_IDS`, `ENTITY_IDS` and the readers, writers and hydrate migration of id
  lists.
- `soa`: `SlotSoa`, `Interner`, `NONE_IDX`, `STANCE_STAND`, `STANCE_CROUCH`, `STANCE_PRONE`.
- `undo_groups`: `GESTURE_WINDOW_MS`, `MAX_UNDO_GROUPS`, `GroupingClock`, `platform_clock`,
  `undo_options`, `hidden_prefix_after`.
- `prelude` holds the slot columns, their codes, the interner and the undo constants.

## Boundaries

- Depends on: `yrs`, `time_source`.
- Used by: `mission_document`; the map engine and the Mission Creator read `SlotSoa` through the
  map engine's document store.
- Rules: mission tier 1 (`cargo xtask verify crate-tiers`); no browser binding in the crate's own
  code and no `yrs` type in the clock seam: the host clock enters as a `time_source::Clock`.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the mission domain's crates and their tiers.
