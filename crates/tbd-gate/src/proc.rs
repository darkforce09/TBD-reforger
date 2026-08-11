//! Running programs without losing the reason they stopped.
//!
//! The 59 shell scripts spend most of their lines doing five things: run a command, look at its
//! exit status, capture its output, give up after a while, and retry. Bash gets three of those
//! subtly wrong in ways that have already cost this repo waves, and the corrections are the
//! entire content of this module.
//!
//! ── 1. `cmd || rc=$?` CANNOT SEE A SIGNAL ────────────────────────────────────────────────────
//!
//! A child killed by SIGKILL has no exit code — the shell synthesises `128+n`, and the `case`
//! arms downstream read 137 as an ordinary numeric failure. Under eight parallel worktrees the
//! OOM killer is a routine visitor, so "the kernel shot the gate" is regularly reported as "the
//! gate found a problem". Here that is [`NotRun::Signalled`], never a `Failed`.
//!
//! ── 2. `timeout(1)` KILLS THE CHILD, NOT THE TREE ────────────────────────────────────────────
//!
//! The scripts' `timeout` usage kills only the direct child. `ArmaReforgerServer`, `cargo`, `ssh`
//! and `podman` all fork, so the grandchildren survive, keep the log file and the port open, and
//! the next run fails for reasons that have nothing to do with the code under test. Every child
//! spawned here is put in **its own process group** via `setsid`, and a timeout kills the group.
//!
//! ── 3. A FULL PIPE DEADLOCKS A CAPTURED CHILD ────────────────────────────────────────────────
//!
//! Capturing stdout while the child also writes stderr deadlocks once either pipe buffer fills —
//! about 64 KiB, which `world-boot.sh` clears comfortably. Both streams are drained by dedicated
//! threads for the child's whole life, so neither can wedge.
//!
//! Statuses are otherwise passed through **raw**: [`Run::status`] hands back the real code and
//! never collapses it, because `compile.sh --selftest` passes only on exactly 1 and `map-export`
//! uses 2 to mean "run the Workbench step first".

use std::ffi::OsStr;
use std::io::Read;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::verdict::{Kind, NotRun, Verdict};

/// How often a pending child is polled while waiting on a deadline. Short enough that a timeout
/// is punctual, long enough that a 20-minute gate does not spin a core.
const POLL: Duration = Duration::from_millis(20);

/// What a finished process produced.
#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

/// A command to run.
pub struct Run {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    envs: Vec<(String, String)>,
    env_removes: Vec<String>,
    timeout: Option<Duration>,
    stdin: Option<String>,
}

impl Run {
    pub fn new(program: impl AsRef<OsStr>) -> Run {
        Run {
            program: program.as_ref().to_string_lossy().into_owned(),
            args: Vec::new(),
            cwd: None,
            envs: Vec::new(),
            env_removes: Vec::new(),
            timeout: None,
            stdin: None,
        }
    }

    pub fn arg(mut self, a: impl AsRef<OsStr>) -> Run {
        self.args.push(a.as_ref().to_string_lossy().into_owned());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Run
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        for a in args {
            self.args.push(a.as_ref().to_string_lossy().into_owned());
        }
        self
    }

    pub fn cwd(mut self, dir: impl AsRef<Path>) -> Run {
        self.cwd = Some(dir.as_ref().to_path_buf());
        self
    }

    pub fn env(mut self, k: impl Into<String>, v: impl Into<String>) -> Run {
        self.envs.push((k.into(), v.into()));
        self
    }

    pub fn env_remove(mut self, k: impl Into<String>) -> Run {
        self.env_removes.push(k.into());
        self
    }

    pub fn timeout(mut self, d: Duration) -> Run {
        self.timeout = Some(d);
        self
    }

    pub fn stdin(mut self, body: impl Into<String>) -> Run {
        self.stdin = Some(body.into());
        self
    }

