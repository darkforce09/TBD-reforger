//! eframe shell (T-915.1 / T-915.2 / T-915.3 / T-915.4) — thin UI over the pure
//! modules.
//!
//! States: NoRepo refusal (both discovery mechanisms + native folder picker),
//! Loading, parse Refusal (the trust surface: file path + verbatim error), Board.
//! Board carries three tabs — Board / Waves / Tree — one shared detail panel, and a
//! composable filter bar (filters hide board cards and tree rows, and dim wave
//! chips). Above everything sits the trust banner (T-915.3): the streamed result
//! of `cargo xtask ticket check --strict` plus the git-dirty chip, refreshed by
//! the `.ai/tickets` file watch (debounced, coalesced), which also auto-reloads
//! the corpus in place. T-915.4 adds the mutation surface: card context menus and
//! a detail-panel action strip whose every write shells `cargo xtask ticket
//! <verb>` through a single-flight queue (CAS-guarded, streamed into a bottom
//! drawer, watch-suppressed while in flight) — the app itself never writes ticket
//! files. All IO happens on worker threads (`std::thread` + `mpsc`,
//! `request_repaint` on completion); the paint path only reads strings precomputed
//! at load or filter-change time.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{
    self, Align, Align2, Button, Color32, ComboBox, DragAndDrop, FontId, Id, Layout, Panel, Rect,
    RichText, ScrollArea, Sense, Spinner, StrokeKind, TextEdit, Ui, pos2, vec2,
};
use egui_extras::{Column as TableColumn, TableBuilder};
use tbd_tickets::StatusName;

use crate::board::{self, BoardModel, Card, Class, ScopeLevel};
use crate::corpus::{self, Corpus, LoadBundle, LoadError};
use crate::discovery;
use crate::facets::{self, FacetOption, FacetOptions, VocabTree};
use crate::filters::{FilterIndex, Filters, KindFilter};
use crate::gitstatus::{self, GitChip};
use crate::metrics::{self, MetricsState, SortPair, TableKind};
use crate::mutate::{self, Dialog, DragCard, MutCtx, Toast, VerbOutcome, VerbRunner};
use crate::subproc::{self, LogRing, ProcEvent, ProcHandle};
use crate::tree::{self, TreeModel};
use crate::trust::{self, CheckModel, Coalescer, Tone};
use crate::verbs;
use crate::watch::{self, Debouncer};
use crate::wavelock::{self, LockState};
use crate::waves::{Lane, Wave0, WaveChip, WavesModel};

/// eframe Storage key for the picked repo root (user config dir — never the repo).
const REPO_ROOT_KEY: &str = "repo_root";

/// 52 through T-915; +12 for the T-918.1 scope-breadcrumb row.
const CARD_H: f32 = 64.0;
const CARD_GAP: f32 = 6.0;
const COL_W: f32 = 236.0;
const CHIP_COL_W: f32 = 92.0;
const DETAIL_W: f32 = 420.0;
const TREE_ROW_H: f32 = 18.0;
const TREE_INDENT: f32 = 16.0;
const WAVE0_ROW_H: f32 = 18.0;
const WAVE0_LIST_MAX_H: f32 = 320.0;
const OUTPUT_ROW_H: f32 = 15.0;
const OUTPUT_MAX_H: f32 = 260.0;
const GIT_LIST_MAX_H: f32 = 160.0;

/// Collision verdict / `ready`-family accent colors (dark-theme legible).
/// pub(crate): the T-915.4 mutation UI reuses the same green/red pair.
pub(crate) const VERDICT_OK: Color32 = Color32::from_rgb(120, 205, 130);
pub(crate) const VERDICT_COLLIDE: Color32 = Color32::from_rgb(235, 110, 100);

/// Status accent for tree rows, wave chips, and the status filter toggles. The RAW
/// status name stays the label everywhere — color is an accent, never a rename.
fn status_color(status: StatusName) -> Color32 {
    match status {
        StatusName::Idea => Color32::from_gray(150),
        StatusName::Queued => Color32::from_rgb(120, 165, 225),
        StatusName::Ready => VERDICT_OK,
        StatusName::Running => Color32::from_rgb(245, 175, 80),
        StatusName::Review => Color32::from_rgb(195, 150, 235),
        StatusName::Shipped => Color32::from_rgb(105, 150, 115),
        StatusName::Deferred => Color32::from_rgb(180, 150, 110),
        StatusName::Cancelled => Color32::from_rgb(215, 115, 105),
    }
}

/// Class chip accent — the palette lives in `board::Class::accent_rgb` (pure,
/// test-pinned, total over the closed class set); this only lifts it to Color32.
fn class_color(class: Class) -> Color32 {
    let (r, g, b) = class.accent_rgb();
    Color32::from_rgb(r, g, b)
}

/// Muted per-level breadcrumb accents (T-918.1) — desaturated hues so the scope
/// path reads as ONE quiet chip trail, distinct from the loud status accents.
fn scope_level_color(level: ScopeLevel) -> Color32 {
    match level {
        ScopeLevel::Domain => Color32::from_rgb(170, 190, 215),
        ScopeLevel::Layer => Color32::from_rgb(160, 195, 175),
        ScopeLevel::Component => Color32::from_rgb(205, 185, 150),
        ScopeLevel::Surface => Color32::from_rgb(185, 165, 205),
    }
}

/// Amber accent for the estimated-scope `~` glyph (the trust banner's "busy"
/// tone — provenance flags read as attention, not error).
const SCOPE_ESTIMATED_COLOR: Color32 = Color32::from_rgb(245, 175, 80);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Tab {
    #[default]
    Board,
    Waves,
    Tree,
    Metrics,
}

const TABS: [(Tab, &str); 4] = [
    (Tab::Board, "Board"),
    (Tab::Waves, "Waves"),
    (Tab::Tree, "Tree"),
    (Tab::Metrics, "Metrics"),
];

enum State {
    /// No repo resolved (or an invalid one named in `note`) — full-window refusal.
    NoRepo {
        note: Option<String>,
    },
    Loading,
    /// Fail-closed corpus refusal — full-window, file + verbatim error.
    Refused(LoadError),
    Board(Box<BoardState>),
}

pub(crate) struct BoardState {
    /// pub(crate): the T-915.4 mutation UI reads tickets + paths for dialogs.
    pub(crate) corpus: Corpus,
    pub(crate) board: BoardModel,
    /// wave.lock outcome — Waves-view-local; refusals never blank the board.
    lock: LockState,
    /// Built only when the lock loaded (rendered verbatim, never recomputed).
    waves: Option<WavesModel>,
    /// Run-receipt dashboard (T-915.5) — Metrics-tab-local; NoReceipts is the
    /// explicit empty state, never zeros.
    metrics: MetricsState,
    /// Per-table sort selections for the metrics tables (survive reloads,
    /// carried like the filters).
    metrics_sort: SortPair,
    tree: TreeModel,
    filter_index: FilterIndex,
    filters: Filters,
    /// Scope-vocab tree (T-918.1) — DISPLAY-ONLY facet-value source; `None`
    /// (missing/broken file) falls back to corpus-present values.
    vocab: Option<VocabTree>,
    /// Narrowed facet dropdown options — recomputed in `refilter`, never per
    /// frame.
    facet_options: FacetOptions,
    /// Per-corpus-index filter verdicts (all true when no filter is active).
    matches: Vec<bool>,
    matched_count: usize,
    /// Board virtualization under filters: per column, the visible card rows.
    visible: [Vec<usize>; 8],
    /// Flattened tree rows for the virtualized tree view.
    tree_flat: Vec<tree::FlatRow>,
    /// Manual tree expansion by corpus index (filters force-expand match paths).
    tree_expanded: Vec<bool>,
    /// Wave 0 id list visibility — ALWAYS collapsed on load (acceptance 2).
    wave0_expanded: bool,
    selected: Option<usize>,
    /// Second selection (shift-click) — the owns-collision explainer pair.
    compare: Option<usize>,
    expanded: [bool; 8],
    /// Precomputed footer base: the acceptance surface against
    /// `ls .ai/tickets/T-*.toml | wc -l`.
    footer_base: String,
    /// Rendered footer — prefixed with `matched/total` while filters are active.
    footer: String,
}

impl BoardState {
    fn new(
        corpus: Corpus,
        lock: LockState,
        mut metrics: MetricsState,
        vocab: Option<VocabTree>,
        filters: Filters,
        metrics_sort: SortPair,
    ) -> Self {
        let board = BoardModel::build(&corpus);
        let waves = match &lock {
            LockState::Loaded(l) => Some(WavesModel::build(&corpus, &board.id_to_index, l)),
            _ => None,
        };
        // Re-apply the carried sort to the fresh aggregation (the model loads
        // tokens-desc by default).
        if let MetricsState::Loaded(m) = &mut metrics {
            m.apply_sort(metrics_sort);
        }
        let tree = TreeModel::build(&corpus, &board.id_to_index);
        let filter_index = FilterIndex::build(&corpus);
        let expanded = board::STATUS_ORDER.map(|s| !board::collapsed_by_default(s));
        let c = corpus.counts;
        let footer_base = format!(
            "{} ticket files — {} parents / {} children",
            c.total, c.parents, c.children
        );
        let total = corpus.tickets.len();
        let mut state = Self {
            corpus,
            board,
            lock,
            waves,
            metrics,
            metrics_sort,
            tree,
            filter_index,
            filters,
            vocab,
            facet_options: FacetOptions::default(),
            matches: vec![true; total],
            matched_count: total,
            visible: Default::default(),
            tree_flat: Vec::new(),
            tree_expanded: vec![false; total],
            wave0_expanded: false,
            selected: None,
            compare: None,
            expanded,
            footer: footer_base.clone(),
            footer_base,
        };
        state.refilter();
        state
    }

    /// Recompute every filter-derived surface: facet options (narrowed, stale
    /// lower selections cleared), verdicts, board rows, tree rows, footer. Runs
    /// on filter change only — never per frame.
    fn refilter(&mut self) {
        self.facet_options = facets::compute(
            self.vocab.as_ref(),
            &self.filter_index.rows,
            &mut self.filters.scope,
        );
        let (matches, matched_count) = self.filters.apply(&self.filter_index);
        self.matches = matches;
        self.matched_count = matched_count;
        for (col, column) in self.board.columns.iter().enumerate() {
            self.visible[col] = column
                .cards
                .iter()
                .enumerate()
                .filter(|(_, card)| self.matches[card.index])
                .map(|(row, _)| row)
                .collect();
        }
        self.reflatten();
        self.footer = if self.filters.is_active() {
            format!(
                "{}/{} tickets match · {}",
                self.matched_count, self.corpus.counts.total, self.footer_base
            )
        } else {
            self.footer_base.clone()
        };
    }

