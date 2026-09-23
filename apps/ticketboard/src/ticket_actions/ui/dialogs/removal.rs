use super::*;
use crate::core::ui::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    services::commands::{self as verbs, FileChangeGuard},
};
use eframe::egui::{Checkbox, RichText, ScrollArea, TextEdit, Ui};

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
pub(crate) fn remove_body_ui(
    ui: &mut Ui,
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
    id: &str,
    is_program: bool,
    guard: &FileChangeGuard,
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
