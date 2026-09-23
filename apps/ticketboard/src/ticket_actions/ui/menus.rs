use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    models::*,
    services::{
        commands::{self as verbs, AdvancedAction},
        dialog_builders::*,
    },
};
use crate::ticket_registry::models::projection as board;
use eframe::egui::{Button, ComboBox, RichText, Ui};
use ticket_engine::{StatusName, Ticket};

// ---- card context menu + detail action strip ----

/// Context menu body for one card. The CAS fingerprint is captured at
/// RENDER-of-menu time — recomputed each frame the menu is open (ticket files
/// are 1-2 KB; the read is trivial) so the clicked action carries the freshest
/// pre-image.
pub fn card_menu_ui(
    ui: &mut Ui,
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
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
    b: &TicketActionContext<'_>,
    mctx: MutationContext<'_>,
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
