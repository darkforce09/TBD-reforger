//! Mutation UI (T-915.4) — card context menus, the detail-panel action strip,
//! the modal dialogs, the verb drawer and the toasts. Thin egui over the pure
//! [`crate::verbs`] module; nothing here spawns or writes — every dispatch is
//! pushed as `Action::Dispatch`, which app.rs CAS-checks and feeds through the
//! single-flight [`verbs::VerbQueue`].
//!
//! Honesty rules carried from the design: every confirm shows the LITERAL
//! command line; refusals stream verbatim in the drawer; the wave-stale
//! recovery command is TEXT, never a button; `running` is never offered as a
//! manual target in the normal UI.

use std::path::Path;
use std::time::Instant;

use eframe::egui::{
    self, Align, Align2, Area, Button, Checkbox, Color32, ComboBox, Frame, Id, Layout, Modal,
    Order, RichText, ScrollArea, Spinner, TextEdit, Ui, vec2,
};
use tbd_tickets::{StatusName, Ticket};

use crate::app::{Action, BoardState, VERDICT_COLLIDE, VERDICT_OK};
use crate::board;
use crate::subproc::ProcHandle;
use crate::verbs::{self, AdvancedAction, CasGuard, Transition, VerbQueue, VerbRequest};

const DIALOG_MIN_W: f32 = 460.0;
const DIALOG_LIST_H: f32 = 240.0;
const DRAWER_ROW_H: f32 = 15.0;
const DRAWER_MAX_H: f32 = 240.0;
const ANCHOR_ROW_H: f32 = 18.0;
const TOAST_SECS: u64 = 6;

/// Drag payload: an idea card dragged onto the queued column opens the same
/// anchor picker as "Queue after…" (the popup is the acceptance surface — no
/// drag-to-position).
pub struct DragCard(pub usize);

// ---- verb runner state (owned by the app, drained in app::poll_verb) ----

/// Everything the running-verb surface needs: the single-flight queue, the
/// in-flight subprocess, the FULL verbatim merged log, and the last outcome.
pub struct VerbRunner {
    pub queue: VerbQueue,
    pub handle: Option<ProcHandle>,
    /// FULL merged stdout+stderr of the current / most recent verb run —
    /// unbounded on purpose (verb output is small; refusals must never truncate)
    /// and painted virtualized.
    pub log: Vec<String>,
    pub last: Option<VerbOutcome>,
    pub drawer_open: bool,
    /// "N pending request(s) dropped" note after a failure — nothing auto-retries.
    pub dropped_note: Option<String>,
}

impl VerbRunner {
    pub fn new() -> Self {
        Self {
            queue: VerbQueue::default(),
            handle: None,
            log: Vec::new(),
            last: None,
            drawer_open: false,
            dropped_note: None,
        }
    }
}

/// A finished verb run — the drawer headline.
pub struct VerbOutcome {
    /// The literal command line that ran.
    pub display: String,
    /// Exit code; `None` = killed by a signal (the mid-verb SIGKILL case).
    pub code: Option<i32>,
    /// Completion wall time, `"HH:MM:SS UTC"`.
    pub at: String,
    /// The spawn itself failed — the verb never ran.
    pub spawn_error: Option<String>,
    /// The log carries the wave-stale signature → show [`verbs::RECOVERY_HINT`].
    pub hint: bool,
}

// ---- toasts ----

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub until: Instant,
}

impl Toast {
    pub fn new(text: String, error: bool) -> Self {
        Self {
            text,
            error,
            until: Instant::now() + std::time::Duration::from_secs(TOAST_SECS),
        }
    }
}

/// Transient top-right toasts (success tails, CAS refusals, queue notes).
pub fn toasts_ui(ctx: &egui::Context, toasts: &mut Vec<Toast>) {
    let now = Instant::now();
    toasts.retain(|t| t.until > now);
    if toasts.is_empty() {
        return;
    }
    Area::new(Id::new("t9154_toasts"))
        .order(Order::Foreground)
        .anchor(Align2::RIGHT_TOP, vec2(-16.0, 48.0))
        .interactable(false)
        .show(ctx, |ui| {
            for toast in toasts.iter() {
                let color = if toast.error {
                    VERDICT_COLLIDE
                } else {
                    VERDICT_OK
                };
                Frame::popup(ui.style()).show(ui, |ui| {
                    ui.label(RichText::new(&toast.text).monospace().small().color(color));
                });
            }
        });
    // Expiry needs a frame even when the user is idle.
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
}

