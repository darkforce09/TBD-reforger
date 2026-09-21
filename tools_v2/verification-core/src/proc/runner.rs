//! Spawning a child in its own process group, and reaping it within a deadline.
//!
//! The three corrections the module documentation describes live here: the group isolation that
//! makes a timeout able to kill a whole tree, the `killpg` that does it, and the signal check
//! that keeps a killed child out of the exit-code path.

use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::verdict::{Kind, NotRun, Verdict};

use super::stream::{SeparateDrains, start_merged_drain};
use super::{Merged, Output, Run};

/// How often a pending child is polled while waiting on a deadline. Short enough that a timeout
/// is punctual, long enough that a 20-minute gate does not spin a core.
const POLL: Duration = Duration::from_millis(20);

impl Run {
    /// Run to completion, capturing both streams separately. The raw exit code is preserved.
    pub fn output(self) -> Result<Output, NotRun> {
        let label = self.display();
        let started = Instant::now();

        let mut cmd = self.command(Stdio::piped(), Stdio::piped());
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        // `setsid` made the child a group leader, so its pgid equals its pid.
        let pgid = child.id() as i32;
        feed_stdin(&mut child, self.stdin.as_deref());

        // Drain both pipes for the child's whole life. See the module documentation §3.
        let drains = SeparateDrains::start(&mut child);

        let status = match wait_within(&mut child, pgid, self.timeout, &label) {
            Ok(status) => status,
            // A timed-out group is dead, so both pipes are at EOF and the drains end at once.
            // Any other cause leaves a live child whose pipes nobody may block on.
            Err(cause) => {
                if matches!(cause, NotRun::Timeout { .. }) {
                    drains.join();
                }
                return Err(cause);
            }
        };
        let (stdout, stderr) = drains.join();

        // A signal is NOT an exit code. See the module documentation §1.
        if let Some(signal) = status.signal() {
            return Err(NotRun::Signalled {
                tool: label,
                signal,
            });
        }

        Ok(Output {
            code: status.code().unwrap_or(-1),
            stdout,
            stderr,
            duration: started.elapsed(),
        })
    }

    /// Run with **stdout and stderr on ONE pipe** — genuinely `2>&1`.
    ///
    /// ── WHY THIS IS NOT `output()` WITH THE TWO STRINGS CONCATENATED ─────────────────────────
    ///
    /// [`Run::output`] drains the two streams into two separate `String`s, which discards the
    /// interleaving. Joining them afterwards invents an order that the child never produced.
    ///
    /// MEASURED 2026-08-12 on `verify editor-orbat-coherency`: that gate runs 25 `cargo test`
    /// invocations and its whole 803-line output is a scraped contract.
    /// Cargo writes `Running unittests …` to **stderr** and libtest writes `running N tests` to
    /// **stdout**. Every package in that gate happens to have exactly one test target today, so
    /// stderr-then-stdout coincidentally matches — and would stop matching the day any package
    /// gains a second one, silently reordering a gate's output and breaking a diff contract that
    /// nobody would think to re-examine.
    ///
    /// A single shared pipe is what a shell does for `2>&1`, so the ordering is the child's own
    /// and cannot drift. `std::io::pipe` (stable 1.87; the toolchain pin is 1.95) makes it cheap
    /// and adds no dependency.
    ///
    /// Reach for this whenever the captured text is compared against a recorded transcript. Use
    /// [`Run::output`] when the two streams are handled separately — a gate that reports stderr
    /// only on failure, say.
    pub fn merged_output(self) -> Result<Merged, NotRun> {
        let label = self.display();
        let started = Instant::now();

        let io_err = |e: std::io::Error| NotRun::ToolError {
            tool: label.clone(),
            status: -1,
            stderr: e.to_string(),
        };
        let (reader, writer) = std::io::pipe().map_err(io_err)?;
        let writer2 = writer.try_clone().map_err(io_err)?;

        let mut cmd = self.command(Stdio::from(writer), Stdio::from(writer2));
        let mut child = spawn(&mut cmd, &self.program, &label)?;
        let pgid = child.id() as i32;
        feed_stdin(&mut child, self.stdin.as_deref());

        // Drop OUR copies of the write end. Without this the read below never sees EOF, because
        // the pipe stays open on handles this process still holds. `cmd` owns both.
        drop(cmd);

        // One reader thread, so a timeout can still fire while the pipe fills.
        let reader_thread = start_merged_drain(reader);

        let status = match wait_within(&mut child, pgid, self.timeout, &label) {
            Ok(status) => status,
            Err(cause) => {
                if matches!(cause, NotRun::Timeout { .. }) {
                    let _ = reader_thread.join();
                }
                return Err(cause);
            }
        };
        let text = reader_thread.join().unwrap_or_default();

        // A signal is NOT an exit code. See the module documentation §1.
        if let Some(signal) = status.signal() {
            return Err(NotRun::Signalled {
                tool: label,
                signal,
            });
        }
        Ok(Merged {
            code: status.code().unwrap_or(-1),
            text,
            duration: started.elapsed(),
        })
    }

