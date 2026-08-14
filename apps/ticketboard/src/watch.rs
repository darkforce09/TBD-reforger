//! File watch (T-915.3 §Read architecture) — notify wiring plus a PURE debounce /
//! suppression state machine (fake-clock tested; the notify shell itself is thin
//! and untested).
//!
//! One recursive watch on `.ai/tickets/` (one watch, not 1200) plus non-recursive
//! parent-dir watches covering the sync targets: `docs/TICKET_*.md` +
//! `docs/MILESTONES.md`, `CLAUDE.md`, and the ROADMAP marker file. Parent-dir
//! watches (with a name filter) rather than per-file watches on purpose: editors
//! save by rename-over, which silently breaks an inotify file watch.
//!
//! Raw events → [`Debouncer`]: ≥500 ms of quiet → ONE fire → the app reloads
//! corpus+lock and (through the check coalescer) re-runs the strict check.
//! Suppression hook for the future T-915.4 verb runner: while set — and for one
//! debounce window after clear — fires keep reloading but never trigger check
//! re-runs (otherwise every app mutation triggers its own multi-second strict
//! check storm).

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::discovery::TICKETS_SUBDIR;

/// Debounce window: fire after this much event quiet. The design bound is
/// ≥500 ms; 600 gives editor write-bursts comfortable room.
pub const DEBOUNCE_MS: u64 = 600;

/// One debounced fire. Reload always happens; `run_check` is false while the
/// suppression rules bite (verb in flight, or trailing window).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fire {
    pub run_check: bool,
}

/// Pure debounce + coalesce + suppression state machine over injected
/// milliseconds — no real clock, no threads, no sleeps.
///
/// Rules:
/// - A burst of raw events fires ONCE, [`DEBOUNCE_MS`] after the last event.
/// - `run_check` per fire: an event is check-worthy when it arrived while
///   unsuppressed AND past the trailing window; the fire runs the check when ANY
///   event in the burst was check-worthy (a real edit mixed into verb residue
///   must not lose its check) AND suppression is not active at fire time (a verb
///   in flight is an absolute no-check).
/// - Clearing suppression opens a trailing window of one debounce span: verb
///   write residue that lands just after the verb exits stays check-silent.
#[derive(Debug)]
pub struct Debouncer {
    window_ms: u64,
    /// Last raw-event time + whether any event in the pending burst is
    /// check-worthy.
    pending: Option<Pending>,
    suppressed: bool,
    /// After suppression clears, events before this instant are not check-worthy.
    trailing_until: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
struct Pending {
    last_event: u64,
    any_checkworthy: bool,
}

impl Debouncer {
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            pending: None,
            suppressed: false,
            trailing_until: None,
        }
    }

    /// A raw filesystem event at `now` (ms).
    pub fn on_event(&mut self, now: u64) {
        let checkworthy = !self.suppressed && self.trailing_until.is_none_or(|until| now >= until);
        let any = self.pending.is_some_and(|p| p.any_checkworthy) || checkworthy;
        self.pending = Some(Pending {
            last_event: now,
            any_checkworthy: any,
        });
    }

    /// Poll at `now`: fires exactly once per burst, after a full window of quiet.
    pub fn poll(&mut self, now: u64) -> Option<Fire> {
        let pending = self.pending?;
        if now.saturating_sub(pending.last_event) < self.window_ms {
            return None;
        }
        self.pending = None;
        Some(Fire {
            run_check: pending.any_checkworthy && !self.suppressed,
        })
    }

    /// Milliseconds until the pending fire is due (`None` when idle) — the app's
    /// `request_repaint_after` hint, so the fire happens without user input.
    pub fn due_in(&self, now: u64) -> Option<u64> {
        let pending = self.pending?;
        Some((pending.last_event + self.window_ms).saturating_sub(now))
    }

    /// The T-915.4 suppression hook: set while a verb subprocess is in flight.
    /// Clearing opens the one-window trailing period; re-setting cancels it.
    pub fn set_suppressed(&mut self, on: bool, now: u64) {
        if self.suppressed && !on {
            self.trailing_until = Some(now + self.window_ms);
        }
        if on {
            self.trailing_until = None;
        }
        self.suppressed = on;
    }
}

// ---- path relevance (pure) ----

/// `docs/`-level sync targets: `TICKET_*.md` and `MILESTONES.md`.
pub fn ticket_doc_name(name: &str) -> bool {
    (name.starts_with("TICKET_") && name.ends_with(".md")) || name == "MILESTONES.md"
}

