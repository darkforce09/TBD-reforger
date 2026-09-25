# Arsenal panels

The views the [arsenal](/documentation_v2/glossary.md#arsenal) draws around one
[slot](/documentation_v2/glossary.md#slot)'s loadout inside the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s Attributes dialog: the doll that
previews the loadout in 3D with a flat SVG fallback, the compatibility panel with its attachment
toggles, and the cargo editor.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/arsenal/
├── mod.rs     the module tree
├── panels/    the cargo editor, a submodule of `panels.rs`
├── panels.rs  the doll host and its SVG paper doll, the compatibility panel, the attachment toggles
└── tests/     unit tests for the cargo editor's commit path
```

## How it works

The Arsenal tab in `apps/website/frontend/src/v2/apps/editor/arsenal/` owns the loadout: the
pick, cargo and catalog signals, the compatibility feed, and the write path. It calls the three
views of `panels.rs` and passes each the signals it draws and the callback it reports through:

- `doll_view` mounts the Arsenal's 3D `ArsenalDoll` in the browser build, unless that renderer
  failed to create; then, and always in the native build, it draws the SVG paper doll, one
  focusable hotspot per loadout region, dashed while empty and tinted once equipped. A click on
  either doll only makes that region the active one.
- `compat_panel` names the active region's pick, then lists, for every loadout row whose edge
  depends on that region, the items the compatibility graph accepts, or says "Pick a … first.",
  "Nothing compatible." or "Compat unavailable." when there is no pick, no match or no graph yet.
  For a weapon it adds the "Attachments" block, the one multi-select: a toggle writes the whole
  packed attachment set as one pick, and a picked attachment the graph rejects stays listed,
  marked "— incompatible", so it can still be removed. A region with neither says "No dependent
  slots.".
- `cargo_panel`, in `panels/`, edits the cargo of each worn container.

Every pick and cargo edit reaches the document through the callback the Arsenal handed in
(`pick_item`, `on_change`), which writes the loadout at once as one undo step; nothing here holds a
pending edit.

## Public surface

- `panels::{doll_view, compat_panel, cargo_panel}` (crate-visible): the three views the Arsenal
  tab renders.

## Boundaries

- Depends on: the Arsenal in `apps/website/frontend/src/v2/apps/editor/arsenal/`: `rules` (the
  loadout rows and weapon slots, `row_options`, `CompatFeed` and its graph), `loadout` (the
  attachment edge and the packing of an attachment set), `region_title`, `MaterialCheck` and, in
  the browser build, `doll::ArsenalDoll`; `RegistryItem` from `crate::v2::core::api::dto`.
- Used by:
  - in the Arsenal's folder, `apps/website/frontend/src/v2/apps/editor/arsenal/`: `mod.rs`,
    which imports the three views, and `tab_content.rs` and `tab_content/selection_grid.rs`, which
    render them;
  - the source pins in `apps/website/frontend/src/v2/apps/editor/arsenal/tests/shell_wiring.rs`;
  - the Arsenal smoke test in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/arsenal.rs`,
    which clicks the compatibility panel's buttons by `data-value`.
- Rules: a panel renders and reports; the loadout rules, the serialisation and the document write
  stay in the Arsenal. Every option button carries `data-value` with its resource name, which the
  smoke test selects on, and an attachment toggle also carries `data-attachment`. A cargo edit
  commits in its own handler (`cargo_mutations_commit_without_a_staging_gate` in
  `tests/panels/cargo_persistence.rs`).

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Arsenal tab the panels draw.
