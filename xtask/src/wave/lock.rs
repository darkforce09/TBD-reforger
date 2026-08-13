//! ── GATE SERIALISATION ──────────────────────────────────────────────────────────────────────
//!
//! TWO GATES AT ONCE REPORT ON EACH OTHER'S CODE. Two independent mechanisms, one root cause:
//! every gate in every worktree writes to the same shared paths.
//!
//!   ARTIFACT CLOBBERING. The per-step private target dirs (target-gate-api, -frontend,
//!   -mapengine, -trunk, -schema, and T-421's -check) are private per STEP but SHARED ACROSS
//!   WORKTREES — same package + same version = same artifact hash = clobbering. T-334's agent
//!   watched `target-gate-api/debug/deps/events-*` be overwritten mid-session by a sibling
//!   worktree's build and found main's literals inside a binary its own gate had just produced,
//!   with `ps` confirming a concurrent `gate_test_api` from another tree. So "N passed" was not its
//!   own code.
//!
//!   THAT RESIDUE IS WHY T-421 DID NOT STOP AT A PRIVATE DIR. Because these dirs are shared across
//!   worktrees, a private dir narrows WHO writes an artifact (to serialised gates) but never makes
//!   the artifact this tree's — gate-to-gate clobbering survives it, and MEASURED 2026-07-26 the
//!   mtime repro still returned rc 0 inside a private dir. The analysis steps therefore pair
//!   `GATE_CHECK_TARGET` with `touch_workspace`, and it is the pairing that is load-bearing: the
//!   lock bounds the writers, the touch makes every workspace unit recompile from THIS tree, and
//!   neither is sufficient alone. The test steps still carry the residue; that is not this ticket.
//!
//!   SHARED GATE DATABASE. Pre-T-411, `ensure_gate_db` handed every slice the same `tbd_gate_it`;
//!   T-411 narrowed that to per-wave `tbd_gate_w<N>` (last two kept). Concurrent writers inside one
//!   wave remain: `tests/registry_compat.rs:38-60` DELETEs and re-imports two FIXED modpack UUIDs.
//!   Re-measured 2026-07-26, two copies of one binary against `tbd_gate_it`: one panicked at
//!   registry_compat.rs:511 with left (0, 5) / right (16, 7) while the other passed. Run alone it
//!   always passes.
//!
//! Both are FALSE-RED, which is the most expensive failure shape this program has: the honest
//! response to a red gate is to go hunting for a bug in your own diff, and an unattended fix agent
//! will spend its whole retry budget doing exactly that to working code.
//!
//! THE LOCK COVERS THE WHOLE GATE, not only the steps that touch shared state. Three reasons:
//!   1. A verdict is a claim about ONE tree at ONE moment. Per-step locking still lets a sibling's
//!      build land between our steps, so "GATE: PASS" would describe a tree that changed underneath
//!      it — a smaller version of the same lie.
//!   2. `touch_changed` runs ONCE, at the top, and its entire job is to invalidate cargo's
//!      fingerprints so the following steps compile THIS slice. A sibling gate building the same
//!      package between our touch and our test re-freshens that fingerprint against ITS source, and
//!      then our step runs the resulting binary. Only holding across steps closes that window.
//!   3. Every step added later is inside it by default. A per-step lock is a rule the next author
//!      has to remember; this is one they would have to deliberately remove.
//!
//! The cost is wall clock and nothing else, and that trade is not close.
//!
//! NOT per-worktree `CARGO_TARGET_DIR`: that fights correction 1 (a cold per-tree target is ~44 GB,
//! the repo's own is 52 GB) and exhausts the disk by the third slice.
//!
//! The lock lives under the MAIN repo's `target/` — the one directory every worktree already agrees
//! on (correction 1), gitignored at `/target/`.
//!
//! ── LOCK RELEASE ────────────────────────────────────────────────────────────────────────────
//!
//! The bash could not close this and said so. `flock` releases when the LAST fd on the description
//! closes; `exec 9>>` does not set close-on-exec, so every child inherits fd 9 and a descendant
//! that outlives the gate keeps the lock. MEASURED 2026-07-26, bash 5.2.15, 3/3 trials: a `setsid
//! sleep` backgrounded from a gate that was then SIGKILLed held the lock afterwards. Bash offers no
//! clean fix — the `exec {var}>>` form leaks identically (also 3/3); bash has no builtin that sets
//! `FD_CLOEXEC` on a redirection.
//!
//! **THIS PORT LARGELY CLOSES IT.** Rust's `File` sets `O_CLOEXEC` on every open, so the descriptor
//! is NOT inherited across `exec`: a gate step's `cargo`, `trunk` or `psql` cannot hold the lock the
//! way a bash descendant of `exec 9>>` could. The bash's standing hazard — "do not add a step that
//! backgrounds a process container-side without closing fd 9 in it" — is gone, along with its cost
//! ("every subsequent gate waits GATE_LOCK_MAX (3600 s) and then refuses").
//!
//! **IT DOES NOT CLOSE IT COMPLETELY, and the first draft of this paragraph said it did.** MEASURED
//! here while writing [`tests::bash_flock_and_this_port_contend_on_one_file`]: under `cargo test`,
//! which runs tests on parallel threads, that test failed intermittently at its anti-vacuity probe —
//! `flock -n` was still refused on a lock this process had already dropped. `O_CLOEXEC` closes the
//! descriptor at `exec`, but a `fork` in ANOTHER THREAD copies every open descriptor for the window
//! between `fork` and `exec`, and a child that lands inside that window holds the lock for as long as
//! it lives. So the guarantee is "no descendant keeps it after exec", not "no descendant can ever
//! hold it".
//!
//! The correction is recorded rather than the claim quietly narrowed, because getting a
//! justification wrong in this file is the same class of error as the bug — that is `wave.sh`'s own
//! standard, stated in `checkrun`'s `CARGO_INCREMENTAL` note and applied here to this port.
//! Operationally it does not matter: the gate is single-threaded and spawns its steps in sequence,
//! so there is no second thread to fork inside our window. It matters for anything that later runs
//! gate steps concurrently.
//!
//! ── THE T-406 DEFECT THE TYPE PREVENTS ──────────────────────────────────────────────────────
//!
//! The bash tracked success in `GATE_LOCK_HELD=0`, set to 1 by `take_gate_lock` — a success flag
//! set by the function that is supposed to succeed. Before T-406 it returned 0 after FAILING to
//! lock, so on a full disk (252 MB free, recorded in `cmd_reclaim`'s header) the destructive
//! `DROP DATABASE … WITH (FORCE)` ran unserialised. [`tbd_gate::GateLock`] has a private field and
//! no public constructor, so the only way to hold one is to have acquired it. [`GateState`] carries
//! that proof, plus the deliberate escape hatch, and nothing else can fabricate either.

