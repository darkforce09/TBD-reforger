use super::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    services::commands::{self as verbs, FileChangeGuard},
};
use eframe::egui::{Checkbox, RichText, TextEdit, Ui};

pub(crate) fn add_body_ui(
    ui: &mut Ui,
    mctx: MutationContext<'_>,
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
pub(crate) fn add_child_body_ui(
    ui: &mut Ui,
    mctx: MutationContext<'_>,
    parent: &str,
    parent_is_work: bool,
    guard: &FileChangeGuard,
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
