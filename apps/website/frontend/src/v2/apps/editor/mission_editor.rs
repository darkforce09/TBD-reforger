//! Scenario Creator page and its editor integration modules.
#![allow(dead_code)]
use crate::v2::apps::editor::bridge::boot::boot_progress::BootSegView;
use leptos::prelude::*;
use website_map_engine::editing::tools::line_of_sight::capture::{
    LosMode, LosState, ViewshedState,
};
use website_map_engine::editing::tools::selection;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::document_host::doc_host as mission_doc;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::document_host::history as mission_history;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::shell::hydrate as mission_hydrate;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::shell::persist as yrs_persist;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::docks::top_strip;
use crate::v2::apps::editor::ui::inspector::validation_panel;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use crate::v2::apps::editor::bridge::pointer_hover::{
    hover_cursor_css, hover_due, hover_next, hover_suppressed, HoverState,
};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use website_map_engine::editing::lanes::comments::{
    comment_drag_lane_xy, comment_lane_xy, comment_points, dragged_comment_points, pick_comment,
    COMMENT_PICK_PX,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use website_map_engine::editing::lanes::comments::{comment_lane_ids, CommentPoint};
#[cfg(target_arch = "wasm32")]
pub(crate) use website_map_engine::editing::lanes::connections::CONN_PICK_PX;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use website_map_engine::editing::lanes::connections::{
    connection_lane_verts, connection_segments, pick_connection, ConnSegment,
};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use website_map_engine::editing::lanes::markers::marker_lane_fields;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use website_map_engine::editing::routing::{
    route_availability, route_target, RouteTarget,
};
#[cfg(target_arch = "wasm32")]
pub(crate) use website_map_engine::editing::selection_universe::map_render_slot_soa;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use website_map_engine::editing::selection_universe::{
    plain_paste_anchor, selectable_ids,
};

#[cfg(target_arch = "wasm32")]
pub(crate) use crate::v2::apps::editor::bridge::overlays::{
    read_widget_pivot, register_widget_pivot,
};
pub(crate) use crate::v2::apps::editor::bridge::overlays::{
    AssetPickerOverlay, CommentEditorOverlay, ConflictDialog, ConnectionsPanelOverlay, SnapReadout,
    TransformWidgetOverlay, WidgetModeHint,
};
pub use crate::v2::apps::editor::bridge::overlays::{AssetPickerState, ConflictInfo};

pub use crate::v2::apps::editor::bridge::boot::boot_progress;
#[cfg(target_arch = "wasm32")]
pub(crate) use crate::v2::apps::editor::bridge::boot::hand_over;
pub(crate) use crate::v2::apps::editor::bridge::boot::BootPhase;
#[cfg(target_arch = "wasm32")]
pub(crate) use crate::v2::apps::editor::bridge::viewport::{
    device_size, register_editor_cam, register_self_checks, register_slot_stats, start_raf,
};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use crate::v2::apps::editor::bridge::viewport::{
    mark_registry_fetch_failed, registry_session,
};

#[path = "mission_editor/registry_loading.rs"]
mod registry_loading;
#[cfg(target_arch = "wasm32")]
use registry_loading::{fetch_compat_cold, fetch_registry_pages};

#[path = "mission_editor/armed_place.rs"]
pub mod armed_place;
#[path = "mission_editor/transform.rs"]
pub mod transform;

#[path = "mission_editor/toolbar_dispatch.rs"]
mod toolbar_dispatch;
pub(crate) use toolbar_dispatch::toolbar_dispatch_generation;
#[cfg(target_arch = "wasm32")]
pub(crate) use toolbar_dispatch::{
    register_editor_toolbar_dispatch, unregister_editor_toolbar_dispatch,
    with_editor_toolbar_dispatch, EditorToolbarDispatch,
};

#[cfg(target_arch = "wasm32")]
#[path = "mission_editor/canvas_mount.rs"]
mod canvas_mount;
#[cfg(target_arch = "wasm32")]
use canvas_mount::{install_canvas_mount, PageMountSignals};

#[path = "mission_editor/page_effects.rs"]
mod page_effects;

#[path = "mission_editor/document_helpers.rs"]
mod document_helpers;
#[cfg(target_arch = "wasm32")]
pub(crate) use document_helpers::{
    arrange_chord, hover_hit, live_connection_segments, set_map_cursor, HoverPoints,
    SubjectResolver,
};
/// Builds the editor workspace and installs its page-level signals.
#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    let save_semver = RwSignal::new("0.1.0".to_string());
    let save_status = RwSignal::new(String::new());

    let can_undo = RwSignal::new(false);
    let can_redo = RwSignal::new(false);
    let obj_count = RwSignal::new(0usize);
    let sel_count = RwSignal::new(0usize);
    let cursor = RwSignal::new(None::<(f64, f64, Option<f64>)>);
    let sz_bytes = RwSignal::new(None::<usize>);
    // framework_synthesis §D.4 #7: the debug HUD is TELEMETRY; Mission-correctness diagnostics are NEVER gated by its visibility.
    let debug_hud = RwSignal::new(String::new());
    let debug_hud_shown = RwSignal::new(false);
    let scale_mpp = RwSignal::new(crate::v2::apps::editor::ui::docks::toolbelt::m_per_px(-2.0));
    let tool_mode = RwSignal::new(website_map_engine::editing::tools::ruler::EditorTool::Select);
    let los_mode = RwSignal::new(LosMode::default());
    let ruler_status = RwSignal::new(None::<String>);
    let ruler_tick = RwSignal::new(0u64);
    let los_tick = RwSignal::new(0u64);
    let snap =
        RwSignal::new(crate::v2::apps::editor::mission_editor::transform::SnapState::default());
    let widget_variant =
        RwSignal::new(crate::v2::apps::editor::mission_editor::transform::WidgetVariant::default());
    let widget_tick = RwSignal::new(0u64);
    let boot = RwSignal::new(BootPhase::Hydrating);
    let map_disabled = RwSignal::new(None::<String>);
    let progress = RwSignal::new(boot_progress::BootProgress::new());
    #[cfg(target_arch = "wasm32")]
    page_effects::track_mission_size(obj_count, sz_bytes);
    #[cfg(target_arch = "wasm32")]
    page_effects::install_arrange_chords();

    let outliner_nodes = RwSignal::new(Vec::<
        crate::v2::apps::editor::ui::outliner::outliner::OutlinerNode,
    >::new());
    let orbat_nodes = RwSignal::new(Vec::<
        crate::v2::apps::editor::ui::outliner::outliner::OutlinerNode,
    >::new());
    let selected_ids = RwSignal::new(Vec::<String>::new());
    page_effects::update_widget_tick(selected_ids, widget_tick);
    let active_layer = RwSignal::new(None::<String>);
    let active_side = RwSignal::new(String::from("BLUFOR"));
    let objects_mode = RwSignal::new(false);
    let catalog =
        RwSignal::new(crate::v2::apps::editor::arsenal::asset_catalog::CatalogState::Loading);
    let vehicle_catalog =
        RwSignal::new(crate::v2::apps::editor::arsenal::asset_catalog::CatalogState::Loading);
    let attrs_open = RwSignal::new(None::<String>);
    let attrs_tab = RwSignal::new(1usize);
    let doc_tick = RwSignal::new(0u64);
    let selected_connection = RwSignal::new(None::<String>);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = selected_connection;
    let settings_open = RwSignal::new(false);
    let fm_open = RwSignal::new(false);
    let orbat_open = RwSignal::new(false);
    let context_menu =
        RwSignal::new(None::<crate::v2::apps::editor::ui::docks::context_menu::MenuState>);
    let asset_picker = RwSignal::new(None::<AssetPickerState>);
    let comment_editor = RwSignal::new(None::<String>);
    let connections_panel = RwSignal::new(false);
    let chrome_hidden = RwSignal::new(false);
    let dock_left_collapsed = RwSignal::new(false);
    let dock_right_collapsed = RwSignal::new(false);
    let registry_items = RwSignal::new(None::<Vec<crate::v2::core::api::dto::RegistryItem>>);
    let registry_failed = RwSignal::new(false);
    let registry_fetch_gen = RwSignal::new(0u64);

    page_effects::update_catalog(active_side, registry_items, catalog);
    let compat = RwSignal::new(crate::v2::apps::editor::arsenal::rules::CompatFeed::default());
    let dirty = RwSignal::new(false);
    let conflict = RwSignal::new(None::<ConflictInfo>);
    let current_semver = RwSignal::new(None::<String>);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = current_semver;

    let mission_id = {
        use leptos_router::hooks::use_params_map;
        use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string())
    };

    #[cfg(target_arch = "wasm32")]
    install_canvas_mount(PageMountSignals {
        container_ref,
        canvas_ref,
        can_undo,
        can_redo,
        obj_count,
        sel_count,
        cursor,
        debug_hud,
        debug_hud_shown,
        scale_mpp,
        tool_mode,
        los_mode,
        ruler_status,
        ruler_tick,
        los_tick,
        snap,
        widget_variant,
        boot,
        map_disabled,
        progress,
        outliner_nodes,
        orbat_nodes,
        selected_ids,
        active_layer,
        active_side,
        objects_mode,
        catalog,
        vehicle_catalog,
        attrs_open,
        attrs_tab,
        doc_tick,
        selected_connection,
        context_menu,
        asset_picker,
        comment_editor,
        connections_panel,
        chrome_hidden,
        dock_left_collapsed,
        dock_right_collapsed,
        registry_items,
        registry_failed,
        registry_fetch_gen,
        compat,
        dirty,
        conflict,
        current_semver,
        mission_id: mission_id.clone(),
    });

    view! {
        <div
            node_ref=container_ref
            class="relative h-screen w-screen overflow-hidden bg-background"
        >
            <canvas node_ref=canvas_ref class="absolute inset-0 z-0 block h-full w-full"></canvas>
            <div
                data-eden-chrome
                class="pointer-events-none absolute inset-0 z-10"
                on:pointerdown=|ev| ev.stop_propagation()
            >
                {
                    let strip_title = mission_id.clone();
                    move || (!chrome_hidden.get()).then(|| view! {
                    <div class="absolute inset-x-0 top-0 z-30 h-12">
                        <crate::v2::apps::editor::shell::eden_chrome::TopCommandStrip
                            title=strip_title.clone()
                            can_undo
                            can_redo
                            save_semver
                            save_status
                            dirty
                            settings_open
                            doc_tick
                            obj_count
                            orbat_open
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_left_collapsed.get() {
                        crate::v2::apps::editor::shell::layout::DOCK_LEFT_MOUNT_COLLAPSED
                    } else {
                        crate::v2::apps::editor::shell::layout::DOCK_LEFT_MOUNT
                    }>
                        <crate::v2::apps::editor::shell::eden_chrome::DockLeft
                            nodes=outliner_nodes
                            selected=selected_ids
                            active_layer
                            collapsed=dock_left_collapsed
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_right_collapsed.get() {
                        crate::v2::apps::editor::shell::layout::DOCK_RIGHT_MOUNT_COLLAPSED
                    } else {
                        crate::v2::apps::editor::shell::layout::DOCK_RIGHT_MOUNT
                    }>
                        <crate::v2::apps::editor::shell::eden_chrome::DockRight
                            catalog
                            vehicle_catalog
                            registry_items
                            registry_failed
                            registry_fetch_gen
                            doc_tick
                            fm_open
                            active_side
                            objects_mode
                            collapsed=dock_right_collapsed
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                <div class="absolute bottom-11 left-1/2 -translate-x-1/2">
                    <crate::v2::apps::editor::ui::docks::toolbelt::ModeToolbar tool_mode los_mode />
                </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                <div class="absolute inset-x-0 bottom-0">
                    <crate::v2::apps::editor::ui::docks::toolbelt::StatusBar
                        cursor
                        sel_count
                        obj_count
                        selected_ids
                        sz_bytes
                        debug_hud
                        hud_shown=debug_hud_shown
                        ruler_status
                        scale_mpp
                    />
                </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! { <crate::v2::apps::editor::ui::docks::toolbelt::MapGridRefs cursor debug_hud=Some(debug_hud) /> })}
                <div class="pointer-events-auto">
                    <crate::v2::apps::editor::ui::inspector::attributes_modal::AttributesModal attrs_open attrs_tab doc_tick registry_items compat />
                </div>
                <div class="pointer-events-auto">
                    <crate::v2::apps::editor::shell::eden_chrome::MissionSettingsDialog open=settings_open doc_tick />
                    <crate::v2::apps::editor::ui::modals::faction_manager::FactionManagerDialog open=fm_open registry=registry_items />
                    <crate::v2::apps::editor::shell::eden_chrome::OrbatManagerDialog
                        open=orbat_open
                        orbat=orbat_nodes
                        selected=selected_ids
                        active_layer
                        registry=registry_items
                    />
                </div>
                <div class="pointer-events-auto">
                    <ConflictDialog conflict conflict_id=mission_id.clone() />
                </div>
                <div class="pointer-events-none absolute inset-x-0 top-3 z-30 px-4">
                    <crate::v2::apps::editor::shell::tab_lock::TabLockBanner />
                </div>
                <div class="pointer-events-auto">
                    <crate::v2::apps::editor::ui::docks::context_menu::ContextMenuOverlay menu=context_menu />
                </div>
                <div class="pointer-events-auto">
                    <CommentEditorOverlay open=comment_editor doc_tick />
                </div>
                <div class="pointer-events-auto">
                    <ConnectionsPanelOverlay open=connections_panel doc_tick />
                </div>
                <div class="pointer-events-auto">
                    <AssetPickerOverlay picker=asset_picker registry=registry_items active_side />
                </div>
                <crate::v2::apps::editor::input::tools::ruler_tool::RulerOverlay cursor debug_hud=Some(debug_hud) tick=ruler_tick />
                <crate::v2::apps::editor::input::tools::los_tool::LosOverlay cursor debug_hud=Some(debug_hud) tick=los_tick />
                <TransformWidgetOverlay
                    cursor
                    debug_hud=Some(debug_hud)
                    tick=widget_tick
                    variant=widget_variant
                />
                <WidgetModeHint cursor variant=widget_variant />
                <validation_panel::ValidationPanel doc_tick />
                {move || (!chrome_hidden.get()).then(|| view! { <SnapReadout snap /> })}
            </div>
            {move || {
                let phase = boot.get();
                let p = progress.get();
                match phase {
                    BootPhase::Ready => None,
                    BootPhase::Failed { seg, reason } => Some(view! {
                        <div class="animate-overlay-fade pointer-events-auto absolute inset-0 z-50 flex items-center justify-center bg-background/90 backdrop-blur-sm">
                            <div class="flex w-80 max-w-[90vw] flex-col items-center gap-3 rounded-xl border border-error/40 bg-surface-variant/40 p-6 text-center">
                                <p class="text-sm font-semibold text-error">
                                    {format!("{} failed", seg.title().trim_end_matches('…'))}
                                </p>
                                <p class="max-h-32 overflow-y-auto break-words font-mono text-[11px] text-on-surface-variant/80">
                                    {reason}
                                </p>
                                <div class="mt-1 flex gap-2">
                                    <button
                                        type="button"
                                        aria-label="Retry"
                                        class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                                        on:click=move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            if let Some(win) = web_sys::window() {
                                                let _ = win.location().reload();
                                            }
                                        }
                                    >
                                        "Retry"
                                    </button>
                                    <button
                                        type="button"
                                        aria-label="Continue without map"
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                                        on:click=move |_| {
                                            boot.set(BootPhase::Ready);
                                        }
                                    >
                                        "Continue without map"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()),
                    _ => {
                        let pct = p.percent();
                        let title = p.stage().title();
                        let caption = p.caption();
                        Some(view! {
                            <div class="animate-overlay-fade pointer-events-none absolute inset-0 z-50 flex items-center justify-center bg-background/85 backdrop-blur-sm">
                                <div class="flex w-64 flex-col items-center gap-2">
                                    <p class="text-sm font-medium text-on-surface-variant">{title}</p>
                                    <div class="h-1.5 w-56 overflow-hidden rounded-full bg-surface-variant/40">
                                        <div
                                            class="mc-load-fill h-full rounded-full bg-primary"
                                            style=format!("width:{pct:.1}%")
                                        ></div>
                                    </div>
                                    <p class="font-mono text-[11px] tabular-nums text-on-surface-variant/60">
                                        {caption}
                                    </p>
                                </div>
                            </div>
                        }.into_any())
                    }
                }
            }}
            {move || {
                let disabled = map_disabled.get();
                let down = boot.get() == BootPhase::Ready;
                (down)
                    .then_some(disabled)
                    .flatten()
                    .map(|reason| view! {
                        <div class="pointer-events-none absolute bottom-16 left-1/2 z-40 -translate-x-1/2 rounded-lg border border-error/30 bg-background/80 px-3 py-1.5 text-center backdrop-blur-sm">
                            <p class="text-label-md font-medium text-error">"Map unavailable"</p>
                            <p class="max-w-xs truncate font-mono text-[10px] text-on-surface-variant/70">
                                {reason}
                            </p>
                        </div>
                    })
            }}
        </div>
    }
}

#[cfg(test)]
#[path = "tests/mission_editor/mod.rs"]
mod tests;