// ---- read-only context every mutation affordance needs ----

#[derive(Clone, Copy)]
pub struct MutCtx<'a> {
    pub repo_root: Option<&'a Path>,
    /// A verb subprocess is in flight — every dispatch affordance disables.
    pub busy: bool,
}

// ---- dialogs ----

/// The one open mutation dialog. Every variant that targets an existing ticket
/// carries the [`CasGuard`] captured when its affordance was rendered/clicked;
/// dispatch re-hashes and refuses on mismatch (app.rs).
pub enum Dialog {
    /// One-verb confirm: the literal command line + optional honesty note.
    Confirm {
        title: String,
        note: Option<String>,
        req: VerbRequest,
    },
    /// "Queue after…" — searchable anchor picker → `ticket reorder`.
    AnchorPick {
        id: String,
        guard: CasGuard,
        filter: String,
        selected: Option<String>,
    },
    /// The Ready-prose form → `ticket mark-ready <id> <spec>`.
    MarkReady {
        id: String,
        guard: CasGuard,
        spec: String,
        /// Existence cache for the live indicator: (spec-as-statted, is_file).
        stat: Option<(String, bool)>,
    },
    /// Toolbar "New ticket…" → `ticket add` (no target file — no guard).
    AddTicket { title: String, summary: String },
    /// "Add child…" → `ticket add-child [--summary] [--promote]`.
    AddChild {
        parent: String,
        parent_is_work: bool,
        guard: CasGuard,
        title: String,
        summary: String,
        promote: bool,
    },
    /// "Remove…" behind type-to-confirm → `ticket remove [--force]`.
    Remove {
        id: String,
        is_program: bool,
        guard: CasGuard,
        force: bool,
        typed: String,
    },
}

// ---- dialog constructors (fingerprints captured HERE) ----

/// Dialog for a normal transition. `guard` was captured at render-of-menu /
/// click time by the caller.
pub fn transition_dialog(b: &BoardState, index: usize, t: Transition, guard: CasGuard) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    let id = loaded.ticket.id().to_owned();
    match t {
        Transition::QueueAfter => Dialog::AnchorPick {
            id,
            guard,
            filter: String::new(),
            selected: None,
        },
        Transition::MarkReady => Dialog::MarkReady {
            spec: ready_spec_prefill(b, index),
            id,
            guard,
            stat: None,
        },
        Transition::Ship
        | Transition::DemoteToQueued
        | Transition::Defer
        | Transition::CancelTicket
        | Transition::ReopenToQueued => {
            let req = verbs::confirm_request(t, &id)
                .expect("non-form transitions map to one verb")
                .with_guard(guard);
            Dialog::Confirm {
                title: format!(
                    "{} — {id}",
                    verbs::transition_label(t).trim_end_matches('…')
                ),
                note: verbs::confirm_note(t).map(str::to_owned),
                req,
            }
        }
    }
}

/// Ready-form spec prefill: the ticket's own spec, else the parent program's
/// spec when it has one, else empty.
fn ready_spec_prefill(b: &BoardState, index: usize) -> String {
    let v = board::view(&b.corpus.tickets[index].ticket);
    if let Some(own) = v.spec {
        return own.to_owned();
    }
    if let Some(parent) = v.parent
        && let Some(&pi) = b.board.id_to_index.get(parent)
        && let Some(spec) = board::view(&b.corpus.tickets[pi].ticket).spec
    {
        return spec.to_owned();
    }
    String::new()
}

/// Anchor picker via drag-onto-queued (fingerprint captured at drop time).
pub fn anchor_dialog(b: &BoardState, index: usize) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::AnchorPick {
        id: loaded.ticket.id().to_owned(),
        guard: verbs::guard_for(&loaded.path),
        filter: String::new(),
        selected: None,
    }
}

