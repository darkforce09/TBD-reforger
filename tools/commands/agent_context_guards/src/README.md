# Agent context guards source

The tool-call guard's hook entry and its Bash and Read rules, the filtered command runner, and
the errors the runner reports.

## Contents

```text
tools/commands/agent_context_guards/src/
├── bash_command_guard.rs  the Bash rules: an uncapped repository search, a bare file reader as the whole command
├── error.rs               `Error` and `Result`
├── lib.rs                 the crate root: module header, `mod` lines and the re-exports
├── output_filter.rs       `run_filtered_command`: runs a command and prints the kept lines, the count line and on failure the raw tail
├── prelude.rs             `run_tool_call_guard` and `run_filtered_command` for glob import
├── read_guard.rs          the Read rules and the session read set under the system temp folder
├── tests/                 unit tests for the Bash and Read rules, the re-read window, the hook entry and the filter
└── tool_call_guard.rs     `run_tool_call_guard`: reads the hook JSON on stdin and answers 0 allow or 2 deny
```

## How it works

- `tool_call_guard` parses the payload and hands a `Read` call to `read_guard` and a `Bash`
  call's `command` to `bash_command_guard`; every other tool, and every payload it cannot parse,
  is allowed.
- `read_guard` takes the wall clock as a `time_source::Clock`; the hook entry passes the platform
  clock, and the tests a `ManualClock`, so the 2 s same-call window is tested at exact ages.
- `output_filter` decides per line index whether a line survives, so the printed counts come from
  the same decision as the printed lines.

## Boundaries

- Depends on: `serde_json`, `time_source` and `thiserror`.
- Used by: the crate root's re-exports, read by the `ai` group of `xtask`.
- Rules: the tests in `tests/` call the rules directly; the Read tests write their read sets and
  probe files under the system temp folder, one session id per test.
