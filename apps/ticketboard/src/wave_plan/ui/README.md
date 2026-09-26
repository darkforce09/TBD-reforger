# Waves tab

The egui view of the [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s Waves tab: the
recorded [wave](/documentation_v2/glossary/n_to_z.md#wave) lanes as chips, or the refusal when the lock
is missing or unreadable.

## Contents

```text
apps/ticketboard/src/wave_plan/ui/
├── lane_view.rs  `waves_ui`: refusal screens, header strip, lanes, wave 0 and the unplanned bucket
└── mod.rs        the module tree; re-exports `waves_ui`
```

## How it works

`waves_ui` paints from a `WavePlanView`. A missing lock shows "No wave plan", the DidNotRun text
and a note that the lock is rendered verbatim; a refused lock shows "wave.lock refused to parse",
the path and the error verbatim. A loaded lock shows, in one scroll area: the header with the
`pack_last` chips, each lane with its label and a "copy TSV" button, wave 0 as its count button
(which expands a virtualized list of its ids) with its own "copy TSV", and the "Unplanned" bucket
with a note that it comes from the ticket files, not the lock.

Each chip is the ticket id in its status colour, struck through when the lock names an id with no
ticket file, with the title as tooltip. Active filters dim the chips of non-matching tickets
instead of hiding them, so a lane always shows the lock's exact membership. A click emits
`WavePlanEvent::Select`, a shift-click `Compare`; the buttons emit `CopyText` and `ToggleWave0`.

## Boundaries

- Depends on: `crate::wave_plan::models` and `crate::wave_plan::services::lock_file::LockState`;
  `crate::ticket_registry::models::palette::status_rgb`; `ticket_engine::StatusName`;
  `eframe::egui`.
- Used by: `crate::application::feature_views`, which paints the Waves tab.
- Rules: the tab only reads and emits events; it never reorders or recomputes a lane, and filters
  dim chips without removing them. `apps/ticketboard/src/application/tests/rendering.rs` paints
  the tab over a loaded lock headlessly; the missing and refused screens have no rendering test.
