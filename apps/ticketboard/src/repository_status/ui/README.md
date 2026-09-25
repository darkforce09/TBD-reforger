# Status banner

The egui banner across the top of the [ticketboard](/documentation_v2/glossary.md#ticketboard):
the strict check's verdict, its output, the state of the file watch and the `git status` chip.

## Contents

```text
apps/ticketboard/src/repository_status/ui/
├── mod.rs            the module tree; re-exports `trust_banner_ui`
└── status_banner.rs  `trust_banner_ui`, the tone colours, the output pane, the chip and its file list
```

## How it works

`trust_banner_ui(view, ui, actions)` draws one row: a " STRICT " badge whose tooltip warns that the
command preflight is not strict, a spinner while the check runs, the headline in its tone
colour (grey, amber, `VERDICT_OK` or `VERDICT_COLLIDE`), "Re-check", "✕ cancel" while running, an
"output (N)" toggle, "watch unavailable" or "watch degraded" with the reason in the tooltip, and the
`git status` chip on the right. Below the row it draws, when toggled, the check's merged output
from the `BoundedLog` (at most 260 points high, stuck to the bottom, with a count of dropped
lines), and the chip's porcelain lines when the chip is dirty and expanded. Clicks come back as
`StatusEvent`: `Recheck`, `CancelCheck`, `ToggleOutput` and `ToggleGitList`.

## Boundaries

- Depends on: `crate::repository_status::models` (`StatusView`, `Tone`, `GitChip`, `CHECK_COMMAND`,
  `STRICT_TOOLTIP`, `GIT_ARGS`) and `crate::repository_status::events`; `crate::core::process`
  (`BoundedLog`) and `crate::core::ui` (`VERDICT_OK`, `VERDICT_COLLIDE`, `OUTPUT_ROW_H`);
  `eframe::egui`.
- Used by: `trust_banner_ui` in `apps/ticketboard/src/application/feature_views.rs`, which lends
  the view and turns the events into actions.
- Rules: the output and the porcelain lines show verbatim; no other feature imports this module
  (`dependency_boundaries_and_external_test_placement_are_enforced` in
  `apps/ticketboard/src/tests/architecture_rules.rs`).
