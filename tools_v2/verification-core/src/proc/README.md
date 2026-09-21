# Child processes

`mod.rs` holds the vocabulary — `Run` builds a command, `Output` and `Merged` carry what it
produced. `runner.rs` spawns it in its own process group, enforces the deadline with a group kill,
and turns a signal death into `NotRun::Signalled` instead of an exit code. `stream.rs` drains the
pipes on dedicated threads so a full buffer cannot deadlock a captured child. `lookup.rs` resolves
programs on `PATH`, retries with a backoff, and waits on a condition. Tests live in
`../tests/proc_tests.rs`.
