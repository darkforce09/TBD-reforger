# Process runner source

Running an external program without losing the reason it stopped: a signal death, a timeout, a
missing program or a spawn failure each come back as a `NotRun` cause, never as an exit code a
check could read as a result. Beside the runner sit the two ways a command leaves this machine's
container: the bridge to the host and the ssh transport to a remote host.

## Contents

```text
tools/foundation/process_runner/src/
├── error.rs                   `Error` and `Result`: a child that was absent, signalled or timed out
├── host_execution.rs          `Host` and `in_container`: host binaries through the container bridge, or directly on the host
├── lib.rs                     the crate root: module header, `mod` lines and the re-exports
├── lookup.rs                  `which`, `retry` and `wait_for`: `PATH` lookup, retries, condition polling
├── prelude.rs                 `Run`, its results, the lookups, `PathGuard`, `Host`, `SshBase` and `ssh_argv` for glob import
├── run.rs                     `Run`, the command builder, its stdin choices, and its results `Output` and `Merged`
├── run_modes/                 the runs that capture no text: terminal, byte pipes, files, detached, line stream, process replacement
├── run_modes.rs               the run modes' module root: `mod` lines and the `BinaryOutput` and `StreamingChild` re-exports
├── runner.rs                  the captures; spawn, stdin writer, new session, group or child kill, signal mapping for every mode
├── search_path.rs             `PathGuard`: one more folder first on `PATH` for a scope, the system tools kept reachable
├── secure_shell_transport.rs  `SshBase` and `ssh_argv`: plain `ssh`, `sshpass -e ssh` or `ssh -i`, and the argv of one remote command
├── stream.rs                  drains stdout and stderr on their own threads, whole (bytes or lossy text) or line by line
└── tests/                     unit tests for the runner, the host bridge, the ssh transport and the `PATH` construction
```

## How it works

```text
Run::new(program).args(..).cwd(..).env(..).timeout(..).stdin(..)
        │
        ├── output()         two pipes, two drain threads  ──► Output { code, stdout, stderr, duration }
        ├── merged_output()  one shared pipe, one thread    ──► Merged { code, text, duration }
        ├── status()         output(), exit code only       ──► i32
        ├── expect_ok() / expect_code(msg, want)            ──► Verdict (Held, Failed, DidNotRun)
        └── run_modes/: terminal(), binary_output(), output_to_files(..), spawn_detached(..),
                        stream_lines(), replace_process()
```

- `runner.rs` builds the `Command` with a `pre_exec` hook that calls `setsid` (falling back to
  `setpgid(0, 0)`), so the child leads its own process group. When the run carries a `timeout`,
  the wait polls every 20 ms and, at the deadline, kills the whole group and returns
  `NotRun::Timeout`, so forked grandchildren (a game server, `cargo`, `ssh`) die with it.
- A child killed by a signal returns `NotRun::Signalled`; it never becomes a synthesised `128+n`
  exit code. A program missing from `PATH` at spawn is `NotRun::ToolAbsent`, any other spawn
  failure `NotRun::ToolError`.
- Exit codes pass through raw: `status` returns the real code, and `expect_code` holds only on an
  exact match, for a command whose success is a non-zero code.
- Stdin is `/dev/null` unless the run chose a body (`stdin`, `stdin_bytes`) or a file
  (`stdin_file`), so a captured child never reads this process's terminal; a body is written on
  its own thread while the pipes drain. `run_modes/README.md` describes the modes that do not
  capture text, the terminal one among them, which inherits stdin and stays in this process's
  process group.
- `stream.rs` starts one reader per pipe before the parent waits, so a child that fills one pipe
  buffer cannot deadlock; `merged_output` gives the child one pipe for both streams, so the text
  keeps the order the child wrote it, as a shell's `2>&1` does.
- `lookup.rs`: `which` answers `NotRun::ToolAbsent` for a program on no `PATH` entry; `retry`
  never retries a `ToolAbsent`; `wait_for` returns `NotRun::Timeout` when the condition never
  held.
- `host_execution.rs`: Steam, the Workbench and `ArmaReforgerServer` are linked against the host's
  glibc, so the development container cannot run them. `Host::detect` asks whether this process
  is in a container (`/run/.containerenv` or `/.dockerenv`) and which bridge is on `PATH`
  (`distrobox-host-exec`, then `host-spawn`); a command goes through the bridge only when both
  hold, and runs directly otherwise. With no bridge in a container, `run` prints a refusal and
  returns 127 and `capture` returns nothing. Cargo is never routed through the bridge.
- `secure_shell_transport.rs`: `SshBase::from_settings` takes a password first, else an identity
  file, else plain `ssh`, and `ssh_argv` appends the destination and the remote words. The
  password reaches `sshpass` only through the spawned process's `SSHPASS` variable
  (`SSH_PASSWORD_VARIABLE`), and `SshBase`'s `Debug` redacts it.

## Boundaries

- Depends on: `verification_core` (`NotRun`, `Verdict`, `Kind`, `Finding`); `libc` for `setsid`,
  `setpgid` and `killpg`; `std::io::pipe` for the shared pipe; `distrobox-host-exec` or
  `host-spawn` inside a container.
- Used by: the xtask command groups that run external programs (`build`, `ci`, `db`, `debug`,
  `deploy`, `fetch`, `map`, `mcp`, `platform`, `reproduction` under
  `tools/xtask/src/commands/`), the command crates under `tools/commands/` (`ci_task_catalog`,
  `database_operations`, `enfusion_mcp` and `mod_operations` through the run modes too, the last
  streaming the lines of its dedicated server and world boot), `host_execution.rs` here, the check
  crates under `tools/checks/`, and `tools/foundation/ticket_manager_client` (the `ttm` calls).
- Rules: a signal death is `Signalled` and never an exit code, a timeout kills the process group,
  a full pipe never blocks the child, and `merged_output` keeps the child's interleaving; the
  tests in `tests/run_tests.rs` hold each of these, and `run_modes/tests/` holds the modes'. The bridge is never used outside a container
  (`bridge_is_never_used_on_the_metal` in `tests/host_execution_tests.rs`), and no argv or
  `Debug` rendering carries the ssh password
  (`secure_shell_transport_keeps_the_password_out_of_argv_and_debug`).