    /// A human-readable rendering of the command, for diagnostics.
    fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }

    /// Run to completion, capturing both streams. The raw exit code is preserved.
    pub fn output(self) -> Result<Output, NotRun> {
        let label = self.display();
        let started = Instant::now();

        let mut cmd = Command::new(&self.program);
        cmd.args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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

        // Own process group, so a timeout can take the whole tree. See module docs §2.
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

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            // The honest form of exit 127. Distinguished from every other spawn failure because
            // "you have not installed it" and "it is there and broke" are different problems.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(NotRun::ToolAbsent(self.program.clone()));
            }
            Err(e) => {
                return Err(NotRun::ToolError {
                    tool: label,
                    status: -1,
                    stderr: format!("spawn failed: {e}"),
                });
            }
        };

        // `setsid` made the child a group leader, so its pgid equals its pid.
        let pgid = child.id() as i32;

        if let Some(body) = self.stdin.as_ref()
            && let Some(mut sink) = child.stdin.take()
        {
            use std::io::Write;
            // A closed stdin (child exited early) is the child's business, not an error here.
            let _ = sink.write_all(body.as_bytes());
        }

        // Drain both pipes for the child's whole life. See module docs §3.
        let mut out_pipe = child.stdout.take();
        let mut err_pipe = child.stderr.take();
        let out_thread = std::thread::spawn(move || drain(&mut out_pipe));
        let err_thread = std::thread::spawn(move || drain(&mut err_pipe));

        let status = match self.timeout {
            None => child.wait().map_err(|e| NotRun::ToolError {
                tool: label.clone(),
                status: -1,
                stderr: format!("wait failed: {e}"),
            })?,
            Some(limit) => {
                let deadline = Instant::now() + limit;
                loop {
                    match child.try_wait() {
                        Ok(Some(s)) => break s,
                        Ok(None) => {
                            if Instant::now() >= deadline {
                                // Kill the GROUP, not just the child.
                                //
                                // SAFETY: `killpg` on a pgid we created. A failure here means the
                                // group is already gone, which is the outcome we wanted anyway.
                                unsafe {
                                    libc::killpg(pgid, libc::SIGKILL);
                                }
                                let _ = child.wait();
                                let _ = out_thread.join();
                                let _ = err_thread.join();
                                return Err(NotRun::Timeout {
                                    tool: label,
                                    secs: limit.as_secs(),
                                });
                            }
                            std::thread::sleep(POLL);
                        }
                        Err(e) => {
                            return Err(NotRun::ToolError {
                                tool: label,
                                status: -1,
                                stderr: format!("try_wait failed: {e}"),
                            });
                        }
                    }
                }
            }
        };

        let stdout = out_thread.join().unwrap_or_default();
        let stderr = err_thread.join().unwrap_or_default();

        // A signal is NOT an exit code. See module docs §1.
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
    /// `compile.sh --selftest` passes **only** on 1 — a deliberately broken input that exits 0
    /// means the gate is hollow, so "success" there is the failure. That contract needs an exact
    /// comparison, not a truthiness test.
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
}