pub fn add_dialog() -> Dialog {
    Dialog::AddTicket {
        title: String::new(),
        summary: String::new(),
    }
}

pub fn add_child_dialog(b: &BoardState, index: usize, guard: CasGuard) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::AddChild {
        parent: loaded.ticket.id().to_owned(),
        parent_is_work: matches!(loaded.ticket, Ticket::Work(_)),
        guard,
        title: String::new(),
        summary: String::new(),
        promote: false,
    }
}

pub fn remove_dialog(b: &BoardState, index: usize, guard: CasGuard) -> Dialog {
    let loaded = &b.corpus.tickets[index];
    Dialog::Remove {
        id: loaded.ticket.id().to_owned(),
        is_program: matches!(loaded.ticket, Ticket::Program(_)),
        guard,
        force: false,
        typed: String::new(),
    }
}

// ---- card context menu + detail action strip ----

/// Context menu body for one card. The CAS fingerprint is captured at
/// RENDER-of-menu time — recomputed each frame the menu is open (ticket files
/// are 1-2 KB; the read is trivial) so the clicked action carries the freshest
/// pre-image.
pub fn card_menu_ui(
    ui: &mut Ui,
    b: &BoardState,
    mctx: MutCtx<'_>,
    index: usize,
    actions: &mut Vec<Action>,
) {
    let loaded = &b.corpus.tickets[index];
    let guard = verbs::guard_for(&loaded.path);
    let status = loaded.ticket.status().name();
    ui.label(
        RichText::new(format!("{} · {}", loaded.ticket.id(), status.as_str()))
            .monospace()
            .small()
            .weak(),
    );
    ui.separator();
    let offered = verbs::offered_transitions(status);
    if offered.is_empty() {
        ui.label(
            RichText::new(
                "running — the runner's claim; Cancel lives in the detail panel's \
                 Advanced section",
            )
            .weak()
            .small(),
        );
    }
    for t in offered {
        if ui
            .add_enabled(!mctx.busy, Button::new(verbs::transition_label(t)))
            .clicked()
        {
            actions.push(Action::OpenDialog(Box::new(transition_dialog(
                b,
                index,
                t,
                guard.clone(),
            ))));
        }
    }
    ui.separator();
    if ui
        .add_enabled(!mctx.busy, Button::new("Add child…"))
        .clicked()
    {
        actions.push(Action::OpenDialog(Box::new(add_child_dialog(
            b, index, guard,
        ))));
    }
}

