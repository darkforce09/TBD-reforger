use super::*;
use crate::core::ui::*;
use crate::execution_metrics::{
    events::MetricsEvent as Action, measured::MetricsState, models::MetricsView,
};
use eframe::egui::{ScrollArea, Ui};

// ---- metrics dashboard (measured + estimated) ----

/// Render measured receipts and historical estimates in separate panels with
/// independent totals, table sets, sort state, and provenance colors.
pub(crate) fn metrics_ui(ui: &mut Ui, b: &MetricsView<'_>, actions: &mut Vec<Action>) {
    ScrollArea::vertical()
        .id_salt("metrics")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);
            panel_badge_ui(ui, " MEASURED — run receipts ", VERDICT_OK);
            match &b.metrics {
                MetricsState::NoReceipts => metrics_empty_ui(ui),
                MetricsState::Loaded(m) => metrics_body_ui(ui, b, m, actions),
            }
            // The hard boundary between measured and estimated — a double rule,
            // never a shared table edge.
            ui.add_space(16.0);
            ui.separator();
            ui.add_space(1.0);
            ui.separator();
            ui.add_space(6.0);
            estimated_panel_ui(ui, b, actions);
            ui.add_space(12.0);
        });
}
