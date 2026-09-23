use super::*;
use crate::ticket_actions::{
    models::TicketActionContext,
    services::dialog_builders,
    ui::{dialogs, feedback},
};
pub(super) use dialog_builders::add_dialog;
pub(super) use feedback::toasts_ui;
pub(super) fn context(b: &WorkspaceState) -> TicketActionContext<'_> {
    TicketActionContext {
        corpus: &b.corpus,
        id_to_index: &b.board.id_to_index,
    }
}
pub(super) fn dialog_ui(
    ctx: &egui::Context,
    b: &WorkspaceState,
    mctx: MutationContext<'_>,
    dialog: &mut Dialog,
    actions: &mut Vec<Action>,
) -> bool {
    let mut events = Vec::new();
    let close = dialogs::dialog_ui(ctx, &context(b), mctx, dialog, &mut events);
    actions.extend(events.into_iter().map(Into::into));
    close
}
pub(super) fn verb_chip_ui(ui: &mut Ui, runner: &CommandExecutionState, actions: &mut Vec<Action>) {
    let mut events = Vec::new();
    feedback::verb_chip_ui(ui, runner, &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn drawer_ui(ui: &mut Ui, runner: &CommandExecutionState, actions: &mut Vec<Action>) {
    let mut events = Vec::new();
    feedback::drawer_ui(ui, runner, &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