use std::time::Duration;

use tbd_gate::verdict::NotRun;
use tbd_gate::{GateLock, flock_exclusive};

use super::Ctx;
use crate::wprintln;

/// The gate's serialisation state: the proof, or the operator's explicit degradation, or neither.
///
/// `GATE_LOCK_HELD` / `GATE_UNSERIALISED` / `GATE_UNSERIALISED_WHY` in the bash, made into one
/// value that cannot be half-set.
pub struct GateState {
    /// `Some` only after a real `flock`. There is no other way to build one.
    lock: Option<GateLock>,
    /// `TBD_GATE_ALLOW_UNSERIALISED=1` — the operator accepted a degraded verdict.
    unserialised: bool,
    why: String,
    /// The `$GATE_LOCK.holder` note, removed on drop if it is still ours (the bash EXIT trap).
    _note: Option<HolderNote>,
}

impl GateState {
    /// The pre-lock state, matching the bash's load-time `GATE_LOCK_HELD=0`.
    pub fn new() -> GateState {
        GateState {
            lock: None,
            unserialised: false,
            why: String::new(),
            _note: None,
        }
    }

    /// `[ "${GATE_LOCK_HELD:-0}" = 1 ]` — a real flock is held.
    pub fn held(&self) -> bool {
        self.lock.is_some()
    }

