# Playtest server boot and shutdown

The live-process half of `cargo xtask mod playtest`: it launches the dedicated server on the host,
waits for its verdict, prints the join details, follows the log and stops the server's process
group on Ctrl-C.

## Contents

```text
tools/commands/mod_operations/src/playtest_server/boot/
├── on_stop_signal.rs  the SIGINT and SIGTERM flag, the launcher script, the wait loop, banner and tail
└── run_deadline.rs    the `--timeout` deadline: parsing and the expire-or-nap decision of both loops
```

## How it works

`tools/commands/mod_operations/src/playtest_server/boot.rs` declares `on_stop_signal.rs` with a
`#[path]` attribute and re-exports `boot_and_wait`.

```text
boot_and_wait
  ├─ remove <run dir>/profile/logs; install the SIGINT/SIGTERM handler (one atomic store)
  ├─ spawn on the host: setsid sh -c "<launcher>"; its output goes to <run dir>/server.out
  │    ArmaReforgerServer -addonsDir <run dir>/addons -config <run dir>/server.json
  │                       -profile <run dir>/profile -maxFPS 60 -logStats 30000 -nothrow
  ├─ wait_for_verdict: poll server.out every 500 ms, up to 300 s
  │    registered · config fatal · process group confirmed dead · never registered · Ctrl-C
  │    · --timeout expired first (a failure, exit 1)
  ├─ a failure prints the phase reached and the engine errors, and returns 1
  ├─ assert_local_addon_won, or kill the group and return 1
  ├─ print_banner: registered address and Direct Join Code from this boot's log
  ├─ after_ready: the platform deployment's confirmation
  └─ tail_until_the_server_stops: follow the log until the group is gone, Ctrl-C, or --timeout
       Ctrl-C or --timeout: before_stop (the telemetry drain, server still up), then kill_run
```

`--timeout=<duration>` (seconds, or a `s`, `m`, `h` or `d` suffix; 0 is none) is a deadline kept
in this process and counted from the launch, not a `timeout(1)` in front of the engine: when it
expires after the server was ready, the `before_stop` hook still sees a live server, and the stop
then goes through `kill_run` like a natural exit (exit 0 once the group is confirmed gone). A
malformed value exits 2 before anything launches.

Liveness is always the process group recorded in `server.pid`, probed through the host bridge,
never the local launcher, which returns as soon as the server detaches. A probe that cannot reach
the host reads as unknown and never as dead.

## Boundaries

- Depends on: `super::lifecycle` (`probe_group`, `read_pgid`, `kill_run`, the stray-server
  warning), `super::logread` (log reads, boot phase, the local-addon gate), `super::host::Host`
  (host spawns), and the `libc` crate for the signal handler.
- Used by: `tools/commands/mod_operations/src/playtest_server/usage_fail.rs`, which calls
  `boot::boot_and_wait` after staging.
- Rules: the launcher argv is fixed (`the_launcher_argv_is_the_one_the_engine_needs`); the
  deadline expires at its limit and never naps past it
  (`the_deadline_expires_at_its_limit_and_never_naps_past_it`,
  `timeout_values_read_the_way_timeout_1_reads_them`),
  and the join details come from the engine's own lines
  (`the_join_details_are_scraped_out_of_the_engines_lines`), all in
  `tools/commands/mod_operations/src/playtest_server/tests/boot/tests.rs`.

## Related documentation

- [Playtest server runbook](/documentation/runbooks/two_client_playtest/playtest_server.md) —
  the banner, the registration hang and the Ctrl-C stop as an operator meets them.
