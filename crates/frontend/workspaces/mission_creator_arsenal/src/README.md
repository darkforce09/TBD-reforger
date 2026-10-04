# Mission Creator Arsenal source tree

The source of the `mission_creator_arsenal` crate: the [arsenal](/documentation/glossary/a_to_f.md#arsenal) of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator): the Attributes dialog's Arsenal
tab, where a mission maker edits one [slot](/documentation/glossary/n_to_z.md#slot)'s loadout, and the
loadout domain behind it, with the panels the tab draws. The loadout rules and the asset catalog
it reads sit in the editor's state layer, `crates/frontend/workspaces/mission_creator_state/src/`.

## Contents

```text
crates/frontend/workspaces/mission_creator_arsenal/src/
├── arsenal_tab.rs       `ArsenalTab`, `ArsenalTabState` and the persistence lines the tab states
├── doll/                the doll's `window.__arsenalDoll` browser hooks
├── doll.rs              `ArsenalDoll`: the 3D paper doll over `paper_doll_renderer`'s `PaperDollRenderer`
├── error.rs             `Error`: a loadout refused by the export, import or Apply gate, one row per refusal
├── lib.rs               the module tree; re-exports `ArsenalTab` and the loadout surface
├── loadout/             loadout JSON, export and import gates, the copy buffer, receipts
├── loadout.rs           the kind-sourced loadout rows; re-exports the loadout items
├── loadout_commands.rs  the document writes: one slot, Apply to a selection, Remove Everything
├── panels/              the cargo editor, a submodule of `panels.rs`
├── panels.rs            the doll host and its SVG paper doll, the compatibility panel, the attachment toggles
├── prelude.rs           the items most callers name: `ArsenalTab` and the loadout conversions
├── tab_content/         the loaded tab's header, selection grid and status sections
├── tab_content.rs       `loaded_catalog`: the loaded view and its action handlers
└── tests/               unit tests for the loadout, the panels' cargo path and the tab wiring
```

## How it works

The Attributes dialog, `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/attributes_modal.rs`,
mounts `ArsenalTab` with the slot's `SlotUid`, its `loadout` JSON, the
[registry](/documentation/glossary/n_to_z.md#registry) rows and the compatibility feed. In the browser
build the tab first asks the map engine to seed the character's default cargo when the loadout has
no `cargo` key, then turns the JSON into picks and holds the picks, the cargo and every outcome in
`ArsenalTabState`. It shows "Loading catalog…" until the registry arrives; `tab_content.rs` then
renders the loaded view.

```text
pick or cargo edit ──> loadout::picks_to_loadout ──> loadout_commands::set_loadout
                                                     ──> map engine: update_slot_loadout
                                                     ──> history tail, only when acknowledged
Copy ──> map engine: copy_loadouts_from_selection ──> the copy buffer
Apply ──> loadout::plan_apply (buffer, seeded draw, rules) ──> map engine: commit_loadout_writes
Remove Everything ──> loadout::plan_remove ──> map engine: commit_loadout_writes
```

The Arsenal has no Save button: every pick and cargo edit is written to the
[mission](/documentation/glossary/g_to_m.md#mission) document at once, as one undo step, and the tab
repeats the mission's unsaved state because the dialog's backdrop hides the top strip's marker. A
write the document refuses, because the entity is gone, shows its own warning instead. The state
layer's `arsenal_rules` decides the options, validity, capacity and weight, `loadout/` serialises
and gates, and `doll.rs` mounts the 3D doll in the browser; `panels.rs` draws the doll host (a flat
SVG doll when the renderer cannot start), the compatibility panel with its attachment toggles, and,
in `panels/`, the cargo editor. Every option button carries `data-value` with its resource name,
which the smoke test selects on, and an attachment toggle also carries `data-attachment`. Apply and Remove Everything ask the browser to confirm before they change
more than ten slots, through the bridge's `confirm_bulk_n_step`.

## Public surface

- `ArsenalTab` (`arsenal_tab.rs`): the tab component, mounted by
  `crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/attributes_modal.rs`.
- `loadout`: `attachments_key`, `attachments_of`, `pack_attachments` and `ATTACHMENT_EDGE`, and
  `region_title`, `MaterialCheck` and `doll::ArsenalDoll`, for the Arsenal panels.
- `panels::{doll_view, compat_panel, cargo_panel}` (crate-visible): the three views the tab
  renders.
- The crate README lists the public surface the workspace reads.

## Boundaries

- Depends on:
  - `frontend_api_dtos` (`RegistryItem`, `RegistryCompatEdge`);
  - `mission_creator_engine_bridge`'s `bridge` (`document_host::history` for the dirty flag and
    the history tail, `host_state::undo_grouped_gestures::confirm_bulk_n_step`, and
    `host_state::editor_context::slots_json`), `mission_creator_state`'s `arsenal_rules`, and
    `mission_creator_session`'s `document_commands::download_json` for the export download;
  - `orbat_slot_ids` (`SlotUid`, the slot the tab edits);
  - `mission_editing_commands::hosted_commands` (the loadout reads and writes, the copy buffer,
    the cargo seed) and `mission_editing_session::host::with_doc`;
  - `paper_doll_renderer` (`PaperDollRenderer`, the browser build only);
  - `mission_operations` (`assets`, `cargo`, `cargo_rules`);
  - `web_sys` in the browser build.
- Used by:
  - in `crates/frontend/workspaces/mission_creator_workspace/src/`: `ui/inspector/attributes_modal.rs` (the
    Attributes dialog mounts `ArsenalTab`) and the page tests under `tests/`;
  - `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/arsenal.rs`, which drives the
    tab in a headless browser and clicks the compatibility panel's buttons by `data-value`;
  - `tools/checks/repository_checks/src/architecture/editor_orbat_coherency.rs`, which scans
    `loadout_commands.rs`.
- Rules:
  - a pick reaches the document only through `loadout_commands` and the map engine's hosted
    commands, and the history tail runs only on an acknowledged write
    (`set_loadout_returns_the_documents_answer_instead_of_a_hardcoded_true` and
    `the_panel_states_the_persistence_contract` in `tests/shell_wiring.rs`);
  - `doll.rs` and `loadout_commands.rs` are `#[cfg(target_arch = "wasm32")]` on their `pub mod`
    lines, and `loadout` stays free of `web_sys`, so the native tests cover it;
  - a panel renders and reports; a cargo edit commits in its own handler
    (`cargo_mutations_commit_without_a_staging_gate` in `tests/panels/cargo_persistence.rs`);
  - `arsenal_tab.rs` cites `set_loadout` and its history tail by line in `loadout_commands.rs`, and
    `arsenal_cites_live_set_loadout_lines` fails when the lines move;
  - `loadout_commands.rs` must keep its path and never call `ensure_default_squad`
    (`cargo xtask verify editor-orbat-coherency`).

## Related documentation

- [Arsenal loadout editor](/documentation/crates/frontend/workspaces/mission_creator_arsenal/arsenal_loadout_editor.md) — the tab's
  flows, rules, data, design references, open work and decisions.
- [Mission Creator feature inventory: attributes dialog](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md) — the Attributes dialog and its Arsenal tab.
- [Mission Creator documentation](/documentation/crates/frontend/workspaces/mission_creator_workspace/README.md) — the
  Mission Creator's documents, starting from its roadmap.
