# Agent context guards

The `agent_context_guards` crate: the work behind the `cargo xtask ai` group. Its tool-call guard
is the PreToolUse hook an AI agent harness runs before each Read or Bash call, and its filtered
command runner prints a bounded view of a noisy command's output. Both keep an agent's context
small, and neither ever hides a failure.

## Contents

```text
tools/commands/agent_context_guards/
├── Cargo.toml  the `agent_context_guards` library package: `serde_json`, `thiserror`, `time_source`; layout tier 1
└── src/        the hook entry, the Bash and Read rules, the output filter and the errors
```

## How it works

```text
harness hook JSON on stdin ──▶ run_tool_call_guard ──▶ Read: read rules (+ session read set)
                                                   └─▶ Bash: bash command rules
                                                   ──▶ exit 0 allow / 2 deny (reason on stderr)

xtask ai run -- <command...> ──▶ run_filtered_command ──▶ sh -c ──▶ kept lines + count line
                                                                 └─▶ non-zero exit: raw last 80 lines
```

The guard reads the harness's PreToolUse JSON (`tool_name`, `session_id`, `tool_input`) and answers
by exit code alone. It fails open: unreadable stdin, JSON it cannot parse, a missing file or any
other surprise allows the call; it denies only on a rule it positively matched.

| Tool | Denied | Always allowed |
|---|---|---|
| Bash | a `rg`, `grep`, `egrep`, `fgrep` or `ag` search as the first segment of the command, with no `head`, `tail` or `wc` later in the pipeline and no `-m`, `--max-count`, `-c` or `--count`; a bare `cat`, `head`, `tail`, `sed` or `nl` of a file as the whole command | `git grep`, a grep reading from a pipe, anything capped by `head` or `tail` |
| Read | a whole-file read of a path this session already read in an earlier turn; a whole-file read of a file over 400 lines (`BIG_FILE_LINES`) or over 4 MB | any read with `offset` or `limit`; a second firing of the same call within 2 s (`SAME_CALL_WINDOW_MS`), which a hook registered twice produces |

The read set lives in `tbd-aiguard/<session_id>.reads` under the system temp folder, one
`<milliseconds>\t<path>` line per recorded read, so concurrent agents never share one. The
milliseconds are wall-clock Unix milliseconds read through `time_source`'s `Clock`: the platform
clock live, a `ManualClock` in the tests, so the re-read window is tested at exact ages.

The runner joins its arguments and runs them with `sh -c`, then prints the kept lines and a count
line `[xtask ai run] exit=<code>  lines: <in> in, <shown> shown, <filtered> filtered`. A line is
kept when it carries a verdict or failure marker (`error`, `FAILED`, `test result:`, `panicked`,
`REFUSED`, `GATE:` and the rest of `is_load_bearing`), for up to 50 non-chatter lines after such a
line, and for the last 20 lines. Compiler progress and passing `test … ok` lines are chatter. A
non-zero exit also prints the raw last 80 lines unfiltered.

## Getting started

Run from the repository root:

```bash
cargo test -p agent_context_guards   # the Bash and Read rules, the re-read window, the hook entry and the filter
echo '{"tool_name":"Bash","session_id":"s","tool_input":{"command":"rg foo src"}}' | cargo xtask ai guard
```

## Configuration

No feature and no environment variable; the read sets live under the system temp folder
(`std::env::temp_dir`), and the runner needs `sh` on the path.

## Public surface

- At the crate root and in `prelude`: `run_tool_call_guard()`, which answers one hook call from
  stdin and returns the exit code (0 allow, 2 deny), and `run_filtered_command(args)`, which
  returns the command's exit code (1 when it has none or no command is given).
- At the crate root: `Error` (a command that could not be run) and `Result`.

## Boundaries

- Depends on: `serde_json` (the hook payload), `time_source` (the wall clock of the read set),
  `thiserror`; `sh` for the runner.
- Used by: the `ai` command group of the xtask binary
  (`tools/xtask/src/commands/agent_context/dispatch.rs`), which the PreToolUse hook in
  `.claude/settings.json` runs with `ai guard` for every Read and Bash call; agents and people,
  for `ai run`.
- Rules: tier 1 of `tools/commands` (`cargo xtask verify crate-tiers`); the guard never denies on
  an unexpected condition (`missing_file_fails_open`,
  `a_payload_that_is_not_json_or_names_another_tool_is_allowed`), and the runner never turns a
  non-zero exit into a clean-looking run (`verdict_and_failure_lines_always_survive`); the deny
  messages, the count line and the read-set line format are what agents and their hooks read, and
  stay as written.

## Related documentation

- [Agent context commands](/tools/xtask/src/commands/agent_context/README.md) — the `ai` command
  line, its synopses and exit codes.
- [Command crates](/tools/commands/README.md) — the command crates and their tiers.
