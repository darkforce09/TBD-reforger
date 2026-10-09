# Wave plan

The models behind the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s Waves tab: it
reads `.ai/tickets/wave.lock` itself, projects the recorded
[wave](/documentation/glossary/n_to_z.md#wave) lanes exactly as stored, lists the dispatchable
[tickets](/documentation/glossary/n_to_z.md#ticket) no wave holds, and lists the colliding `owns`
pairs the ticket comparison explains. The desktop application paints the tab from
`tools/tickets/ticketboard_desktop/src/wave_plan/ui/`.

## Contents

```text
tools/tickets/ticketboard_model/src/wave_plan/
├── events.rs  `WavePlanEvent`: select, compare, copy a lane's TSV, toggle wave 0
├── mod.rs     the module tree
├── models/    `WavesModel` (lanes as recorded, wave 0, unplanned) and the borrowed `WavePlanView`
└── services/  the read-only lock reader with its missing and refused states, and the collision rule
```

## How it works

```text
.ai/tickets/wave.lock ──load_lock (load thread)──▶ LockState
    Loaded(WaveLock) + Corpus ──WavesModel::build──▶ lanes, wave 0, unplanned
    Missing / Refused ──▶ refusal screen on the Waves tab only
WavePlanView (borrowed) ──▶ the application's Waves tab ──▶ WavePlanEvent ──▶ Action
```

The combined load (`crate::application_state::background_loading`) calls `services::lock_file::load_lock` on its worker thread, and
`WorkspaceState` builds `WavesModel` when the lock loaded. The viewer never packs waves: lanes keep
the lock's order and membership, filters only dim chips, and the unplanned bucket is set
arithmetic over the ticket files. `cargo xtask wave repack` is the lock's only writer; after a
failed ticket command the viewer shows that command as text, and the next reload picks up the lock
it writes. `ticket_wave_lock::collides` and this reader's `colliding_pairs` give the comparison view its
"never the same wave" verdict and the pairs behind it for two compared tickets.

## Public surface

- `services::lock_file`: `load_lock` and `LockState`, loaded by
  `crate::application_state::background_loading`; `WaveLock` and `LockWave`, built by the
  application tests; `paths_collide` and `colliding_pairs`, used by the comparison view.
- `models::wave_projection::WavesModel` and `models::view::WavePlanView`, which `WorkspaceState`
  builds and the desktop application lends and paints.
- `events::WavePlanEvent`, which `crate::application_state::events` converts into `Action`s.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, `projection`); the crate's `Error`
  (`WaveLockUnparsable`); `ticket_model` (`StatusName`, `Ticket`, `TicketId`); `ticket_wave_lock`
  (`lock_path`, `missing_lock_error`); `serde` and `toml`.
- Used by: `crate::application_state` (`background_loading.rs`, `workspace_state.rs`,
  `events.rs`) and its tests; the desktop application: `tools/tickets/ticketboard_desktop/src/wave_plan/ui/`,
  `tools/tickets/ticketboard_desktop/src/application/` (`mod.rs`, `feature_views.rs`) and the comparison view in
  `tools/tickets/ticketboard_desktop/src/ticket_browser/ui/detail_panel/comparison.rs`.
- Rules:
  - the feature reads the lock and never writes it or recomputes packing
    (`lanes_render_the_lock_verbatim_never_sorted` in `models/tests/wave_projection.rs`);
  - a missing or malformed lock is a refusal on this tab and never empties the board
    (`missing_lock_is_the_did_not_run_refusal` in `services/tests/lock_file.rs`);
  - nothing here names egui (the test
    `model_dependency_boundaries_and_external_test_placement_are_enforced` in
    `tools/tickets/ticketboard_model/src/tests/architecture_rules.rs`).

## Related documentation

- [Wave lock command group](/tools/xtask/src/commands/wave/README.md) — the commands that
  write and check the lock.
- [Wave lock](/tools/tickets/ticket_wave_lock/src/README.md) — the packing and collision
  rules this viewer mirrors.
- [Factory waves](/documentation/runbooks/factory_waves/README.md) — how waves are packed and
  run.
