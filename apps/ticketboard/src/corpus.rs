//! Typed corpus load (T-915.1 §Read architecture).
//!
//! Loads ALL `.ai/tickets/T-*.toml` — parents AND children — through
//! `ticket_engine::parse_ticket_toml`. Deliberately NOT `load_phase2_tree`, which is
//! parents-only; the app's whole point includes the children that projection hides.
//!
//! Fail-closed: the FIRST parse failure aborts the load and names the file with the
//! verbatim error — no partial board (the DidNotRun philosophy). IO runs on a worker
//! thread (`spawn_load`); no egui types in this module.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use ticket_engine::Ticket;

use crate::estimates::{self, RawEstimates};
use crate::facets::VocabTree;
use crate::metrics::{self, MetricsState};
use crate::wavelock::{self, LockState};
use ticket_engine::repository::TICKETS_DIR;

/// One parsed ticket plus the file it came from (the refusal / reveal surface).
#[derive(Debug)]
pub struct LoadedTicket {
    pub ticket: Ticket,
    pub path: PathBuf,
}

/// Footer acceptance surface: `total` equals `ls .ai/tickets/T-*.toml | wc -l` at
/// load time, and `parents + children == total` by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub total: usize,
    pub parents: usize,
    pub children: usize,
}

#[derive(Debug)]
pub struct Corpus {
    pub tickets: Vec<LoadedTicket>,
    pub counts: Counts,
}

/// Fail-closed load refusal: the offending file and the VERBATIM error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub file: PathBuf,
    pub error: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.error)
    }
}

pub type LoadResult = Result<Corpus, LoadError>;

/// Corpus + wave.lock + run receipts, loaded together on the worker thread
/// (T-915.2 / T-915.5). The lock rides alongside because its refusals are
/// Waves-view-local: a missing lock is a PLAN refusal (the DidNotRun text), not
/// a corpus refusal — the board must still render. The metrics state rides for
/// the same reason (its empty/error states are Metrics-tab-local), and because
/// `.ai/tickets/metrics/` sits inside the watched tree, so the same debounced
/// watch fires refresh receipts with corpus + lock.
pub struct LoadBundle {
    pub corpus: LoadResult,
    pub lock: LockState,
    pub metrics: MetricsState,
    /// Scope-vocab tree for the facet dropdowns (T-918.1) — DISPLAY-ONLY input;
    /// missing/broken file is `None` (facets fall back to corpus-present values),
    /// never a load refusal.
    pub vocab: Option<VocabTree>,
    /// Token-estimate files (T-918.2) — raw per-file parse results off
    /// `.ai/tickets/estimates/` (inside the watched tree, hence the shared
    /// load). The per-class/per-domain aggregation joins the corpus later, in
    /// `BoardState::new` (`estimates::build_state`). Estimated figures NEVER
    /// enter the metrics receipts model — structurally separate trees.
    pub estimates: RawEstimates,
}

/// Child = dotted id (`T-915.1`); parent = undotted (`T-915`).
pub fn is_child_id(id: &str) -> bool {
    id.contains('.')
}

/// The `T-*.toml` files under `dir`, sorted by name so the first failure is
/// deterministic. Mirrors the `T-*.toml` shell glob the acceptance line counts with
/// (`ROOT`, `schema.json`, `queue.json`, … never match).
fn ticket_files(dir: &Path) -> Result<Vec<PathBuf>, LoadError> {
    let err = |e: std::io::Error| LoadError {
        file: dir.to_path_buf(),
        error: e.to_string(),
    };
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(err)? {
        let path = entry.map_err(err)?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("T-") && name.ends_with(".toml") && path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Load every ticket under `repo_root/.ai/tickets/`. The first unreadable or
/// unparsable file aborts the whole load.
pub fn load_corpus(repo_root: &Path) -> LoadResult {
    let dir = repo_root.join(TICKETS_DIR);
    if !dir.is_dir() {
        return Err(LoadError {
            file: dir.clone(),
            error: format!("no {TICKETS_DIR}/ directory under {}", repo_root.display()),
        });
    }
    let files = ticket_files(&dir)?;
    let mut tickets = Vec::with_capacity(files.len());
    let mut parents = 0usize;
    let mut children = 0usize;
    for path in files {
        let text = fs::read_to_string(&path).map_err(|e| LoadError {
            file: path.clone(),
            error: e.to_string(),
        })?;
        let ticket = ticket_engine::parse_ticket_toml(&text).map_err(|error| LoadError {
            file: path.clone(),
            error,
        })?;
        if is_child_id(ticket.id()) {
            children += 1;
        } else {
            parents += 1;
        }
        tickets.push(LoadedTicket { ticket, path });
    }
    Ok(Corpus {
        counts: Counts {
            total: tickets.len(),
            parents,
            children,
        },
        tickets,
    })
}

/// Run `load_corpus` + the wave.lock load on a worker thread; the UI thread never
/// touches the disk. `on_done` fires after the result is sent (the app passes
/// `egui::Context::request_repaint`).
pub fn spawn_load(
    repo_root: PathBuf,
    on_done: impl FnOnce() + Send + 'static,
) -> mpsc::Receiver<LoadBundle> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let bundle = LoadBundle {
            corpus: load_corpus(&repo_root),
            lock: wavelock::load_lock(&repo_root),
            metrics: metrics::load_metrics(&repo_root),
            vocab: VocabTree::load(&repo_root),
            estimates: estimates::load_raw(&repo_root),
        };
        let _ = tx.send(bundle);
        on_done();
    });
    rx
}

#[cfg(test)]
#[path = "tests/corpus_tests.rs"]
mod tests;
