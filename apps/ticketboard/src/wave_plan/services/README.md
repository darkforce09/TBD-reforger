# Wave lock reader

The [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)'s own read-only reader of
`.ai/tickets/wave.lock`, the recorded [wave](/documentation_v2/glossary/n_to_z.md#wave) plan, and its copy
of the ownership collision rule that decides which tickets may share a wave.

## Contents

```text
apps/ticketboard/src/wave_plan/services/
├── lock_file.rs  `load_lock`, `LockState`, the tolerant `WaveLock` mirror and the collision rule
├── mod.rs        the module tree
└── tests/        unit tests for parsing, missing and refused locks, collisions, the live lock
```

## How it works

`load_lock` never fails and never invents an empty plan. It returns one of three states:

| State | When | What the tab shows |
|---|---|---|
| `Loaded(WaveLock)` | the file parses | the lanes |
| `Missing { message }` | no file at the lock path | "No wave plan" and the DidNotRun text |
| `Refused { path, error }` | unreadable or unparsable | "wave.lock refused to parse", the error |

`WaveLock` and `LockWave` mirror the lock's fields (`version`, `max_concurrent`, `wave_base`,
`pack_last`, `waves`, `owns`, `depends_on`) and accept unknown fields such as `emptied`, because
`cargo xtask wave repack` owns the format and the `ticket_engine::wave_lock` model, which refuses
unknown fields, validates it; a lock without `wave_base` reads it as 0. The missing-lock text
copies `missing_lock_error` of `ticket_engine::wave_lock` word for word.

`paths_collide` says two owned paths collide when they are equal or one contains the other on a
`/` boundary (`a/bc` does not collide with `a/b`). `collides` is true when any pair of two `owns`
lists collides, as `ticket_engine::wave_lock::collides` decides, and `colliding_pairs` lists every
such pair for the detail panel's comparison.

A missing or refused lock stays local to the Waves tab: the board and every other tab keep
working.

## Boundaries

- Depends on: `ticket_engine::repository::WAVE_LOCK` for the path; `serde` and `toml`.
- Used by: `crate::wave_plan::models` and `crate::wave_plan::ui`;
  `crate::application::background_loading`, which calls `load_lock` on the load thread;
  `crate::ticket_browser::ui::detail_panel::comparison` (`collides`, `colliding_pairs`);
  `apps/ticketboard/src/application/tests/rendering.rs` and the other application tests.
- Rules:
  - reading only: nothing in the viewer writes the lock
    (`missing_lock_is_the_did_not_run_refusal`, `unparsable_lock_refuses_with_verbatim_error` in
    `tests/lock_file.rs`);
  - the collision rule matches the packer's cases (`collides_mirror_cases`,
    `colliding_pairs_lists_every_pair`); the tests pin the cases literally rather than comparing
    with `ticket_engine`;
  - the committed lock parses (`live_lock_parses_verbatim`, which reads the repository's own
    `.ai/tickets/wave.lock`).
