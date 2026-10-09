# Wave lock reader

The [ticketboard](/documentation/glossary/n_to_z.md#ticketboard)'s own read-only reader of
`.ai/tickets/wave.lock`, the recorded [wave](/documentation/glossary/n_to_z.md#wave) plan, and the
path pairs behind the ownership collision rule that decides which tickets may share a wave.

## Contents

```text
tools/tickets/ticketboard_model/src/wave_plan/services/
├── lock_file.rs  `load_lock`, `LockState`, the tolerant `WaveLock` mirror and the colliding path pairs
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
`cargo xtask wave repack` owns the format and the `ticket_wave_lock` model, which refuses
unknown fields, validates it; a lock without `wave_base` reads it as 0. The lock path and the
missing-lock text are `ticket_wave_lock::lock_path` and `ticket_wave_lock::missing_lock_error`.

`paths_collide` says two owned paths collide when they are equal or one contains the other on a
`/` boundary (`a/bc` does not collide with `a/b`), the inner comparison of
`ticket_wave_lock::collides`, which decides whether any pair of two `owns` lists collides;
`colliding_pairs` lists every such pair for the comparison view.

A missing or refused lock stays local to the Waves tab: the board and every other tab keep
working.

## Boundaries

- Depends on: `ticket_wave_lock` (`lock_path`, `missing_lock_error`); the crate's `Error`
  (`WaveLockUnparsable`); `serde` and `toml`.
- Used by: `crate::wave_plan::models`; `crate::application_state::background_loading`, which
  calls `load_lock` on the load thread; the desktop application:
  `tools/tickets/ticketboard_desktop/src/wave_plan/ui/`, the comparison view in
  `tools/tickets/ticketboard_desktop/src/ticket_browser/ui/detail_panel/comparison.rs` (`colliding_pairs`, beside
  `ticket_wave_lock::collides`), and `tools/tickets/ticketboard_desktop/src/application/tests/rendering.rs`.
- Rules:
  - reading only: nothing in the viewer writes the lock
    (`missing_lock_is_the_did_not_run_refusal`, `unparsable_lock_refuses_with_verbatim_error` in
    `tests/lock_file.rs`);
  - the collision rule matches the packer's cases (`collides_mirror_cases`,
    `colliding_pairs_lists_every_pair`); the tests pin the cases literally rather than comparing
    with `ticket_wave_lock`;
  - the committed lock parses (`live_lock_parses_verbatim`, which reads the repository's own
    `.ai/tickets/wave.lock`).