    /// Recompute the flattened tree rows (expansion toggle or filter change).
    fn reflatten(&mut self) {
        let filter = self.filters.is_active().then_some(self.matches.as_slice());
        self.tree_flat = tree::flatten(&self.tree, &self.tree_expanded, filter);
    }
}

/// UI events, collected during paint and applied afterwards.
pub(crate) enum Action {
    Reload,
    /// Manual trust-banner re-run (coalesced while a check is in flight).
    Recheck,
    /// Kill the in-flight strict check.
    CancelCheck,
    ToggleOutput,
    ToggleGitList,
    PickFolder,
    Select(usize),
    SelectId(String),
    /// Shift-click: pick the second ticket of the owns-collision pair.
    Compare(usize),
    ClearCompare,
    ToggleColumn(usize),
    OpenPath(PathBuf),
    CloseDetail,
    SetTab(Tab),
    ToggleNode(usize),
    ToggleWave0,
    /// Copy a lane's `n<TAB>id` lines to the clipboard (acceptance 1).
    CopyTsv(String),
    FiltersChanged,
    /// Metrics table header click: toggle/replace that table's sort (T-915.5).
    SortMetrics(TableKind, metrics::SortKey),
    // ---- T-915.4 mutation surface ----
    /// Open a mutation dialog (built at click/menu-render time, guard included).
    OpenDialog(Box<Dialog>),
    /// Dispatch a verb: CAS-check, then feed the single-flight queue. Closes the
    /// dialog.
    Dispatch(verbs::VerbRequest),
    ToggleVerbDrawer,
}

/// Plain click selects; shift-click picks the comparison ticket.
fn select_or_compare(ui: &Ui, index: usize) -> Action {
    if ui.input(|i| i.modifiers.shift) {
        Action::Compare(index)
    } else {
        Action::Select(index)
    }
}

pub struct TicketboardApp {
    repo_root: Option<PathBuf>,
    state: State,
    tab: Tab,
    load_rx: Option<Receiver<LoadBundle>>,
    pick_rx: Option<Receiver<Option<PathBuf>>>,
    // ---- T-915.3: trust banner + watch ----
    /// Resolved once at startup ($CARGO → PATH → ~/.cargo/bin — GUI PATH is bare).
    cargo: PathBuf,
    check: CheckModel,
    check_log: LogRing,
    check_handle: Option<ProcHandle>,
    /// Verbatim-output pane toggle (auto-opens when a check lands red).
    show_output: bool,
    git_chip: GitChip,
    git_lines: Vec<String>,
    git_handle: Option<ProcHandle>,
    git_flight: Coalescer,
    git_expanded: bool,
    watch: Option<watch::WatchHandle>,
    watch_rx: Option<Receiver<()>>,
    /// The load-bearing `.ai/tickets` watch failed to arm — banner note.
    watch_error: Option<String>,
    debounce: Debouncer,
    /// Millisecond clock base for the debouncer (monotonic).
    epoch: Instant,
    // ---- T-915.4: mutation UI over subprocess xtask verbs ----
    /// Single-flight verb queue + in-flight subprocess + verbatim log + drawer.
    verb: VerbRunner,
    /// The one open mutation dialog (confirm / form), CAS guard inside.
    dialog: Option<Dialog>,
    toasts: Vec<Toast>,
    /// Advanced raw set-status dropdown selection (detail panel).
    advanced_status: Option<StatusName>,
}

