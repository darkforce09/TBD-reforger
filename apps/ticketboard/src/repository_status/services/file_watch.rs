//! File watching and a pure debounce model for registry and generated-document changes.
//!
//! A 600 ms quiet window coalesces filesystem bursts. Mutations suppress strict checks
//! while preserving reloads; a trailing quiet window absorbs the command's final writes.
//! The registry watch is required; missing supplementary targets report degradation.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use ticket_engine::repository::TICKETS_DIR;
use ticket_engine::repository::documentation::ROADMAP;

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

    /// The suppression hook: set while a verb subprocess is in flight.
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

/// The directory holding the roadmap, watched non-recursively so a rename-over is still seen.
fn roadmap_dir(root: &Path) -> PathBuf {
    root.join(ROADMAP)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf())
}

/// Is `path` one of the watched surfaces under `root`? Everything under the ticket registry
/// counts — ticket files, the wave lock, receipts — while elsewhere only the two named files do,
/// `CLAUDE.md` and the roadmap: the non-recursive directory watches deliver sibling noise
/// (Cargo.lock, build output, unrelated documents) that must not trigger reload storms.
pub fn relevant(root: &Path, path: &Path) -> bool {
    if path.starts_with(root.join(TICKETS_DIR)) {
        return true;
    }
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    if path.parent() == Some(root) {
        return name == "CLAUDE.md";
    }
    path == root.join(ROADMAP)
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
        .watch(&root.join(TICKETS_DIR), RecursiveMode::Recursive)
        .map_err(|e| format!("{}: {e}", root.join(TICKETS_DIR).display()))?;
    let mut degraded = Vec::new();
    let best_effort: [PathBuf; 2] = [
        // Non-recursive parent-dir watches for the file targets (rename-over safe).
        root.to_path_buf(), // CLAUDE.md
        roadmap_dir(root),
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
#[path = "tests/file_watch.rs"]
mod tests;
