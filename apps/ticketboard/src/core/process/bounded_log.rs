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
    pub fn new(cap: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(cap.min(64)),
            dropped: 0,
            cap,
        }
    }

    pub fn push(&mut self, line: String) {
        if self.lines.len() == self.cap {
            self.lines.pop_front();
            self.dropped += 1;
        }
        self.lines.push_back(line);
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.dropped = 0;
    }

    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn dropped(&self) -> usize {
        self.dropped
    }
}
