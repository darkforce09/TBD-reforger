# Playtest server lifecycle guards

The kill discipline, liveness probe and run lock behind `cargo xtask mod playtest`, plus its
`--selftest`, which proves the kill path cannot report a stop it did not achieve.

## Contents

```text
tools_v2/xtask/src/commands/mod_ops/playtest_server/lifecycle/
├── probe_group.rs  the host-side group probe, kill_run, the run lock, the live-server refusal
└── selftest.rs     `mod playtest --selftest`: disposable host process groups killed through a broken bridge
```

## How it works

`tools_v2/xtask/src/commands/mod_ops/playtest_server/lifecycle.rs` holds the shared types
(`RunPaths`, `Probe`, `Volume`, `LockGuard`, `LiveVerdict`) and re-exports both files' functions.

- `probe_group` runs a probe on the host that prints its own `TBDPROBE=alive|zombie|dead`
  sentinel. Any other answer, including a bridge that never started, is `Probe::Unknown`, and
  `Unknown` never counts as dead.
- `kill_run` signals the process group in `<run dir>/server.pid`, never a process name: TERM,
  10 s of probes, then KILL and 5 s more. It deletes the pidfile only once the group is confirmed
  gone. Otherwise it returns the group id, and `print_stray_warning` names the group, the ports
  and the command to stop it by hand.
- `claim_lock` takes `<run dir>/.run.lock` with an atomic `mkdir` and records the owner pid. A
  live owner refuses with exit 1; a dead one is taken over. The `LockGuard` removes the folder on
  drop.
- `check_no_live_server` and `assert_no_live_server` refuse to stage over a server the pidfile
  still names, and refuse when the probe cannot tell.
- `selftest` spawns `sleep` groups on the host and kills them through a deliberately broken
  bridge, checking that `kill_run` refuses to claim success and names the stray group. It boots
  no game server and exits 3 without a host bridge.

## Boundaries

- Depends on: `super::host::Host` (the host bridge from `crate::core::host_execution`) and the
  playtest `Opts`.
- Used by: `tools_v2/xtask/src/commands/mod_ops/playtest_server/usage_fail.rs` (lock, live-server
  refusal, `--selftest`) and `tools_v2/xtask/src/commands/mod_ops/playtest_server/boot/on_stop_signal.rs`
  (probe, kill, stray warning).
- Rules: an unknown probe is never read as death (`unknown_is_not_dead_and_has_no_bool_shortcut`,
  `a_broken_bridge_probes_unknown_not_dead`); `kill_run` keeps the pidfile when it cannot confirm
  (`kill_run_keeps_the_pidfile_when_it_cannot_confirm`); the lock is exclusive and released on drop
  (`the_lock_is_exclusive_and_released_on_drop`); all in
  `tools_v2/xtask/src/commands/mod_ops/playtest_server/tests/lifecycle/tests.rs`.
