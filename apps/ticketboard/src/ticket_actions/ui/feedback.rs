use super::*;
use crate::core::ui::*;
use crate::ticket_actions::{
    events::TicketActionEvent as Action,
    models::*,
    services::commands::{self as verbs},
};
use eframe::egui::{
    self, Align, Align2, Area, Color32, Frame, Id, Layout, Order, RichText, ScrollArea, Spinner,
    Ui, vec2,
};
use std::time::Instant;

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

// ---- drawer + footer chip ----

/// Footer chip: the live "verb running" indicator / last-exit summary; click
/// toggles the drawer.
pub fn verb_chip_ui(ui: &mut Ui, runner: &CommandExecutionState, actions: &mut Vec<Action>) {
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

pub(crate) fn outcome_headline(last: &CommandOutcome) -> (String, Color32) {
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
pub fn drawer_ui(ui: &mut Ui, runner: &CommandExecutionState, actions: &mut Vec<Action>) {
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
