pub(crate) mod appearance;
pub(crate) mod detail_panel;
pub(crate) mod filter_bar;
pub(crate) mod program_tree;
pub(crate) mod status_board;
pub(crate) mod ticket_card;
use appearance::*;
use eframe::egui::Ui;
use ticket_card::card_ui;
use ticketboard_model::ticket_browser::events::BrowserEvent;
fn select_or_compare(ui: &Ui, index: usize) -> BrowserEvent {
    if ui.input(|i| i.modifiers.shift) {
        BrowserEvent::Compare(index)
    } else {
        BrowserEvent::Select(index)
    }
}

pub(crate) type TicketMenu<'a> = dyn FnMut(&mut Ui, usize) -> Vec<ticketboard_model::ticket_actions::events::TicketActionEvent>
    + 'a;
pub(crate) type TicketActionStrip<'a> =
    dyn FnMut(&mut Ui) -> Vec<ticketboard_model::ticket_actions::events::TicketActionEvent> + 'a;
