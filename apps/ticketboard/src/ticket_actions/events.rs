use super::{models::Dialog, services::commands::TicketCommand};
pub(crate) enum TicketActionEvent {
    OpenDialog(Box<Dialog>),
    Dispatch(TicketCommand),
    ToggleVerbDrawer,
}