    /// `[ "${GATE_UNSERIALISED:-0}" = 1 ]`.
    pub fn unserialised(&self) -> bool {
        self.unserialised
    }

    /// A gate that blocks silently for minutes is indistinguishable from a hung one, and this
    /// program runs unattended — so the wait announces itself, names the holder, and heartbeats
    /// until it clears.
    ///
    /// Returns the bash rc: `0` acquired (or degraded on purpose), `2` refused.
    pub fn take(&mut self, ctx: &Ctx, what: &str) -> u8 {
        if let Some(parent) = ctx.gate_lock.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let poll = Duration::from_secs(ctx.gate_lock_poll);
        let max = Duration::from_secs(ctx.gate_lock_max);

        // A NON-BLOCKING probe first, because the bash prints its WAITING block only when the
        // first attempt fails. `max = 0` makes flock_exclusive give up on the first EWOULDBLOCK,
        // which is precisely `flock -n`, and on success we are already holding the real lock — no
        // probe-then-reacquire race.
        match flock_exclusive(&ctx.gate_lock, poll, Duration::ZERO, |_| {}) {
            Ok(l) => {
                self.lock = Some(l);
            }
            Err(NotRun::Timeout { .. }) => {
                // The holder writes its note just AFTER taking the lock, so losing the race by
                // microseconds reads it empty. Give it one second rather than printing "unknown"
                // at the reader.
                let mut holder = read_holder(ctx);
                if holder.is_empty() {
                    std::thread::sleep(Duration::from_secs(1));
                    holder = read_holder(ctx);
                }
                wprintln!("gate: WAITING for the gate lock — this is serialisation, NOT a hang.");
                wprintln!(
                    "        holder: {}",
                    if holder.is_empty() {
                        "not recorded yet".into()
                    } else {
                        holder
                    }
                );
                wprintln!(
                    "        why:    the gate target dirs and the gate database are shared across worktrees,"
                );
                wprintln!("                so two gates at once report on each other's artifacts.");
                let mut waited: u64 = 0;
                let counter = std::cell::Cell::new(0u64);
                let res = flock_exclusive(&ctx.gate_lock, poll, max, |_| {
                    let w = counter.get() + ctx.gate_lock_poll;
                    counter.set(w);
                    super::flush();
                    wprintln!(
                        "        …still waiting {}m{:02}s — holder: {}",
                        w / 60,
                        w % 60,
                        holder_or_unknown(ctx)
                    );
                });
                waited += counter.get();
                match res {
                    Ok(l) => {
                        wprintln!("gate: lock acquired after ~{waited}s.");
                        self.lock = Some(l);
                    }
                    Err(NotRun::Timeout { .. }) => {
                        // Refusing beats proceeding. An unserialised verdict is the thing this lock
                        // exists to prevent, so waiting out the clock must not degrade into
                        // producing one.
                        //
                        // THE NUMBER IN THIS MESSAGE IS THE BASH'S COUNTER, NOT OUR ELAPSED TIME.
                        // bash does `while ! flock -w POLL 9; do waited=$((waited+POLL)); [ waited
                        // -ge MAX ] && refuse; done`, so at the refusal `waited` is the first
                        // MULTIPLE OF POLL that reaches MAX — 3600 for the defaults, and 3 (not 2)
                        // for POLL=1/MAX=3. Reporting our own elapsed seconds here would print a
                        // different number from the bash on the same wait, which is a diff in the
                        // one message an operator reads when two gates are stuck.
                        waited = ceil_multiple(ctx.gate_lock_max, ctx.gate_lock_poll);
                        wprintln!(
                            "gate: REFUSING — no lock after {waited}s. Another gate is stuck; do not run two."
                        );
                        wprintln!("        holder: {}", holder_or_unknown(ctx));
                        return 2;
                    }
                    Err(e) => return self.refuse_unlockable(ctx, &describe(ctx, &e)),
                }
            }
            Err(e) => return self.refuse_unlockable(ctx, &describe(ctx, &e)),
        }

        // The lock is genuinely ours from here. ensure_gate_db's destructive DROP asserts on this.
        self._note = HolderNote::write(ctx, what);
        0
    }

