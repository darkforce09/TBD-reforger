use super::*;
use crate::core::ui::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    services::commands::{self as verbs, FileChangeGuard},
};
use crate::ticket_registry::models::projection as board;
use eframe::egui::{RichText, TextEdit, Ui};

#[expect(clippy::too_many_arguments)] // dialog fields destructured by the one caller
pub(crate) fn ready_body_ui(
    ui: &mut Ui,
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
    id: &str,
    guard: &FileChangeGuard,
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
    if let Some(&index) = b.id_to_index.get(id) {
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
