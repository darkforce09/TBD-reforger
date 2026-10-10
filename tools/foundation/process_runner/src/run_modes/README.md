# Process runner run modes

The runs of `process_runner` that do not capture a child's output as text: a child on this
process's terminal, a child fed and read as bytes, a child writing into files, a child that
outlives this process, a child whose lines a caller reads while it runs, and the replacement of
this process by the child. Each is a method of `Run`, built on the spawn, stdin, isolation, kill
and reaping helpers of `../runner.rs`.

## Contents

```text
tools/foundation/process_runner/src/run_modes/
├── binary_pipes.rs         `Run::binary_output` and `BinaryOutput`: bytes on stdin, stdout captured as bytes
├── detached.rs             `Run::spawn_detached` and `Run::spawn_detached_to_files`: a child in its own session, its pid returned at once
├── file_output.rs          `Run::output_to_files`: stdout and stderr written straight into two files the caller opened
├── line_stream.rs          `Run::stream_lines` and `StreamingChild`: each output line on a channel, the child polled, awaited or killed
├── process_replacement.rs  `Run::replace_process`: this process replaced by the child, as a shell's `exec`
├── terminal.rs             `Run::terminal`: stdin, stdout and stderr inherited, the raw exit code returned
└── tests/                  unit tests for each mode
```

## How it works

```text
Run::new(program).args(..).cwd(..).env(..).timeout(..).stdin*(..)
        │
        ├── terminal()                      inherited stdio, caller's group  ──► i32
        ├── binary_output()                 stdin bytes, stdout bytes         ──► BinaryOutput { code, stdout, stderr, duration }
        ├── output_to_files(out, err)       both streams into files           ──► i32
        ├── spawn_detached[_to_files](..)   new session, reaper thread        ──► pid
        ├── stream_lines()                  line channel, own group           ──► (StreamingChild, Receiver<String>)
        └── replace_process()               exec in place                     ──► NotRun (only on failure)
```

- A signal is `NotRun::Signalled` in every mode that reaps its child, never an exit code; an
  absent program is `NotRun::ToolAbsent` at spawn.
- The stdin choice is the run's: `stdin` (text), `stdin_bytes`, `stdin_file` or `stdin_null`. An
  unset stdin is `/dev/null`, except in `terminal` and `replace_process`, where it is this
  process's own. A body is written on its own thread while the pipes drain, so a child that
  answers while it reads cannot deadlock.
- `terminal` leaves the child in this process's process group and session: the terminal's Ctrl-C
  and job control reach it as they reach a shell's foreground job, and its deadline kills the
  child alone, since a group kill would take this process with it.
- `binary_output`, `output_to_files` and `stream_lines` give the child a new session, so a
  deadline or `StreamingChild::kill` reaches every grandchild.
- `spawn_detached` gives the child a new session, so a group kill aimed at this process (a CI
  step teardown, the operator's Ctrl-C) does not reach it, and a reaper thread waits on it, so a
  child that exits first leaves no zombie; a timeout does not apply.
- `replace_process` refuses a stdin body and a timeout, which nothing would remain to honour.

## Boundaries

- Depends on: `../runner.rs` (`spawn`, `feed_stdin`, `isolate`, `kill`, `wait_within`,
  `exit_code`), `../stream.rs` (the byte and line drains), `verification_core` (`NotRun`).
- Used by: the xtask command crates under `tools/commands/` (`ci_task_catalog`,
  `database_operations`, `enfusion_mcp` and `mod_operations`, whose
  dedicated server launcher and world boot read the server's lines through `stream_lines`) and
  `tools/checks/mod_script_checks`.
- Rules: each mode's signal, timeout, stdin and session behaviour is pinned in `tests/`;
  `terminal_child_stays_in_the_callers_process_group` and `spawn_detached_leads_a_new_session`
  pin the two session choices.
