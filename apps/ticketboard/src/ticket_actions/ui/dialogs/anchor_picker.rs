use super::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    services::{
        commands::{self as verbs, FileChangeGuard},
        dialog_builders::*,
    },
};
use crate::ticket_registry::models::projection as board;
use eframe::egui::{RichText, ScrollArea, TextEdit, Ui};

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
pub(crate) fn anchor_body_ui(
    ui: &mut Ui,
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
    id: &str,
    guard: &FileChangeGuard,
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
