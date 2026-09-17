//! Toolbar and status.

use super::*;

const MODEBAR: &str = "pointer-events-auto rounded-xl border border-white/10 bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex items-center gap-1 px-1.5 py-1.5";

/// Tailwind classes for the full width bottom status bar.
pub(super) const STATUSBAR: &str = "pointer-events-auto bg-surface-container-lowest/55 shadow-xl backdrop-blur-xl flex h-9 w-full items-center gap-3 border-t border-white/10 px-3";

/// Height of the status bar in CSS pixels.
pub const STATUSBAR_H_PX: f64 = 36.0;

const TOOL_BASE: &str = "flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-label-md";

/// Formats a coordinate for the compact status readout.
pub(super) fn fmt_coord(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{n:>9.3}"),
        None => "       —".to_string(),
    }
}

/// Formats a coordinate with Eden-style precision.
pub(super) fn fmt_coord_eden(v: Option<f64>) -> String {
    match v {
        Some(_) => format!("{} m", fmt_coord(v)),
        None => fmt_coord(None),
    }
}

/// Renders the Select, Ruler, and line of sight mode controls.
#[component]
pub fn ModeToolbar(
    tool_mode: RwSignal<website_map_engine::editing::tools::ruler::EditorTool>,
    los_mode: RwSignal<LosMode>,
) -> impl IntoView {
    use website_map_engine::editing::tools::ruler::EditorTool;
    let cls = move |mine: EditorTool| {
        if tool_mode.get() == mine {
            cn(&[TOOL_BASE, TOGGLED_PLATE])
        } else {
            cn(&[TOOL_BASE, "text-on-surface-variant", HOVER_FILL])
        }
    };
    let pressed = move |mine: EditorTool| (tool_mode.get() == mine).to_string();
    view! {
        <div class=MODEBAR>
            <button
                type="button"
                class=move || cls(EditorTool::Select)
                aria-pressed=move || pressed(EditorTool::Select)
                title="Select"
                on:pointerdown=move |_| tool_mode.set(EditorTool::Select)
            >
                <MaterialIcon name="arrow_selector_tool" class="block text-base" />
                <span class="hidden sm:inline">"Select"</span>
            </button>
            <button
                type="button"
                class=move || cls(EditorTool::Ruler)
                aria-pressed=move || pressed(EditorTool::Ruler)
                title="Ruler — click a chain of points; Esc clears, double-click ends"
                on:pointerdown=move |_| tool_mode.set(EditorTool::Ruler)
            >
                <MaterialIcon name="straighten" class="block text-base" />
                <span class="hidden sm:inline">"Ruler"</span>
            </button>
            <button
                type="button"
                class=move || cls(EditorTool::LoS)
                aria-pressed=move || pressed(EditorTool::LoS)
                title=move || {
                    if los_mode.get().is_viewshed() {
                        "Line of sight (viewshed) — click one observer to shade the visible disc; \
                         click LoS again for ray; Esc clears"
                    } else {
                        "Line of sight (ray) — click observer, click target; click LoS again for \
                         viewshed; Esc clears"
                    }
                }
                on:pointerdown=move |_| {
                    if tool_mode.get_untracked().is_los() {
                        los_mode.update(|m| *m = m.toggled());
                    } else {
                        tool_mode.set(EditorTool::LoS);
                    }
                }
            >
                <MaterialIcon name="visibility" class="block text-base" />
                <span class="hidden sm:inline">
                    {move || if los_mode.get().is_viewshed() { "LoS · viewshed" } else { "LoS · ray" }}
                </span>
            </button>
        </div>
    }
}

