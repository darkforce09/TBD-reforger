//! `hostrun` / `checkrun` — the container→host bridge, exactly as `wave.sh` defines it.
//!
//! ── WHAT IS SHARED WITH `crate::hostrun`, AND WHAT IS NOT ───────────────────────────────────
//!
//! The container test comes from [`crate::hostrun::in_container`] — the lifted primitive — rather
//! than a third copy of distrobox's `distrobox-host-exec:130`. That module's own note asks for
//! exactly this, and flags the one difference:
//!
//! > NOTE for the `wave.sh` port: `scripts/platform/wave.sh:168` carries a THIRD clause,
//! > `|| [ -n "${container:-}" ]`, which `scripts/lib/hostrun.sh` does not. The two-clause form
//! > here is the one both existing Rust callers were built and measured against, so the lift keeps
//! > it exactly. Widening it is a behaviour change and belongs in its own ticket.
//!
//! So [`in_container`] below is `hostrun::in_container()` OR `$container`, which is `wave.sh`'s
//! three-clause test to the letter, built on the shared two. Narrowing it to the shared form would
//! change which side of the bridge this file believes it is on — a behaviour change in the
//! highest-risk file in the program, introduced by a refactor.
//!
//! Three more things stay here because they are `wave.sh` semantics, not bridge semantics:
//!
//!   * BRIDGE SET. `hostrun::Host::detect` accepts `distrobox-host-exec` **or** `host-spawn`.
//!     `wave.sh` knows only `distrobox-host-exec`. On a machine carrying `host-spawn` alone the
//!     shared detector would bridge where the bash ran natively.
//!   * ENV FORWARDING. `hostrun` bakes in `env CARGO_TARGET_DIR=… TEST_DATABASE_URL=…`, read at
//!     CALL time. The shared bridge forwards nothing.
//!   * TIMEOUT PLACEMENT. `timeout` is inside the bridge invocation on purpose (see below).
//!
//! ── THE MEASUREMENTS THIS FILE EXISTS FOR (verbatim from the bash) ───────────────────────────
//!
//! The container's glibc (2.36) is older than the host's (2.43), so binaries built on the host —
//! including `target/debug/xtask` — refuse to run in here. Route those through the host when we
//! can.
//!
//! MEASURED 2026-07-26: `distrobox-host-exec` does NOT forward the environment.
//! ```text
//! $ FOO=bar distrobox-host-exec sh -c 'echo [$FOO]'  ->  []
//! ```
//! So the exported `CARGO_TARGET_DIR` is invisible to cargo on the host, and every worktree
//! silently builds its own `target/` — 1.4 GB within 25 s of a single `cargo check`, ~44 GB for a
//! full build. Eight worktrees would exhaust 129 GB of free disk around the third slice, and every
//! gate after that fails with a No-space error that reads exactly like a compile error. It must be
//! passed explicitly through `env`.
//!
//! The timeout lives HERE, not in the step runner. Two reasons: `command -v` matches shell
//! functions, so a run()-level wrapper tried to `timeout hostrun` and failed outright; and
//! wrapping on this side kills the actual host process rather than just severing the bridge and
//! orphaning a cargo build.
//!
//! **TEST_DATABASE_URL IS IN THE WHITELIST FOR A REASON — read before removing it.** The whitelist
//! used to carry `CARGO_TARGET_DIR` alone, and `run "test api"` runs `cargo test -p website-api`.
//! Every DB-backed integration test does `let Some(x) = boot() else { eprintln!("skip: …");
//! return; }`, and `boot()` returns `None` without `TEST_DATABASE_URL` — so 30 of them SKIPPED and
//! the step printed PASS. Measured 2026-07-26: `TEST_DATABASE_URL=x distrobox-host-exec sh -c
//! 'echo [$TEST_DATABASE_URL]'` -> `[]`, and the suite finishing in 0.00s for a DB-backed crate is
//! the tell. Consequence, which is why this is a BLOCKER and not a nit: EVERY regression test this
//! program added — T-343, T-346, T-347, T-348, T-349, T-366 all live in
//! `tests/{missions,events,telemetry}.rs` — was invisible to the gate that cleared their slices.
//!
//! T-575: `MIGRATE_TEST_DATABASE_URL` was forwarded here too and is gone with its consumer — see
//! [`super::db::ensure_gate_db`]. Forwarding an unset variable is harmless; forwarding one that
//! looks live is how a dead path survives four waves of readers.

use std::process::{Command, Stdio};

/// Which side of the bridge we are on, resolved once at load — `HOST_BRIDGE` in the bash.
#[derive(Debug, Clone)]
pub struct Host {
    /// `HOST_BRIDGE=1`: in a container, with `distrobox-host-exec` available.
    pub bridge: bool,
    /// `GATE_TIMEOUT`.
    pub timeout_secs: u64,
}

/// `wave.sh:169` — the shared two-clause test, plus the `$container` clause only `wave.sh` has.
///
/// ```text
/// in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ] || [ -n "${container:-}" ]; }
/// ```
fn in_container() -> bool {
    crate::hostrun::in_container()
        || std::env::var("container")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
}

