# Arsenal cargo editor

The cargo editor of the [arsenal](/documentation_v2/glossary.md#arsenal): what one
[slot](/documentation_v2/glossary.md#slot) carries inside its vest, pants, jacket and backpack, with
each container's weight and volume against the worn garment's capacity. It is a submodule of the
`panels` module in the parent folder, which re-exports `cargo_panel`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/arsenal/panels/
└── cargo_panel.rs  `cargo_panel`: per-container cargo rows, the add picker and the capacity readout
```

## How it works

`cargo_panel` draws one group for each container in the Arsenal's `rules::CARGO_CONTAINERS`
(`vest`, `pants`, `jacket`, `backpack`) that has a garment picked or holds cargo; the `vest` group
also takes an armoured vest (`rules::cargo_garment`). A group's rows carry "−", "+" and "✕"
buttons, and its "+ Add item…" picker offers the item
[registry](/documentation_v2/glossary.md#registry)'s concrete items of the kinds `magazine`,
`ammo`, `gear_item`, `gear_throwable` and `gear_explosive`, sorted by display name; adding an item
the container already holds raises its quantity. The budget line sums the rows' weight and volume
(`rules::cargo_budget`) against the garment's `max_weight_kg` and `max_volume_cm3`, and turns to
the alert colour when either is exceeded. Every change updates the cargo signal and calls
`on_change` in the same handler, which writes the loadout at once.

## Boundaries

- Depends on: `rules` of the Arsenal in `apps/website/frontend/src/v2/apps/editor/arsenal/`
  (`CargoRow`, `CARGO_CONTAINERS`, `cargo_garment`, `cargo_budget`, `index_by_name`), reached
  through the parent module; `RegistryItem` from `crate::v2::core::api::dto`.
- Used by: the Arsenal tab's `tab_content.rs` in `apps/website/frontend/src/v2/apps/editor/arsenal/`,
  through the parent's re-export; the source pins in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs` and
  `apps/website/frontend/src/v2/apps/editor/ui/arsenal/tests/panels/cargo_persistence.rs`.
- Rules: a cargo edit never waits for a Save; each of the four mutations calls `on_change` in its
  own handler (`cargo_mutations_commit_without_a_staging_gate` in
  `apps/website/frontend/src/v2/apps/editor/ui/arsenal/tests/panels/cargo_persistence.rs`).
