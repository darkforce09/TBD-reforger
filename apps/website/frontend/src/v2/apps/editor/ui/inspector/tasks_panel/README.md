# Tasks panel view

The view of the tasks panel: the primary, secondary and optional assignments a
[mission](/documentation_v2/glossary/g_to_m.md#mission) authors in its `tasks` block, each with the
trigger that completes it, the marker the HUD points at and an optional schedule. The model and the
document write live in the parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/tasks_panel.rs`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/tasks_panel/
└── view.rs  `tasks_panel`: the task list with its pickers, schedule fields and "Add task" action
```

## How it works

In the browser build the panel reads `meta.environment.tasks` once per build and offers the
document's triggers and markers as picks. Each task edits its title, "Tier", "State", "Trigger"
("None — this task does not auto-complete"), "Marker" ("None — HUD uses objective_marker"),
"Description", "Start after (s)" and "Window (s)", moves with "Up" and "Down", and goes with
"Remove". A schedule is checked against the mission's authored time limit, `timeLimitSeconds`. An
accepted edit writes the whole list as one environment update, one undo step, and removing the last
task writes `null`; a refused edit (a blank title, a duplicate id, a half-filled or impossible
schedule) shows its reason. The native build renders nothing.

## Boundaries

- Depends on: the parent module (`tasks_from_block`, `add_task`, `move_task`, `remove_task`,
  `with_field`, `with_schedule`, the trigger and marker options read through
  `website_map_engine::editing::hosted_commands`, and the write through the bridge's
  `editor_context::update_environment`), whose tiers, states and schedule check come from
  `website_map_engine::data::scenario::tasks`; `read_flow_seconds` and `FLOW_DEFAULT_TIMELIMIT_S`
  of the sibling `env` module.
- Used by: the parent module, which re-exports `tasks_panel`. No surface mounts the panel: the
  Mission Settings dialog in `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`
  does not render it.
- Rules: a schedule is written whole or not at all (`a_half_filled_schedule_is_refused` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/tasks_panel/task_authoring.rs`).
