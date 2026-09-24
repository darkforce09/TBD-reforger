# Win conditions card view

The view of the rule that ends a [mission](/documentation_v2/glossary.md#mission)'s round, a card
of the Mission Settings dialog: the mission authors it in its `winConditions` block as a mode, the
mode's own field and the checklist of triggers that end the round. The model and the document
write live in the parent module,
`apps/website/frontend/src/v2/apps/editor/ui/inspector/win_conditions_card.rs`.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/ui/inspector/win_conditions_card/
└── view.rs  `win_conditions_card`: the "Win rule" picker, the mode's field and the end-on checklist
```

## How it works

In the browser build the card reads `meta.environment.winConditions` once per build. The "Win rule"
picker offers "None — attrition, derived from the ORBAT" and one entry per authored mode (attrition,
objective, extraction, VIP, timeout); None writes `null`, which leaves the compiler's derived
attrition rule in force. With a mode picked, the card shows that mode's field (a timeout takes whole
minutes from `TIMEOUT_MINUTES_MIN`, 1, to `TIMEOUT_MINUTES_MAX`, 1440) and the "Ends the round on"
checklist. An accepted edit writes the whole block as one environment update, one undo step; a
refused edit, such as unticking the last trigger, shows its reason, and a block the map engine's
validation does not yet accept says the mission will run on the derived attrition rule. The native
build renders nothing.

## Boundaries

- Depends on: the parent module (`mode_options`, `param_fields`, `with_mode`, `with_param`,
  `with_trigger` and the write through the bridge's `editor_context::update_environment`), whose
  modes, triggers and limits come from `website_map_engine::data::scenario::win_conditions`.
- Used by: the parent module, which re-exports `win_conditions_card`; the Mission Settings dialog,
  which renders it after the mission flow section
  (`apps/website/frontend/src/v2/apps/editor/ui/modals/settings_modal/mission_dialog.rs`).
- Rules: the checklist never goes empty (`unticking_the_last_trigger_is_refused` in
  `apps/website/frontend/src/v2/apps/editor/ui/inspector/tests/win_conditions_card/mode_authoring.rs`).