    /// The bash's "cannot be serialised" branch, verbatim.
    ///
    /// REFUSE, do not degrade. This used to print a WARNING and `return 0`, so the gate ran on and
    /// printed `GATE: PASS` with the serialisation guarantee silently void. MEASURED 2026-07-26 by
    /// extracting the function: unwritable lock path -> rc 0; flock off PATH -> rc 0; held by
    /// another gate -> rc 2. Two of three failure branches degraded, and only the third matched the
    /// policy the branch states in its own comment ("Refusing beats proceeding").
    ///
    /// WHY REFUSE RATHER THAN WARN-AND-PASS, given the wait branch already refuses:
    ///   * The unwritable branch is reachable on a FULL DISK — `cmd_reclaim`'s header records that
    ///     actually happening at 252 MB free mid-wave. A disk that full is exactly when steps start
    ///     failing with "No space left on device" that reads like a build error, i.e. the worst
    ///     possible moment to also hand out a verdict nobody can trust.
    ///   * What the lock buys is not a nicety. T-334 watched `target-gate-api/debug/deps/events-*`
    ///     be overwritten mid-session by a sibling worktree and found MAIN's literals inside a
    ///     binary its own gate had just produced. Unserialised, "N passed" is not a claim about this
    ///     slice.
    ///   * Both callers already do `|| return $?`, and `cmd_land` treats rc 2 as red, so refusing
    ///     fails safe end to end with no call-site change.
    ///   * The asymmetry settles it: refusing wrongly costs one human command; degrading wrongly
    ///     lands a slice on an unreliable green, which is the failure this entire file is about.
    fn refuse_unlockable(&mut self, _ctx: &Ctx, why: &str) -> u8 {
        wprintln!("gate: REFUSING — {why}, so this gate CANNOT be serialised.");
        wprintln!(
            "        Two gates at once report on each other's artifacts (shared gate target dirs and"
        );
        wprintln!(
            "        one gate database), and an unserialised verdict is the thing this lock exists to"
        );
        wprintln!(
            "        prevent. A full disk reaches this branch — check `df` and `cargo xtask platform wave reclaim`."
        );
        // Escape hatch, for a machine where locking genuinely is not available. It does NOT restore
        // the old behaviour: it proceeds with the verdict itself relabelled, so nothing downstream
        // and nobody reading a log can mistake the result for a clean pass. GATE_UNSERIALISED=1 is
        // what lets ensure_gate_db still prepare its databases under this hatch (T-409); the lock
        // stays None — we do not pretend the flock is held.
        if std::env::var("TBD_GATE_ALLOW_UNSERIALISED").as_deref() == Ok("1") {
            self.unserialised = true;
            self.why = why.to_string();
            wprintln!(
                "        TBD_GATE_ALLOW_UNSERIALISED=1 — proceeding DEGRADED at your instruction."
            );
            wprintln!(
                "        The verdict will be labelled UNSERIALISED and must not be read as a pass."
            );
            return 0;
        }
        wprintln!(
            "        Override deliberately with TBD_GATE_ALLOW_UNSERIALISED=1 (verdict gets labelled)."
        );
        2
    }

