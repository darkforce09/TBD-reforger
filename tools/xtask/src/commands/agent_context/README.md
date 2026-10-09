# Agent context commands

The `cargo xtask ai` group: a tool-call guard that AI agent harnesses run before each Read or Bash
call, and a runner that prints a filtered view of a noisy command's output. Both exist to keep an
agent's context small without ever hiding a failure. The work lives in the `agent_context_guards`
crate; this folder holds the command line.

## Contents

```text
tools/xtask/src/commands/agent_context/
├── cli.rs       the `AiCmd` clap enum: `guard` and `run`
├── dispatch.rs  routes `AiCmd` to the two entry points of `agent_context_guards`
└── mod.rs       the module tree
```

## How it works

`dispatch.rs` hands `guard` to `run_tool_call_guard` and `run` to `run_filtered_command` in
[`agent_context_guards`](/tools/commands/agent_context_guards/README.md) and returns their exit
code; the Bash and Read rules, the session read set and the filter's keep rules are described
in that crate's README.

## Commands

Each runs as `cargo xtask ai <command>`.

### guard

- Synopsis: `cargo xtask ai guard`, with the hook JSON on stdin.
- Does: applies the crate's Read and Bash rules to one tool call; fails open on anything it did
  not positively match.
- Exit codes: 0 allow; 2 deny, with the reason and the bounded alternative on stderr.
- Example: `echo '{"tool_name":"Bash","session_id":"s","tool_input":{"command":"rg foo src"}}' | cargo xtask ai guard`

### run

- Synopsis: `cargo xtask ai run -- <command...>`
- Does: runs the command through `sh -c` and prints its combined stdout and stderr, filtered by
  the crate's keep rules, then the count line; a non-zero exit also prints the raw last 80 lines.
- Exit codes: the command's own exit code; 1 when no command is given; 1 with `xtask: <cause>`
  when `sh` cannot be run.
- Example: `cargo xtask ai run -- 'cargo test -p agent_context_guards'`

## Boundaries

- Depends on: clap; the entry points of `agent_context_guards`
  (`tools/commands/agent_context_guards`).
- Used by: `tools/xtask/src/cli/dispatch.rs`, which routes the `ai` group; the PreToolUse hook in
  `.claude/settings.json`, which runs the built `xtask` binary with `ai guard` for every Read and
  Bash call and exits 0 when no binary is built; agents and people, for `ai run`.
- Rules: the group's arguments stay in `cli.rs`; the guard never denies on an unexpected
  condition, and `run` never turns a non-zero exit into a clean-looking run (the raw tail and the
  exit code survive), as the crate's tests pin.
