use crate::core::ui::*;
use crate::document_viewer::{
    events::DocumentEvent as Action, services::document_loading::ViewerState,
};
use eframe::egui::{Align, Layout, RichText, ScrollArea, Spinner, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::{Path, PathBuf};

/// Render a read-only document column beside the ticket detail panel. Back closes
/// only this column. The header also offers external opening; the body shows Markdown,
/// a labeled raw-text fallback, or progress while a worker reads.
pub(crate) fn viewer_pane_ui(
    ui: &mut Ui,
    state: &ViewerState,
    cache: &mut CommonMarkCache,
    repo_root: Option<&Path>,
    actions: &mut Vec<Action>,
) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui
            .button("← Back")
            .on_hover_text("collapse the viewer column (selection unchanged)")
            .clicked()
        {
            actions.push(Action::CloseViewer);
        }
        if let Some(path) = state.path() {
            ui.label(RichText::new(path).monospace().small());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if let Some(path) = state.path()
                && ui
                    .small_button("open externally")
                    .on_hover_text("OS handler (xdg-open) — the whole file, outside the board")
                    .clicked()
            {
                let abs = match repo_root {
                    Some(root) => root.join(path),
                    None => PathBuf::from(path),
                };
                actions.push(Action::OpenPath(abs));
            }
        });
    });
    ui.separator();
    match state {
        ViewerState::Closed => {}
        ViewerState::Loading { path } => {
            ui.horizontal(|ui| {
                ui.add(Spinner::new().size(14.0));
                ui.label(RichText::new(format!("reading {path}…")).weak());
            });
        }
        ViewerState::Rendered { text, .. } => {
            ScrollArea::vertical()
                .id_salt("viewer_md")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    CommonMarkViewer::new().show(ui, cache, text);
                    ui.add_space(12.0);
                });
        }
        ViewerState::Fallback { text, note, .. } => {
            ui.label(
                RichText::new(note)
                    .color(ui.visuals().warn_fg_color)
                    .small(),
            );
            ui.separator();
            if text.is_empty() {
                ui.label(RichText::new("nothing was read").weak().small());
                return;
            }
            // Raw monospace lines, virtualized (the fallback text is capped at
            // SIZE_CAP_BYTES, so the per-frame line count/skip walk is bounded).
            let total = text.lines().count();
            ScrollArea::vertical()
                .id_salt("viewer_raw")
                .auto_shrink([false, false])
                .show_rows(ui, OUTPUT_ROW_H, total, |ui, row_range| {
                    for line in text.lines().skip(row_range.start).take(row_range.len()) {
                        ui.label(RichText::new(line).monospace().small());
                    }
                });
        }
    }
}
