use super::{DocumentOutcome, LoadedDocument};
// ---- state machine ----

/// Viewer pane state. `path` is always the repo-relative string as clicked —
/// the display label and the stale-result identity in [`ViewerState::land`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerState {
    Closed,
    /// Worker read in flight.
    Loading {
        path: String,
    },
    /// Markdown rendering (egui_commonmark) of the read text.
    Rendered {
        path: String,
        text: String,
    },
    /// Raw monospace text plus the note naming why (read failure / non-UTF8 /
    /// oversize / escape) — the never-a-crash surface.
    Fallback {
        path: String,
        text: String,
        note: String,
    },
}

impl ViewerState {
    /// A spec/plan/citation click: enter `Loading` for `rel` (replacing
    /// whatever was open — clicking another doc restarts the machine).
    pub fn open(&mut self, rel: &str) {
        *self = ViewerState::Loading {
            path: rel.to_owned(),
        };
    }

    /// Back: the pane closes; board state (selection included) is not this
    /// machine's to touch.
    pub fn close(&mut self) {
        *self = ViewerState::Closed;
    }

    pub fn is_open(&self) -> bool {
        !matches!(self, ViewerState::Closed)
    }

    /// The repo-relative path this pane is about (`None` when closed).
    pub fn path(&self) -> Option<&str> {
        match self {
            ViewerState::Closed => None,
            ViewerState::Loading { path }
            | ViewerState::Rendered { path, .. }
            | ViewerState::Fallback { path, .. } => Some(path),
        }
    }

    /// Land a worker result. Applies ONLY while `Loading` the same path — a
    /// stale read (superseded click, or Back pressed mid-read) is dropped on
    /// the floor, never rendered.
    pub fn land(&mut self, doc: LoadedDocument) {
        let ViewerState::Loading { path } = self else {
            return;
        };
        if *path != doc.rel {
            return;
        }
        let path = std::mem::take(path);
        *self = match doc.outcome {
            DocumentOutcome::Rendered { text } => ViewerState::Rendered { path, text },
            DocumentOutcome::Fallback { text, note } => ViewerState::Fallback { path, text, note },
        };
    }
}