/// The detail panel's action strip: the same offered set as the context menu,
/// plus the collapsed Advanced section (raw set-status, advance-slice for
/// programs, remove behind type-to-confirm, and the running-only Cancel).
/// Fingerprints here are captured at CLICK time.
pub fn action_strip_ui(
    ui: &mut Ui,
    b: &BoardState,
    mctx: MutCtx<'_>,
    index: usize,
    advanced_status: &mut Option<StatusName>,
    actions: &mut Vec<Action>,
) {
    let loaded = &b.corpus.tickets[index];
    let status = loaded.ticket.status().name();
    let is_program = matches!(loaded.ticket, Ticket::Program(_));
    let id = loaded.ticket.id().to_owned();

    ui.horizontal_wrapped(|ui| {
        let offered = verbs::offered_transitions(status);
        if offered.is_empty() {
            ui.label(
                RichText::new("running — runner's claim; no manual transitions")
                    .weak()
                    .small(),
            );
        }
        for t in offered {
            if ui
                .add_enabled(!mctx.busy, Button::new(verbs::transition_label(t)).small())
                .clicked()
            {
                let guard = verbs::guard_for(&loaded.path);
                actions.push(Action::OpenDialog(Box::new(transition_dialog(
                    b, index, t, guard,
                ))));
            }
        }
        if ui
            .add_enabled(!mctx.busy, Button::new("Add child…").small())
            .clicked()
        {
            let guard = verbs::guard_for(&loaded.path);
            actions.push(Action::OpenDialog(Box::new(add_child_dialog(
                b, index, guard,
            ))));
        }
    });

    ui.collapsing("Advanced", |ui| {
        for action in verbs::advanced_actions(status, is_program) {
            match action {
                AdvancedAction::RawSetStatus => {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("set-status").monospace().small());
                        ComboBox::from_id_salt("advanced_set_status")
                            .selected_text(advanced_status.map_or("status…", StatusName::as_str))
                            .show_ui(ui, |ui| {
                                for s in board::STATUS_ORDER {
                                    ui.selectable_value(advanced_status, Some(s), s.as_str());
                                }
                            });
                        let pick = *advanced_status;
                        if ui
                            .add_enabled(
                                pick.is_some() && !mctx.busy,
                                Button::new("Dispatch…").small(),
                            )
                            .clicked()
                            && let Some(s) = pick
                        {
                            let guard = verbs::guard_for(&loaded.path);
                            let req = verbs::set_status(&id, s).with_guard(guard);
                            actions.push(Action::OpenDialog(Box::new(Dialog::Confirm {
                                title: format!("set-status — {id} → {}", s.as_str()),
                                note: Some(
                                    "raw set-status — illegal transitions refuse server-side; \
                                     the refusal streams verbatim."
                                        .to_owned(),
                                ),
                                req,
                            })));
                        }
                    });
                }
                AdvancedAction::AdvanceSlice => {
                    if ui
                        .add_enabled(!mctx.busy, Button::new("Advance slice…").small())
                        .clicked()
                    {
                        let guard = verbs::guard_for(&loaded.path);
                        actions.push(Action::OpenDialog(Box::new(Dialog::Confirm {
                            title: format!("Advance slice — {id}"),
                            note: None,
                            req: verbs::advance_slice(&id).with_guard(guard),
                        })));
                    }
                }
                AdvancedAction::Remove => {
                    if ui
                        .add_enabled(!mctx.busy, Button::new("Remove…").small())
                        .clicked()
                    {
                        let guard = verbs::guard_for(&loaded.path);
                        actions.push(Action::OpenDialog(Box::new(remove_dialog(b, index, guard))));
                    }
                }
                AdvancedAction::CancelRunning => {
                    if ui
                        .add_enabled(!mctx.busy, Button::new("Cancel (running)…").small())
                        .clicked()
                    {
                        let guard = verbs::guard_for(&loaded.path);
                        let req = verbs::set_status(&id, StatusName::Cancelled).with_guard(guard);
                        actions.push(Action::OpenDialog(Box::new(Dialog::Confirm {
                            title: format!("Cancel — {id} (running)"),
                            note: Some(
                                "running is the runner's claim — cancel only when you know \
                                 the run is dead. cancelled stamps completed_at."
                                    .to_owned(),
                            ),
                            req,
                        })));
                    }
                }
            }
        }
    });
}

// ---- dialog rendering ----

/// Render the open dialog as a modal. Returns `true` when it should close
/// (Cancel, Esc, backdrop, or a dispatch).
pub fn dialog_ui(
    ctx: &egui::Context,
    b: &BoardState,
    mctx: MutCtx<'_>,
    dialog: &mut Dialog,
    actions: &mut Vec<Action>,
) -> bool {
    let modal = Modal::new(Id::new("t9154_dialog")).show(ctx, |ui| {
        ui.set_min_width(DIALOG_MIN_W);
        match dialog {
            Dialog::Confirm { title, note, req } => {
                confirm_body_ui(ui, mctx, title, note.as_deref(), req, actions)
            }
            Dialog::AnchorPick {
                id,
                guard,
                filter,
                selected,
            } => anchor_body_ui(ui, b, mctx, id, guard, filter, selected, actions),
            Dialog::MarkReady {
                id,
                guard,
                spec,
                stat,
            } => ready_body_ui(ui, b, mctx, id, guard, spec, stat, actions),
            Dialog::AddTicket { title, summary } => add_body_ui(ui, mctx, title, summary, actions),
            Dialog::AddChild {
                parent,
                parent_is_work,
                guard,
                title,
                summary,
                promote,
            } => add_child_body_ui(
                ui,
                mctx,
                parent,
                *parent_is_work,
                guard,
                title,
                summary,
                promote,
                actions,
            ),
            Dialog::Remove {
                id,
                is_program,
                guard,
                force,
                typed,
            } => remove_body_ui(ui, b, mctx, id, *is_program, guard, force, typed, actions),
        }
    });
    modal.inner || modal.should_close()
}

