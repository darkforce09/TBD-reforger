//! Bottom toolbelt.

use super::*;

/// Renders the toolbar and status readouts as the bottom editor chrome.
#[component]
pub fn BottomToolbelt(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    selected_ids: RwSignal<Vec<String>>,
    sz_bytes: RwSignal<Option<usize>>,
) -> impl IntoView {
    let tool_mode = RwSignal::new(website_map_engine::editing::tools::ruler::EditorTool::Select);
    let los_mode = RwSignal::new(LosMode::default());
    view! {
        <ModeToolbar tool_mode los_mode />
        <StatusBar cursor sel_count obj_count selected_ids sz_bytes />
    }
}
