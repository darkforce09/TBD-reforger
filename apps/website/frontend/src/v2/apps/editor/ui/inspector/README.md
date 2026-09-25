# Mission Creator inspectors

The [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s editing surfaces for one
subject at a time: the Attributes dialog for placed [slots](/documentation_v2/glossary.md#slot) and
vehicles, the zones tab, the live validation of the [mission](/documentation_v2/glossary.md#mission),
and the panels that author the mission's own settings blocks (win conditions, spawn modules, radio
nets, tasks, audio, weather timeline).

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/
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
├── vehicles_panel.rs       `placed_vehicles_panel`: placed vehicles with heading, cargo and delete
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
| Radio nets, tasks, audio and weather timeline panels | `radio_panel`, `tasks_panel`, `audio_emitters`, `weather_timeline` | nothing |
| Placed vehicles panel | `vehicles_panel` | nothing; the Attributes dialog edits a vehicle |

`meta.environment` is the mission's settings bag. Each block module owns one key of it (`radioPlan`,
`tasks`, `audio`, `weatherTimeline`, `spawnModules`, `winConditions`): its panel reads the key back
through the bridge's `editor_context::read_env_value` and writes the whole rebuilt block as a merge
patch through `editor_context::update_environment`, with `null` to clear it, and its `*_READERS`
table names the key's readers from the compiler to the [mod](/documentation_v2/glossary.md#mod).
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
  `known_asset_ids_from_registry`) for the canvas mount; `publish_compile_findings`, `PanelFinding`
  and `clear_compile_findings` for the document export and the mission boot; `chip_findings`,
  `Rollup` and `findings_dropdown` for the top strip; `route_select_by_subject_id` and
  `subject_id_routes` for the left dock, the outliner rows and the All Settings dialog;
  `install_seam` and `SeamRegistration` for the input tools and the world-assets host.
- `zones_panel`: `zones_panel` for the right dock; the shape predicates, `DrawTarget` and
  `ZoneShape` for the bridge's zone draw, through `shell::eden_chrome`, and for the right dock's
  triggers panel; `MISSION_SCHEMA`, `humanize_token`, `humanize_key`, `zone_rule_fields` and
  `project_owner_line` for the right dock's markers and triggers panels and the Mission Settings
  dialog.
- `win_conditions_card::win_conditions_card` and `spawn_modules::spawn_modules_panel`: rendered by
  the Mission Settings dialog.
- `env`: `author_env` for the top strip and the Mission Settings dialog; the flow helpers and
  defaults (`read_flow_seconds`, `read_flow_jip`, `parse_flow_seconds`, `fmt_duration_secs`,
  `FLOW_DEFAULT_*`, `JIP_OPTIONS`) and the notes `ENV_UNCARRIED_NOTE` and `SETTINGS_UNREAD_NOTE`
  for the Mission Settings dialog.

## Boundaries

- Depends on:
  - `website_map_engine`: `editing::hosted_commands` and `editing::host`; the `data::scenario`
    modules `validate`, `radio_plan`, `tasks`, `audio`, `weather`, `spawn_modules`,
    `win_conditions`, `flatten`, `compile` and `tactical_graphics`; `data::store::operations`;
  - the editor's bridge (`host_state::editor_context`, `host_state::armed_placement`,
    `tactical_graphics_authoring`), the [Arsenal](/documentation_v2/glossary.md#arsenal)
    (`ArsenalTab`, `asset_catalog`, `rules::CompatFeed`), the outliner's row classes and
    `shell::layout::HOVER_FILL`;
  - `crate::v2::core`: `ui::modal_stack`, `MaterialIcon`, `cn` and the `RegistryItem` DTO;
  - `contracts_v2/definitions/mission.schema.json`, embedded for the zone vocabulary.
- Used by:
  - the editor page, `apps/website/frontend/src/v2/apps/editor/mission_editor.rs`, and, in
    `apps/website/frontend/src/v2/apps/editor/mission_editor/`, `canvas_mount.rs`,
    `canvas_mount/boot_tasks.rs` and `canvas_mount/review_restore.rs`;
  - in `apps/website/frontend/src/v2/apps/editor/`: `shell/eden_chrome.rs`,
    `shell/document_commands/imp/exports.rs`, `bridge/host_state/armed_placement/zone_draw.rs`,
    `bridge/world_assets.rs`, `input/tools/ruler_tool.rs` and `input/tools/los_tool.rs`;
  - the sibling surfaces in `apps/website/frontend/src/v2/apps/editor/ui/`: the right and left
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

- [Mission Creator feature inventory: attributes dialog](/documentation_v2/website/frontend/apps/editor/feature_inventory/attributes_and_settings.md) — the Attributes dialog and its tabs.
- [Eden attribute catalog](/documentation_v2/website/frontend/apps/editor/eden_editor_reference/attributes.md)
  — the Arma 3 Eden attributes the inspectors are measured against.