/// The literal command line — the operator sees exactly what runs.
fn command_line_ui(ui: &mut Ui, display: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new("This will run:").weak().small());
    ui.label(RichText::new(display).monospace().strong());
}

/// Standard footer: right-aligned Run (disabled while a verb is in flight or
/// `run_enabled` is false) + Cancel. Returns true to close the dialog.
fn run_cancel_ui(
    ui: &mut Ui,
    mctx: MutCtx<'_>,
    run_enabled: bool,
    req: impl FnOnce() -> VerbRequest,
    actions: &mut Vec<Action>,
) -> bool {
    let mut close = false;
    ui.add_space(8.0);
    if mctx.busy {
        ui.label(
            RichText::new("a verb is already running — dispatch disabled until it exits")
                .weak()
                .small(),
        );
    }
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        if ui
            .add_enabled(run_enabled && !mctx.busy, Button::new("Run"))
            .clicked()
        {
            actions.push(Action::Dispatch(req()));
            close = true;
        }
        if ui.button("Cancel").clicked() {
            close = true;
        }
    });
    close
}

fn confirm_body_ui(
    ui: &mut Ui,
    mctx: MutCtx<'_>,
    title: &str,
    note: Option<&str>,
    req: &VerbRequest,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(title);
    if let Some(note) = note {
        ui.label(RichText::new(note).weak().small());
    }
    command_line_ui(ui, &req.display);
    ui.label(
        RichText::new("refusals stream verbatim in the verb drawer.")
            .weak()
            .small(),
    );
    let req = req.clone();
    run_cancel_ui(ui, mctx, true, move || req, actions)
}

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
fn anchor_body_ui(
    ui: &mut Ui,
    b: &BoardState,
    mctx: MutCtx<'_>,
    id: &str,
    guard: &CasGuard,
    filter: &mut String,
    selected: &mut Option<String>,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(format!("Queue {id} after…"));
    ui.label(
        RichText::new(
            "pick the anchor ticket: the verb sets order = anchor + 1 and flips \
             idea → queued server-side.",
        )
        .weak()
        .small(),
    );
    ui.add_space(4.0);
    ui.add(
        TextEdit::singleline(filter)
            .desired_width(f32::INFINITY)
            .hint_text("filter anchors by id / title"),
    );
    let candidates = anchor_candidates(b, id, filter);
    ui.add_space(4.0);
    if candidates.is_empty() {
        ui.label(RichText::new("no ordered tickets match").weak());
    } else {
        ScrollArea::vertical()
            .id_salt("anchor_pick")
            .max_height(DIALOG_LIST_H)
            .auto_shrink([false, true])
            .show_rows(ui, ANCHOR_ROW_H, candidates.len(), |ui, row_range| {
                for (order, cid, title) in &candidates[row_range] {
                    let row = format!("#{order}  {cid} — {}", board::truncate_chars(title, 56));
                    let on = selected.as_deref() == Some(cid.as_str());
                    if ui
                        .selectable_label(on, RichText::new(row).monospace().small())
                        .clicked()
                    {
                        *selected = Some(cid.clone());
                    }
                }
            });
    }
    match selected.as_deref() {
        Some(anchor) => {
            let req = verbs::reorder(id, anchor).with_guard(guard.clone());
            command_line_ui(ui, &req.display);
            run_cancel_ui(ui, mctx, true, move || req, actions)
        }
        None => {
            ui.add_space(4.0);
            ui.label(
                RichText::new("select an anchor to see the command")
                    .weak()
                    .small(),
            );
            run_cancel_ui(ui, mctx, false, || unreachable!("run disabled"), actions)
        }
    }
}

