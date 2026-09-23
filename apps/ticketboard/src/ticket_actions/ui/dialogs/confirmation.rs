use super::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action, services::commands::TicketCommand,
};
use eframe::egui::{RichText, Ui};

pub(crate) fn confirm_body_ui(
    ui: &mut Ui,
    mctx: MutationContext<'_>,
    title: &str,
    note: Option<&str>,
    req: &TicketCommand,
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
