# Wave plan display models

The projection of the recorded [wave](/documentation_v2/glossary/n_to_z.md#wave) plan that the
[ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s Waves tab paints: the lanes exactly as
the lock stores them, wave 0, and the dispatchable tickets no wave holds.

## Contents

```text
apps/ticketboard/src/wave_plan/models/
├── mod.rs              the module tree
├── tests/              unit tests for the dispatchable rule, unplanned ids, lanes and the TSV
├── view.rs             `WavePlanView`: lock state, model, verdicts and selection lent to the tab
└── wave_projection.rs  `WavesModel` with its lanes, wave 0 and unplanned bucket, and `dispatchable`
```

## How it works

`WavesModel::build` runs once per load when the lock parsed. From the lock it takes the header
(`wave_base <n> · max_concurrent <n>`), the `pack_last` ids, one `Lane` per wave numbered above 0 in
lock order with its tickets in lock order, labelled `wave <n> · <count>`, and wave 0 as a
`<count> parked` chip. Each id becomes a `WaveChip` that resolves through the corpus to its status
and title; an id with no ticket file keeps its place, with no status and the tooltip "no ticket
file — rendered as recorded in the lock". Each lane and wave 0 carry their `<n><TAB><id>` lines for
the copy button.

The unplanned bucket is derived from the ticket files, not the lock: every dispatchable ticket
that no wave (wave 0 included) lists, in numeric id order. `dispatchable` copies the rule of
`TicketView::dispatchable` in `tools_v2/ticket-engine/src/wave_lock/model.rs`: a work ticket with
a live status (`queued`, `ready`, `running`, `review`) whose executor is `claude-code`, the default
when unset.

`WavePlanView` lends the tab the lock state, the model, whether filters are active with the
per-ticket verdicts, the selection and comparison, and whether wave 0 is expanded.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, the `projection` title, executor and
  sort helpers); `crate::wave_plan::services::lock_file` (`WaveLock`, `LockState`);
  `ticket_engine` (`StatusName`, `Ticket`).
- Used by: `crate::wave_plan::ui`; `crate::application`, whose `workspace_state.rs` builds the
  model from a loaded lock and whose `feature_views.rs` lends the view.
- Rules:
  - lanes keep the lock's order and membership, and the viewer never recomputes packing
    (`lanes_render_the_lock_verbatim_never_sorted` in `tests/wave_projection.rs`);
  - unplanned is set arithmetic over the lock and the dispatchable tickets, never lane membership
    from status (`unplanned_is_pure_set_arithmetic`);
  - a lock id without a ticket file stays visible and flagged
    (`lock_id_without_ticket_file_is_flagged`);
  - the copy text is one `<n><TAB><id>` line per ticket (`lane_tsv_format_is_n_tab_id_lines`).
