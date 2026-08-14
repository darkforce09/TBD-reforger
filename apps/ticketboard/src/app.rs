//! eframe shell (T-915.1) — thin UI over the pure modules.
//!
//! States: NoRepo refusal (both discovery mechanisms + native folder picker),
//! Loading, parse Refusal (the trust surface: file path + verbatim error), Board.
//! All IO happens on worker threads (`std::thread` + `mpsc`, `request_repaint` on
//! completion); the paint path only reads strings precomputed at load time.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{
    self, Align, Align2, Button, FontId, Id, Layout, Panel, Rect, RichText, ScrollArea, Sense,
    Spinner, StrokeKind, Ui, pos2, vec2,
};
use egui_extras::{Column as TableColumn, TableBuilder};

use crate::board::{self, BoardModel, Card};
use crate::corpus::{self, Corpus, LoadError, LoadResult};
use crate::discovery;

/// eframe Storage key for the picked repo root (user config dir — never the repo).
const REPO_ROOT_KEY: &str = "repo_root";

const CARD_H: f32 = 52.0;
const CARD_GAP: f32 = 6.0;
const COL_W: f32 = 236.0;
const CHIP_COL_W: f32 = 92.0;
const DETAIL_W: f32 = 420.0;

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

struct BoardState {
    corpus: Corpus,
    board: BoardModel,
    selected: Option<usize>,
    expanded: [bool; 8],
    /// Precomputed footer: the acceptance surface against
    /// `ls .ai/tickets/T-*.toml | wc -l`.
    footer: String,
}

impl BoardState {
    fn new(corpus: Corpus) -> Self {
        let board = BoardModel::build(&corpus);
        let expanded = board::STATUS_ORDER.map(|s| !board::collapsed_by_default(s));
        let c = corpus.counts;
        let footer = format!(
            "{} ticket files — {} parents / {} children",
            c.total, c.parents, c.children
        );
        Self {
            corpus,
            board,
            selected: None,
            expanded,
            footer,
        }
    }
}

/// UI events, collected during paint and applied afterwards.
enum Action {
    Reload,
    PickFolder,
    Select(usize),
    SelectId(String),
    ToggleColumn(usize),
    OpenPath(PathBuf),
    CloseDetail,
}

pub struct TicketboardApp {
    repo_root: Option<PathBuf>,
    state: State,
    load_rx: Option<Receiver<LoadResult>>,
    pick_rx: Option<Receiver<Option<PathBuf>>>,
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
            load_rx: None,
            pick_rx: None,
        };
        match discovery::resolve_repo_root(arg, cwd.as_deref()) {
            Some(root) if discovery::has_tickets_dir(&root) => {
                app.repo_root = Some(root);
                app.start_load(&cc.egui_ctx);
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
                        app.repo_root = Some(root);
                        app.start_load(&cc.egui_ctx);
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

    /// Kick the corpus load on a worker thread; the UI shows Loading meanwhile.
    fn start_load(&mut self, ctx: &egui::Context) {
        if let Some(root) = self.repo_root.clone() {
            let repaint_ctx = ctx.clone();
            self.load_rx = Some(corpus::spawn_load(root, move || {
                repaint_ctx.request_repaint()
            }));
            self.state = State::Loading;
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
            && let Ok(result) = rx.try_recv()
        {
            self.load_rx = None;
            self.state = match result {
                Ok(corpus) => State::Board(Box::new(BoardState::new(corpus))),
                Err(e) => State::Refused(e),
            };
        }
        if let Some(rx) = &self.pick_rx
            && let Ok(picked) = rx.try_recv()
        {
            self.pick_rx = None;
            if let Some(root) = picked {
                if discovery::has_tickets_dir(&root) {
                    self.repo_root = Some(root);
                    self.start_load(ctx);
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
    }

    fn apply(&mut self, actions: Vec<Action>, ctx: &egui::Context) {
        for action in actions {
            match action {
                Action::Reload => self.start_load(ctx),
                Action::PickFolder => self.start_pick(ctx),
                Action::OpenPath(path) => open_path(&path),
                Action::Select(index) => {
                    if let State::Board(b) = &mut self.state {
                        b.selected = Some(index);
                    }
                }
                Action::SelectId(id) => {
                    if let State::Board(b) = &mut self.state
                        && let Some(&index) = b.board.id_to_index.get(&id)
                    {
                        b.selected = Some(index);
                    }
                }
                Action::ToggleColumn(col) => {
                    if let State::Board(b) = &mut self.state {
                        b.expanded[col] = !b.expanded[col];
                    }
                }
                Action::CloseDetail => {
                    if let State::Board(b) = &mut self.state {
                        b.selected = None;
                    }
                }
            }
        }
    }
}

impl eframe::App for TicketboardApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);

        let mut actions: Vec<Action> = Vec::new();
        let busy = self.load_rx.is_some();

        Panel::top(Id::new("topbar")).show(ui, |ui| {
            topbar_ui(ui, self.repo_root.as_deref(), busy, &mut actions);
        });
        if let State::Board(b) = &self.state {
            Panel::bottom(Id::new("footer")).show(ui, |ui| {
                ui.horizontal(|ui| ui.label(&b.footer));
            });
            if let Some(selected) = b.selected {
                Panel::right(Id::new("detail"))
                    .resizable(true)
                    .default_size(DETAIL_W)
                    .show(ui, |ui| {
                        detail_ui(ui, self.repo_root.as_deref(), b, selected, &mut actions);
                    });
            }
        }
        egui::CentralPanel::default().show(ui, |ui| match &self.state {
            State::NoRepo { note } => {
                norepo_ui(ui, note.as_deref(), self.pick_rx.is_some(), &mut actions);
            }
            State::Loading => loading_ui(ui, self.repo_root.as_deref()),
            State::Refused(e) => refusal_ui(ui, e, &mut actions),
            State::Board(b) => board_ui(ui, b, &mut actions),
        });

        self.apply(actions, &ctx);
    }

    /// Persist the active repo root (revalidated on next launch). This is the
    /// app's ONLY write, and it goes to the user config dir — never the repo.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if let Some(root) = &self.repo_root {
            storage.set_string(REPO_ROOT_KEY, root.display().to_string());
        }
    }
}

