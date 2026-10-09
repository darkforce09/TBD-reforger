# Mission Creator Arsenal

The `mission_creator_arsenal` crate: the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
[arsenal](/documentation/glossary/a_to_f.md#arsenal), the Attributes dialog's tab where a mission
maker edits one [slot](/documentation/glossary/n_to_z.md#slot)'s loadout. It holds the tab and its
loaded catalog view, the panels drawn around a slot's loadout (the doll host with its SVG fallback,
the compatibility panel, the cargo editor), the 3D paper doll host, the pure loadout core (the
loadout JSON, the export and import gates, the copy-and-apply buffer and the receipts) and the
loadout writes to the [mission](/documentation/glossary/g_to_m.md#mission) document.

## Contents

```text
crates/frontend/workspaces/mission_creator_arsenal/
├── Cargo.toml  the package: the three lower Mission Creator crates, the API DTOs, the mission and editing crates, layout tier 11, any target
└── src/        the tab, its loaded view and panels, the paper doll host, the loadout core and the document writes
```

## How it works

The workspace's Attributes dialog mounts `ArsenalTab` with the slot's `SlotUid`, its `loadout`
JSON, the [registry](/documentation/glossary/n_to_z.md#registry) rows and the compatibility feed.
Every pick and cargo edit is written to the document at once through `loadout_commands`, as one
undo step, and the history tail runs only when the document acknowledges the write. The rules
that decide the options, validity, capacity and weight sit in the state crate's
`arsenal_rules`; the [source tree README](src/README.md) walks through each module.

Everything that touches `web_sys`, the paper doll renderer or the live document is compiled for
`wasm32` only; the loadout core compiles on every target, so its tests run natively.

## Getting started

Run from the repository root:

```bash
cargo test -p mission_creator_arsenal   # the loadout core, the export and import gates and the source pins
```

## Public surface

- `ArsenalTab` (`arsenal_tab`, `wasm32`): the tab component the Attributes dialog mounts.
- `loadout`: `loadout_to_picks`, `picks_to_loadout`, `picks_to_export`, `try_export`,
  `try_import` with `ImportedLoadout`, `refusal_line`, the buffer verbs (`plan_apply`,
  `plan_remove`, `commit_one_write`, `buffer_draw`, `buffer_refusals`, `stripped_loadout`) and the
  receipts (`copy_receipt`, `apply_receipt`, `remove_receipt`); the modpack a file names is a
  `frontend_api_dtos::identifiers::ModpackId`.
- `loadout_commands` (`wasm32`): `set_loadout`, `apply_loadout_buffer_to_selection`,
  `remove_all_loadouts_from_selection`.
- `doll` (`wasm32`): `ArsenalDoll`; `panels`: the panel views the tab renders.
- `prelude`: `loadout_to_picks`, `picks_to_loadout` and, on `wasm32`, `ArsenalTab`.

## Boundaries

- Depends on: `mission_creator_state`, `mission_creator_engine_bridge`, `mission_creator_session`,
  `frontend_api_dtos`, `mission_operations`, `mission_editing_commands`, `mission_editing_session`,
  `orbat_slot_ids`, `deterministic_random`, `leptos`, `serde_json`; on `wasm32` `paper_doll_renderer`, `web-sys`, `js-sys`,
  `wasm-bindgen`, `wasm-bindgen-futures`; `frontend_test_support` and the engine bridge's
  `test_fixtures` for its tests.
- Used by: the single-page app (`crates/frontend/shell/frontend_application`), whose Attributes dialog mounts `ArsenalTab`;
  `tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/arsenal.rs`, which drives the
  tab in a headless browser; `tools/checks/repository_checks/src/architecture/editor_orbat_coherency.rs`,
  which scans `src/loadout_commands.rs`.
- Rules: depends on no Mission Creator crate above `mission_creator_session`
  (`cargo xtask ci verify-workspace-laws`); a pick reaches the document only through
  `loadout_commands` and the hosted commands.

## Related documentation

- [Source tree](src/README.md) — each module.
- [Arsenal loadout editor](/documentation/crates/frontend/workspaces/mission_creator_arsenal/arsenal_loadout_editor.md)
  — the tab's flows, rules, data, design references, open work and decisions.
- [Mission Creator feature inventory: attributes dialog](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md)
  — the Attributes dialog and its Arsenal tab.