/// Anchor candidates: every corpus ticket carrying an order (except the ticket
/// itself), `(order, id)`-sorted, filtered on lowercase id+title.
fn anchor_candidates(b: &BoardState, exclude: &str, filter: &str) -> Vec<(i64, String, String)> {
    let needle = filter.trim().to_lowercase();
    let mut out: Vec<(i64, String, String)> = b
        .corpus
        .tickets
        .iter()
        .filter_map(|loaded| {
            let v = board::view(&loaded.ticket);
            let order = v.status.order()?;
            if v.id == exclude {
                return None;
            }
            if !needle.is_empty()
                && !format!("{}\n{}", v.id, v.title)
                    .to_lowercase()
                    .contains(&needle)
            {
                return None;
            }
            Some((order, v.id.to_owned(), v.title.to_owned()))
        })
        .collect();
    out.sort_by_key(|a| (a.0, board::id_sort_key(&a.1)));
    out
}

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
fn ready_body_ui(
    ui: &mut Ui,
    b: &BoardState,
    mctx: MutCtx<'_>,
    id: &str,
    guard: &CasGuard,
    spec: &mut String,
    stat: &mut Option<(String, bool)>,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(format!("Mark {id} ready"));
    ui.label(RichText::new("spec path (repo-relative):").weak().small());
    ui.add(TextEdit::singleline(spec).desired_width(f32::INFINITY));

    // Live existence indicator: re-stat only when the text changed.
    let trimmed = spec.trim().to_owned();
    if trimmed.is_empty() {
        ui.label(
            RichText::new("no spec path — the verb refuses (\"Ticket … needs a spec path\")")
                .color(VERDICT_COLLIDE)
                .small(),
        );
    } else {
        if stat.as_ref().is_none_or(|(s, _)| *s != trimmed) {
            let exists = mctx
                .repo_root
                .is_some_and(|root| root.join(&trimmed).is_file());
            *stat = Some((trimmed.clone(), exists));
        }
        match stat {
            Some((_, true)) => {
                ui.label(
                    RichText::new("spec exists on disk")
                        .color(VERDICT_OK)
                        .small(),
                );
            }
            _ => {
                ui.label(
                    RichText::new(format!(
                        "no file at {} — the verb refuses (\"Spec file not found\")",
                        mctx.repo_root.map_or_else(
                            || trimmed.clone(),
                            |root| root.join(&trimmed).display().to_string()
                        )
                    ))
                    .color(VERDICT_COLLIDE)
                    .small(),
                );
            }
        }
    }

    // Current main_goal / acceptance — READ-ONLY on purpose: the CLI verb
    // takes only id + spec; story/acceptance backfill is the verb's own
    // behavior. The UI must not pretend it can set them.
    if let Some(&index) = b.board.id_to_index.get(id) {
        let v = board::view(&b.corpus.tickets[index].ticket);
        ui.add_space(6.0);
        ui.label(RichText::new("main_goal (current)").strong().small());
        match v.main_goal {
            Some(s) => {
                ui.label(s);
            }
            None => {
                ui.label(RichText::new("—").weak());
            }
        }
        ui.label(
            RichText::new(format!("acceptance (current, {})", v.acceptance.len()))
                .strong()
                .small(),
        );
        if v.acceptance.is_empty() {
            ui.label(RichText::new("—").weak());
        }
        for (i, item) in v.acceptance.iter().enumerate() {
            ui.label(format!("{}. {item}", i + 1));
        }
        ui.label(
            RichText::new(
                "read-only — the verb takes only id + spec and backfills \
                 main_goal/acceptance ONLY when they are empty.",
            )
            .weak()
            .small(),
        );
    }
    ui.label(
        RichText::new(
            "unshipped depends_on refuse server-side (\"Blocked by …\") — the \
             refusal streams verbatim.",
        )
        .weak()
        .small(),
    );

    let spec_arg = (!trimmed.is_empty()).then_some(trimmed.as_str());
    let req = verbs::mark_ready(id, spec_arg).with_guard(guard.clone());
    command_line_ui(ui, &req.display);
    run_cancel_ui(ui, mctx, true, move || req, actions)
}