// ---- chrome ----

fn topbar_ui(ui: &mut Ui, repo_root: Option<&Path>, busy: bool, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Ticketboard").strong());
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
    });
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

fn board_ui(ui: &mut Ui, b: &BoardState, actions: &mut Vec<Action>) {
    ScrollArea::horizontal()
        .id_salt("board")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (col_index, column) in b.board.columns.iter().enumerate() {
                    if b.expanded[col_index] {
                        column_ui(ui, col_index, column, b.selected, actions);
                    } else {
                        chip_column_ui(ui, col_index, column, actions);
                    }
                }
            });
        });
}

fn column_ui(
    ui: &mut Ui,
    col_index: usize,
    column: &board::Column,
    selected: Option<usize>,
    actions: &mut Vec<Action>,
) {
    ui.push_id(col_index, |ui| {
        ui.vertical(|ui| {
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
                .show_rows(ui, CARD_H, column.cards.len(), |ui, row_range| {
                    for card in &column.cards[row_range] {
                        let response = card_ui(ui, card, selected == Some(card.index));
                        if response.clicked() {
                            actions.push(Action::Select(card.index));
                        }
                    }
                });
        });
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
/// virtualized column stays well inside the 17 ms frame budget.
fn card_ui(ui: &mut Ui, card: &Card, selected: bool) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_H), Sense::click());
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
    response
}

// ---- detail panel ----

/// One detail-row value; ids and paths are live links.
enum Cell {
    Text(String),
    Mono(String),
    IdRef(String),
    PathRef { label: String, path: PathBuf },
    Missing,
}

fn detail_ui(
    ui: &mut Ui,
    repo_root: Option<&Path>,
    b: &BoardState,
    selected: usize,
    actions: &mut Vec<Action>,
) {
    let Some(loaded) = b.corpus.tickets.get(selected) else {
        return;
    };
    let v = board::view(&loaded.ticket);
    let ids = &b.board.id_to_index;

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(v.id).monospace().size(16.0).strong());
        ui.label(RichText::new(v.kind).weak().small());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕").clicked() {
                actions.push(Action::CloseDetail);
            }
        });
    });
    ui.label(RichText::new(v.title).size(14.0).strong());
    ui.add_space(4.0);
    ui.separator();

    ScrollArea::vertical()
        .id_salt("detail")
        .auto_shrink([false, false])
        .show(ui, |ui| {
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
                ("scope", v.scope.clone().map_or(Cell::Missing, Cell::Text)),
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
        Cell::Missing => {
            ui.label(RichText::new("—").weak());
        }
    }
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