impl TicketboardApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        arg: Option<PathBuf>,
        cwd: Option<PathBuf>,
    ) -> Self {
        let mut app = Self {
            repo_root: None,
            state: State::NoRepo { note: None },
            tab: Tab::default(),
            load_rx: None,
            pick_rx: None,
            cargo: subproc::resolve_cargo(),
            check: CheckModel::default(),
            check_log: LogRing::new(subproc::LOG_CAP),
            check_handle: None,
            show_output: false,
            git_chip: GitChip::default(),
            git_lines: Vec::new(),
            git_handle: None,
            git_flight: Coalescer::default(),
            git_expanded: false,
            watch: None,
            watch_rx: None,
            watch_error: None,
            debounce: Debouncer::new(watch::DEBOUNCE_MS),
            epoch: Instant::now(),
            verb: VerbRunner::new(),
            dialog: None,
            toasts: Vec::new(),
            advanced_status: None,
        };
        match discovery::resolve_repo_root(arg, cwd.as_deref()) {
            Some(root) if discovery::has_tickets_dir(&root) => {
                app.adopt_root(root, &cc.egui_ctx);
            }
            Some(root) => {
                app.state = State::NoRepo {
                    note: Some(format!(
                        "{} has no {}/ directory",
                        root.display(),
                        discovery::TICKETS_SUBDIR
                    )),
                };
            }
            None => {
                // Fall back to the persisted picker choice — revalidated on load.
                let saved = cc
                    .storage
                    .and_then(|s| s.get_string(REPO_ROOT_KEY))
                    .map(PathBuf::from);
                match saved {
                    Some(root) if discovery::has_tickets_dir(&root) => {
                        app.adopt_root(root, &cc.egui_ctx);
                    }
                    Some(root) => {
                        app.state = State::NoRepo {
                            note: Some(format!(
                                "saved path {} no longer contains {}/",
                                root.display(),
                                discovery::TICKETS_SUBDIR
                            )),
                        };
                    }
                    None => {}
                }
            }
        }
        app
    }

    /// A validated repo root becomes active: arm the watch, load the corpus, and
    /// run the launch strict check (T-915.3 acceptance 1).
    fn adopt_root(&mut self, root: PathBuf, ctx: &egui::Context) {
        self.repo_root = Some(root);
        self.arm_watch(ctx);
        self.start_load(ctx, false);
        self.trigger_check(ctx);
    }

    /// Monotonic milliseconds since app start — the debouncer's clock.
    fn now_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// T-915.4 hook: the verb runner sets this while a mutation subprocess is in
    /// flight. Watch fires keep reloading, but check re-runs stay suppressed for
    /// the flag's duration plus one debounce window after clear — the app's own
    /// writes must not trigger a strict-check storm.
    pub fn set_verb_in_flight(&mut self, in_flight: bool) {
        let now = self.now_ms();
        self.debounce.set_suppressed(in_flight, now);
    }

    /// Kick the corpus + lock load on a worker thread. `in_place` (watch-triggered)
    /// keeps the current surface up — the Board keeps rendering, a Refusal keeps
    /// naming the file — and swaps on arrival, so an external edit lands without a
    /// Loading flash and a fixed-on-disk file auto-recovers the board.
    fn start_load(&mut self, ctx: &egui::Context, in_place: bool) {
        if let Some(root) = self.repo_root.clone() {
            let repaint_ctx = ctx.clone();
            self.load_rx = Some(corpus::spawn_load(root, move || {
                repaint_ctx.request_repaint()
            }));
            if !in_place {
                self.state = State::Loading;
            }
        }
    }

    // ---- strict check (trust banner) ----

    /// Request a strict-check run. Single-flight: while one is in flight the
    /// trigger sets the dirty flag and the run exit starts exactly one follow-up.
    fn trigger_check(&mut self, ctx: &egui::Context) {
        if self.repo_root.is_none() {
            return;
        }
        if self.check.coalescer.trigger() {
            self.start_check(ctx);
        }
    }

    /// Spawn `cargo xtask ticket check --strict` (alias-expanded argv) at the repo
    /// root — the app never re-implements check, it invokes it.
    fn start_check(&mut self, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.check.on_start();
        self.check_log.clear();
        let repaint_ctx = ctx.clone();
        self.check_handle = Some(subproc::spawn_streaming(
            &self.cargo,
            &trust::CHECK_ARGS,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    // ---- git-dirty chip ----

    /// Refresh the git chip (runs after every reload and every check exit).
    /// Same single-flight coalescing as the check.
    fn trigger_git(&mut self, ctx: &egui::Context) {
        if self.repo_root.is_none() {
            return;
        }
        if self.git_flight.trigger() {
            self.start_git(ctx);
        }
    }

    fn start_git(&mut self, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        self.git_lines.clear();
        let repaint_ctx = ctx.clone();
        self.git_handle = Some(subproc::spawn_streaming(
            "git",
            &gitstatus::GIT_ARGS,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    // ---- file watch ----

    /// Arm the notify watches for the active root. Only the `.ai/tickets` watch is
    /// load-bearing; its failure is surfaced in the banner, never a crash.
    fn arm_watch(&mut self, ctx: &egui::Context) {
        self.watch = None;
        self.watch_rx = None;
        self.watch_error = None;
        let Some(root) = self.repo_root.clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        match watch::spawn(&root, tx, move || repaint_ctx.request_repaint()) {
            Ok(handle) => {
                self.watch = Some(handle);
                self.watch_rx = Some(rx);
            }
            Err(e) => self.watch_error = Some(e),
        }
    }

    /// Native folder picker on a worker thread (rfd blocks the calling thread).
    fn start_pick(&mut self, ctx: &egui::Context) {
        if self.pick_rx.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let repaint_ctx = ctx.clone();
        std::thread::spawn(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Pick a repo root containing .ai/tickets/")
                .pick_folder();
            let _ = tx.send(picked);
            repaint_ctx.request_repaint();
        });
        self.pick_rx = Some(rx);
    }

    /// Drain worker results (non-blocking) and advance the state machine.
    fn poll(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.load_rx
            && let Ok(bundle) = rx.try_recv()
        {
            self.load_rx = None;
            self.state = match bundle.corpus {
                Ok(corpus) => {
                    // Filters, the selection AND the metrics sort survive a
                    // reload (T-915.3 reloads on every watched change) — the
                    // selection re-resolves by id, because raw indices are
                    // stale against the new corpus.
                    let (filters, selected_id, compare_id, metrics_sort) = match &self.state {
                        State::Board(b) => (
                            b.filters.clone(),
                            b.selected
                                .map(|i| b.corpus.tickets[i].ticket.id().to_owned()),
                            b.compare
                                .map(|i| b.corpus.tickets[i].ticket.id().to_owned()),
                            b.metrics_sort,
                        ),
                        _ => (Filters::default(), None, None, SortPair::default()),
                    };
                    let mut board = BoardState::new(
                        corpus,
                        bundle.lock,
                        bundle.metrics,
                        bundle.vocab,
                        filters,
                        metrics_sort,
                    );
                    board.selected =
                        selected_id.and_then(|id| board.board.id_to_index.get(&id).copied());
                    board.compare =
                        compare_id.and_then(|id| board.board.id_to_index.get(&id).copied());
                    State::Board(Box::new(board))
                }
                Err(e) => State::Refused(e),
            };
            // Registry files may have changed under git too — refresh the chip.
            self.trigger_git(ctx);
        }
        if let Some(rx) = &self.pick_rx
            && let Ok(picked) = rx.try_recv()
        {
            self.pick_rx = None;
            if let Some(root) = picked {
                if discovery::has_tickets_dir(&root) {
                    self.adopt_root(root, ctx);
                } else {
                    self.state = State::NoRepo {
                        note: Some(format!(
                            "picked folder {} has no {}/ directory",
                            root.display(),
                            discovery::TICKETS_SUBDIR
                        )),
                    };
                }
            }
        }
        self.poll_check(ctx);
        self.poll_git(ctx);
        self.poll_watch(ctx);
        self.poll_verb(ctx);
    }

    // ---- verb runner (T-915.4) ----

    fn toast(&mut self, text: String, error: bool) {
        self.toasts.push(Toast::new(text, error));
    }

    /// Dispatch a verb request from the UI: re-hash the CAS guard NOW; a
    /// mismatch refuses the dispatch (no subprocess) and reloads. Otherwise the
    /// request enters the single-flight queue — spawned immediately when idle,
    /// parked FIFO when a verb is already running.
    fn dispatch_verb(&mut self, req: verbs::VerbRequest, ctx: &egui::Context) {
        if !verbs::cas_ok(req.guard.as_ref()) {
            self.toast(
                format!("{} — file changed on disk — reloading", req.display),
                true,
            );
            self.start_load(ctx, true);
            return;
        }
        match self.verb.queue.submit(req) {
            Some(start) => self.spawn_verb(start, ctx),
            None => {
                let pending = self.verb.queue.pending_len();
                self.toast(
                    format!("queued behind the running verb ({pending} pending)"),
                    false,
                );
            }
        }
    }

    /// Spawn `cargo <alias-expanded verb argv>` at the repo root through the
    /// T-915.3 subproc helper. Sets the watch-suppression flag and opens the
    /// drawer so the streamed log is visible while the verb runs.
    fn spawn_verb(&mut self, req: verbs::VerbRequest, ctx: &egui::Context) {
        let Some(root) = self.repo_root.clone() else {
            // No repo root — the queue slot must not stay claimed forever.
            let _ = self.verb.queue.finish(false);
            return;
        };
        self.verb.log.clear();
        self.verb.last = None;
        self.verb.dropped_note = None;
        self.verb.drawer_open = true;
        self.set_verb_in_flight(true);
        let args: Vec<&str> = req.args.iter().map(String::as_str).collect();
        let repaint_ctx = ctx.clone();
        self.verb.handle = Some(subproc::spawn_streaming(
            &self.cargo,
            &args,
            &root,
            move || repaint_ctx.request_repaint(),
        ));
    }

    /// Drain the in-flight verb stream. On exit — success or refusal — ALWAYS
    /// reload corpus+lock and refresh the git chip. Success shows the stdout
    /// tail as a toast; a nonzero exit keeps the drawer open with the FULL
    /// verbatim output + exit code, drops the pending queue (never auto-retry),
    /// triggers ONE strict re-check (the T-915.3 banner goes red on wave-stale
    /// state), and — when the log carries the wave-stale signature — shows the
    /// recovery command as text. The app never runs `wave repack` itself.
    fn poll_verb(&mut self, ctx: &egui::Context) {
        let mut term: Option<(Option<i32>, Option<String>)> = None;
        if let Some(handle) = &self.verb.handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcEvent::Line(line) => self.verb.log.push(line),
                    ProcEvent::Exited { code } => term = Some((code, None)),
                    ProcEvent::SpawnFailed(error) => term = Some((None, Some(error))),
                }
            }
        }
        let Some((code, spawn_error)) = term else {
            return;
        };
        self.verb.handle = None;
        let display = self
            .verb
            .queue
            .running_display()
            .unwrap_or("verb")
            .to_owned();
        let success = code == Some(0) && spawn_error.is_none();
        // Signal-killed (the mid-verb SIGKILL between save and repack) counts as
        // the wave-stale hazard even though the dead process printed nothing.
        let killed = code.is_none() && spawn_error.is_none();
        let hint = !success
            && verbs::recovery_hint_applies(killed, self.verb.log.iter().map(String::as_str));
        self.verb.last = Some(VerbOutcome {
            display: display.clone(),
            code,
            at: trust::utc_hms(epoch_secs()),
            spawn_error,
            hint,
        });
        // ALWAYS — even (especially) after a refusal: the verb may have exited
        // between save and sync/repack, and the board must show the disk truth.
        self.start_load(ctx, true);
        self.trigger_git(ctx);
        if success {
            let tail = verbs::success_tail(self.verb.log.iter().map(String::as_str))
                .unwrap_or_else(|| format!("exit 0 — {display}"));
            self.toast(tail, false);
            self.verb.drawer_open = false;
        } else {
            self.verb.drawer_open = true;
            // One deliberate strict re-check (coalesced) — the T-915.3 banner is
            // how mid-verb-crash wave-stale state surfaces as check-red.
            self.trigger_check(ctx);
        }
        let mut fin = self.verb.queue.finish(success);
        if fin.dropped > 0 {
            self.verb.dropped_note = Some(format!(
                "{} pending verb request(s) dropped after this failure — nothing auto-retries",
                fin.dropped
            ));
        }
        while let Some(next) = fin.next {
            if verbs::cas_ok(next.guard.as_ref()) {
                self.spawn_verb(next, ctx);
                break;
            }
            self.toast(
                format!("{} — file changed on disk — reloading", next.display),
                true,
            );
            fin = self.verb.queue.finish(true);
        }
        if !self.verb.queue.busy() {
            self.set_verb_in_flight(false);
        }
    }

    /// Drain the strict-check stream: lines advance the building→checking phase
    /// split and the ERROR count; the exit resolves green/red and may start the
    /// single coalesced follow-up run.
    fn poll_check(&mut self, ctx: &egui::Context) {
        let mut finished = false;
        if let Some(handle) = &self.check_handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcEvent::Line(line) => {
                        self.check.on_line(&line);
                        self.check_log.push(line);
                    }
                    ProcEvent::Exited { code } => {
                        self.check.on_exit(code, trust::utc_hms(epoch_secs()));
                        if code != Some(0) {
                            // Red auto-opens the verbatim pane — the errors are
                            // the point, not a number.
                            self.show_output = true;
                        }
                        finished = true;
                    }
                    ProcEvent::SpawnFailed(error) => {
                        self.check
                            .on_spawn_failed(error, trust::utc_hms(epoch_secs()));
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.check_handle = None;
            self.trigger_git(ctx);
            if self.check.coalescer.finished() {
                self.start_check(ctx);
            }
        }
    }

    /// Drain the git-status stream into the chip.
    fn poll_git(&mut self, ctx: &egui::Context) {
        let mut finished = false;
        if let Some(handle) = &self.git_handle {
            while let Ok(event) = handle.rx.try_recv() {
                match event {
                    ProcEvent::Line(line) => self.git_lines.push(line),
                    ProcEvent::Exited { code } => {
                        self.git_chip = gitstatus::chip_from_exit(
                            code,
                            self.git_lines.iter().map(String::as_str),
                        );
                        finished = true;
                    }
                    ProcEvent::SpawnFailed(error) => {
                        // Git absent — the chip says so; never a crash.
                        self.git_chip = GitChip::Unavailable(error);
                        finished = true;
                    }
                }
            }
        }
        if finished {
            self.git_lines.clear();
            self.git_handle = None;
            if self.git_flight.finished() {
                self.start_git(ctx);
            }
        }
    }

    /// Feed raw watch events into the debouncer; a fire reloads corpus+lock in
    /// place and (unless suppressed) re-runs the check through the coalescer.
    fn poll_watch(&mut self, ctx: &egui::Context) {
        let now = self.now_ms();
        if let Some(rx) = &self.watch_rx {
            while rx.try_recv().is_ok() {
                self.debounce.on_event(now);
            }
        }
        if let Some(fire) = self.debounce.poll(now) {
            self.start_load(ctx, true);
            if fire.run_check {
                self.trigger_check(ctx);
            }
        }
        // The fire needs a frame after the quiet window even when the user is
        // idle — schedule the wakeup.
        if let Some(due) = self.debounce.due_in(now) {
            ctx.request_repaint_after(Duration::from_millis(due + 5));
        }
    }

    fn apply(&mut self, actions: Vec<Action>, ctx: &egui::Context) {
        for action in actions {
            match action {
                Action::Reload => self.start_load(ctx, false),
                Action::Recheck => self.trigger_check(ctx),
                Action::CancelCheck => {
                    if let Some(handle) = &self.check_handle {
                        handle.kill();
                    }
                }
                Action::ToggleOutput => self.show_output = !self.show_output,
                Action::ToggleGitList => self.git_expanded = !self.git_expanded,
                Action::PickFolder => self.start_pick(ctx),
                Action::OpenPath(path) => open_path(&path),
                Action::SetTab(tab) => self.tab = tab,
                Action::CopyTsv(tsv) => ctx.copy_text(tsv),
                Action::Select(index) => {
                    if let State::Board(b) = &mut self.state {
                        b.selected = Some(index);
                        if b.compare == Some(index) {
                            b.compare = None;
                        }
                    }
                }
                Action::SelectId(id) => {
                    if let State::Board(b) = &mut self.state
                        && let Some(&index) = b.board.id_to_index.get(&id)
                    {
                        b.selected = Some(index);
                        if b.compare == Some(index) {
                            b.compare = None;
                        }
                    }
                }
                Action::Compare(index) => {
                    if let State::Board(b) = &mut self.state {
                        match b.selected {
                            // Nothing selected yet: shift-click behaves like select.
                            None => b.selected = Some(index),
                            Some(sel) if sel == index => {}
                            Some(_) => b.compare = Some(index),
                        }
                    }
                }
                Action::ClearCompare => {
                    if let State::Board(b) = &mut self.state {
                        b.compare = None;
                    }
                }
                Action::ToggleColumn(col) => {
                    if let State::Board(b) = &mut self.state {
                        b.expanded[col] = !b.expanded[col];
                    }
                }
                Action::ToggleNode(index) => {
                    if let State::Board(b) = &mut self.state {
                        b.tree_expanded[index] = !b.tree_expanded[index];
                        b.reflatten();
                    }
                }
                Action::ToggleWave0 => {
                    if let State::Board(b) = &mut self.state {
                        b.wave0_expanded = !b.wave0_expanded;
                    }
                }
                Action::FiltersChanged => {
                    if let State::Board(b) = &mut self.state {
                        b.refilter();
                    }
                }
                Action::SortMetrics(table, key) => {
                    // Re-sort ONCE on click — the paint path never sorts.
                    if let State::Board(b) = &mut self.state
                        && let MetricsState::Loaded(m) = &mut b.metrics
                    {
                        match table {
                            TableKind::Ticket => {
                                b.metrics_sort.ticket = b.metrics_sort.ticket.toggled(key);
                                metrics::sort_rows(&mut m.per_ticket, b.metrics_sort.ticket);
                            }
                            TableKind::Agent => {
                                b.metrics_sort.agent = b.metrics_sort.agent.toggled(key);
                                metrics::sort_rows(&mut m.per_agent, b.metrics_sort.agent);
                            }
                        }
                    }
                }
                Action::CloseDetail => {
                    if let State::Board(b) = &mut self.state {
                        b.selected = None;
                        b.compare = None;
                    }
                }
                Action::OpenDialog(dialog) => self.dialog = Some(*dialog),
                Action::Dispatch(req) => {
                    self.dialog = None;
                    self.dispatch_verb(req, ctx);
                }
                Action::ToggleVerbDrawer => self.verb.drawer_open = !self.verb.drawer_open,
            }
        }
    }

    /// The trust banner strip (T-915.3 §UI shape): STRICT badge + doc-tooltip,
    /// the phase/result headline (building xtask… / checking… / green / red with
    /// exit code), Re-check + cancel, the expandable verbatim-output pane, watch
    /// health, and the git-dirty chip with its expandable file list.
    fn trust_banner_ui(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let (headline, tone) = self.check.banner();
        ui.horizontal(|ui| {
            let color = tone_color(ui, tone);
            // STRICT, prominently — this banner is the --strict bar, NOT the
            // (non-strict) mutator preflight.
            ui.label(
                RichText::new(" STRICT ")
                    .strong()
                    .monospace()
                    .background_color(color.gamma_multiply(0.22))
                    .color(color),
            )
            .on_hover_text(trust::STRICT_TOOLTIP);
            if self.check_handle.is_some() {
                ui.add(Spinner::new().size(12.0));
            }
            ui.label(RichText::new(&headline).color(color).strong());
            if ui
                .button("Re-check")
                .on_hover_text(trust::CHECK_COMMAND)
                .clicked()
            {
                actions.push(Action::Recheck);
            }
            if self.check_handle.is_some() && ui.small_button("✕ cancel").clicked() {
                actions.push(Action::CancelCheck);
            }
            let output_label = format!("output ({})", self.check_log.len());
            if ui
                .selectable_label(self.show_output, RichText::new(output_label).small())
                .clicked()
            {
                actions.push(Action::ToggleOutput);
            }
            if let Some(error) = &self.watch_error {
                ui.label(
                    RichText::new("watch unavailable")
                        .color(ui.visuals().warn_fg_color)
                        .small(),
                )
                .on_hover_text(error);
            } else if let Some(watch) = &self.watch
                && !watch.degraded.is_empty()
            {
                ui.label(RichText::new("watch degraded").weak().small())
                    .on_hover_text(watch.degraded.join("\n"));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                git_chip_ui(ui, &self.git_chip, self.git_expanded, actions);
            });
        });
        if self.show_output {
            output_pane_ui(ui, &self.check_log);
        }
        if self.git_expanded
            && let GitChip::Dirty(files) = &self.git_chip
        {
            git_list_ui(ui, files);
        }
    }
}

