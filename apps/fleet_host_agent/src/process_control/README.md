# Process control

Starts, stops and restarts the game server's systemd user unit without a shell, and decides each
action's verdict from the unit state systemd reports afterwards rather than from the exit status of
`systemctl`.

## Contents

```text
apps/fleet_host_agent/src/process_control/
├── mod.rs                 the module tree; `ProcessControl`, `ProcessAction`, `SystemdUnitName` and the settings
├── systemctl_runner.rs    runs systemctl with a fixed argument vector, a cleared environment and a timeout
├── tests/                 unit tests for unit names, actions, state values and verdicts
└── unit_state_verdict.rs  `ProcessActionReport` and its verdict, and how the verb invocation ended
```

## How it works

`ProcessControl::perform` runs `systemctl --user start|stop|restart <unit>`, waits out the dwell
after start and restart, and reads the state back with
`systemctl --user show --property=LoadState --value <unit>` and
`systemctl --user show --property=ActiveState --value <unit>`. The verb's exit status is recorded
but never decides the verdict: an Arma Reforger server that fails to start exits with status 0 a
few seconds after systemd reported the start as done. `ProcessActionReport::verdict` succeeds only
when LoadState is `loaded` and ActiveState is the one the action intends, `active` after start and
restart and `inactive` after stop; `systemctl show` reports a unit that does not exist as
inactive, which is why LoadState is read too. A failure reason names the state observed, for
example `restart left tbd-reforger.service failed instead of active after waiting 8s; systemctl
--user restart exited with status 0`.

Every invocation is a fixed argument vector whose only variable element is the unit name.
`SystemdUnitName::parse` accepts 1 to 255 bytes of ASCII letters, digits and `:_.@\-`, ending in
`.service`, never starting with `-` or `.`, so the name cannot read as an option.
`run_systemctl` clears the environment except `XDG_RUNTIME_DIR` and `DBUS_SESSION_BUS_ADDRESS`,
which locate the user's systemd manager, gives the program no standard input, and kills it at its
timeout: 100 s for a verb (longer than systemd's 90 s stop timeout) and 5 s for a state read. With
the dwell of at most 30 s, start and restart stay inside the ledger's 180 s execution window and
stop inside its 120 s window. A state value must be one lowercase word of at most 32 bytes; the
verb's error output is kept as one line of at most 160 bytes.

## Boundaries

- Depends on: `crate::action_verdict`; the `tokio` (process, time), `serde_json`, `thiserror` and
  `tracing` crates; a `systemctl` program reaching the calling user's systemd manager.
- Used by: `crate::agent_configuration`, which builds `ProcessControlSettings` and validates the
  unit name; `crate::command_execution`, which performs `start`, `stop`, `restart` and the restart
  of `restart_with_mission`; `apps/fleet_host_agent/src/main.rs`; and the integration tests
  `apps/fleet_host_agent/tests/process_control.rs` and
  `apps/fleet_host_agent/tests/host_agent_ledger.rs`, which point `systemctl_program` at a
  stand-in script.
- Rules: no shell and no argument from outside the validated unit name (the
  `process_control_runs_systemctl_with_a_fixed_argument_vector` test); a unit that is not loaded
  is never a success, and the verb's exit status never decides the verdict
  (`tests/unit_state_verdict.rs`).
