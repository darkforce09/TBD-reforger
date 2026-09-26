# Wave plan

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) feature behind the Waves tab: it
reads `.ai/tickets/wave.lock` itself, shows the recorded [wave](/documentation_v2/glossary/n_to_z.md#wave)
lanes exactly as stored, lists the dispatchable [tickets](/documentation_v2/glossary/n_to_z.md#ticket)
no wave holds, and supplies the ownership collision rule the ticket comparison explains.

## Contents

```text
apps/ticketboard/src/wave_plan/
├── events.rs  `WavePlanEvent`: select, compare, copy a lane's TSV, toggle wave 0
├── mod.rs     the module tree
├── models/    `WavesModel` (lanes as recorded, wave 0, unplanned) and the borrowed `WavePlanView`
├── services/  the read-only lock reader with its missing and refused states, and the collision rule
└── ui/        the Waves tab: refusal screens, header, lane chips, wave 0 and the unplanned bucket
```

## How it works

```text
.ai/tickets/wave.lock ──load_lock (load thread)──▶ LockState
    Loaded(WaveLock) + Corpus ──WavesModel::build──▶ lanes, wave 0, unplanned
    Missing / Refused ──▶ refusal screen on the Waves tab only
WavePlanView (borrowed) ──▶ ui::waves_ui ──▶ WavePlanEvent ──▶ application actions
```

The application's combined load calls `services::lock_file::load_lock` on its worker thread, and
`WorkspaceState` builds `WavesModel` when the lock loaded. The viewer never packs waves: lanes keep
the lock's order and membership, filters only dim chips, and the unplanned bucket is set
arithmetic over the ticket files. `cargo xtask wave repack` is the lock's only writer; after a
failed ticket command the viewer shows that command as text, and the next reload picks up the lock
it writes. The same lock reader's `collides` and `colliding_pairs` give the detail panel its
"never the same wave" verdict for two compared tickets.

## Public surface

- `services::lock_file`: `load_lock` and `LockState`, loaded by `crate::application`; `WaveLock`
  and `LockWave`, built by the application tests; `collides` and `colliding_pairs`, used by
  `crate::ticket_browser`'s comparison.
- `models::wave_projection::WavesModel` and `models::view::WavePlanView`, which the application
  builds and lends; `ui::waves_ui`, which it paints.
- `events::WavePlanEvent`, which the application converts into its actions.

## Boundaries

- Depends on: `crate::ticket_registry::models` (`Corpus`, `projection`, `palette`);
  `ticket_engine` (`StatusName`, `Ticket`, `repository::WAVE_LOCK`); `serde` and `toml`;
  `eframe::egui` in `ui/` only.
- Used by: `crate::application` (`background_loading.rs`, `workspace_state.rs`, `mod.rs`,
  `events.rs`, `feature_views.rs`) and its tests; `crate::ticket_browser::ui::detail_panel`.
- Rules:
  - the feature reads the lock and never writes it or recomputes packing
    (`lanes_render_the_lock_verbatim_never_sorted` in `models/tests/wave_projection.rs`);
  - a missing or malformed lock is a refusal on this tab and never empties the board
    (`missing_lock_is_the_did_not_run_refusal` in `services/tests/lock_file.rs`);
  - `models/` and `services/` never name egui, and no other feature imports `ui/` (the test
    `dependency_boundaries_and_external_test_placement_are_enforced` in
    `apps/ticketboard/src/tests/architecture_rules.rs`).

## Related documentation

- [Wave lock command group](/tools_v2/xtask/src/commands/wave/README.md) — the commands that
  write and check the lock.
- [Wave lock](/tools_v2/ticket-engine/src/wave_lock/README.md) — the packing and collision
  rules this viewer mirrors.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — how waves are packed and
  run.