fn add_body_ui(
    ui: &mut Ui,
    mctx: MutCtx<'_>,
    title: &mut String,
    summary: &mut String,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading("New ticket");
    ui.label(RichText::new("title:").weak().small());
    ui.add(TextEdit::singleline(title).desired_width(f32::INFINITY));
    ui.label(RichText::new("summary:").weak().small());
    ui.add(
        TextEdit::multiline(summary)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    ui.label(
        RichText::new(
            "id is minted by the verb (max parent numeric + 1) · kind work · \
             status idea.",
        )
        .weak()
        .small(),
    );
    let req = verbs::add(title, summary);
    command_line_ui(ui, &req.display);
    let ok = !title.trim().is_empty();
    run_cancel_ui(ui, mctx, ok, move || req, actions)
}

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
fn add_child_body_ui(
    ui: &mut Ui,
    mctx: MutCtx<'_>,
    parent: &str,
    parent_is_work: bool,
    guard: &CasGuard,
    title: &mut String,
    summary: &mut String,
    promote: &mut bool,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(format!("Add child under {parent}"));
    if parent_is_work {
        // The promote checkbox exists ONLY for work parents (design Decisions #4).
        ui.add(Checkbox::new(
            promote,
            RichText::new(format!("--promote — rewrite {parent} work → program")),
        ));
        ui.label(
            RichText::new(
                "the parent is kind work: add-child refuses without --promote. \
                 --promote atomically rewrites it work → program while adding this \
                 first child (the parent's [scope] is dropped — programs forbid \
                 scope).",
            )
            .weak()
            .small(),
        );
    }
    ui.label(RichText::new("title:").weak().small());
    ui.add(TextEdit::singleline(title).desired_width(f32::INFINITY));
    ui.label(RichText::new("summary:").weak().small());
    ui.add(
        TextEdit::multiline(summary)
            .desired_rows(3)
            .desired_width(f32::INFINITY),
    );
    let req = verbs::add_child(parent, title, summary, *promote).with_guard(guard.clone());
    command_line_ui(ui, &req.display);
    let ok = !title.trim().is_empty();
    run_cancel_ui(ui, mctx, ok, move || req, actions)
}

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
fn remove_body_ui(
    ui: &mut Ui,
    b: &BoardState,
    mctx: MutCtx<'_>,
    id: &str,
    is_program: bool,
    guard: &CasGuard,
    force: &mut bool,
    typed: &mut String,
    actions: &mut Vec<Action>,
) -> bool {
    ui.heading(format!("Remove {id}"));
    if is_program && !*force {
        ui.label(
            RichText::new(
                "programs refuse removal without --force (the verb exits 1; the \
                 refusal streams verbatim).",
            )
            .weak()
            .small(),
        );
    }
    ui.add(Checkbox::new(
        force,
        RichText::new("--force — cascade-delete every descendant ticket file")
            .color(VERDICT_COLLIDE),
    ));
    if *force {
        let kids = verbs::descendants(b.corpus.tickets.iter().map(|t| t.ticket.id()), id);
        if kids.is_empty() {
            ui.label(
                RichText::new("no descendant files in the corpus")
                    .weak()
                    .small(),
            );
        } else {
            ui.label(
                RichText::new(format!(
                    "--force will DELETE {} descendant ticket file(s):",
                    kids.len()
                ))
                .color(VERDICT_COLLIDE)
                .strong(),
            );
            ScrollArea::vertical()
                .id_salt("remove_children")
                .max_height(DIALOG_LIST_H)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for kid in &kids {
                        ui.label(
                            RichText::new(kid)
                                .monospace()
                                .small()
                                .color(VERDICT_COLLIDE),
                        );
                    }
                });
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new(format!("type {id} to confirm:")).small());
    ui.add(TextEdit::singleline(typed).desired_width(f32::INFINITY));
    let gate_ok = verbs::remove_gate_ok(typed, id);
    if !gate_ok && !typed.trim().is_empty() {
        ui.label(
            RichText::new("does not match the ticket id")
                .color(VERDICT_COLLIDE)
                .small(),
        );
    }
    let req = verbs::remove(id, *force).with_guard(guard.clone());
    command_line_ui(ui, &req.display);
    run_cancel_ui(ui, mctx, gate_ok, move || req, actions)
}