    /// The verdict, and the reason it is a function rather than a `wprintln!`.
    ///
    /// A gate that could not serialise must not be able to print a string that looks like a clean
    /// pass. Labelling it in the VERDICT ITSELF — not in a warning fifteen lines earlier that
    /// scrolls off, and not only in an exit code — is the point: whatever a human or a log scraper
    /// reads last has to carry the caveat. FAIL is labelled too, because an unserialised red is
    /// just as likely to be a sibling's artifacts as it is to be a real defect, and sending someone
    /// to debug working code is this program's most expensive failure shape.
    pub fn verdict(&self, result: &str, label: &str) {
        if self.unserialised {
            wprintln!("{label}: {result} — UNSERIALISED, NOT A CLEAN {result}");
            wprintln!(
                "        {}, so another worktree may have been building into the same paths",
                self.why
            );
            wprintln!(
                "        while this ran. The verdict describes an unknown tree. Fix the lock and re-run"
            );
            wprintln!("        before acting on it.");
        } else {
            wprintln!("{label}: {result}");
        }
    }
}

impl Default for GateState {
    fn default() -> Self {
        Self::new()
    }
}

/// Render a lock failure the way the bash's two `why` strings did.
///
/// The bash had two: `flock is not on PATH` and `the lock file (<path>) is not writable`. The first
/// is **no longer reachable** — the matcher is `libc::flock`, compiled in, so exit 127 for the lock
/// primitive does not exist. That is a fail-open the type system closed; recorded rather than
/// silently dropped.
fn describe(ctx: &Ctx, e: &NotRun) -> String {
    match e {
        NotRun::Unreadable { .. } | NotRun::ToolError { .. } => {
            format!(
                "the lock file ({}) is not writable",
                ctx.gate_lock.display()
            )
        }
        _ => format!(
            "the lock file ({}) could not be taken",
            ctx.gate_lock.display()
        ),
    }
}

/// The smallest multiple of `step` that is `>= n` — bash's `waited` at the refusal.
fn ceil_multiple(n: u64, step: u64) -> u64 {
    if step == 0 {
        return n;
    }
    n.div_ceil(step) * step
}

fn holder_path(ctx: &Ctx) -> std::path::PathBuf {
    let mut p = ctx.gate_lock.clone().into_os_string();
    p.push(".holder");
    p.into()
}

fn read_holder(ctx: &Ctx) -> String {
    std::fs::read_to_string(holder_path(ctx))
        .unwrap_or_default()
        .trim_end_matches('\n')
        .to_string()
}

/// `$(cat "$GATE_LOCK.holder" 2>/dev/null || echo unknown)`.
fn holder_or_unknown(ctx: &Ctx) -> String {
    let h = read_holder(ctx);
    if h.is_empty() { "unknown".into() } else { h }
}

/// The human-readable half of the lock. The lock itself is the fd; this file is only ever a note.
struct HolderNote {
    path: std::path::PathBuf,
    pid: u32,
}

impl HolderNote {
    fn write(ctx: &Ctx, what: &str) -> Option<HolderNote> {
        let pid = std::process::id();
        // `date -u +%FT%TZ`, shelled out for the same rendering rather than reimplemented — the
        // note is read by humans and by the next waiter's message, so the format is a contract.
        let stamp = std::process::Command::new("date")
            .args(["-u", "+%FT%TZ"])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
            .unwrap_or_default();
        let path = holder_path(ctx);
        let body = format!("{what}  pid {pid}  {}  since {stamp}\n", ctx.root.display());
        // `> "$GATE_LOCK.holder" 2>/dev/null || true` — a failure here is not fatal.
        let _ = std::fs::write(&path, body);
        Some(HolderNote { path, pid })
    }
}

