# Radio nets panel view

The view of the radio nets panel: the nets a [mission](/documentation_v2/glossary.md#mission)
authors in its `radioPlan` block (id, label, frequency, faction assignment and range) and the action
that returns the mission to the plan the compiler derives. The model and the document write live in
the parent module, `apps/website/frontend/src/v2/apps/editor/ui/inspector/radio_panel.rs`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/radio_panel/
└── view.rs  `radio_panel`: the net list with its "Add net" and "Reset to derived" actions
```

## How it works

In the browser build the panel reads `meta.environment.radioPlan` once per build. Without an
authored plan its hint says the compiler derives one net per side plus one per squad. Each net row
edits "Id", the label, "Frequency (MHz)", "Faction assignment" and "Range", moves with "Up" and
"Down", and goes with "Remove"; "Add net" appends a net with a free id and frequency. An accepted
edit writes the whole plan as one environment update, one undo step, and removing the last net or
"Reset to derived" writes an explicit `null`, which leaves the plan to the compiler. A refused edit,
such as a duplicate or out-of-band frequency or a net past the cap, shows its reason under the list.
The native build renders nothing.

## Boundaries

- Depends on: the parent module (`nets_from_block`, `add_net`, `move_net`, `remove_net`,
  `with_field`, `plan_from_nets` and the write through the bridge's
  `editor_context::update_environment`), whose limits come from
  `website_map_engine::data::scenario::radio_plan`.
- Used by: the parent module, which re-exports `radio_panel`. No surface mounts the panel: the
  Mission Settings dialog in `apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/`
  does not render it.
- Rules: clearing the plan writes `null` rather than omitting the key, because an environment
  update merges (`reset_to_derived_writes_an_explicit_null_patch` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/radio_panel/radio_network_authoring.rs`).
