//! Gate serialisation: one gate at a time on this machine.
//!
//! **Role:** `GateState` takes the gate lock (a `flock` on a file under the main checkout's
//! `target/`), waits for a running gate up to a bound, writes a holder note naming the gate, and
//! prints the verdict line.
//!
//! **Position:** taken by both gate drivers before their first cargo step and passed to the steps
//! that mutate shared state (`db`, `migrate`); the lock itself is [`verification_core::GateLock`].
//!
//! **Signals & state:** the held lock (or the deliberate unserialised escape hatch,
//! `TBD_GATE_ALLOW_UNSERIALISED=1`) for the lifetime of one gate; the holder note file.
//!
//! **Invariants:** the per-step build folders and the gate database are shared by every worktree,
//! so two concurrent gates report on each other's code; the lock covers the whole gate, because a
//! verdict describes one tree at one moment and the fingerprint invalidation must share the
//! critical section with the steps it protects. Holding a lock is provable only by having acquired
//! one (`GateLock` has no public constructor). A lock that cannot be taken refuses rather than
//! running unserialised; a degraded gate cannot print a clean verdict. Rust opens the lock file
//! close-on-exec, so no step's child keeps it after `exec`; a fork from another thread can still
//! hold it until its `exec`, which a single-threaded gate never does. The holder note is cleared
//! only while it is still this gate's.

use std::time::Duration;

use verification_core::verdict::NotRun;
use verification_core::{GateLock, flock_exclusive};

use super::Ctx;
use crate::wave_execution::wprintln;

/// The gate's serialisation state: the proof, or the operator's explicit degradation, or neither.
///
/// `GATE_LOCK_HELD` / `GATE_UNSERIALISED` / `GATE_UNSERIALISED_WHY` in the bash, made into one
/// value that cannot be half-set.
pub(crate) struct GateState {
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
    pub(crate) fn new() -> GateState {
        GateState {
            lock: None,
            unserialised: false,
            why: String::new(),
            _note: None,
        }
    }

    /// `[ "${GATE_LOCK_HELD:-0}" = 1 ]` — a real flock is held.
    pub(crate) fn held(&self) -> bool {
        self.lock.is_some()
    }

    /// `[ "${GATE_UNSERIALISED:-0}" = 1 ]`.
    pub(crate) fn unserialised(&self) -> bool {
        self.unserialised
    }

    /// A gate that blocks silently for minutes is indistinguishable from a hung one, and this
    /// program runs unattended — so the wait announces itself, names the holder, and heartbeats
    /// until it clears.
    ///
    /// Returns the bash rc: `0` acquired (or degraded on purpose), `2` refused.
    pub(crate) fn take(&mut self, ctx: &Ctx, what: &str) -> u8 {
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
    ///   * What the lock buys is not a nicety. A run watched `target/gate-api/debug/deps/events-*`
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
        // what lets ensure_gate_db still prepare its databases under this hatch; the lock
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
    pub(crate) fn verdict(&self, result: &str, label: &str) {
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
        // `date -u +%FT%TZ`'s rendering (`2026-08-14T12:34:56Z`) — the note is read by humans
        // and by the next waiter's message, so the format is a contract.
        let stamp = time_source::now_utc_rfc3339();
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
        if let Ok(body) = std::fs::read_to_string(&self.path)
            && body.contains(&needle)
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}