impl eframe::App for TicketboardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);

        let mut actions: Vec<Action> = Vec::new();
        let busy = self.load_rx.is_some();
        let tab = self.tab;
        // T-915.4: while a verb subprocess is in flight, every mutation
        // affordance (menus, strip, dialog Run, drag, New ticket) is disabled.
        let verb_busy = self.verb.queue.busy();
        let board_active = matches!(self.state, State::Board(_));

        // Trust banner ABOVE the tabs — persistent on every tab and every state
        // once a repo is active (T-915.3 §UI shape).
        if self.repo_root.is_some() {
            Panel::top(Id::new("trustbanner")).show(ui, |ui| {
                self.trust_banner_ui(ui, &mut actions);
            });
        }
        Panel::top(Id::new("topbar")).show(ui, |ui| {
            topbar_ui(
                ui,
                self.repo_root.as_deref(),
                busy,
                tab,
                board_active,
                verb_busy,
                &mut actions,
            );
        });
        if board_active {
            Panel::top(Id::new("filterbar")).show(ui, |ui| {
                if let State::Board(b) = &mut self.state
                    && filter_bar_ui(
                        ui,
                        &mut b.filters,
                        &b.filter_index.executors,
                        &b.facet_options,
                    )
                {
                    actions.push(Action::FiltersChanged);
                }
            });
        }
        if let State::Board(b) = &self.state {
            Panel::bottom(Id::new("footer")).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(&b.footer);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        mutate::verb_chip_ui(ui, &self.verb, &mut actions);
                    });
                });
            });
            // Verb drawer ABOVE the footer: streamed log while a verb runs;
            // stays open with the full verbatim output on a nonzero exit.
            if self.verb.drawer_open {
                Panel::bottom(Id::new("verb_drawer")).show(ui, |ui| {
                    mutate::drawer_ui(ui, &self.verb, &mut actions);
                });
            }
            if let Some(selected) = b.selected {
                Panel::right(Id::new("detail"))
                    .resizable(true)
                    .default_size(DETAIL_W)
                    .show(ui, |ui| {
                        detail_ui(
                            ui,
                            MutCtx {
                                repo_root: self.repo_root.as_deref(),
                                busy: verb_busy,
                            },
                            b,
                            selected,
                            &mut self.advanced_status,
                            &mut actions,
                        );
                    });
            }
        }
        egui::CentralPanel::default().show(ui, |ui| match &self.state {
            State::NoRepo { note } => {
                norepo_ui(ui, note.as_deref(), self.pick_rx.is_some(), &mut actions);
            }
            State::Loading => loading_ui(ui, self.repo_root.as_deref()),
            State::Refused(e) => refusal_ui(ui, e, &mut actions),
            State::Board(b) => {
                let mctx = MutCtx {
                    repo_root: self.repo_root.as_deref(),
                    busy: verb_busy,
                };
                match tab {
                    Tab::Board => board_ui(ui, b, mctx, &mut actions),
                    Tab::Waves => waves_ui(ui, b, &mut actions),
                    Tab::Tree => tree_ui(ui, b, &mut actions),
                    Tab::Metrics => metrics_ui(ui, b, &mut actions),
                }
            }
        });

        // T-915.4 dialog pass — one modal above everything, closed on Cancel /
        // Esc / backdrop / dispatch (dialogs only exist over a loaded board).
        if let Some(dialog) = &mut self.dialog {
            let mut keep = false;
            if let State::Board(b) = &self.state {
                let mctx = MutCtx {
                    repo_root: self.repo_root.as_deref(),
                    busy: verb_busy,
                };
                keep = !mutate::dialog_ui(&ctx, b, mctx, dialog, &mut actions);
            }
            if !keep {
                self.dialog = None;
            }
        }
        mutate::toasts_ui(&ctx, &mut self.toasts);

        self.apply(actions, &ctx);
    }

    /// Persist the active repo root (revalidated on next launch). This is the
    /// app's ONLY direct file write, and it goes to the user config dir — never
    /// the repo (T-915.4 mutations shell the xtask verbs; the app writes no
    /// ticket bytes itself).
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(root) = &self.repo_root {
            storage.set_string(REPO_ROOT_KEY, root.display().to_string());
        }
    }
}

// ---- chrome ----

/// Banner accent per tone. Raw egui text colors elsewhere; the green/red pair
/// reuses the verdict palette so "check OK" and "no collision" read the same.
fn tone_color(ui: &Ui, tone: Tone) -> Color32 {
    match tone {
        Tone::Neutral => ui.visuals().weak_text_color(),
        Tone::Busy => Color32::from_rgb(245, 175, 80),
        Tone::Green => VERDICT_OK,
        Tone::Red => VERDICT_COLLIDE,
    }
}

/// Git-dirty chip: subdued "clean", loud "N uncommitted registry file(s)" (click
/// expands the verbatim porcelain list), "git unavailable" when git is absent or
/// refuses — never a crash, never a fake clean.
fn git_chip_ui(ui: &mut Ui, chip: &GitChip, expanded: bool, actions: &mut Vec<Action>) {
    let dirty = matches!(chip, GitChip::Dirty(_));
    let text = match chip {
        GitChip::Dirty(_) => RichText::new(chip.label()).color(ui.visuals().warn_fg_color),
        GitChip::Unavailable(_) => RichText::new(chip.label()).weak().italics(),
        GitChip::Clean | GitChip::Unknown => RichText::new(chip.label()).weak(),
    };
    let response = ui.selectable_label(expanded && dirty, text);
    let response = match chip {
        GitChip::Unavailable(reason) => response.on_hover_text(reason),
        _ => response.on_hover_text(format!("git {}", gitstatus::GIT_ARGS.join(" "))),
    };
    if response.clicked() && dirty {
        actions.push(Action::ToggleGitList);
    }
}