/// `command -v <prog>` — a PATH lookup, matching what the bash actually tested.
fn on_path(prog: &str) -> bool {
    tbd_gate::proc::which(prog).is_ok()
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
    pub fn detect(timeout_secs: u64) -> Host {
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
            crate::werr!("wave.sh: NOTE — this is the HOST shell, not the dev container.");
            crate::werr!(
                "         distrobox-host-exec is installed here too but refuses outside a container"
            );
            crate::werr!(
                "         ('You must run  distrobox-host-exec inside a container!', rc 126). Bridging"
            );
            crate::werr!(
                "         through it would have failed EVERY step and read as a broken tree."
            );
            crate::werr!(
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
    /// the new value. Baking it in at startup is the whole T-575-adjacent hazard.
    pub fn hostrun_argv(&self, cmd: &[String]) -> Vec<String> {
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
    /// `CARGO_INCREMENTAL=0`, and NOT for the reason it first looks like. An earlier draft of this
    /// comment justified it as "another mtime-keyed cache layered on top of the one that lied".
    /// That was wrong, and getting a justification wrong in this file is the same class of error as
    /// the bug — so it is corrected here rather than quietly dropped. Incremental state is
    /// CONTENT-keyed, not mtime-keyed, so it is emphatically not the mechanism T-421 is about:
    /// MEASURED 2026-07-26, repro A goes red with incremental left ON exactly as it does with it
    /// off. It is disabled because it is one more cache standing between this tree's bytes and the
    /// verdict, and the whole subject here is a verdict that came from a cache instead of from the
    /// source.
    ///
    /// THE PRICE IS RECORDED so the trade can be re-made knowingly rather than re-derived. With
    /// `touch_workspace` in front of it, `cargo check --workspace` costs 0.17 s untouched, 1.09 s
    /// touched with incremental ON, 6.05 s touched with it OFF — so this one setting is most of the
    /// difference between a 4.5 s slice gate and a 9.0 s one. Both are inside the ~10 s this gate
    /// is written to, and spending half that budget on having one less thing to trust is the right
    /// way round for the step whose entire job is to be believed. Turn it back on if the budget
    /// ever gets tight; not for tidiness.
    pub fn checkrun_argv(&self, gate_check_target: &str, cmd: &[String]) -> Vec<String> {
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
/// this, deliberately — the step runners were written against that number. (`tbd_gate::proc`
/// models it honestly as `NotRun::Signalled`, which is the right shape for a NEW gate and the
/// wrong one for a byte-for-byte port of a runner that already branches on 128+n.)
pub fn capture(argv: &[String]) -> (String, i32) {
    let Some((prog, args)) = argv.split_first() else {
        return (String::new(), 127);
    };
    super::flush();
    let out = Command::new(prog).args(args).stdin(Stdio::null()).output();
    match out {
        Ok(o) => {
            let mut s = String::from_utf8_lossy(&o.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            (s, status_code(&o.status))
        }
        // `command not found` is 127 in a shell. Reaching this means PATH lost `timeout` or the
        // bridge binary between detect and use.
        Err(_) => (String::new(), 127),
    }
}

/// Spawn `argv` with our own stdout/stderr INHERITED — bash's bare `hostrun cmd`.
pub fn inherit(argv: &[String]) -> i32 {
    let Some((prog, args)) = argv.split_first() else {
        return 127;
    };
    super::flush();
    match Command::new(prog).args(args).status() {
        Ok(st) => status_code(&st),
        Err(_) => 127,
    }
}

/// bash's `$?` for a child: the exit code, or `128 + signal` when it died on one.
pub fn status_code(st: &std::process::ExitStatus) -> i32 {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = st.signal() {
            return 128 + sig;
        }
    }
    st.code().unwrap_or(127)
}

/// Convenience: build a `Vec<String>` argv from string slices.
pub fn v(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_hostrun_wraps_in_timeout_only() {
        let h = Host {
            bridge: false,
            timeout_secs: 1200,
        };
        assert_eq!(
            h.hostrun_argv(&v(&["cargo", "check"])),
            v(&["timeout", "1200", "cargo", "check"])
        );
    }

    #[test]
    fn bridged_hostrun_forwards_the_whitelist_because_distrobox_does_not() {
        // MEASURED 2026-07-26: distrobox-host-exec does not forward the environment. If this
        // assertion ever loosens, every worktree silently builds its own 44 GB target dir.
        unsafe { std::env::set_var("CARGO_TARGET_DIR", "/tmp/ctd") };
        unsafe { std::env::set_var("TEST_DATABASE_URL", "postgres://x") };
        let h = Host {
            bridge: true,
            timeout_secs: 60,
        };
        assert_eq!(
            h.hostrun_argv(&v(&["cargo", "test"])),
            v(&[
                "distrobox-host-exec",
                "timeout",
                "60",
                "env",
                "CARGO_TARGET_DIR=/tmp/ctd",
                "TEST_DATABASE_URL=postgres://x",
                "cargo",
                "test",
            ])
        );
        unsafe { std::env::remove_var("TEST_DATABASE_URL") };
    }

    #[test]
    fn checkrun_second_env_wins_over_the_baked_in_one() {
        let h = Host {
            bridge: false,
            timeout_secs: 5,
        };
        assert_eq!(
            h.checkrun_argv("/gate/check", &v(&["cargo", "clippy"])),
            v(&[
                "timeout",
                "5",
                "env",
                "CARGO_TARGET_DIR=/gate/check",
                "CARGO_INCREMENTAL=0",
                "cargo",
                "clippy",
            ])
        );
    }

    #[test]
    fn timeout_rc_124_survives_as_124() {
        // The step runners branch on exactly this number.
        let (_, rc) = capture(&v(&["timeout", "1", "sleep", "5"]));
        assert_eq!(
            rc, 124,
            "timeout(1) must surface as 124, not as a generic failure"
        );
    }
}
