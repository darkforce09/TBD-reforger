# Arsenal tab sections

The three sections of the [arsenal](/documentation_v2/glossary.md#arsenal)'s loaded view, which the
parent file `apps/website/frontend/src/v2/apps/editor/arsenal/tab_content.rs` stacks around the
cargo editor and the action bar once the [registry](/documentation_v2/glossary.md#registry) has
loaded.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/arsenal/tab_content/
├── catalog_header.rs          the badges: compatibility feed status, capacity and live weight
├── selection_grid.rs          region rail, filtered item list, doll and compatibility panel
└── status_and_persistence.rs  import and buffer receipts and refusals, and the persistence line
```

## How it works

`loaded_catalog` in `tab_content.rs` takes the Arsenal's `ArsenalTabState` apart and renders
`catalog_header`, `selection_grid`, the cargo editor and the action bar, then
`status_and_persistence`. The header shows the compatibility feed ("Compat active",
"Compat loading…", "Compat unavailable"), the catalogued capacity of the active container's
garment, and the weight of the current picks. The grid lays out four columns: the rail of loadout
regions, the active region's items behind a filter box, the doll from `doll_view`, and
`compat_panel`; a pick goes back through the `pick_item` callback the parent passes in, which
writes the loadout at once. The status section lists every refusal of an import or an Apply, or
the receipt of what landed, and ends with the persistence line: the last pick was refused, the
[mission](/documentation_v2/glossary.md#mission) has unsaved changes, or it has none.

## Boundaries

- Depends on: the parent `tab_content.rs` and the arsenal root
  `apps/website/frontend/src/v2/apps/editor/arsenal/mod.rs`, through `use super::*`
  (`ArsenalTabState`, the persistence texts, the region titles and icons, the rules and loadout
  items); `doll_view` and `compat_panel` from
  `apps/website/frontend/src/v2/apps/editor/ui/arsenal/panels.rs`; `RegistryItem` from
  `crate::v2::core::api::dto`.
- Used by: `tab_content.rs`, the only caller of the three section functions.
- Rules: the sections render and report; a write happens only in the parent's handlers. The
  persistence line reads the document's dirty flag and the refusal flag, never the pick counter,
  so it never reports a refused pick as saved (`the_panel_states_the_persistence_contract` and
  `a_refused_pick_is_visible_in_the_panel_not_silent` in
  `apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs`).

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Arsenal tab as the mission maker uses it.