/// Verbatim merged stdout+stderr of the strict check — the last ~500 lines,
/// never paraphrased; drops are named, not hidden.
fn output_pane_ui(ui: &mut Ui, log: &LogRing) {
    ui.separator();
    if log.dropped() > 0 {
        ui.label(
            RichText::new(format!("… {} earlier line(s) dropped", log.dropped()))
                .weak()
                .small(),
        );
    }
    if log.is_empty() {
        ui.label(RichText::new("no output yet").weak().small());
        return;
    }
    ScrollArea::vertical()
        .id_salt("check_output")
        .max_height(OUTPUT_MAX_H)
        .stick_to_bottom(true)
        .auto_shrink([false, true])
        .show_rows(ui, OUTPUT_ROW_H, log.len(), |ui, row_range| {
            for line in log.lines().skip(row_range.start).take(row_range.len()) {
                ui.label(RichText::new(line).monospace().small());
            }
        });
}

/// The expanded git-dirty file list — porcelain entries verbatim (`XY path`).
fn git_list_ui(ui: &mut Ui, files: &[String]) {
    ui.separator();
    ScrollArea::vertical()
        .id_salt("git_dirty_list")
        .max_height(GIT_LIST_MAX_H)
        .auto_shrink([false, true])
        .show_rows(ui, OUTPUT_ROW_H, files.len(), |ui, row_range| {
            for line in &files[row_range] {
                ui.label(RichText::new(line).monospace().small());
            }
        });
}

fn topbar_ui(
    ui: &mut Ui,
    repo_root: Option<&Path>,
    busy: bool,
    tab: Tab,
    board_active: bool,
    verb_busy: bool,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ticketboard").strong());
        ui.separator();
        for (t, label) in TABS {
            if ui.selectable_label(tab == t, label).clicked() {
                actions.push(Action::SetTab(t));
            }
        }
        ui.separator();
        match repo_root {
            Some(root) => ui.monospace(root.display().to_string()),
            None => ui.label(RichText::new("no repo").weak()),
        };
        if ui
            .add_enabled(repo_root.is_some() && !busy, Button::new("Reload"))
            .clicked()
        {
            actions.push(Action::Reload);
        }
        if busy {
            ui.add(Spinner::new().size(14.0));
        }
        // T-915.4: mint a new ticket through the one write path.
        if board_active {
            ui.separator();
            if ui
                .add_enabled(!verb_busy, Button::new("New ticket…"))
                .on_hover_text("cargo xtask ticket add <title> [--summary …]")
                .clicked()
            {
                actions.push(Action::OpenDialog(Box::new(mutate::add_dialog())));
            }
        }
    });
}

/// One scope-facet dropdown (T-918.1). Options are the narrowed vocab ∪ corpus
/// union from `facets::compute`; corpus values the vocabulary does not know are
/// marked — display-only marking, `ticket check` stays the validation authority.
fn facet_combo(
    ui: &mut Ui,
    salt: &str,
    any_label: &str,
    sel: &mut Option<String>,
    options: &[FacetOption],
) {
    ComboBox::from_id_salt(salt)
        .selected_text(sel.clone().unwrap_or_else(|| any_label.to_owned()))
        .show_ui(ui, |ui| {
            ui.selectable_value(sel, None, any_label);
            for opt in options {
                let text = if opt.vocab_unknown {
                    RichText::new(format!("{} (not in vocab)", opt.value)).italics()
                } else {
                    RichText::new(&opt.value)
                };
                ui.selectable_value(sel, Some(opt.value.clone()), text);
            }
        });
}

/// Composable filter bar — mutates `filters` in place; returns true when anything
/// changed this frame (the caller then refilters once, outside the paint).
fn filter_bar_ui(
    ui: &mut Ui,
    filters: &mut Filters,
    executors: &[String],
    facet_options: &FacetOptions,
) -> bool {
    let before = filters.clone();
    ui.horizontal_wrapped(|ui| {
        ui.add(
            TextEdit::singleline(&mut filters.text)
                .desired_width(190.0)
                .hint_text("filter id / title / summary"),
        );
        ComboBox::from_id_salt("executor_filter")
            .selected_text(filters.executor.as_deref().unwrap_or("any executor"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filters.executor, None, "any executor");
                for executor in executors {
                    ui.selectable_value(&mut filters.executor, Some(executor.clone()), executor);
                }
            });
        ComboBox::from_id_salt("kind_filter")
            .selected_text(filters.kind.label())
            .show_ui(ui, |ui| {
                for kind in KindFilter::ALL {
                    ui.selectable_value(&mut filters.kind, kind, kind.label());
                }
            });
        // T-918.1 scope facets — higher selections narrow the lower dropdowns
        // (recomputed in refilter, where stale lower picks are also cleared).
        facet_combo(
            ui,
            "domain_facet",
            "any domain",
            &mut filters.scope.domain,
            &facet_options.domains,
        );
        facet_combo(
            ui,
            "layer_facet",
            "any layer",
            &mut filters.scope.layer,
            &facet_options.layers,
        );
        facet_combo(
            ui,
            "component_facet",
            "any component",
            &mut filters.scope.component,
            &facet_options.components,
        );
        facet_combo(
            ui,
            "surface_facet",
            "any surface",
            &mut filters.scope.surface,
            &facet_options.surfaces,
        );
        // T-918.1 class facet — the closed 5-value set, chip-colored.
        ComboBox::from_id_salt("class_facet")
            .selected_text(filters.class.map_or("any class", Class::as_str))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut filters.class, None, "any class");
                for class in Class::ALL {
                    ui.selectable_value(
                        &mut filters.class,
                        Some(class),
                        RichText::new(class.as_str()).color(class_color(class)),
                    );
                }
            });
        for (i, status) in board::STATUS_ORDER.iter().enumerate() {
            let on = filters.statuses[i];
            let text = if on {
                RichText::new(status.as_str())
                    .small()
                    .color(status_color(*status))
            } else {
                RichText::new(status.as_str()).small().weak()
            };
            if ui.selectable_label(on, text).clicked() {
                filters.statuses[i] = !on;
            }
        }
        ui.add(
            TextEdit::singleline(&mut filters.parent)
                .desired_width(90.0)
                .hint_text("parent id"),
        );
        // One-click clear — restores the full measured count (acceptance 4).
        if ui
            .add_enabled(filters.is_active(), Button::new("clear"))
            .clicked()
        {
            filters.clear();
        }
    });
    *filters != before
}

// ---- full-window states ----

fn norepo_ui(ui: &mut Ui, note: Option<&str>, picking: bool, actions: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.heading("No ticket registry found");
    ui.add_space(8.0);
    ui.label(format!(
        "Ticketboard needs a repo root containing {}/ — two ways to point it at one:",
        discovery::TICKETS_SUBDIR
    ));
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("1.");
        ui.label("Pass the repo root as the positional CLI argument:");
        ui.monospace("ticketboard /path/to/repo");
    });
    ui.horizontal(|ui| {
        ui.label("2.");
        ui.label(format!(
            "Launch from anywhere inside the repo — the app walks up from the current \
             directory looking for {}/.",
            discovery::TICKETS_SUBDIR
        ));
    });
    if let Some(note) = note {
        ui.add_space(8.0);
        ui.label(RichText::new(note).color(ui.visuals().warn_fg_color));
    }
    ui.add_space(16.0);
    if ui
        .add_enabled(!picking, Button::new("Pick repo folder…"))
        .clicked()
    {
        actions.push(Action::PickFolder);
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new(
            "The picked folder is remembered (eframe storage in your user config dir) \
             and revalidated on the next launch.",
        )
        .weak()
        .small(),
    );
}

fn loading_ui(ui: &mut Ui, repo_root: Option<&Path>) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        ui.add(Spinner::new().size(28.0));
        ui.add_space(12.0);
        if let Some(root) = repo_root {
            ui.monospace(format!(
                "parsing {}/T-*.toml …",
                root.join(discovery::TICKETS_SUBDIR).display()
            ));
        }
    });
}

/// The trust surface: fail-closed refusal naming the file, verbatim error in
/// monospace, and a recovery path that needs no restart (fix on disk → Reload).
fn refusal_ui(ui: &mut Ui, error: &LoadError, actions: &mut Vec<Action>) {
    ui.add_space(24.0);
    ui.heading("Ticket corpus refused to load");
    ui.add_space(8.0);
    ui.label(
        "Fail-closed: nothing renders until every ticket parses — no partial board. \
         Fix the file on disk, then Reload; no restart needed.",
    );
    ui.add_space(16.0);
    ui.label(
        RichText::new(error.file.display().to_string())
            .monospace()
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if ui.button("Reload").clicked() {
            actions.push(Action::Reload);
        }
        if ui.button("Reveal in file manager").clicked() {
            let dir = error.file.parent().unwrap_or(&error.file).to_path_buf();
            actions.push(Action::OpenPath(dir));
        }
    });
    ui.add_space(12.0);
    ScrollArea::vertical()
        .id_salt("refusal_error")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // The parse error, VERBATIM — never paraphrased.
            ui.label(RichText::new(&error.error).monospace());
        });
}

// ---- board ----

fn board_ui(ui: &mut Ui, b: &BoardState, mctx: MutCtx<'_>, actions: &mut Vec<Action>) {
    ScrollArea::horizontal()
        .id_salt("board")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (col_index, column) in b.board.columns.iter().enumerate() {
                    if b.expanded[col_index] {
                        column_ui(ui, b, col_index, mctx, actions);
                    } else {
                        chip_column_ui(ui, col_index, column, actions);
                    }
                }
            });
        });
}

fn column_ui(
    ui: &mut Ui,
    b: &BoardState,
    col_index: usize,
    mctx: MutCtx<'_>,
    actions: &mut Vec<Action>,
) {
    let column = &b.board.columns[col_index];
    let visible = &b.visible[col_index];
    let selected = b.selected;
    ui.push_id(col_index, |ui| {
        let inner = ui.vertical(|ui| {
            ui.set_width(COL_W);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&column.header).strong());
                if board::collapsed_by_default(column.status) && ui.small_button("−").clicked() {
                    actions.push(Action::ToggleColumn(col_index));
                }
            });
            ui.separator();
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            ScrollArea::vertical()
                .id_salt("cards")
                .auto_shrink([false, false])
                .show_rows(ui, CARD_H, visible.len(), |ui, row_range| {
                    for &row in &visible[row_range] {
                        let card = &column.cards[row];
                        // T-915.4: idea cards drag onto the queued column — the
                        // drop opens the same anchor picker as "Queue after…".
                        let draggable = !mctx.busy && column.status == StatusName::Idea;
                        let response = card_ui(ui, card, selected == Some(card.index), draggable);
                        if response.clicked() {
                            actions.push(select_or_compare(ui, card.index));
                        }
                        if draggable && response.drag_started() {
                            DragAndDrop::set_payload(ui.ctx(), DragCard(card.index));
                        }
                        response.context_menu(|ui| {
                            mutate::card_menu_ui(ui, b, mctx, card.index, actions);
                        });
                    }
                });
        });
        // The queued column is the drag target (drag-target only — no
        // drag-to-position; the popup anchor picker is the acceptance surface).
        if column.status == StatusName::Queued {
            let rect = inner.response.rect;
            let drop = ui.interact(rect, ui.id().with("queued_drop"), Sense::hover());
            if drop.dnd_hover_payload::<DragCard>().is_some() {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    ui.visuals().selection.stroke,
                    StrokeKind::Inside,
                );
            }
            if let Some(payload) = drop.dnd_release_payload::<DragCard>()
                && !mctx.busy
            {
                actions.push(Action::OpenDialog(Box::new(mutate::anchor_dialog(
                    b, payload.0,
                ))));
            }
        }
    });
}

