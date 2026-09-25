# Agent context commands

The `cargo xtask ai` group: a tool-call guard that AI agent harnesses run before each Read or Bash
call, and a runner that prints a filtered view of a noisy command's output. Both exist to keep an
agent's context small without ever hiding a failure.

## Contents

```text
tools_v2/xtask/src/commands/agent_context/
├── cli.rs       the `AiCmd` clap enum: `guard` and `run`
├── dispatch.rs  routes `AiCmd` to the two entry points in `guards.rs`
├── guards.rs    `cmd_guard`, the Read and Bash guard, and `cmd_run`, the output filter
├── mod.rs       the module tree
└── tests/       unit tests for the Bash and Read rules, the re-read window and the filter
```

## How it works

`guard` reads the harness's PreToolUse JSON on stdin (`tool_name`, `session_id`, `tool_input`)
and answers by exit code alone. It fails open: unreadable stdin, JSON it cannot parse, a missing
file or any other surprise allows the call; it denies only on a rule it positively matched.

| Tool | Denied | Always allowed |
|---|---|---|
| Bash | a `rg`, `grep`, `egrep`, `fgrep` or `ag` search as the first segment of the command, with no `head`, `tail` or `wc` later in the pipeline and no `-m`, `--max-count`, `-c` or `--count`; a bare `cat`, `head`, `tail`, `sed` or `nl` of a file as the whole command | `git grep`, a grep reading from a pipe, anything capped by `head` or `tail` |
| Read | a whole-file read of a path this session already read in an earlier turn; a whole-file read of a file over 400 lines (`BIG_FILE_LINES`) or over 4 MB | any read with `offset` or `limit`; a second firing of the same call within 2 s (`SAME_CALL_WINDOW_MS`), which a hook registered twice produces |

The read set lives in `tbd-aiguard/<session_id>.reads` under the system temp folder, one
`<milliseconds>\t<path>` line per recorded read, so concurrent agents never share one.

`run` joins its arguments and runs them with `sh -c`, then prints the kept lines and a count line
`[xtask ai run] exit=<code>  lines: <in> in, <shown> shown, <filtered> filtered`. A line is kept
when it carries a verdict or failure marker (`error`, `FAILED`, `test result:`, `panicked`,
`REFUSED`, `GATE:` and the rest of `is_load_bearing`), for up to 50 non-chatter lines after such a
line, and for the last 20 lines. Compiler progress and passing `test … ok` lines are chatter. A
non-zero exit also prints the raw last 80 lines unfiltered.

## Commands

Each runs as `cargo xtask ai <command>`.

### guard

- Synopsis: `cargo xtask ai guard`, with the hook JSON on stdin.
- Does: applies the Read and Bash rules above to one tool call.
- Exit codes: 0 allow; 2 deny, with the reason and the bounded alternative on stderr.
- Example: `echo '{"tool_name":"Bash","session_id":"s","tool_input":{"command":"rg foo src"}}' | cargo xtask ai guard`

### run

- Synopsis: `cargo xtask ai run -- <command...>`
- Does: runs the command through `sh -c` and prints its combined stdout and stderr, filtered as
  above.
- Exit codes: the command's own exit code; 1 when no command is given.
- Example: `cargo xtask ai run -- 'cargo test -p xtask agent_context'`

## Boundaries

- Depends on: `serde_json` for the hook payload; `sh` for `run`.
- Used by: the PreToolUse hook in `.claude/settings.json`, which runs the built `xtask` binary
  with `ai guard` for every Read and Bash call and exits 0 when no binary is built; agents and
  people, for `ai run`.
- Rules: the guard never denies on an unexpected condition, and `run` never turns a non-zero exit
  into a clean-looking run (the raw tail and the exit code survive); the tests in `tests/guards/`
  pin both, among them `missing_file_fails_open` and `verdict_and_failure_lines_always_survive`.