fn drain(pipe: &mut Option<impl Read>) -> String {
    let mut buf = Vec::new();
    if let Some(p) = pipe.as_mut() {
        let _ = p.read_to_end(&mut buf);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// Resolve a program on `PATH`, or report it absent.
///
/// Preflighting the pin honestly, rather than discovering mid-run that the tool was never there.
pub fn which(program: &str) -> Result<PathBuf, NotRun> {
    let path = std::env::var_os("PATH").ok_or_else(|| NotRun::ToolAbsent(program.to_string()))?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(NotRun::ToolAbsent(program.to_string()))
}

/// Retry a fallible operation with a fixed backoff.
///
/// A [`NotRun::ToolAbsent`] is **never** retried: an absent program will still be absent in two
/// seconds, and retrying it only delays the real diagnosis.
pub fn retry<T>(
    attempts: u32,
    backoff: Duration,
    mut f: impl FnMut() -> Result<T, NotRun>,
) -> Result<T, NotRun> {
    let mut last = None;
    for i in 0..attempts.max(1) {
        match f() {
            Ok(v) => return Ok(v),
            Err(e @ NotRun::ToolAbsent(_)) => return Err(e),
            Err(e) => {
                last = Some(e);
                if i + 1 < attempts {
                    std::thread::sleep(backoff);
                }
            }
        }
    }
    Err(last.unwrap_or_else(|| NotRun::ToolAbsent("retry: no attempt ran".into())))
}

/// Poll until `cond` holds or the deadline passes.
///
/// For "the socket is up" / "the port is listening" waits that the scripts spell as a `for i in
/// $(seq …); do … sleep 1; done` loop whose exhaustion is easy to mistake for success.
pub fn wait_for(
    label: &str,
    timeout: Duration,
    poll: Duration,
    mut cond: impl FnMut() -> bool,
) -> Result<(), NotRun> {
    let deadline = Instant::now() + timeout;
    loop {
        if cond() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(NotRun::Timeout {
                tool: label.to_string(),
                secs: timeout.as_secs(),
            });
        }
        std::thread::sleep(poll);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_stdout_and_raw_code() {
        let out = Run::new("sh")
            .arg("-c")
            .arg("echo hello; exit 3")
            .output()
            .unwrap();
        assert_eq!(out.stdout.trim(), "hello");
        assert_eq!(out.code, 3, "raw exit codes must never be collapsed");
    }

    #[test]
    fn captures_stderr_separately() {
        let out = Run::new("sh")
            .arg("-c")
            .arg("echo oops >&2")
            .output()
            .unwrap();
        assert_eq!(out.stderr.trim(), "oops");
        assert!(out.stdout.is_empty());
    }

    #[test]
    fn absent_tool_is_tool_absent_not_a_failure() {
        let got = Run::new("tbd-definitely-not-a-real-program-9f3a").status();
        assert!(matches!(got, Err(NotRun::ToolAbsent(_))));
    }

    #[test]
    fn signal_death_is_signalled_not_an_exit_code() {
        // THE CONTRACT. bash would report 137 here and a `case` arm would read it as a failure.
        let got = Run::new("sh").arg("-c").arg("kill -9 $$").status();
        match got {
            Err(NotRun::Signalled { signal, .. }) => assert_eq!(signal, 9),
            other => panic!("expected Signalled, got {other:?}"),
        }
    }

    #[test]
    fn timeout_reports_timeout() {
        let got = Run::new("sleep")
            .arg("30")
            .timeout(Duration::from_millis(150))
            .status();
        assert!(matches!(got, Err(NotRun::Timeout { .. })));
    }

    #[test]
    fn timeout_kills_the_whole_process_group() {
        // The grandchild outlives its parent unless the GROUP is killed. Marker file proves it:
        // if the tree survived, the sleep completes and writes.
        let marker = std::env::temp_dir().join(format!("tbd-gate-pg-{}", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let script = format!("( sleep 2; touch {} ) & sleep 30", marker.display());
        let got = Run::new("sh")
            .arg("-c")
            .arg(&script)
            .timeout(Duration::from_millis(200))
            .status();
        assert!(matches!(got, Err(NotRun::Timeout { .. })));
        std::thread::sleep(Duration::from_millis(2500));
        assert!(
            !marker.exists(),
            "grandchild survived the timeout — the group was not killed"
        );
    }

    #[test]
    fn large_output_does_not_deadlock() {
        // Well past a 64 KiB pipe buffer on both streams at once.
        let out = Run::new("sh")
            .arg("-c")
            .arg("seq 1 60000; seq 1 60000 >&2")
            .timeout(Duration::from_secs(30))
            .output()
            .unwrap();
        assert_eq!(out.code, 0);
        assert!(out.stdout.lines().count() == 60000);
        assert!(out.stderr.lines().count() == 60000);
    }

    #[test]
    fn stdin_is_delivered() {
        let out = Run::new("cat").stdin("piped body").output().unwrap();
        assert_eq!(out.stdout, "piped body");
    }

    #[test]
    fn env_and_cwd_apply() {
        let out = Run::new("sh")
            .arg("-c")
            .arg("echo $TBD_X; pwd")
            .cwd("/tmp")
            .env("TBD_X", "set")
            .output()
            .unwrap();
        assert!(out.stdout.contains("set"));
        assert!(out.stdout.contains("/tmp"));
    }

    #[test]
    fn expect_code_wants_exactly_that_code() {
        // compile.sh --selftest passes ONLY on 1.
        let v = Run::new("sh")
            .arg("-c")
            .arg("exit 1")
            .expect_code("selftest must fail", 1);
        assert!(matches!(v, Verdict::Held));
        let v = Run::new("sh")
            .arg("-c")
            .arg("exit 0")
            .expect_code("selftest must fail", 1);
        assert!(
            matches!(v, Verdict::Failed(_)),
            "a hollow selftest must not read as a pass"
        );
    }

    #[test]
    fn expect_ok_maps_absent_tool_to_did_not_run() {
        let v = Run::new("tbd-not-real-8c21").expect_ok("thing must work");
        assert!(matches!(v, Verdict::DidNotRun(NotRun::ToolAbsent(_), _)));
        assert_eq!(v.into_exit(), 2);
    }

    #[test]
    fn which_finds_and_misses() {
        assert!(which("sh").is_ok());
        assert!(matches!(
            which("tbd-not-real-4b77"),
            Err(NotRun::ToolAbsent(_))
        ));
    }

    #[test]
    fn retry_gives_up_and_returns_the_last_error() {
        let mut n = 0;
        let got: Result<(), NotRun> = retry(3, Duration::from_millis(1), || {
            n += 1;
            Err(NotRun::Timeout {
                tool: "t".into(),
                secs: 0,
            })
        });
        assert!(got.is_err());
        assert_eq!(n, 3);
    }

    #[test]
    fn retry_succeeds_on_a_later_attempt() {
        let mut n = 0;
        let got = retry(5, Duration::from_millis(1), || {
            n += 1;
            if n < 3 {
                Err(NotRun::Timeout {
                    tool: "t".into(),
                    secs: 0,
                })
            } else {
                Ok(n)
            }
        });
        assert_eq!(got.unwrap(), 3);
    }

    #[test]
    fn retry_does_not_retry_an_absent_tool() {
        let mut n = 0;
        let got: Result<(), NotRun> = retry(5, Duration::from_millis(1), || {
            n += 1;
            Err(NotRun::ToolAbsent("nope".into()))
        });
        assert!(matches!(got, Err(NotRun::ToolAbsent(_))));
        assert_eq!(n, 1, "an absent tool will still be absent next time");
    }

    #[test]
    fn wait_for_returns_when_the_condition_holds() {
        let start = Instant::now();
        let mut n = 0;
        let got = wait_for(
            "thing",
            Duration::from_secs(5),
            Duration::from_millis(5),
            || {
                n += 1;
                n >= 3
            },
        );
        assert!(got.is_ok());
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn wait_for_times_out_rather_than_reporting_success() {
        let got = wait_for(
            "thing",
            Duration::from_millis(80),
            Duration::from_millis(5),
            || false,
        );
        assert!(matches!(got, Err(NotRun::Timeout { .. })));
    }
}