/// Collapsed `shipped` / `cancelled` column: a count chip; click expands.
fn chip_column_ui(
    ui: &mut Ui,
    col_index: usize,
    column: &board::Column,
    actions: &mut Vec<Action>,
) {
    ui.push_id(col_index, |ui| {
        ui.vertical(|ui| {
            ui.set_width(CHIP_COL_W);
            if ui.button(&column.chip).clicked() {
                actions.push(Action::ToggleColumn(col_index));
            }
        });
    });
}

/// Card paint: one allocated rect, painter-only text — no nested widgets, so a
/// virtualized column stays well inside the 17 ms frame budget. `draggable`
/// (idea cards, T-915.4) adds drag sense for the drag-onto-queued affordance.
fn card_ui(ui: &mut Ui, card: &Card, selected: bool, draggable: bool) -> egui::Response {
    let width = ui.available_width();
    let sense = if draggable {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_H), sense);
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let visuals = ui.visuals();
    let bg = if selected {
        visuals.selection.bg_fill.gamma_multiply(0.35)
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.faint_bg_color
    };
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    painter.rect_filled(rect, 4.0, bg);
    if selected {
        painter.rect_stroke(rect, 4.0, visuals.selection.stroke, StrokeKind::Inside);
    }

    let pad = 8.0;
    let left = rect.left() + pad;
    // id — monospace, prominent.
    painter.text(
        pos2(left, rect.top() + 5.0),
        Align2::LEFT_TOP,
        &card.id,
        FontId::monospace(13.0),
        visuals.strong_text_color(),
    );
    // order — right-aligned.
    if !card.order_label.is_empty() {
        painter.text(
            pos2(rect.right() - pad, rect.top() + 6.0),
            Align2::RIGHT_TOP,
            &card.order_label,
            FontId::monospace(11.0),
            visuals.weak_text_color(),
        );
    }
    // title.
    painter.text(
        pos2(left, rect.top() + 21.0),
        Align2::LEFT_TOP,
        &card.title,
        FontId::proportional(12.0),
        visuals.text_color(),
    );
    // scope breadcrumb (work tickets; T-918.1) — compact chip path, per-level
    // muted accents, painter-clipped at the card edge. The card form omits the
    // "(no surface)" marker (detail-panel only). An owns-inferred scope is
    // PREFIXED with the ~ glyph (always visible even when the tail clips) and
    // gets a hover tooltip on the glyph.
    if let Some(bc) = &card.breadcrumb {
        let font = FontId::proportional(9.0);
        let y = rect.top() + 36.0;
        let mut x = left;
        if bc.estimated {
            let r = painter.text(
                pos2(x, y),
                Align2::LEFT_TOP,
                board::SCOPE_ESTIMATED_GLYPH,
                font.clone(),
                SCOPE_ESTIMATED_COLOR,
            );
            x = r.right() + 2.0;
            ui.interact(
                r.expand(2.0),
                response.id.with("scope_estimated"),
                Sense::hover(),
            )
            .on_hover_text(board::SCOPE_ESTIMATED_TIP);
        }
        for (i, seg) in bc.segs.iter().enumerate() {
            if i > 0 {
                let r = painter.text(
                    pos2(x, y),
                    Align2::LEFT_TOP,
                    board::SCOPE_SEP,
                    font.clone(),
                    visuals.weak_text_color(),
                );
                x = r.right() + 3.0;
            }
            let r = painter.text(
                pos2(x, y),
                Align2::LEFT_TOP,
                &seg.text,
                font.clone(),
                scope_level_color(seg.level),
            );
            x = r.right() + 3.0;
        }
    }
    // executor chip.
    let galley = painter.layout_no_wrap(
        card.executor.clone(),
        FontId::proportional(10.0),
        visuals.weak_text_color(),
    );
    let chip_pos = pos2(left, rect.bottom() - 5.0 - galley.size().y);
    let chip_rect = Rect::from_min_size(chip_pos, galley.size()).expand2(vec2(4.0, 1.5));
    painter.rect_filled(chip_rect, 6.0, visuals.extreme_bg_color);
    painter.galley(chip_pos, galley, visuals.weak_text_color());
    // class chip (T-918.1) — accent-colored text on the same chip ground; a
    // ticket without a class (programs, pre-triage work) renders none.
    if let Some(class) = card.class {
        let color = class_color(class);
        let galley =
            painter.layout_no_wrap(class.as_str().to_owned(), FontId::proportional(10.0), color);
        let class_pos = pos2(
            chip_rect.right() + 6.0,
            rect.bottom() - 5.0 - galley.size().y,
        );
        let class_rect = Rect::from_min_size(class_pos, galley.size()).expand2(vec2(4.0, 1.5));
        painter.rect_filled(class_rect, 6.0, visuals.extreme_bg_color);
        painter.galley(class_pos, galley, color);
    }
    response
}

// ---- waves ----

fn waves_ui(ui: &mut Ui, b: &BoardState, actions: &mut Vec<Action>) {
    match &b.lock {
        LockState::Missing { message } => lock_missing_ui(ui, message),
        LockState::Refused { path, error } => lock_refused_ui(ui, path, error),
        LockState::Loaded(_) => {
            if let Some(model) = &b.waves {
                waves_body_ui(ui, b, model, actions);
            }
        }
    }
}

/// A deleted/renamed wave.lock renders the DidNotRun refusal — never empty lanes
/// (acceptance 3).
fn lock_missing_ui(ui: &mut Ui, message: &str) {
    ui.add_space(24.0);
    ui.heading("No wave plan");
    ui.add_space(8.0);
    // The DidNotRun refusal, VERBATIM (mirrors wave_lock::missing_lock_error).
    ui.label(RichText::new(message).monospace().size(14.0));
    ui.add_space(8.0);
    ui.label(
        RichText::new("The lock is rendered verbatim; the app never recomputes packing.")
            .weak()
            .small(),
    );
}

fn lock_refused_ui(ui: &mut Ui, path: &Path, error: &str) {
    ui.add_space(24.0);
    ui.heading("wave.lock refused to parse");
    ui.add_space(8.0);
    ui.label(
        RichText::new(path.display().to_string())
            .monospace()
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);
    ScrollArea::vertical()
        .id_salt("lock_error")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // The parse error, VERBATIM — never paraphrased.
            ui.label(RichText::new(error).monospace());
        });
}

fn waves_body_ui(ui: &mut Ui, b: &BoardState, model: &WavesModel, actions: &mut Vec<Action>) {
    ScrollArea::vertical()
        .id_salt("waves")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);
            // Header strip: wave_base / max_concurrent / pack_last, off the lock.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&model.header).monospace().strong());
                ui.separator();
                ui.label(RichText::new("pack_last:").weak().small());
                if model.pack_last.is_empty() {
                    ui.label(RichText::new("—").weak());
                }
                for chip in &model.pack_last {
                    wave_chip_ui(ui, b, chip, actions);
                }
            });
            ui.separator();
            for lane in &model.lanes {
                lane_ui(ui, b, lane, actions);
            }
            if let Some(w0) = &model.wave0 {
                wave0_ui(ui, b, w0, actions);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("Unplanned").strong());
            ui.label(
                RichText::new(
                    "derived from the ticket files, not from the lock — dispatchable ids \
                     absent from every lock wave",
                )
                .weak()
                .small(),
            );
            if model.unplanned.is_empty() {
                ui.label(RichText::new("—").weak());
            } else {
                ui.horizontal_wrapped(|ui| {
                    for chip in &model.unplanned {
                        wave_chip_ui(ui, b, chip, actions);
                    }
                });
            }
            ui.add_space(12.0);
        });
}

fn lane_ui(ui: &mut Ui, b: &BoardState, lane: &Lane, actions: &mut Vec<Action>) {
    // The lock's wave number salts the lane's widget ids (stable across repaints).
    ui.push_id(lane.n, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&lane.label).strong());
            // The acceptance surface: paste against the lock's [[waves]] block.
            if ui.small_button("copy TSV").clicked() {
                actions.push(Action::CopyTsv(lane.tsv.clone()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for chip in &lane.chips {
                wave_chip_ui(ui, b, chip, actions);
            }
        });
    });
}

/// Wave 0 — ALWAYS a count chip; click expands a virtualized flat id list, never
/// cards (acceptance 2).
fn wave0_ui(ui: &mut Ui, b: &BoardState, w0: &Wave0, actions: &mut Vec<Action>) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("wave 0").strong());
        if ui.button(&w0.label).clicked() {
            actions.push(Action::ToggleWave0);
        }
        if ui.small_button("copy TSV").clicked() {
            actions.push(Action::CopyTsv(w0.tsv.clone()));
        }
    });
    if b.wave0_expanded {
        ScrollArea::vertical()
            .id_salt("wave0_ids")
            .max_height(WAVE0_LIST_MAX_H)
            .auto_shrink([false, true])
            .show_rows(ui, WAVE0_ROW_H, w0.chips.len(), |ui, row_range| {
                for chip in &w0.chips[row_range] {
                    wave_chip_ui(ui, b, chip, actions);
                }
            });
    }
}

