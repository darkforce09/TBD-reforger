use super::*;
use crate::execution_metrics::{models::MetricsView, ui as metrics_ui};
use crate::repository_status::{models::view::StatusView, ui as status_ui};
use crate::ticket_browser::{models::view::BrowserView, ui as browser_ui};
use crate::wave_plan::{models::view::WavePlanView, ui as wave_ui};

fn browser_view(b: &WorkspaceState) -> BrowserView<'_> {
    BrowserView {
        corpus: &b.corpus,
        board: &b.board,
        expanded: &b.expanded,
        visible: &b.visible,
        selected: b.selected,
        compare: b.compare,
        tree: &b.tree,
        tree_flat: &b.tree_flat,
        legacy_expanded: b.legacy_expanded,
        estimates: &b.estimates,
    }
}
pub(super) fn board_ui(
    ui: &mut Ui,
    b: &WorkspaceState,
    mctx: MutationContext<'_>,
    actions: &mut Vec<Action>,
) {
    let mut events = Vec::new();
    browser_ui::status_board::board_ui(
        ui,
        &browser_view(b),
        mctx.busy,
        &mut |ui, index| {
            let mut events = Vec::new();
            crate::ticket_actions::ui::menus::card_menu_ui(
                ui,
                &mutate::context(b),
                mctx,
                index,
                &mut events,
            );
            events
        },
        &mut events,
    );
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn tree_ui(ui: &mut Ui, b: &WorkspaceState, actions: &mut Vec<Action>) {
    let mut events = Vec::new();
    browser_ui::program_tree::tree_ui(ui, &browser_view(b), &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn detail_ui(
    ui: &mut Ui,
    mctx: MutationContext<'_>,
    b: &WorkspaceState,
    selected: usize,
    advanced_status: &mut Option<StatusName>,
    actions: &mut Vec<Action>,
) {
    let mut events = Vec::new();
    browser_ui::detail_panel::metadata::detail_ui(
        ui,
        mctx.repo_root,
        &browser_view(b),
        selected,
        &mut |ui| {
            let mut events = Vec::new();
            crate::ticket_actions::ui::menus::action_strip_ui(
                ui,
                &mutate::context(b),
                mctx,
                selected,
                advanced_status,
                &mut events,
            );
            events
        },
        &mut events,
    );
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn waves_ui(ui: &mut Ui, b: &WorkspaceState, actions: &mut Vec<Action>) {
    let mut events = Vec::new();
    let view = WavePlanView {
        lock: &b.lock,
        waves: b.waves.as_ref(),
        filters_active: b.filters.is_active(),
        matches: &b.matches,
        selected: b.selected,
        compare: b.compare,
        wave0_expanded: b.wave0_expanded,
    };
    wave_ui::waves_ui(ui, &view, &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn metrics_ui(ui: &mut Ui, b: &WorkspaceState, actions: &mut Vec<Action>) {
    let mut events = Vec::new();
    let view = MetricsView {
        metrics: &b.metrics,
        estimates: &b.estimates,
        metrics_sort: b.metrics_sort,
        est_sort: b.est_sort,
        id_to_index: &b.board.id_to_index,
    };
    metrics_ui::dashboard::metrics_ui(ui, &view, &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
pub(super) fn viewer_pane_ui(
    ui: &mut Ui,
    state: &ViewerState,
    cache: &mut CommonMarkCache,
    repo_root: Option<&Path>,
    actions: &mut Vec<Action>,
) {
    let mut events = Vec::new();
    crate::document_viewer::ui::viewer_pane_ui(ui, state, cache, repo_root, &mut events);
    actions.extend(events.into_iter().map(Into::into));
}
impl TicketboardApp {
    pub(super) fn trust_banner_ui(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let view = StatusView {
            check: &self.check,
            check_running: self.check_handle.is_some(),
            check_log: &self.check_log,
            show_output: self.show_output,
            watch_error: self.watch_error.as_deref(),
            degraded_watches: self
                .watch
                .as_ref()
                .map_or(&[], |watch| watch.degraded.as_slice()),
            git_chip: &self.git_chip,
            git_expanded: self.git_expanded,
        };
        let mut events = Vec::new();
        status_ui::trust_banner_ui(&view, ui, &mut events);
        actions.extend(events.into_iter().map(Into::into));
    }
}
