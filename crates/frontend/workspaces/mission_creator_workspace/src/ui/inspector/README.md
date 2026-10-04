# Mission Creator inspectors

The [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s editing surfaces for one
subject at a time: the Attributes dialog for placed [slots](/documentation/glossary/n_to_z.md#slot) and
vehicles, the zones tab, the live validation of the [mission](/documentation/glossary/g_to_m.md#mission),
and the panels that author the mission's own settings blocks (win conditions, spawn modules, radio
nets, tasks, audio, weather timeline).

## Contents

```text
crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/
├── attributes_modal/       the Attributes dialog's tabs, fields, multi-edit gates and vehicle view
├── attributes_modal.rs     `AttributesModal`: the dialog over the selected slots or one vehicle
├── audio_emitters/         the audio panel's view
├── audio_emitters.rs       the `audio` block: emitters, music cues, the place-on-map point
├── env.rs                  `author_env`, the environment write gate; the mission flow fields
├── mod.rs                  the module tree
├── radio_panel/            the radio nets panel's view
├── radio_panel.rs          the `radioPlan` block: nets, frequencies, ranges, reset to derived
├── spawn_modules/          the spawn modules section's view
├── spawn_modules.rs        the `spawnModules` block: waves and garrisons
├── tasks_panel/            the tasks panel's view
├── tasks_panel.rs          the `tasks` block: tiers, states, triggers, markers, schedules
├── tests/                  unit tests for every module, one folder each
├── validation_panel/       the validation loop, the findings dropdown and the subject routes
├── validation_panel.rs     the finding model: `PanelFinding`, `Rollup`, rule groups, the debouncer
├── weather_timeline.rs     the `weatherTimeline` block and its panel: keyframes over mission time
├── win_conditions_card/    the win conditions card's view
├── win_conditions_card.rs  the `winConditions` block: the mode, its field, the end-on triggers
├── zones_panel/            the zones tab: list, draw tools, attributes, geometry, schema vocabulary
└── zones_panel.rs          the zones module tree; re-exports the zone geometry and vocabulary
```

## How it works

| Surface | Module | Mounted by |
|---|---|---|
| Attributes dialog | `attributes_modal` | the editor page |
| Validation loop | `validation_panel` | the editor page; its findings drop down from the top strip's chip |
| Zones tab | `zones_panel` | the right dock |
| Win conditions card, spawn modules section | `win_conditions_card`, `spawn_modules` | the Mission Settings dialog |
| Radio nets, tasks, audio and weather timeline panels | `radio_panel`, `tasks_panel`, `audio_emitters`, `weather_timeline` | the Mission Settings dialog, after the win conditions card |

`meta.environment` is the mission's settings bag. Each block module owns one key of it (`radioPlan`,
`tasks`, `audio`, `weatherTimeline`, `spawnModules`, `winConditions`): its panel reads the key back
through the bridge's `editor_context::read_env_value` and writes the whole rebuilt block as a merge
patch through `editor_context::update_environment`, with `null` to clear it, and its `*_READERS`
table names the key's readers from the compiler to the [mod](/documentation/glossary/g_to_m.md#mod).
Single keys that the top strip and the Mission Settings dialog write go through `env::author_env`,
which refuses any key missing from `CARRIED_ENV_KEYS` (`time`, `weather`, `showHillshade`,
`hillshadeOpacity`, `showGrid`) and `AUTHORED_FLOW_KEYS` (`briefingSeconds`, `safeStartSeconds`,
`timeLimitSeconds`, `jip`); the flow defaults are the compiler's own constants, re-exported.

Across the folder, a write is one undo step and reaches the document through the map engine's
hosted commands or the environment update, never by changing a row in place. Every field re-reads
the document when `doc_tick` moves, so an undo taken while a panel is open refreshes it. The eleven
modules compile in the native build: their browser-only bodies sit inside the views and their
handlers, and each view has a native stand-in that renders nothing.

## Public surface

- `attributes_modal::AttributesModal`: mounted by the editor page.
- `validation_panel`: `ValidationPanel`, mounted by the editor page; the hook registrations
  (`register_payload_source`, `register_route_probe`, `register_select_by_id`, `PayloadSource`,
  `known_asset_ids_from_registry`) for the canvas mount; `register_compile_findings_publisher` for
  the editor page, which fills the session's compile-findings publisher at mount; `PanelFinding`
  and `clear_compile_findings` for the mission boot; `chip_findings`,
  `Rollup` and `findings_dropdown` for the top strip; `route_select_by_subject_id` and
  `subject_id_routes` for the left dock, the outliner rows and the All Settings dialog;
  the seam registration it installs its hooks with is the state layer's `install_seam`.
- `zones_panel`: `zones_panel` for the right dock; the zone vocabulary and geometry it draws are the
  state layer's `zones` module.
- `win_conditions_card::win_conditions_card` and `spawn_modules::spawn_modules_panel`: rendered by
  the Mission Settings dialog.
- `env`: `author_env` for the top strip and the Mission Settings dialog; the flow helpers and
  defaults (`read_flow_seconds`, `read_flow_jip`, `parse_flow_seconds`, `fmt_duration_secs`,
  `FLOW_DEFAULT_*`, `JIP_OPTIONS`) and the notes `ENV_UNCARRIED_NOTE` and `SETTINGS_UNREAD_NOTE`
  for the Mission Settings dialog.

## Boundaries

- Depends on:
  - `mission_editing_commands::hosted_commands` and `mission_editing_session::host`;
  - the mission crates: `mission_validation`; `mission_model`'s `radio_plan`, `tasks`, `audio`,
    `weather`, `spawn_modules`, `win_conditions` and `tactical_graphics`; `mission_compiler`;
    `mission_payload`; `mission_operations`;
  - the editor's bridge (`host_state::editor_context`, `host_state::armed_placement`,
    `tactical_graphics_authoring`), the [Arsenal](/documentation/glossary/a_to_f.md#arsenal)
    (`ArsenalTab`, `asset_catalog`, `rules::CompatFeed`), the outliner's row classes and
    `frontend_ui::tokens::HOVER_FILL`;
  - the foundation crates: `ui::modal_stack`, `MaterialIcon`, `cn` and the `RegistryItem` DTO;
  - `contracts/definitions/mission.schema.json`, embedded for the zone vocabulary.
- Used by:
  - the editor page, `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor.rs`, and, in
    `crates/frontend/workspaces/mission_creator_workspace/src/mission_editor/`, `canvas_mount.rs`,
    `canvas_mount/boot_tasks.rs` and `canvas_mount/review_restore.rs`;
  - in `crates/frontend/workspaces/mission_creator_workspace/src/`: `session/document_commands/imp/exports.rs`;
  - the sibling surfaces in `crates/frontend/workspaces/mission_creator_workspace/src/ui/`: the right and left
    docks, the top strip, the outliner rows and the Mission Settings dialog.
- Rules:
  - an environment key written on its own has a reader in `env`'s tables
    (`keys_nothing_reads_are_not_authored` and `every_carried_key_names_its_reader` in
    `tests/env/environment_flow_contract.rs`);
  - every block panel clears its key with an explicit `null` and names each reader of the key
    (each block's `null`-patch test and its `the_reader_chain_names_every_hop`);
  - the Attributes dialog registers with `core::ui::modal_stack` and answers Escape only while
    topmost (`tests/attributes_modal/modal_escape_stack.rs`).

## Related documentation

- [Mission Creator feature inventory: attributes dialog](/documentation/crates/frontend/workspaces/mission_creator_workspace/feature_inventory/attributes_and_settings.md) — the Attributes dialog and its tabs.
- [Eden attribute catalog](/documentation/crates/frontend/workspaces/mission_creator_workspace/eden_editor_reference/attributes.md)
  — the Arma 3 Eden attributes the inspectors are measured against.
