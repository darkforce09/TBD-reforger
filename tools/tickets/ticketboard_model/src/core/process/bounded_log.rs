//! The bounded copy of a subprocess's output.
//!
//! **Role:** `BoundedLog`, a ring of at most `LOG_CAP` lines that counts what it dropped.
//! **Position:** part of `crate::core::process`; held by the strict-check and command models.
//! **Signals & state:** the log's line buffer, owned by its holder.
//! **Invariants:** memory stays bounded however much a child prints; the newest lines are kept.

use super::*;
/// Retained-output bound for the UI ring buffer (the stream is unbounded but
/// drained per frame; only the last ~500 lines are kept for the verbatim pane).
pub const LOG_CAP: usize = 500;

// ---- retained output ----

/// Bounded verbatim-output buffer: keeps the LAST `cap` lines and counts what was
/// dropped, so the pane can say "… N earlier lines dropped" instead of lying by
/// omission.
pub struct BoundedLog {
    lines: VecDeque<String>,
    dropped: usize,
    cap: usize,
}

impl BoundedLog {
    /// An empty log keeping at most `cap` lines.
    pub fn new(cap: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(cap.min(64)),
            dropped: 0,
            cap,
        }
    }

    /// Appends a line, dropping the oldest when the log is full.
    pub fn push(&mut self, line: String) {
        if self.lines.len() == self.cap {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(line);
    }

    /// Empties the log and resets the dropped count.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
    }

    /// The kept lines, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    /// How many lines the log keeps now.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// True when the log keeps no line.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// How many earlier lines were dropped to stay within the cap.
    pub fn dropped(&self) -> usize {
        self.dropped
    }
}