    /// Run and return only the raw exit code.
    pub fn status(self) -> Result<i32, NotRun> {
        Ok(self.output()?.code)
    }

    /// A [`Verdict`] that holds when the command exits 0.
    pub fn expect_ok(self, msg: &str) -> Verdict {
        self.expect_code(msg, 0)
    }

    /// A [`Verdict`] that holds only on an exact exit code.
    ///
    /// `cargo xtask mod compile --selftest` passes **only** on 1 — it feeds the compiler a
    /// deliberately broken input, so an exit 0 there means the check is hollow and "success" is
    /// the failure. That contract needs an exact comparison, not a truthiness test.
    pub fn expect_code(self, msg: &str, want: i32) -> Verdict {
        let label = self.display();
        match self.output() {
            Err(cause) => Verdict::did_not_run(msg, Kind::Pin, cause),
            Ok(out) if out.code == want => Verdict::Held,
            Ok(out) => Verdict::Failed(crate::verdict::Finding {
                headline: format!("{msg} — `{label}` exited {} (want {want})", out.code),
                detail: out.stderr.lines().take(10).map(str::to_string).collect(),
            }),
        }
    }

    /// The configured [`Command`], with the child placed in its own process group.
    ///
    /// Stdin is a pipe only when this run carries a body to write; otherwise it is `/dev/null`,
    /// so a child that reads stdin sees EOF rather than inheriting this process's terminal.
    fn command(&self, stdout: Stdio, stderr: Stdio) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args)
            .stdout(stdout)
            .stderr(stderr)
            .stdin(if self.stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            });
        if let Some(ref d) = self.cwd {
            cmd.current_dir(d);
        }
        for (k, v) in &self.envs {
            cmd.env(k, v);
        }
        for k in &self.env_removes {
            cmd.env_remove(k);
        }

        // Own process group, so a timeout can take the whole tree. See the module docs §2.
        //
        // SAFETY: `pre_exec` runs between fork and exec, where only async-signal-safe calls are
        // permitted. `setsid`/`setpgid` are both on that list and neither allocates.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setsid() == -1 {
                    // Already a process-group leader (possible when the parent was itself
                    // spawned by a shell job-control setup); isolating the group still suffices.
                    if libc::setpgid(0, 0) == -1 {
                        return Err(std::io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        cmd
    }
}

/// Start the child, telling "the program is not installed" apart from every other spawn failure.
fn spawn(cmd: &mut Command, program: &str, label: &str) -> Result<Child, NotRun> {
    match cmd.spawn() {
        Ok(child) => Ok(child),
        // The honest form of exit 127. Distinguished from every other spawn failure because
        // "you have not installed it" and "it is there and broke" are different problems.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(NotRun::ToolAbsent(program.to_string()))
        }
        Err(e) => Err(NotRun::ToolError {
            tool: label.to_string(),
            status: -1,
            stderr: format!("spawn failed: {e}"),
        }),
    }
}

/// Write `body` to the child's stdin and close it.
///
/// A closed stdin (the child exited early) is the child's business, not an error here.
fn feed_stdin(child: &mut Child, body: Option<&str>) {
    if let Some(body) = body
        && let Some(mut sink) = child.stdin.take()
    {
        use std::io::Write;
        let _ = sink.write_all(body.as_bytes());
    }
}

/// Reap `child`, enforcing `limit` by killing its whole process group.
///
/// On a deadline the group is killed and the child reaped before returning, so a caller's pipe
/// drains see EOF immediately and join without blocking.
fn wait_within(
    child: &mut Child,
    pgid: i32,
    limit: Option<Duration>,
    label: &str,
) -> Result<ExitStatus, NotRun> {
    let Some(limit) = limit else {
        return child.wait().map_err(|e| NotRun::ToolError {
            tool: label.to_string(),
            status: -1,
            stderr: format!("wait failed: {e}"),
        });
    };

    let deadline = Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    // Kill the GROUP, not just the child.
                    //
                    // SAFETY: `killpg` on a pgid we created. A failure here means the group is
                    // already gone, which is the outcome we wanted anyway.
                    unsafe {
                        libc::killpg(pgid, libc::SIGKILL);
                    }
                    let _ = child.wait();
                    return Err(NotRun::Timeout {
                        tool: label.to_string(),
                        secs: limit.as_secs(),
                    });
                }
                std::thread::sleep(POLL);
            }
            Err(e) => {
                return Err(NotRun::ToolError {
                    tool: label.to_string(),
                    status: -1,
                    stderr: format!("try_wait failed: {e}"),
                });
            }
        }
    }
}