/// Marker file watched under `docs/specs/Mission_Creator_Architecture/`.
pub const ROADMAP_REL: &str = "docs/specs/Mission_Creator_Architecture/ROADMAP.md";

/// Is `path` one of the watched surfaces under `root`? Everything under
/// `.ai/tickets/` counts (tickets, wave.lock, metrics); elsewhere only the named
/// sync targets do — the non-recursive dir watches deliver sibling noise
/// (Cargo.lock, target/, unrelated docs) that must not trigger reload storms.
pub fn relevant(root: &Path, path: &Path) -> bool {
    if path.starts_with(root.join(TICKETS_SUBDIR)) {
        return true;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if path.parent() == Some(root) {
        return name == "CLAUDE.md";
    }
    if path.parent() == Some(root.join("docs").as_path()) {
        return ticket_doc_name(name);
    }
    path == root.join(ROADMAP_REL)
}

// ---- notify shell (thin, untested — the state machine above carries the logic) ----

/// Keeps the watcher alive; dropping it stops the watch.
pub struct WatchHandle {
    _watcher: RecommendedWatcher,
    /// Best-effort targets that failed to arm (missing dir etc.) — surfaced
    /// subdued in the banner, never fatal.
    pub degraded: Vec<String>,
}

/// Arm the watches for `root`. Relevant events send `()` on `tx` and call
/// `on_event` (the app passes `request_repaint`); debouncing happens UI-side.
/// Only the `.ai/tickets/` watch is load-bearing — its failure is the `Err`.
pub fn spawn(
    root: &Path,
    tx: Sender<()>,
    on_event: impl Fn() + Send + 'static,
) -> Result<WatchHandle, String> {
    let filter_root = root.to_path_buf();
    let handler = move |result: Result<notify::Event, notify::Error>| {
        let Ok(event) = result else { return };
        // Access events are reads (our own corpus loads) — never a change.
        if matches!(event.kind, EventKind::Access(_)) {
            return;
        }
        if event.paths.iter().any(|p| relevant(&filter_root, p)) && tx.send(()).is_ok() {
            on_event();
        }
    };
    let mut watcher = notify::recommended_watcher(handler).map_err(|e| e.to_string())?;
    watcher
        .watch(&root.join(TICKETS_SUBDIR), RecursiveMode::Recursive)
        .map_err(|e| format!("{}: {e}", root.join(TICKETS_SUBDIR).display()))?;
    let mut degraded = Vec::new();
    let best_effort: [PathBuf; 3] = [
        // Non-recursive parent-dir watches for the file targets (rename-over safe).
        root.to_path_buf(),                                   // CLAUDE.md
        root.join("docs"),                                    // TICKET_*.md, MILESTONES.md
        root.join("docs/specs/Mission_Creator_Architecture"), // ROADMAP.md
    ];
    for dir in best_effort {
        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            degraded.push(format!("{}: {e}", dir.display()));
        }
    }
    Ok(WatchHandle {
        _watcher: watcher,
        degraded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: u64 = DEBOUNCE_MS;

    /// A burst of events fires exactly once, one window after the LAST event.
    #[test]
    fn burst_coalesces_to_one_fire() {
        let mut d = Debouncer::new(W);
        assert_eq!(d.poll(0), None, "idle never fires");
        d.on_event(0);
        d.on_event(100);
        d.on_event(300);
        assert_eq!(d.poll(300 + W - 1), None, "still inside the quiet window");
        assert_eq!(d.due_in(300), Some(W));
        assert_eq!(
            d.poll(300 + W),
            Some(Fire { run_check: true }),
            "fires one window after the last event"
        );
        assert_eq!(d.poll(300 + W + 1), None, "exactly one fire per burst");
        assert_eq!(d.due_in(300 + W + 1), None);
    }

    /// A new event during the quiet countdown restarts it (debounce, not delay).
    #[test]
    fn new_event_extends_the_quiet_window() {
        let mut d = Debouncer::new(W);
        d.on_event(0);
        assert_eq!(d.poll(W - 10), None);
        d.on_event(W - 5);
        assert_eq!(d.poll(W + 10), None, "the late event reset the window");
        assert_eq!(d.poll(W - 5 + W), Some(Fire { run_check: true }));
    }

    /// While suppressed: fires still happen (reload allowed) but never run the
    /// check.
    #[test]
    fn suppressed_fires_reload_only() {
        let mut d = Debouncer::new(W);
        d.set_suppressed(true, 0);
        d.on_event(10);
        assert_eq!(d.poll(10 + W), Some(Fire { run_check: false }));
    }

    /// The trailing window: events landing within one window of suppression
    /// clearing are verb-write residue — reload-only. Events after it are real.
    #[test]
    fn trailing_window_after_clear_stays_check_silent() {
        let mut d = Debouncer::new(W);
        d.set_suppressed(true, 0);
        d.set_suppressed(false, 1000);
        // Residue inside the trailing window (1000..1600): no check.
        d.on_event(1100);
        assert_eq!(d.poll(1100 + W), Some(Fire { run_check: false }));
        // A real edit after the trailing window: check runs again.
        d.on_event(1700);
        assert_eq!(d.poll(1700 + W), Some(Fire { run_check: true }));
    }

    /// A burst spanning the trailing edge: ONE fire, and the real (post-window)
    /// edit in it wins — the check runs.
    #[test]
    fn mixed_burst_across_the_trailing_edge_keeps_its_check() {
        let mut d = Debouncer::new(W);
        d.set_suppressed(true, 0);
        d.set_suppressed(false, 1000);
        d.on_event(1100); // residue (trailing until 1600)
        d.on_event(1650); // real edit — same burst
        let fire = d.poll(1650 + W);
        assert_eq!(fire, Some(Fire { run_check: true }));
        assert_eq!(d.poll(1650 + W + 1), None, "still exactly one fire");
    }

    /// Suppression turning ON between event and fire wins: a verb in flight is
    /// an absolute no-check, even for a pre-verb edit.
    #[test]
    fn suppression_at_fire_time_beats_a_checkworthy_event() {
        let mut d = Debouncer::new(W);
        d.on_event(0);
        d.set_suppressed(true, 100);
        assert_eq!(d.poll(W), Some(Fire { run_check: false }));
    }

    /// Re-setting suppression cancels a pending trailing window.
    #[test]
    fn resuppression_cancels_the_trailing_window() {
        let mut d = Debouncer::new(W);
        d.set_suppressed(true, 0);
        d.set_suppressed(false, 1000); // trailing until 1600
        d.set_suppressed(true, 1200);
        d.set_suppressed(false, 1300); // NEW trailing until 1900
        d.on_event(1700); // inside the new window — residue
        assert_eq!(d.poll(1700 + W), Some(Fire { run_check: false }));
        d.on_event(2000); // past it — real
        assert_eq!(d.poll(2000 + W), Some(Fire { run_check: true }));
    }

    #[test]
    fn ticket_doc_names() {
        assert!(ticket_doc_name("TICKET_LEAD.md"));
        assert!(ticket_doc_name("TICKET_REGISTRY.md"));
        assert!(ticket_doc_name("MILESTONES.md"));
        assert!(!ticket_doc_name("TICKET_LEAD.txt"));
        assert!(!ticket_doc_name("ROADMAP.md"));
        assert!(!ticket_doc_name("README.md"));
    }

    #[test]
    fn relevance_filter_matches_the_watched_surfaces_only() {
        let root = Path::new("/repo");
        // Everything under .ai/tickets/, recursively.
        assert!(relevant(root, Path::new("/repo/.ai/tickets/T-915.3.toml")));
        assert!(relevant(root, Path::new("/repo/.ai/tickets/wave.lock")));
        assert!(relevant(
            root,
            Path::new("/repo/.ai/tickets/metrics/T-1/x.json")
        ));
        // Root level: CLAUDE.md only — sibling noise must not fire.
        assert!(relevant(root, Path::new("/repo/CLAUDE.md")));
        assert!(!relevant(root, Path::new("/repo/Cargo.lock")));
        assert!(!relevant(root, Path::new("/repo/target")));
        // docs/ level: the sync targets only.
        assert!(relevant(root, Path::new("/repo/docs/TICKET_LEAD.md")));
        assert!(relevant(root, Path::new("/repo/docs/MILESTONES.md")));
        assert!(!relevant(root, Path::new("/repo/docs/README.md")));
        // Nested docs files never match through the docs/ rule…
        assert!(!relevant(
            root,
            Path::new("/repo/docs/platform/TICKET_X.md")
        ));
        // …except the one ROADMAP marker file.
        assert!(relevant(
            root,
            Path::new("/repo/docs/specs/Mission_Creator_Architecture/ROADMAP.md")
        ));
        assert!(!relevant(
            root,
            Path::new("/repo/docs/specs/Mission_Creator_Architecture/other.md")
        ));
        assert!(!relevant(
            Path::new("/elsewhere"),
            Path::new("/repo/CLAUDE.md")
        ));
    }
}
