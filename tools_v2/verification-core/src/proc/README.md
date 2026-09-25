# Child processes

Running an external program without losing the reason it stopped: a signal death, a timeout, a
missing program or a spawn failure each come back as a `NotRun` cause, never as an exit code a
check could read as a result.

## Contents

```text
tools_v2/verification-core/src/proc/
├── lookup.rs  `which`, `retry` and `wait_for`: `PATH` lookup, retries, condition polling
├── mod.rs     the module tree; `Run`, the command builder, and its results `Output` and `Merged`
├── runner.rs  spawns in a new process group, enforces the deadline with a group kill, maps signals
└── stream.rs  drains stdout and stderr on their own threads, decoding lossily
```

## How it works

```text
Run::new(program).args(..).cwd(..).env(..).timeout(..).stdin(..)
        │
        ├── output()         two pipes, two drain threads  ──► Output { code, stdout, stderr, duration }
        ├── merged_output()  one shared pipe, one thread    ──► Merged { code, text, duration }
        ├── status()         output(), exit code only       ──► i32
        └── expect_ok() / expect_code(msg, want)            ──► Verdict (Held, Failed, DidNotRun)
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
- Stdin is `/dev/null` unless the run carries a body, so a child never reads this process's
  terminal.
- `stream.rs` starts one reader per pipe before the parent waits, so a child that fills one pipe
  buffer cannot deadlock; `merged_output` gives the child one pipe for both streams, so the text
  keeps the order the child wrote it, as a shell's `2>&1` does.
- `lookup.rs`: `which` answers `NotRun::ToolAbsent` for a program on no `PATH` entry; `retry`
  never retries a `ToolAbsent`; `wait_for` returns `NotRun::Timeout` when the condition never
  held.

## Boundaries

- Depends on: `crate::verdict` (`NotRun`, `Verdict`, `Kind`, `Finding`); `libc` for `setsid`,
  `setpgid` and `killpg`; `std::io::pipe` for the shared pipe.
- Used by: the xtask command groups that run external programs (`build`, `ci`, `db`, `debug`,
  `deploy`, `fetch`, `map`, `mcp`, `mod_ops`, `platform`, `reproduction`, `setup` under
  `tools_v2/xtask/src/commands/`), `tools_v2/xtask/src/core/host_execution.rs` and
  `tools_v2/xtask/src/core/cargo_target_directory.rs`, and the `api_readiness`, `architecture`,
  `documentation`, `licensing` and `mod_scripts` verifications under
  `tools_v2/xtask/src/verifications/`.
- Rules: a signal death is `Signalled` and never an exit code, a timeout kills the process group,
  a full pipe never blocks the child, and `merged_output` keeps the child's interleaving; the
  tests in `tools_v2/verification-core/src/tests/proc_tests.rs` hold each of these.
