use crate::core::process::BoundedLog;
use crate::core::ui::*;
use crate::repository_status::models::git_status as gitstatus;
use crate::repository_status::{
    events::StatusEvent as Action,
    models::{
        check_status::{self as trust, Tone},
        git_status::GitChip,
        view::StatusView,
    },
};
use eframe::egui::{Align, Color32, Layout, RichText, ScrollArea, Spinner, Ui};

pub(crate) const OUTPUT_MAX_H: f32 = 260.0;

pub(crate) const GIT_LIST_MAX_H: f32 = 160.0;

// ---- chrome ----

/// Banner accent per tone. Raw egui text colors elsewhere; the green/red pair
/// reuses the verdict palette so "check OK" and "no collision" read the same.
pub(crate) fn tone_color(ui: &Ui, tone: Tone) -> Color32 {
    match tone {
        Tone::Neutral => ui.visuals().weak_text_color(),
        Tone::Busy => Color32::from_rgb(245, 175, 80),
        Tone::Green => VERDICT_OK,
        Tone::Red => VERDICT_COLLIDE,
    }
}

/// Git-dirty chip: subdued "clean", loud "N uncommitted registry file(s)" (click
/// expands the verbatim porcelain list), "git unavailable" when git is absent or
/// refuses — never a crash, never a fake clean.
pub(crate) fn git_chip_ui(ui: &mut Ui, chip: &GitChip, expanded: bool, actions: &mut Vec<Action>) {
    let dirty = matches!(chip, GitChip::Dirty(_));
    let text = match chip {
        GitChip::Dirty(_) => RichText::new(chip.label()).color(ui.visuals().warn_fg_color),
        GitChip::Unavailable(_) => RichText::new(chip.label()).weak().italics(),
        GitChip::Clean | GitChip::Unknown => RichText::new(chip.label()).weak(),
    };
    let response = ui.selectable_label(expanded && dirty, text);
    let response = match chip {
        GitChip::Unavailable(reason) => response.on_hover_text(reason),
        _ => response.on_hover_text(format!("git {}", gitstatus::GIT_ARGS.join(" "))),
    };
    if response.clicked() && dirty {
        actions.push(Action::ToggleGitList);
    }
}

/// Verbatim merged stdout+stderr of the strict check — the last ~500 lines,
/// never paraphrased; drops are named, not hidden.
pub(crate) fn output_pane_ui(ui: &mut Ui, log: &BoundedLog) {
    ui.separator();
    if log.dropped() > 0 {
        ui.label(
            RichText::new(format!("… {} earlier line(s) dropped", log.dropped()))
                .weak()
                .small(),
        );
    }
    if log.is_empty() {
        ui.label(RichText::new("no output yet").weak().small());
        return;
    }
    ScrollArea::vertical()
        .id_salt("check_output")
        .max_height(OUTPUT_MAX_H)
        .stick_to_bottom(true)
        .auto_shrink([false, true])
        .show_rows(ui, OUTPUT_ROW_H, log.len(), |ui, row_range| {
            for line in log.lines().skip(row_range.start).take(row_range.len()) {
                ui.label(RichText::new(line).monospace().small());
            }
        });
}

/// The expanded git-dirty file list — porcelain entries verbatim (`XY path`).
pub(crate) fn git_list_ui(ui: &mut Ui, files: &[String]) {
    ui.separator();
    ScrollArea::vertical()
        .id_salt("git_dirty_list")
        .max_height(GIT_LIST_MAX_H)
        .auto_shrink([false, true])
        .show_rows(ui, OUTPUT_ROW_H, files.len(), |ui, row_range| {
            for line in &files[row_range] {
                ui.label(RichText::new(line).monospace().small());
            }
        });
}
pub(crate) fn trust_banner_ui(b: &StatusView<'_>, ui: &mut Ui, actions: &mut Vec<Action>) {
    let (headline, tone) = b.check.banner();
    ui.horizontal(|ui| {
        let color = tone_color(ui, tone);
        // STRICT, prominently — this banner is the --strict bar, NOT the
        // (non-strict) mutator preflight.
        ui.label(
            RichText::new(" STRICT ")
                .strong()
                .monospace()
                .background_color(color.gamma_multiply(0.22))
                .color(color),
        )
        .on_hover_text(trust::STRICT_TOOLTIP);
        if b.check_running {
            ui.add(Spinner::new().size(12.0));
        }
        ui.label(RichText::new(&headline).color(color).strong());
        if ui
            .button("Re-check")
            .on_hover_text(trust::CHECK_COMMAND)
            .clicked()
        {
            actions.push(Action::Recheck);
        }
        if b.check_running && ui.small_button("✕ cancel").clicked() {
            actions.push(Action::CancelCheck);
        }
        let output_label = format!("output ({})", b.check_log.len());
        if ui
            .selectable_label(b.show_output, RichText::new(output_label).small())
            .clicked()
        {
            actions.push(Action::ToggleOutput);
        }
        if let Some(error) = &b.watch_error {
            ui.label(
                RichText::new("watch unavailable")
                    .color(ui.visuals().warn_fg_color)
                    .small(),
            )
            .on_hover_text(*error);
        } else if !b.degraded_watches.is_empty() {
            ui.label(RichText::new("watch degraded").weak().small())
                .on_hover_text(b.degraded_watches.join("\n"));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            git_chip_ui(ui, b.git_chip, b.git_expanded, actions);
        });
    });
    if b.show_output {
        output_pane_ui(ui, b.check_log);
    }
    if b.git_expanded
        && let GitChip::Dirty(files) = b.git_chip
    {
        git_list_ui(ui, files);
    }
}
