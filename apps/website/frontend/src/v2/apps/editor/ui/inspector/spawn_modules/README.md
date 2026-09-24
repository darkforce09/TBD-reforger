# Spawn modules panel view

The view of the waves and garrisons of AI groups a [mission](/documentation_v2/glossary.md#mission)
authors in its `spawnModules` block, a section of the Mission Settings dialog. The model and the
document write live in the parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/spawn_modules.rs`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/spawn_modules/
└── view.rs  `spawn_modules_panel`: the module list with its "Add module" action
```

## How it works

In the browser build the section reads `meta.environment.spawnModules` once per build and lists
each module with "Kind" (wave or garrison), "Faction", "Group template", "X" and "Z" or a
"Zone id", "Count", "Interval (s)", "Max alive" and "Trigger id", and a "Delete". Its hint states
the model: a wave restocks on an interval up to its maximum alive, a garrison spawns once and
holds, and a module is placed by X and Z or by a zone, never both. "Add module" appends a wave with
a fresh `sm-N` id. An accepted edit writes the whole block as one environment update, one undo
step, and deleting the last module writes `null`; a refused edit shows its reason under the list.
The native build renders nothing.

## Boundaries

- Depends on: the parent module (`modules_from_block`, `add_module`, `remove_module`, `with_field`,
  `block_from_modules` and the write through the bridge's `editor_context::update_environment`),
  whose vocabularies and limits come from `website_map_engine::data::scenario::spawn_modules`.
- Used by: the parent module, which re-exports `spawn_modules_panel`; the Mission Settings dialog,
  which renders it after the win conditions card
  (`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/mission_dialog.rs`).
- Rules: a module placed both by position and by zone is refused in the panel
  (`both_position_and_zone_are_refused_in_the_panel` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/spawn_modules/spawn_module_authoring.rs`).
