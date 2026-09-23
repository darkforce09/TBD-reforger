use super::*;
use crate::ticket_browser::{
    events::BrowserEvent as Action,
    models::{
        status_board::{self},
        view::{BrowserView, DraggedTicket},
    },
};
use crate::ticket_registry::models::projection as board;
use eframe::egui::{DragAndDrop, RichText, ScrollArea, Sense, StrokeKind, Ui};
use ticket_engine::StatusName;

/// Card height includes the title, identifier, executor, and scope breadcrumb rows.
pub(crate) const CARD_H: f32 = 64.0;

pub(crate) const CARD_GAP: f32 = 6.0;

pub(crate) const COL_W: f32 = 236.0;

pub(crate) const CHIP_COL_W: f32 = 92.0;

// ---- board ----

pub(crate) fn board_ui(
    ui: &mut Ui,
    b: &BrowserView<'_>,
    mutation_busy: bool,
    ticket_menu: &mut TicketMenu<'_>,
    actions: &mut Vec<Action>,
) {
    ScrollArea::horizontal()
        .id_salt("board")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                for (col_index, column) in b.board.columns.iter().enumerate() {
                    if b.expanded[col_index] {
                        column_ui(ui, b, col_index, mutation_busy, ticket_menu, actions);
                    } else {
                        chip_column_ui(ui, col_index, column, actions);
                    }
                }
            });
        });
}

pub(crate) fn column_ui(
    ui: &mut Ui,
    b: &BrowserView<'_>,
    col_index: usize,
    mutation_busy: bool,
    ticket_menu: &mut TicketMenu<'_>,
    actions: &mut Vec<Action>,
) {
    let column = &b.board.columns[col_index];
    let visible = &b.visible[col_index];
    let selected = b.selected;
    ui.push_id(col_index, |ui| {
        let inner = ui.vertical(|ui| {
            ui.set_width(COL_W);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&column.header).strong());
                if board::collapsed_by_default(column.status) && ui.small_button("−").clicked() {
                    actions.push(Action::ToggleColumn(col_index));
                }
            });
            ui.separator();
            ui.spacing_mut().item_spacing.y = CARD_GAP;
            ScrollArea::vertical()
                .id_salt("cards")
                .auto_shrink([false, false])
                .show_rows(ui, CARD_H, visible.len(), |ui, row_range| {
                    for &row in &visible[row_range] {
                        let card = &column.cards[row];
                        // idea cards drag onto the queued column — the
                        // drop opens the same anchor picker as "Queue after…".
                        let draggable = !mutation_busy && column.status == StatusName::Idea;
                        let response = card_ui(ui, card, selected == Some(card.index), draggable);
                        if response.clicked() {
                            actions.push(select_or_compare(ui, card.index));
                        }
                        if draggable && response.drag_started() {
                            DragAndDrop::set_payload(ui.ctx(), DraggedTicket(card.index));
                        }
                        response.context_menu(|ui| {
                            actions.extend(
                                ticket_menu(ui, card.index)
                                    .into_iter()
                                    .map(Action::TicketAction),
                            );
                        });
                    }
                });
        });
        // The queued column is the drag target (drag-target only — no
        // drag-to-position; the popup anchor picker is the acceptance surface).
        if column.status == StatusName::Queued {
            let rect = inner.response.rect;
            let drop = ui.interact(rect, ui.id().with("queued_drop"), Sense::hover());
            if drop.dnd_hover_payload::<DraggedTicket>().is_some() {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    ui.visuals().selection.stroke,
                    StrokeKind::Inside,
                );
            }
            if let Some(payload) = drop.dnd_release_payload::<DraggedTicket>()
                && !mutation_busy
            {
                actions.push(Action::OpenAnchorDialog(payload.0));
            }
        }
    });
}

/// Collapsed `shipped` / `cancelled` column: a count chip; click expands.
pub(crate) fn chip_column_ui(
    ui: &mut Ui,
    col_index: usize,
    column: &status_board::Column,
    actions: &mut Vec<Action>,
) {
    ui.push_id(col_index, |ui| {
        ui.vertical(|ui| {
            ui.set_width(CHIP_COL_W);
            if ui.button(&column.chip).clicked() {
                actions.push(Action::ToggleColumn(col_index));
            }
        });
    });
}