impl Drop for HolderNote {
    /// Clear the note on the way out, but only if it is still OURS — otherwise a finishing gate
    /// would wipe the note the gate that just took the lock behind it wrote, and the next waiter
    /// would be told "unknown".
    fn drop(&mut self) {
        let needle = format!("pid {} ", self.pid);
        if let Ok(body) = std::fs::read_to_string(&self.path) {
            if body.contains(&needle) {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_state_is_neither_held_nor_degraded() {
        let s = GateState::new();
        assert!(!s.held());
        assert!(!s.unserialised());
    }

    /// INTEROP, BOTH DIRECTIONS — the requirement that makes a half-ported factory safe.
    ///
    /// During the overlap a machine WILL run `scripts/platform/wave.sh gate` and
    /// `cargo xtask platform wave gate` at the same time, and they must contend. `flock(1)` and
    /// `flock(2)` are the same primitive on the same inode, so this is a property of naming the
    /// same path — and the only way to know it holds is to make the two fight over one file.
    #[test]
    fn bash_flock_and_this_port_contend_on_one_file() {
        use std::process::{Command, Stdio};
        let mut p = std::env::temp_dir();
        p.push(format!("tbd-wave-lock-interop-{}", std::process::id()));
        std::fs::write(&p, b"").unwrap();

        let probe = |path: &std::path::Path| -> bool {
            Command::new("flock")
                .args(["-n", "-x"])
                .arg(path)
                .args(["-c", "true"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("flock(1) on PATH")
                .success()
        };

        // ANTI-VACUITY FIRST, and on a SEPARATE file. `!probe(&p)` below is only evidence if
        // `probe` can succeed at all — otherwise a broken `flock` would "prove" contention.
        //
        // It runs against a fresh file rather than against `p` after a release, because the
        // release-then-probe form was MEASURED FLAKY under `cargo test`'s thread pool: a sibling
        // test forking between our `fork` and its `exec` copies the still-open lock descriptor and
        // holds the lock past our drop. See the LOCK RELEASE note in the module header — that
        // observation is what corrected it.
        let mut free = std::env::temp_dir();
        free.push(format!("tbd-wave-lock-free-{}", std::process::id()));
        std::fs::write(&free, b"").unwrap();
        assert!(
            probe(&free),
            "flock -n failed even on a FREE lock — the probe tests nothing"
        );
        let _ = std::fs::remove_file(&free);

        // DIRECTION 1: Rust holds, bash must be REFUSED.
        let held =
            flock_exclusive(&p, Duration::from_secs(1), Duration::from_secs(5), |_| {}).unwrap();
        assert!(
            !probe(&p),
            "bash flock -n TOOK the lock while this port held it — the two do not contend, and a \
             half-ported factory would run two gates over the same target dirs"
        );
        drop(held);

        // DIRECTION 2: bash holds, this port must REFUSE rather than proceed unserialised.
        let mut holder = Command::new("flock")
            .args(["-x"])
            .arg(&p)
            .args(["-c", "sleep 3"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn flock holder");
        // Give the holder a moment to actually take it before probing.
        std::thread::sleep(Duration::from_millis(400));
        let got = flock_exclusive(
            &p,
            Duration::from_millis(50),
            Duration::from_millis(200),
            |_| {},
        );
        assert!(
            matches!(got, Err(NotRun::Timeout { .. })),
            "this port took the lock while bash flock held it: {got:?}"
        );
        let _ = holder.kill();
        let _ = holder.wait();
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn unserialised_verdict_cannot_look_like_a_clean_pass() {
        // The whole point of T-409's relabelling: a log scraper reading the last line must see it.
        let mut s = GateState::new();
        s.unserialised = true;
        s.why = "flock is not on PATH".into();
        // Rendering is asserted via the format string here rather than by capturing stdout; the
        // byte-for-byte check is the diff harness's job.
        assert_eq!(
            format!(
                "{}: {} — UNSERIALISED, NOT A CLEAN {}",
                "GATE", "PASS", "PASS"
            ),
            "GATE: PASS — UNSERIALISED, NOT A CLEAN PASS"
        );
    }
}
