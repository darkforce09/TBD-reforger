use crate::ticket_registry::models::palette::status_rgb;
use crate::wave_plan::{
    events::WavePlanEvent as Action,
    models::{
        view::WavePlanView,
        wave_projection::{Lane, Wave0, WaveChip, WavesModel},
    },
    services::lock_file::LockState,
};
use eframe::egui::{Color32, RichText, ScrollArea, Ui};
use std::path::Path;
use ticket_engine::StatusName;
fn status_color(status: StatusName) -> Color32 {
    let (r, g, b) = status_rgb(status);
    Color32::from_rgb(r, g, b)
}
/// Plain click selects; shift-click picks the comparison ticket.
fn select_or_compare(ui: &Ui, index: usize) -> Action {
    if ui.input(|i| i.modifiers.shift) {
        Action::Compare(index)
    } else {
        Action::Select(index)
    }
}

pub(crate) const WAVE0_ROW_H: f32 = 18.0;

pub(crate) const WAVE0_LIST_MAX_H: f32 = 320.0;

// ---- waves ----

pub(crate) fn waves_ui(ui: &mut Ui, b: &WavePlanView<'_>, actions: &mut Vec<Action>) {
    match &b.lock {
        LockState::Missing { message } => lock_missing_ui(ui, message),
        LockState::Refused { path, error } => lock_refused_ui(ui, path, error),
        LockState::Loaded(_) => {
            if let Some(model) = &b.waves {
                waves_body_ui(ui, b, model, actions);
            }
        }
    }
}

/// A deleted/renamed wave.lock renders the DidNotRun refusal — never empty lanes
/// .
pub(crate) fn lock_missing_ui(ui: &mut Ui, message: &str) {
    ui.add_space(24.0);
    ui.heading("No wave plan");
    ui.add_space(8.0);
    // The DidNotRun refusal, VERBATIM (mirrors wave_lock::missing_lock_error).
    ui.label(RichText::new(message).monospace().size(14.0));
    ui.add_space(8.0);
    ui.label(
        RichText::new("The lock is rendered verbatim; the app never recomputes packing.")
            .weak()
            .small(),
    );
}

pub(crate) fn lock_refused_ui(ui: &mut Ui, path: &Path, error: &str) {
    ui.add_space(24.0);
    ui.heading("wave.lock refused to parse");
    ui.add_space(8.0);
    ui.label(
        RichText::new(path.display().to_string())
            .monospace()
            .size(15.0)
            .strong(),
    );
    ui.add_space(8.0);
    ScrollArea::vertical()
        .id_salt("lock_error")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // The parse error, VERBATIM — never paraphrased.
            ui.label(RichText::new(error).monospace());
        });
}

pub(crate) fn waves_body_ui(
    ui: &mut Ui,
    b: &WavePlanView<'_>,
    model: &WavesModel,
    actions: &mut Vec<Action>,
) {
    ScrollArea::vertical()
        .id_salt("waves")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);
            // Header strip: wave_base / max_concurrent / pack_last, off the lock.
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&model.header).monospace().strong());
                ui.separator();
                ui.label(RichText::new("pack_last:").weak().small());
                if model.pack_last.is_empty() {
                    ui.label(RichText::new("—").weak());
                }
                for chip in &model.pack_last {
                    wave_chip_ui(ui, b, chip, actions);
                }
            });
            ui.separator();
            for lane in &model.lanes {
                lane_ui(ui, b, lane, actions);
            }
            if let Some(w0) = &model.wave0 {
                wave0_ui(ui, b, w0, actions);
            }
            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("Unplanned").strong());
            ui.label(
                RichText::new(
                    "derived from the ticket files, not from the lock — dispatchable ids \
                     absent from every lock wave",
                )
                .weak()
                .small(),
            );
            if model.unplanned.is_empty() {
                ui.label(RichText::new("—").weak());
            } else {
                ui.horizontal_wrapped(|ui| {
                    for chip in &model.unplanned {
                        wave_chip_ui(ui, b, chip, actions);
                    }
                });
            }
            ui.add_space(12.0);
        });
}

pub(crate) fn lane_ui(ui: &mut Ui, b: &WavePlanView<'_>, lane: &Lane, actions: &mut Vec<Action>) {
    // The lock's wave number salts the lane's widget ids (stable across repaints).
    ui.push_id(lane.n, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new(&lane.label).strong());
            // The acceptance surface: paste against the lock's [[waves]] block.
            if ui.small_button("copy TSV").clicked() {
                actions.push(Action::CopyText(lane.tsv.clone()));
            }
        });
        ui.horizontal_wrapped(|ui| {
            for chip in &lane.chips {
                wave_chip_ui(ui, b, chip, actions);
            }
        });
    });
}

/// Wave 0 — ALWAYS a count chip; click expands a virtualized flat id list, never
/// cards.
pub(crate) fn wave0_ui(ui: &mut Ui, b: &WavePlanView<'_>, w0: &Wave0, actions: &mut Vec<Action>) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("wave 0").strong());
        if ui.button(&w0.label).clicked() {
            actions.push(Action::ToggleWave0);
        }
        if ui.small_button("copy TSV").clicked() {
            actions.push(Action::CopyText(w0.tsv.clone()));
        }
    });
    if b.wave0_expanded {
        ScrollArea::vertical()
            .id_salt("wave0_ids")
            .max_height(WAVE0_LIST_MAX_H)
            .auto_shrink([false, true])
            .show_rows(ui, WAVE0_ROW_H, w0.chips.len(), |ui, row_range| {
                for chip in &w0.chips[row_range] {
                    wave_chip_ui(ui, b, chip, actions);
                }
            });
    }
}

/// One lock-verbatim ticket chip: status-colored when the ticket file exists,
/// struck through when the lock names an id with no file (display-only, no
/// judgment). Active filters dim non-matching chips instead of hiding them — the
/// lane must always show the lock's exact membership.
pub(crate) fn wave_chip_ui(
    ui: &mut Ui,
    b: &WavePlanView<'_>,
    chip: &WaveChip,
    actions: &mut Vec<Action>,
) {
    let dimmed = b.filters_active && chip.corpus_index.is_none_or(|i| !b.matches[i]);
    let mut text = RichText::new(&chip.id).monospace();
    match chip.status {
        Some(status) => {
            let mut color = status_color(status);
            if dimmed {
                color = color.gamma_multiply(0.35);
            }
            text = text.color(color);
        }
        None => text = text.strikethrough().weak(),
    }
    let selected_now = chip.corpus_index.is_some()
        && (b.selected == chip.corpus_index || b.compare == chip.corpus_index);
    let response = ui
        .selectable_label(selected_now, text)
        .on_hover_text(chip.tooltip.as_str());
    if response.clicked()
        && let Some(index) = chip.corpus_index
    {
        actions.push(select_or_compare(ui, index));
    }
}
