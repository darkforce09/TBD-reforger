use super::*;
use crate::ticket_browser::{
    events::BrowserEvent as Action,
    models::{program_tree as tree, view::BrowserView},
};
use eframe::egui::{RichText, ScrollArea, Ui};

pub(crate) const TREE_ROW_H: f32 = 18.0;

pub(crate) const TREE_INDENT: f32 = 16.0;

// ---- program tree ----

pub(crate) fn tree_ui(ui: &mut Ui, b: &BrowserView<'_>, actions: &mut Vec<Action>) {
    ScrollArea::vertical()
        .id_salt("tree")
        .auto_shrink([false, false])
        .show_rows(ui, TREE_ROW_H, b.tree_flat.len(), |ui, row_range| {
            for row in &b.tree_flat[row_range] {
                tree_row_ui(ui, b, *row, actions);
            }
        });
}

pub(crate) fn tree_row_ui(
    ui: &mut Ui,
    b: &BrowserView<'_>,
    row: tree::FlatRow,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        ui.add_space(f32::from(row.depth) * TREE_INDENT);
        if row.has_children {
            let glyph = if row.expanded { "▼" } else { "▶" };
            if ui.small_button(glyph).clicked() {
                actions.push(Action::ToggleNode(row.index));
            }
        } else {
            ui.add_space(24.0);
        }
        let ticket = &b.corpus.tickets[row.index].ticket;
        let mut color = status_color(ticket.status().name());
        if row.dimmed {
            color = color.gamma_multiply(0.45);
        }
        let selected_now = b.selected == Some(row.index) || b.compare == Some(row.index);
        let response = ui.selectable_label(
            selected_now,
            RichText::new(ticket.id()).monospace().color(color),
        );
        if response.clicked() {
            actions.push(select_or_compare(ui, row.index));
        }
        ui.label(RichText::new(&b.tree.titles[row.index]).weak().small());
    });
}
