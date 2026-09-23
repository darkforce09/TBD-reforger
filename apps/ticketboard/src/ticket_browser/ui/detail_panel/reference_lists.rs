use super::*;
use crate::ticket_browser::events::BrowserEvent as Action;
use eframe::egui::{RichText, Ui};
use std::collections::HashMap;

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

pub(crate) fn id_list_section(
    ui: &mut Ui,
    title: &str,
    ids_list: &[String],
    ids: &HashMap<String, usize>,
    actions: &mut Vec<Action>,
) {
    section_header(ui, &format!("{title} ({})", ids_list.len()));
    if ids_list.is_empty() {
        missing_marker(ui);
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for id in ids_list {
            id_link_ui(ui, id, ids, actions);
        }
    });
}

pub(crate) fn owns_section(ui: &mut Ui, owns: &[String]) {
    section_header(ui, &format!("owns ({})", owns.len()));
    if owns.is_empty() {
        missing_marker(ui);
        return;
    }
    for path in owns {
        ui.label(RichText::new(path).monospace().small());
    }
}