/// One lock-verbatim ticket chip: status-colored when the ticket file exists,
/// struck through when the lock names an id with no file (display-only, no
/// judgment). Active filters dim non-matching chips instead of hiding them — the
/// lane must always show the lock's exact membership.
fn wave_chip_ui(ui: &mut Ui, b: &BoardState, chip: &WaveChip, actions: &mut Vec<Action>) {
    let dimmed = b.filters.is_active() && chip.corpus_index.is_none_or(|i| !b.matches[i]);
    let mut text = RichText::new(&chip.id).monospace();
    match chip.status {
        Some(status) => {
            let mut color = status_color(status);
            if dimmed {
                color = color.gamma_multiply(0.35);
            }
            text = text.color(color);
        }
        None => text = text.strikethrough().weak(),
    }
    let selected_now = chip.corpus_index.is_some()
        && (b.selected == chip.corpus_index || b.compare == chip.corpus_index);
    let response = ui
        .selectable_label(selected_now, text)
        .on_hover_text(chip.tooltip.as_str());
    if response.clicked()
        && let Some(index) = chip.corpus_index
    {
        actions.push(select_or_compare(ui, index));
    }
}

// ---- program tree ----

fn tree_ui(ui: &mut Ui, b: &BoardState, actions: &mut Vec<Action>) {
    ScrollArea::vertical()
        .id_salt("tree")
        .auto_shrink([false, false])
        .show_rows(ui, TREE_ROW_H, b.tree_flat.len(), |ui, row_range| {
            for row in &b.tree_flat[row_range] {
                tree_row_ui(ui, b, *row, actions);
            }
        });
}

fn tree_row_ui(ui: &mut Ui, b: &BoardState, row: tree::FlatRow, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.add_space(f32::from(row.depth) * TREE_INDENT);
        if row.has_children {
            let glyph = if row.expanded { "▼" } else { "▶" };
            if ui.small_button(glyph).clicked() {
                actions.push(Action::ToggleNode(row.index));
            }
        } else {
            ui.add_space(24.0);
        }
        let ticket = &b.corpus.tickets[row.index].ticket;
        let mut color = status_color(ticket.status().name());
        if row.dimmed {
            color = color.gamma_multiply(0.45);
        }
        let selected_now = b.selected == Some(row.index) || b.compare == Some(row.index);
        let response = ui.selectable_label(
            selected_now,
            RichText::new(ticket.id()).monospace().color(color),
        );
        if response.clicked() {
            actions.push(select_or_compare(ui, row.index));
        }
        ui.label(RichText::new(&b.tree.titles[row.index]).weak().small());
    });
}

// ---- metrics dashboard (T-915.5) ----

fn metrics_ui(ui: &mut Ui, b: &BoardState, actions: &mut Vec<Action>) {
    match &b.metrics {
        MetricsState::NoReceipts => metrics_empty_ui(ui),
        MetricsState::Loaded(m) => metrics_body_ui(ui, b, m, actions),
    }
}

/// The T-915.5 acceptance-1 surface: an ABSENT (or empty) receipts directory is
/// this explicit state — never a table of zeros.
fn metrics_empty_ui(ui: &mut Ui) {
    ui.add_space(24.0);
    ui.heading("No receipts yet");
    ui.add_space(8.0);
    ui.label(
        RichText::new(metrics::NO_RECEIPTS_TEXT)
            .monospace()
            .size(14.0),
    );
    ui.add_space(8.0);
    ui.label(
        RichText::new(
            "The dashboard renders real run files only — tokens are never invented, \
             so an empty tree is this message, not zeros.",
        )
        .weak()
        .small(),
    );
    ui.add_space(4.0);
    ui.label(RichText::new(metrics::COVERAGE_NOTE).weak().small());
}

fn metrics_body_ui(
    ui: &mut Ui,
    b: &BoardState,
    m: &metrics::MetricsModel,
    actions: &mut Vec<Action>,
) {
    ScrollArea::vertical()
        .id_salt("metrics")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);
            // Grand-total strip (precomputed at load; with zero valid runs it
            // says "no valid receipts", never a zeros row).
            ui.label(RichText::new(&m.grand.strip).monospace().strong());
            ui.label(RichText::new(metrics::COVERAGE_NOTE).weak().small());
            if !m.errors.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} malformed receipt file(s) — excluded from every sum; listed below",
                        m.errors.len()
                    ))
                    .color(VERDICT_COLLIDE)
                    .strong(),
                );
            }
            ui.separator();
            ui.label(RichText::new("Per agent").strong());
            metrics_table_ui(ui, b, &m.per_agent, TableKind::Agent, actions);
            ui.add_space(10.0);
            ui.separator();
            ui.label(RichText::new("Per ticket").strong());
            metrics_table_ui(ui, b, &m.per_ticket, TableKind::Ticket, actions);
            if !m.errors.is_empty() {
                ui.add_space(10.0);
                ui.separator();
                ui.label(
                    RichText::new(format!("Malformed receipts ({})", m.errors.len())).strong(),
                );
                ui.label(
                    RichText::new(
                        "named per file, reason verbatim — observations are listed broken, \
                         never silently skipped and never coerced to numbers",
                    )
                    .weak()
                    .small(),
                );
                for error in &m.errors {
                    ui.label(
                        RichText::new(&error.rel)
                            .monospace()
                            .small()
                            .color(VERDICT_COLLIDE),
                    );
                    ui.label(RichText::new(&error.reason).monospace().small());
                }
            }
            ui.add_space(12.0);
        });
}

/// One aggregation table (per agent / per ticket): egui_extras striped, the
/// runs / tokens / elapsed headers sort on click. Ticket ids link into the
/// detail panel through the existing selection plumbing.
fn metrics_table_ui(
    ui: &mut Ui,
    b: &BoardState,
    rows: &[metrics::AggRow],
    table: TableKind,
    actions: &mut Vec<Action>,
) {
    if rows.is_empty() {
        ui.label(RichText::new("—").weak());
        return;
    }
    let (sort, key_header, salt) = match table {
        TableKind::Ticket => (b.metrics_sort.ticket, "ticket", "metrics_per_ticket"),
        TableKind::Agent => (b.metrics_sort.agent, "agent", "metrics_per_agent"),
    };
    ui.push_id(salt, |ui| {
        TableBuilder::new(ui)
            .striped(true)
            .vscroll(false)
            .column(TableColumn::auto().at_least(96.0)) // key
            .column(TableColumn::auto().at_least(52.0)) // runs
            .column(TableColumn::auto().at_least(150.0)) // tokens
            .column(TableColumn::auto().at_least(96.0)) // elapsed
            .column(TableColumn::auto().at_least(130.0)) // in flight / unfinished
            .column(TableColumn::auto().at_least(170.0)) // first started
            .column(TableColumn::remainder()) // last finished
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label(RichText::new(key_header).weak().small());
                });
                header.col(|ui| {
                    sort_header_ui(ui, "runs", metrics::SortKey::Runs, sort, table, actions);
                });
                header.col(|ui| {
                    sort_header_ui(
                        ui,
                        "tokens_consumed.total",
                        metrics::SortKey::Tokens,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    sort_header_ui(
                        ui,
                        "elapsed",
                        metrics::SortKey::Elapsed,
                        sort,
                        table,
                        actions,
                    );
                });
                header.col(|ui| {
                    ui.label(RichText::new("in flight / unfinished").weak().small())
                        .on_hover_text(
                            "runs with no finished stamp — counted, never folded into elapsed",
                        );
                });
                header.col(|ui| {
                    ui.label(RichText::new("first started").weak().small());
                });
                header.col(|ui| {
                    ui.label(RichText::new("last finished").weak().small());
                });
            })
            .body(|mut body| {
                for agg in rows {
                    body.row(20.0, |mut row| {
                        row.col(|ui| match table {
                            TableKind::Ticket => {
                                id_link_ui(ui, &agg.key, &b.board.id_to_index, actions);
                            }
                            TableKind::Agent => {
                                ui.monospace(&agg.key);
                            }
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.runs_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.tokens_str);
                        });
                        row.col(|ui| {
                            // "—" while nothing finished — an all-in-flight key
                            // has UNKNOWN elapsed, and 0s would fabricate one.
                            ui.monospace(&agg.elapsed_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.unfinished_str);
                        });
                        row.col(|ui| {
                            ui.monospace(&agg.min_started);
                        });
                        row.col(|ui| match &agg.max_finished {
                            Some(fin) => {
                                ui.monospace(fin);
                            }
                            None => {
                                ui.label(RichText::new("—").weak());
                            }
                        });
                    });
                }
            });
    });
}

/// A sortable column header: click toggles direction on the active column and
/// starts descending on a new one; the arrow marks the active sort.
fn sort_header_ui(
    ui: &mut Ui,
    label: &str,
    key: metrics::SortKey,
    sort: metrics::Sort,
    table: TableKind,
    actions: &mut Vec<Action>,
) {
    let active = sort.key == key;
    let text = if active {
        format!("{label} {}", if sort.desc { "▼" } else { "▲" })
    } else {
        label.to_owned()
    };
    if ui
        .selectable_label(active, RichText::new(text).small())
        .clicked()
    {
        actions.push(Action::SortMetrics(table, key));
    }
}

// ---- detail panel ----

/// One detail-row value; ids and paths are live links.
enum Cell {
    Text(String),
    Mono(String),
    IdRef(String),
    PathRef {
        label: String,
        path: PathBuf,
    },
    /// Scope breadcrumb (T-918.1) — the detail tier renders the "(no surface)"
    /// marker and the estimated-scope glyph.
    Scope(board::Breadcrumb),
    Missing,
}

