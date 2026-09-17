# Process Execution Engine (`verification-core/src/proc`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Low-level POSIX process execution engine that prevents the three failure modes of shell scripts:

1. **Process Group Termination**: Wraps subprocesses in their own session via `setsid(2)`. When a deadline expires, `killpg(2)` terminates the entire process group (child, grand-children, game servers, Wine processes), preventing orphaned zombie processes.
2. **Deadlock-Free Concurrent Drains**: Spawns concurrent reader threads for stdout and stderr to prevent deadlocks when subprocesses write more than 64 KiB of output into OS pipe buffers.
3. **Signal Trapping**: Intercepts `SIGSEGV`, `SIGKILL`, and `SIGTERM`, turning crash exits into explicit `NotRun::Signalled` verdicts rather than silent failures.
