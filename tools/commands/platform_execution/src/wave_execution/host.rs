//! `hostrun` / `checkrun`: the container-to-host bridge as the wave driver defines it.
//!
//! **Role:** detects whether the driver runs in the development container and whether
//! `distrobox-host-exec` is available, builds the bridged argv of a host command (with its
//! environment and timeout) and of a check-class command, and runs captured or inherited children
//! with bash's exit-code rendering.
//!
//! **Position:** `Host` is resolved into `Ctx` at entry and used by every gate step, `db`,
//! `migrate`, `trunk` and `status`; the container test builds on
//! [`process_runner::host_execution::in_container`].
//!
//! **Signals & state:** none held; spawns children through [`process_runner::Run`].
//!
//! **Invariants:** the container test is the shared two-clause test or a set `$container`; the only
//! bridge is `distrobox-host-exec` (a machine with `host-spawn` alone runs natively). The bridge
//! forwards no environment, so `hostrun` passes `CARGO_TARGET_DIR` and `TEST_DATABASE_URL`
//! explicitly through `env`, read at call time — without the first every worktree builds its own
//! target folder, without the second every database test skips. The timeout runs inside the bridge
//! invocation, so it kills the host process rather than orphaning a cargo build. A child killed by
//! signal `n` reads as `128 + n`, a child that cannot start as 127.

use process_runner::Run;
use verification_core::NotRun;

/// Which side of the bridge we are on, resolved once at load — `HOST_BRIDGE` in the bash.
#[derive(Debug, Clone)]
pub(crate) struct Host {
    /// `HOST_BRIDGE=1`: in a container, with `distrobox-host-exec` available.
    pub bridge: bool,
    /// `GATE_TIMEOUT`.
    pub timeout_secs: u64,
}

/// The shared two-clause test, plus the `$container` clause only this driver has.
///
/// ```text
/// in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ] || [ -n "${container:-}" ]; }
/// ```
fn in_container() -> bool {
    process_runner::host_execution::in_container()
        || std::env::var("container")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}

/// `command -v <prog>` — a PATH lookup, matching what the bash actually tested.
fn on_path(prog: &str) -> bool {
    process_runner::which(prog).is_ok()
}

impl Host {
    /// `command -v distrobox-host-exec IS TRUE ON THE HOST TOO` — read before simplifying this
    /// back.
    ///
    /// The binary is installed on BOTH sides of the bridge: `/usr/bin/distrobox-host-exec` exists
    /// in the container AND on the host. So `command -v` alone selected the bridge even from a
    /// host shell, where it refuses. MEASURED 2026-07-26 on the host:
    /// ```text
    /// $ distrobox-host-exec echo hi
    /// You must run  distrobox-host-exec inside a container!      (exit 126)
    /// ```
    /// The step runner cannot tell that from a compile error, so it reported an ordinary step FAIL
    /// — OBSERVED 10/10 steps red, which reads as a catastrophically broken tree and sends whoever
    /// is holding the pager hunting a phantom for an hour. Same family as everything else in this
    /// file: the tool was confident about a thing it had not actually checked.
    ///
    /// On the host the bridge is not merely unavailable, it is UNNECESSARY: cargo, rustfmt and
    /// trunk are native there — being native on the host is the entire reason the bridge exists in
    /// the other direction — so run them directly. Erroring out instead would replace a phantom
    /// failure with a hard stop on a run that would have worked. But do NOT switch behaviour
    /// silently either: announce it once, by name, so the log says what happened and why.
    pub(crate) fn detect(timeout_secs: u64) -> Host {
        let have = on_path("distrobox-host-exec");
        if have && in_container() {
            return Host {
                bridge: true,
                timeout_secs,
            };
        }
        if have {
            // Printed at LOAD time in the bash (it is a top-level `if`), so it lands before any
            // command's own output. Same placement here.
            crate::wave_execution::werr!(
                "wave: NOTE — this is the HOST shell, not the dev container."
            );
            crate::wave_execution::werr!(
                "         distrobox-host-exec is installed here too but refuses outside a container"
            );
            crate::wave_execution::werr!(
                "         ('You must run  distrobox-host-exec inside a container!', rc 126). Bridging"
            );
            crate::wave_execution::werr!(
                "         through it would have failed EVERY step and read as a broken tree."
            );
            crate::wave_execution::werr!(
                "         Running cargo/rustfmt/trunk natively instead — correct here, and expected."
            );
        }
        Host {
            bridge: false,
            timeout_secs,
        }
    }