/// Renders cursor, selection, object, scale, and diagnostic readouts.
#[component]
pub fn StatusBar(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    sel_count: RwSignal<usize>,
    obj_count: RwSignal<usize>,
    selected_ids: RwSignal<Vec<String>>,
    #[prop(optional)] sz_bytes: Option<RwSignal<Option<usize>>>,
    #[prop(optional)] debug_hud: Option<RwSignal<String>>,
    #[prop(optional)] hud_shown: Option<RwSignal<bool>>,
    #[prop(optional)] ruler_status: Option<RwSignal<Option<String>>>,
    #[prop(optional)] scale_mpp: Option<RwSignal<f64>>,
) -> impl IntoView {
    let sel_xyz = Memo::new(move |_| -> Option<(f64, f64, f64)> {
        let ids = selected_ids.get();
        if ids.len() == 1 {
            #[cfg(target_arch = "wasm32")]
            {
                return website_map_engine::editing::hosted_commands::read_attrs(&ids[0])
                    .map(|a| (a.x, a.y, a.z));
            }
        }
        let _ = ids;
        None
    });
    let axis_val = move |i: usize| match sel_xyz.get() {
        Some((x, y, z)) => fmt_coord_eden(Some([x, y, z][i])),
        None => fmt_coord_eden(cursor.get().and_then(|c| match i {
            0 => Some(c.0),
            1 => Some(c.1),
            _ => c.2,
        })),
    };
    view! {
        <div class=STATUSBAR>
            <div class="flex items-center gap-2 font-mono text-code-md text-on-surface-variant">
                <span class="text-outline" title="Cursor">
                    {move || if sel_xyz.get().is_some() { "SEL" } else { "CUR" }}
                </span>
                <span title="Cursor X">
                    "X"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(0)}</span>
                </span>
                <span title="Cursor Y">
                    "Y"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(1)}</span>
                </span>
                <span title="Cursor Z">
                    "Z"
                    <span class="ml-1 text-on-surface tabular-nums">{move || axis_val(2)}</span>
                </span>
            </div>
            <span class="h-5 w-px bg-white/10"></span>
            <div
                class="flex items-center gap-2 font-mono text-code-md tabular-nums text-on-surface-variant"
                title="Placed slots on map / current selection"
            >
                <span>
                    "OBJ"
                    <span class="ml-1 text-on-surface">{move || obj_count.get()}</span>
                </span>
                <span>
                    "SEL"
                    <span class="ml-1 text-on-surface">{move || sel_count.get()}</span>
                </span>
                <span title="Estimated save payload">
                    "SZ"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            sz_bytes
                                .and_then(|s| s.get())
                                .map_or_else(
                                    || "—".to_string(),
                                    crate::v2::apps::editor::shell::mission_size::format_bytes,
                                )
                        }}
                    </span>
                </span>
                <span data-status-scale title="Map scale — metres per screen pixel">
                    "SCL"
                    <span class="ml-1 text-on-surface">
                        {move || {
                            format_m_per_px(
                                scale_mpp.map_or_else(|| m_per_px(-2.0), |s| s.get()),
                            )
                        }}
                    </span>
                </span>
            </div>
            {move || {
                ruler_status.and_then(|s| s.get()).map(|text| {
                    view! {
                        <span class="h-5 w-px bg-white/10"></span>
                        <span
                            data-status-ruler
                            class="flex items-center whitespace-nowrap font-mono text-code-md text-primary"
                            title="Ruler — running total · last leg"
                        >
                            {text}
                        </span>
                    }
                })
            }}
            <span class="h-5 w-px bg-white/10"></span>
            <div
                data-status-furniture
                class="flex min-w-0 flex-1 items-center justify-center gap-2 font-mono text-code-md text-outline"
                title="Scale bar (T-667)"
            >
                <ScaleBar cursor debug_hud scale_mpp />
            </div>
            {move || {
                let text = debug_hud.map(|h| h.get()).unwrap_or_default();
                let on = hud_shown.map(|s| s.get()).unwrap_or(false);
                (on && !text.is_empty()).then(|| {
                    view! {
                        <div
                            data-status-hud
                            class="pointer-events-none flex items-center font-mono text-[11px] text-success/90"
                        >
                            {text}
                        </div>
                    }
                })
            }}
            <button
                type="button"
                data-status-open
                class="flex items-center gap-1.5 rounded-md bg-primary/90 px-3 py-1 text-label-md font-medium text-on-primary transition-colors hover:bg-primary"
                title="Open"
            >
                <MaterialIcon name="folder_open" class="block text-base" />
                <span>"OPEN"</span>
            </button>
        </div>
    }
}
