use super::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action, models::*, services::commands::TicketCommand,
};
use eframe::egui::{self, Align, Button, Id, Layout, Modal, RichText, Ui};
pub(crate) mod anchor_picker;
pub(crate) mod confirmation;
pub(crate) mod mark_ready;
pub(crate) mod removal;
pub(crate) mod ticket_creation;
use anchor_picker::*;
use confirmation::*;
use mark_ready::*;
use removal::*;
use ticket_creation::*;
// ---- dialog rendering ----

/// Render the open dialog as a modal. Returns `true` when it should close
/// (Cancel, Esc, backdrop, or a dispatch).
pub fn dialog_ui(
    ctx: &egui::Context,
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
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
pub(crate) fn command_line_ui(ui: &mut Ui, display: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new("This will run:").weak().small());
    ui.label(RichText::new(display).monospace().strong());
}

/// Standard footer: right-aligned Run (disabled while a verb is in flight or
/// `run_enabled` is false) + Cancel. Returns true to close the dialog.
pub(crate) fn run_cancel_ui(
    ui: &mut Ui,
    mctx: MutationContext<'_>,
    run_enabled: bool,
    req: impl FnOnce() -> TicketCommand,
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