fn detail_ui(
    ui: &mut Ui,
    mctx: MutCtx<'_>,
    b: &BoardState,
    selected: usize,
    advanced_status: &mut Option<StatusName>,
    actions: &mut Vec<Action>,
) {
    let Some(loaded) = b.corpus.tickets.get(selected) else {
        return;
    };
    let repo_root = mctx.repo_root;
    let v = board::view(&loaded.ticket);
    let ids = &b.board.id_to_index;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(v.id).monospace().size(16.0).strong());
        ui.label(RichText::new(v.kind).weak().small());
        // T-918.1 class chip — colored accent, absent class renders nothing.
        if let Some(class) = v.class.and_then(Class::parse) {
            ui.label(
                RichText::new(class.as_str())
                    .small()
                    .color(class_color(class)),
            );
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕").clicked() {
                actions.push(Action::CloseDetail);
            }
        });
    });
    ui.label(RichText::new(v.title).size(14.0).strong());
    ui.add_space(4.0);
    ui.separator();
    // T-915.4 action strip: offered transitions + Add child + Advanced.
    mutate::action_strip_ui(ui, b, mctx, selected, advanced_status, actions);
    ui.separator();

    ScrollArea::vertical()
        .id_salt("detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            match b.compare {
                Some(compare) if compare != selected => {
                    compare_ui(ui, b, selected, compare, actions);
                    ui.separator();
                }
                _ => {
                    // The compare affordance, discoverable where it acts.
                    ui.label(
                        RichText::new("shift-click another ticket to compare owns")
                            .weak()
                            .small(),
                    );
                }
            }
            let resolve = |p: &str| match repo_root {
                Some(root) => root.join(p),
                None => PathBuf::from(p),
            };
            let opt_mono =
                |value: Option<&str>| value.map_or(Cell::Missing, |s| Cell::Mono(s.to_owned()));
            let opt_id =
                |value: Option<&str>| value.map_or(Cell::Missing, |s| Cell::IdRef(s.to_owned()));
            let rows: Vec<(&str, Cell)> = vec![
                ("status", Cell::Text(board::status_label(v.status))),
                (
                    "executor",
                    Cell::Text(v.executor.map_or_else(
                        || format!("{} (default)", board::EXECUTOR_DEFAULT),
                        str::to_owned,
                    )),
                ),
                (
                    "priority",
                    v.priority
                        .map_or(Cell::Missing, |p| Cell::Text(p.to_string())),
                ),
                (
                    "spec",
                    v.spec.map_or(Cell::Missing, |s| Cell::PathRef {
                        label: s.to_owned(),
                        path: resolve(s),
                    }),
                ),
                ("parent", opt_id(v.parent)),
                ("active", opt_id(v.active)),
                ("shipped_at", opt_mono(v.shipped_at)),
                ("created_at", opt_mono(v.created_at)),
                ("completed_at", opt_mono(v.completed_at)),
                (
                    "pack_last",
                    v.pack_last
                        .map_or(Cell::Missing, |p| Cell::Text(p.to_string())),
                ),
                (
                    "scope",
                    v.scope.map_or(Cell::Missing, |s| {
                        Cell::Scope(board::breadcrumb(s, v.estimated))
                    }),
                ),
                (
                    "file",
                    Cell::PathRef {
                        label: loaded.path.display().to_string(),
                        path: loaded.path.clone(),
                    },
                ),
            ];
            TableBuilder::new(ui)
                .striped(true)
                .vscroll(false)
                .column(TableColumn::auto().at_least(84.0))
                .column(TableColumn::remainder())
                .body(|mut body| {
                    for (label, cell) in &rows {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                ui.label(RichText::new(*label).weak().small());
                            });
                            row.col(|ui| cell_ui(ui, cell, ids, actions));
                        });
                    }
                });

            text_section(ui, "summary", Some(v.summary).filter(|s| !s.is_empty()));
            text_section(ui, "user_story", v.user_story);
            acceptance_section(ui, v.acceptance);
            text_section(ui, "notes", v.notes);
            id_list_section(ui, "depends_on", v.depends_on, ids, actions);
            id_list_section(ui, "unblocks", v.unblocks, ids, actions);
            id_list_section(ui, "children", v.children, ids, actions);
            owns_section(ui, v.owns);
            ui.add_space(12.0);
        });
}

/// Owns-collision explainer (design §UI shape): with exactly two tickets selected,
/// both owns lists plus EVERY colliding pair under the prefix-containment rule
/// (`wavelock::paths_collide`, the `wave_lock::collides` mirror) and the verdict —
/// why these two can never share a wave, or that they can.
fn compare_ui(
    ui: &mut Ui,
    b: &BoardState,
    selected: usize,
    compare: usize,
    actions: &mut Vec<Action>,
) {
    let a = board::view(&b.corpus.tickets[selected].ticket);
    let z = board::view(&b.corpus.tickets[compare].ticket);
    ui.horizontal(|ui| {
        ui.label(RichText::new("owns collision").strong().small());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕ stop comparing").clicked() {
                actions.push(Action::ClearCompare);
            }
        });
    });
    ui.monospace(format!("{}  vs  {}", a.id, z.id));
    // The verdict IS the mirrored rule; the pairs are its explanation.
    let pairs = wavelock::colliding_pairs(a.owns, z.owns);
    if !wavelock::collides(a.owns, z.owns) {
        ui.label(RichText::new("no collision").color(VERDICT_OK).strong());
        ui.label(
            RichText::new(
                "owns paths are disjoint — the packer may put these two in the same wave",
            )
            .weak()
            .small(),
        );
    } else {
        ui.label(
            RichText::new("never the same wave")
                .color(VERDICT_COLLIDE)
                .strong(),
        );
        ui.label(
            RichText::new(
                "colliding pairs — equal, or one prefix-contains the other on a '/' boundary:",
            )
            .weak()
            .small(),
        );
        for (x, y) in &pairs {
            ui.label(
                RichText::new(format!("{x}  ×  {y}"))
                    .monospace()
                    .small()
                    .color(VERDICT_COLLIDE),
            );
        }
    }
    let left: HashSet<&String> = pairs.iter().map(|(x, _)| x).collect();
    let right: HashSet<&String> = pairs.iter().map(|(_, y)| y).collect();
    owns_compare_list(ui, a.id, a.owns, &left);
    owns_compare_list(ui, z.id, z.owns, &right);
    ui.add_space(6.0);
}

fn owns_compare_list(ui: &mut Ui, id: &str, owns: &[String], colliding: &HashSet<&String>) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(format!("{id} owns ({})", owns.len()))
            .strong()
            .small(),
    );
    if owns.is_empty() {
        missing_marker(ui);
        return;
    }
    for path in owns {
        let mut text = RichText::new(path).monospace().small();
        if colliding.contains(path) {
            text = text.color(VERDICT_COLLIDE);
        }
        ui.label(text);
    }
}

fn cell_ui(ui: &mut Ui, cell: &Cell, ids: &HashMap<String, usize>, actions: &mut Vec<Action>) {
    match cell {
        Cell::Text(s) => {
            ui.label(s);
        }
        Cell::Mono(s) => {
            ui.monospace(s);
        }
        Cell::IdRef(id) => id_link_ui(ui, id, ids, actions),
        Cell::PathRef { label, path } => {
            if ui.link(RichText::new(label).monospace()).clicked() {
                actions.push(Action::OpenPath(path.clone()));
            }
        }
        Cell::Scope(bc) => scope_breadcrumb_ui(ui, bc),
        Cell::Missing => {
            ui.label(RichText::new("—").weak());
        }
    }
}

/// Detail-tier breadcrumb (T-918.1): per-level muted accents, the explicit
/// "(no surface)" marker when a component carries no surface (detail ONLY —
/// cards omit it), and the ~ glyph with its owns-inferred tooltip. Single line
/// (the table row is fixed-height); hovering the row shows the plain-text path.
fn scope_breadcrumb_ui(ui: &mut Ui, bc: &board::Breadcrumb) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if bc.estimated {
            ui.label(
                RichText::new(board::SCOPE_ESTIMATED_GLYPH)
                    .small()
                    .color(SCOPE_ESTIMATED_COLOR),
            )
            .on_hover_text(board::SCOPE_ESTIMATED_TIP);
        }
        for (i, seg) in bc.segs.iter().enumerate() {
            if i > 0 {
                ui.label(RichText::new(board::SCOPE_SEP).weak().small());
            }
            ui.label(
                RichText::new(&seg.text)
                    .small()
                    .color(scope_level_color(seg.level)),
            );
        }
        if bc.no_surface {
            ui.label(
                RichText::new(board::NO_SURFACE_MARKER)
                    .weak()
                    .small()
                    .italics(),
            );
        }
    })
    .response
    .on_hover_text(bc.label());
}

/// Clickable when the id exists in the corpus; plain monospace when dangling.
fn id_link_ui(ui: &mut Ui, id: &str, ids: &HashMap<String, usize>, actions: &mut Vec<Action>) {
    if ids.contains_key(id) {
        if ui.link(RichText::new(id).monospace()).clicked() {
            actions.push(Action::SelectId(id.to_owned()));
        }
    } else {
        ui.monospace(id);
    }
}

fn section_header(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(title).strong().small());
}

fn missing_marker(ui: &mut Ui) {
    ui.label(RichText::new("—").weak());
}

fn text_section(ui: &mut Ui, title: &str, text: Option<&str>) {
    section_header(ui, title);
    match text {
        Some(t) => {
            ui.label(t);
        }
        None => missing_marker(ui),
    }
}

fn acceptance_section(ui: &mut Ui, items: &[String]) {
    section_header(ui, &format!("acceptance ({})", items.len()));
    if items.is_empty() {
        missing_marker(ui);
        return;
    }
    for (i, item) in items.iter().enumerate() {
        ui.label(format!("{}. {item}", i + 1));
    }
}

fn id_list_section(
    ui: &mut Ui,
    title: &str,
    ids_list: &[String],
    ids: &HashMap<String, usize>,
    actions: &mut Vec<Action>,
) {
    section_header(ui, &format!("{title} ({})", ids_list.len()));
    if ids_list.is_empty() {
        missing_marker(ui);
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for id in ids_list {
            id_link_ui(ui, id, ids, actions);
        }
    });
}

fn owns_section(ui: &mut Ui, owns: &[String]) {
    section_header(ui, &format!("owns ({})", owns.len()));
    if owns.is_empty() {
        missing_marker(ui);
        return;
    }
    for path in owns {
        ui.label(RichText::new(path).monospace().small());
    }
}

// ---- OS integration ----

/// Wall-clock seconds since the Unix epoch — the banner's timestamp source
/// (rendered as explicit UTC; the registry's own timestamps are UTC too).
fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Open a path with the OS handler: `xdg-open` (cfg-gated `start` on Windows).
/// Spawn-and-forget — the UI thread never waits on the child.
fn open_path(path: &Path) {
    #[cfg(target_os = "windows")]
    let spawned = std::process::Command::new("cmd")
        .arg("/C")
        .arg("start")
        .arg("")
        .arg(path)
        .spawn();
    #[cfg(not(target_os = "windows"))]
    let spawned = std::process::Command::new("xdg-open").arg(path).spawn();
    if let Err(e) = spawned {
        eprintln!("ticketboard: opening {} failed: {e}", path.display());
    }
}
