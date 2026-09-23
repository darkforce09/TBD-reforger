use std::path::PathBuf;
pub(crate) enum BrowserEvent {
    Select(usize),
    SelectId(String),
    Compare(usize),
    ClearCompare,
    ToggleColumn(usize),
    OpenPath(PathBuf),
    OpenDoc(String),
    CloseDetail,
    ToggleNode(usize),
    CopyText(String),
    ToggleLegacyExpand,
    OpenAnchorDialog(usize),
    TicketAction(crate::ticket_actions::events::TicketActionEvent),
}
