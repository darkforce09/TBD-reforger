//! The window's tabs and the actions a frame collects.
//!
//! **Role:** `Tab`, `TABS` and `Action`, with one `From` conversion per feature's events.
//! **Position:** feature events (`BrowserEvent`, `WavePlanEvent`, `MetricsEvent`, `DocumentEvent`,
//! `StatusEvent`, `TicketActionEvent`) convert into `Action`s here; the desktop application's
//! action dispatch applies them after painting.
//! **Signals & state:** none; plain enums.
//! **Invariants:** every feature event has exactly one `Action` meaning, so a view never reaches
//! application state directly.

use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The four main tabs of the window.
pub enum Tab {
    #[default]
    /// The status board.
    Board,
    /// The recorded wave lanes.
    Waves,
    /// The program tree.
    Tree,
    /// The measured and estimated metrics.
    Metrics,
}

/// Each tab with its label, in display order.
pub const TABS: [(Tab, &str); 4] = [
    (Tab::Board, "Board"),
    (Tab::Waves, "Waves"),
    (Tab::Tree, "Tree"),
    (Tab::Metrics, "Metrics"),
];

/// UI events, collected during paint and applied afterwards.
pub enum Action {
    /// Reload the corpus, the lock, the receipts, the estimates and the vocabulary.
    Reload,
    /// Manual trust-banner re-run (coalesced while a check is in flight).
    Recheck,
    /// Kill the in-flight strict check.
    CancelCheck,
    /// Show or hide the strict check's verbatim output.
    ToggleOutput,
    /// Expand or collapse the `git status` file list.
    ToggleGitList,
    /// Open the native folder picker for a repository root.
    PickFolder,
    /// Select the ticket at this corpus index.
    Select(usize),
    /// Select the ticket with this id (a link or a metrics row).
    SelectId(String),
    /// Shift-click: pick the second ticket of the owns-collision pair.
    Compare(usize),
    /// Clear the comparison selection.
    ClearCompare,
    /// Expand or collapse the status column at this index.
    ToggleColumn(usize),
    /// Open this path with the operating system's handler.
    OpenPath(PathBuf),
    /// open a repo-relative `.md` document in the in-app viewer pane
    /// (spec/plan/citation click; non-`.md` paths stay on [`Action::OpenPath`]).
    OpenDoc(String),
    /// Back, semantics: collapse the viewer COLUMN only — the
    /// detail column keeps rendering the untouched board selection beside it.
    CloseViewer,
    /// Close the detail column.
    CloseDetail,
    /// Switch to this tab.
    SetTab(Tab),
    /// Expand or collapse the tree node at this corpus index.
    ToggleNode(usize),
    /// Show or hide the wave 0 id list.
    ToggleWave0,
    /// Copy text to the clipboard: a wave lane's `n<TAB>id` lines or the triage block.
    CopyText(String),
    /// Expand/collapse the quarantined `migration_legacy` lines (detail panel).
    ToggleQuarantineExpand,
    /// The filter selections changed: recompute the verdicts and visible rows.
    FiltersChanged,
    /// Metrics table header click: toggle/replace that table's sort.
    SortMetrics(TableKind, metrics::SortKey),
    /// Estimated-table header click — a SEPARATE action over the
    /// separate estimated model; measured and estimated sorts never share state.
    SortEstimates(EstimatedTableKind, estimates::EstimatedSortKey),
    // ---- mutation surface ----
    // Open a mutation dialog (built at click/menu-render time, guard included).
    /// Open this mutation dialog.
    OpenDialog(Box<Dialog>),
    /// Dispatch a verb: CAS-check, then feed the single-flight queue. Closes the
    /// dialog.
    Dispatch(verbs::TicketCommand),
    /// Show or hide the ticket-command output drawer.
    ToggleVerbDrawer,
    /// Open the anchor picker for the ticket at this corpus index.
    OpenAnchorDialog(usize),
}

impl From<crate::ticket_browser::events::BrowserEvent> for Action {
    fn from(event: crate::ticket_browser::events::BrowserEvent) -> Self {
        use crate::ticket_browser::events::BrowserEvent;
        match event {
            BrowserEvent::Select(value0) => Self::Select(value0),
            BrowserEvent::SelectId(value0) => Self::SelectId(value0),
            BrowserEvent::Compare(value0) => Self::Compare(value0),
            BrowserEvent::ClearCompare => Self::ClearCompare,
            BrowserEvent::ToggleColumn(value0) => Self::ToggleColumn(value0),
            BrowserEvent::OpenPath(value0) => Self::OpenPath(value0),
            BrowserEvent::OpenDoc(value0) => Self::OpenDoc(value0),
            BrowserEvent::CloseDetail => Self::CloseDetail,
            BrowserEvent::ToggleNode(value0) => Self::ToggleNode(value0),
            BrowserEvent::CopyText(value0) => Self::CopyText(value0),
            BrowserEvent::ToggleQuarantineExpand => Self::ToggleQuarantineExpand,
            BrowserEvent::OpenAnchorDialog(value0) => Self::OpenAnchorDialog(value0),
            BrowserEvent::TicketAction(event) => event.into(),
        }
    }
}

impl From<crate::wave_plan::events::WavePlanEvent> for Action {
    fn from(event: crate::wave_plan::events::WavePlanEvent) -> Self {
        use crate::wave_plan::events::WavePlanEvent;
        match event {
            WavePlanEvent::Select(value0) => Self::Select(value0),
            WavePlanEvent::Compare(value0) => Self::Compare(value0),
            WavePlanEvent::CopyText(value0) => Self::CopyText(value0),
            WavePlanEvent::ToggleWave0 => Self::ToggleWave0,
        }
    }
}

impl From<crate::execution_metrics::events::MetricsEvent> for Action {
    fn from(event: crate::execution_metrics::events::MetricsEvent) -> Self {
        use crate::execution_metrics::events::MetricsEvent;
        match event {
            MetricsEvent::SelectId(value0) => Self::SelectId(value0),
            MetricsEvent::SortMetrics(value0, value1) => Self::SortMetrics(value0, value1),
            MetricsEvent::SortEstimates(value0, value1) => Self::SortEstimates(value0, value1),
        }
    }
}

impl From<crate::document_viewer::events::DocumentEvent> for Action {
    fn from(event: crate::document_viewer::events::DocumentEvent) -> Self {
        use crate::document_viewer::events::DocumentEvent;
        match event {
            DocumentEvent::CloseViewer => Self::CloseViewer,
            DocumentEvent::OpenPath(value0) => Self::OpenPath(value0),
        }
    }
}

impl From<crate::repository_status::events::StatusEvent> for Action {
    fn from(event: crate::repository_status::events::StatusEvent) -> Self {
        use crate::repository_status::events::StatusEvent;
        match event {
            StatusEvent::Recheck => Self::Recheck,
            StatusEvent::CancelCheck => Self::CancelCheck,
            StatusEvent::ToggleOutput => Self::ToggleOutput,
            StatusEvent::ToggleGitList => Self::ToggleGitList,
        }
    }
}

impl From<crate::ticket_actions::events::TicketActionEvent> for Action {
    fn from(event: crate::ticket_actions::events::TicketActionEvent) -> Self {
        use crate::ticket_actions::events::TicketActionEvent;
        match event {
            TicketActionEvent::OpenDialog(value0) => Self::OpenDialog(value0),
            TicketActionEvent::Dispatch(value0) => Self::Dispatch(value0),
            TicketActionEvent::ToggleVerbDrawer => Self::ToggleVerbDrawer,
        }
    }
}