    /// The full argv `hostrun <cmd…>` expands to.
    ///
    /// `TEST_DATABASE_URL` is read HERE, at call time, not at detect time: [`super::db::ensure_gate_db`]
    /// exports it partway through the wave gate and every test step after that depends on seeing
    /// the new value. Baking it in at startup is the whole hazard.
    pub(crate) fn hostrun_argv(&self, cmd: &[String]) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        if self.bridge {
            v.push("distrobox-host-exec".into());
            v.push("timeout".into());
            v.push(self.timeout_secs.to_string());
            v.push("env".into());
            v.push(format!(
                "CARGO_TARGET_DIR={}",
                std::env::var("CARGO_TARGET_DIR").unwrap_or_default()
            ));
            v.push(format!(
                "TEST_DATABASE_URL={}",
                std::env::var("TEST_DATABASE_URL").unwrap_or_default()
            ));
        } else {
            v.push("timeout".into());
            v.push(self.timeout_secs.to_string());
        }
        v.extend(cmd.iter().cloned());
        v
    }

    /// `checkrun` — hostrun into the gate's private analysis dir.
    ///
    /// The second `env` wins over the one `hostrun` bakes in, which is the same idiom the test
    /// steps already use. It is a named function rather than that idiom repeated seven times
    /// because the whole point is that NO analysis step is left on the shared dir, and one name is
    /// auditable: `grep -n 'hostrun cargo'` should find nothing in the gate steps.
    ///
    /// `CARGO_INCREMENTAL=0`, and NOT because incremental state is another mtime-keyed cache:
    /// incremental state is CONTENT-keyed, so it is not the mtime mechanism, and the set-back-mtime
    /// case [`super::touch::touch_workspace`] describes goes red with incremental left ON exactly
    /// as it does with it off. It is disabled because it is one more cache standing between this
    /// tree's bytes and the verdict, and the whole subject here is a verdict that came from a cache
    /// instead of from the source.
    ///
    /// THE PRICE IS RECORDED so the trade can be re-made knowingly rather than re-derived. With
    /// `touch_workspace` in front of it, `cargo check --workspace` costs 0.17 s untouched, 1.09 s
    /// touched with incremental ON, 6.05 s touched with it OFF — so this one setting is most of the
    /// difference between a 4.5 s slice gate and a 9.0 s one. Both are inside the ~10 s this gate
    /// is written to, and spending half that budget on having one less thing to trust is the right
    /// way round for the step whose entire job is to be believed. Turn it back on if the budget
    /// ever gets tight; not for tidiness.
    pub(crate) fn checkrun_argv(&self, gate_check_target: &str, cmd: &[String]) -> Vec<String> {
        let mut inner: Vec<String> = vec![
            "env".into(),
            format!("CARGO_TARGET_DIR={gate_check_target}"),
            "CARGO_INCREMENTAL=0".into(),
        ];
        inner.extend(cmd.iter().cloned());
        self.hostrun_argv(&inner)
    }
}

/// Spawn `argv`, capturing stdout and stderr MERGED, and return `(combined, rc)`.
///
/// This is bash's `out="$(cmd 2>&1)"; rc=$?`. `rc` is the RAW status: 124 from `timeout` must
/// stay 124, because both step runners branch on it to print `FAIL (TIMEOUT)` rather than
/// relabelling the most expensive step's deadline as a code error.
///
/// A child killed by a signal has no exit code; bash's `$?` renders that as `128+n` and so does
/// this, deliberately — the step runners were written against that number. (`process_runner`
/// models it honestly as `NotRun::Signalled`, which is the right shape for a NEW gate and the
/// wrong one for a byte-for-byte port of a runner that already branches on 128+n.)
pub(crate) fn capture(argv: &[String]) -> (String, i32) {
    let Some((prog, args)) = argv.split_first() else {
        return (String::new(), 127);
    };
    super::flush();
    match Run::new(prog).args(args).output() {
        Ok(o) => {
            let mut s = o.stdout;
            s.push_str(&o.stderr);
            (s, o.code)
        }
        // A signalled child's captured text goes with it; its `128+n` stays.
        Err(cause) => (String::new(), status_code(Err(cause))),
    }
}

/// Spawn `argv` with our own stdout/stderr INHERITED — bash's bare `hostrun cmd`.
pub(crate) fn inherit(argv: &[String]) -> i32 {
    let Some((prog, args)) = argv.split_first() else {
        return 127;
    };
    super::flush();
    status_code(Run::new(prog).args(args).terminal())
}

/// bash's `$?` for a child: the exit code, `128 + signal` when it died on one, and 127 when it
/// could not be started (`command not found` is 127 in a shell; reaching that here means `PATH`
/// lost `timeout` or the bridge binary between detect and use).
pub(crate) fn status_code(outcome: Result<i32, NotRun>) -> i32 {
    match outcome {
        Ok(code) => code,
        Err(NotRun::Signalled { signal, .. }) => 128 + signal,
        Err(_) => 127,
    }
}

/// Convenience: build a `Vec<String>` argv from string slices.
pub(crate) fn v(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_string()).collect()
}

#[cfg(test)]
#[path = "tests/host/tests.rs"]
mod tests;