// ---- drawer + footer chip ----

/// Footer chip: the live "verb running" indicator / last-exit summary; click
/// toggles the drawer.
pub fn verb_chip_ui(ui: &mut Ui, runner: &VerbRunner, actions: &mut Vec<Action>) {
    if runner.queue.busy() {
        let label = format!(
            "verb: {}",
            runner.queue.running_display().unwrap_or("starting…")
        );
        if ui
            .selectable_label(runner.drawer_open, RichText::new(label).small())
            .clicked()
        {
            actions.push(Action::ToggleVerbDrawer);
        }
        ui.add(Spinner::new().size(12.0));
        let pending = runner.queue.pending_len();
        if pending > 0 {
            ui.label(RichText::new(format!("+{pending} pending")).weak().small());
        }
    } else if let Some(last) = &runner.last {
        let (text, color) = outcome_headline(last);
        if ui
            .selectable_label(runner.drawer_open, RichText::new(text).small().color(color))
            .clicked()
        {
            actions.push(Action::ToggleVerbDrawer);
        }
    }
}

fn outcome_headline(last: &VerbOutcome) -> (String, Color32) {
    if let Some(err) = &last.spawn_error {
        return (
            format!("verb did not run — {err} — {}", last.display),
            VERDICT_COLLIDE,
        );
    }
    match last.code {
        Some(0) => (
            format!("exit 0 — {} · {}", last.display, last.at),
            VERDICT_OK,
        ),
        Some(code) => (
            format!("exit {code} — {} · {}", last.display, last.at),
            VERDICT_COLLIDE,
        ),
        None => (
            format!("killed — {} · {}", last.display, last.at),
            VERDICT_COLLIDE,
        ),
    }
}

/// The bottom drawer: streamed log while a verb runs; on a nonzero exit it
/// stays open with the FULL merged stdout+stderr verbatim, the exit code, and
/// (on the wave-stale signature) the recovery command as TEXT.
pub fn drawer_ui(ui: &mut Ui, runner: &VerbRunner, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(" VERB ").strong().monospace());
        if runner.queue.busy() {
            ui.add(Spinner::new().size(12.0));
            ui.label(
                RichText::new(format!(
                    "running — {}",
                    runner.queue.running_display().unwrap_or("starting…")
                ))
                .strong(),
            );
            let pending = runner.queue.pending_len();
            if pending > 0 {
                ui.label(RichText::new(format!("+{pending} pending")).weak().small());
            }
        } else if let Some(last) = &runner.last {
            let (text, color) = outcome_headline(last);
            ui.label(RichText::new(text).color(color).strong());
        } else {
            ui.label(RichText::new("no verb run yet").weak());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("✕").clicked() {
                actions.push(Action::ToggleVerbDrawer);
            }
        });
    });
    if let Some(note) = &runner.dropped_note {
        ui.label(
            RichText::new(note)
                .color(ui.visuals().warn_fg_color)
                .small(),
        );
    }
    if runner.last.as_ref().is_some_and(|l| l.hint) {
        // TEXT ONLY — no button runs this; the app must never repack.
        ui.label(
            RichText::new(verbs::RECOVERY_HINT)
                .color(VERDICT_COLLIDE)
                .strong(),
        );
        ui.label(
            RichText::new(
                "a crashed/refused verb can leave wave.lock stale; the app never \
                 repacks on its own — run the command, then Reload.",
            )
            .weak()
            .small(),
        );
    }
    ui.separator();
    if runner.log.is_empty() {
        ui.label(RichText::new("no output yet").weak().small());
        return;
    }
    ScrollArea::vertical()
        .id_salt("verb_log")
        .max_height(DRAWER_MAX_H)
        .stick_to_bottom(true)
        .auto_shrink([false, true])
        .show_rows(ui, DRAWER_ROW_H, runner.log.len(), |ui, row_range| {
            for line in &runner.log[row_range] {
                ui.label(RichText::new(line).monospace().small());
            }
        });
}
