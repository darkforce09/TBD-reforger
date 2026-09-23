pub(crate) mod dashboard;
pub(crate) mod estimated_tables;
pub(crate) mod measured_tables;
use super::events::MetricsEvent as Action;
use eframe::egui::{Color32, RichText, Ui};
use estimated_tables::*;
use measured_tables::*;
use std::collections::HashMap;
/// A tinted panel badge — the measured (green) vs estimated (amber) header
/// distinction the acceptance asks for.
pub(crate) fn panel_badge_ui(ui: &mut Ui, label: &str, color: Color32) {
    ui.label(
        RichText::new(label)
            .strong()
            .monospace()
            .background_color(color.gamma_multiply(0.18))
            .color(color),
    );
}

/// Clickable when the id exists in the corpus; plain monospace when dangling.
pub(crate) fn id_link_ui(
    ui: &mut Ui,
    id: &str,
    ids: &HashMap<String, usize>,
    actions: &mut Vec<Action>,
) {
    if crate::core::ui::identifier_link(ui, id, ids.contains_key(id)) {
        actions.push(Action::SelectId(id.to_owned()));
    }
}
