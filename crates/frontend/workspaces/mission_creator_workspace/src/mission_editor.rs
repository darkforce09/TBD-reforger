//! Scenario Creator page and its editor integration modules.
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use map_editing_tools::line_of_sight::capture::{LosMode, LosState, ViewshedState};
#[cfg(target_arch = "wasm32")]
use map_editing_tools::selection;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::boot::boot_progress::BootSegView;

#[cfg(target_arch = "wasm32")]
use crate::ui::docks::top_strip;
#[cfg(target_arch = "wasm32")]
use crate::ui::inspector::validation_panel;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::document_host::doc_host as mission_doc;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::document_host::history as mission_history;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::host_state::editor_context;
#[cfg(target_arch = "wasm32")]
use mission_creator_session::hydrate as mission_hydrate;
#[cfg(target_arch = "wasm32")]
use mission_creator_session::persist as yrs_persist;
#[cfg(target_arch = "wasm32")]
use mission_editing_commands::hosted_commands as engine_ops;

#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::pointer_hover::HoverState;
#[cfg(target_arch = "wasm32")]
use mission_editing_session::lanes::comments::{comment_lane_ids, comment_lane_xy};
#[cfg(target_arch = "wasm32")]
use mission_editing_session::lanes::connections::connection_lane_verts;
#[cfg(target_arch = "wasm32")]
use mission_editing_session::routing::{RouteTarget, route_availability, route_target};
#[cfg(target_arch = "wasm32")]
use mission_editing_session::selection_universe::map_render_slot_soa;

#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::overlays::register_widget_pivot;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::overlays::{
    AssetPickerOverlay, AssetPickerState, CommentEditorOverlay, ConnectionsPanelOverlay,
    SnapReadout, TransformWidgetOverlay, WidgetModeHint,
};
#[cfg(target_arch = "wasm32")]
use mission_creator_session::conflict_dialog::{ConflictDialog, ConflictInfo};

#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_engine_bridge::bridge::boot::BootPhase;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_engine_bridge::bridge::boot::boot_progress;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::boot::hand_over;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::viewport::mark_registry_fetch_failed;
#[cfg(any(test, target_arch = "wasm32"))]
use mission_creator_engine_bridge::bridge::viewport::registry_session;
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::viewport::{
    register_editor_cam, register_self_checks, register_slot_stats, start_raf,
};

#[path = "mission_editor/registry_loading.rs"]
mod registry_loading;
#[cfg(target_arch = "wasm32")]
use registry_loading::{fetch_compat_cold, fetch_registry_pages};

/// The toolbar dispatch the page registers and the top strip's buttons run.
#[path = "mission_editor/toolbar_dispatch.rs"]
pub(crate) mod toolbar_dispatch;
#[cfg(target_arch = "wasm32")]
use toolbar_dispatch::{
    EditorToolbarDispatch, register_editor_toolbar_dispatch, unregister_editor_toolbar_dispatch,
};

#[cfg(target_arch = "wasm32")]
#[path = "mission_editor/canvas_mount.rs"]
mod canvas_mount;
#[cfg(target_arch = "wasm32")]
use canvas_mount::{PageMountSignals, install_canvas_mount};

#[path = "mission_editor/page_effects.rs"]
mod page_effects;

#[path = "mission_editor/document_helpers.rs"]
mod document_helpers;
#[cfg(target_arch = "wasm32")]
use document_helpers::{SubjectResolver, arrange_chord};
#[cfg(target_arch = "wasm32")]
use mission_creator_engine_bridge::bridge::hover_hit_testing::{
    HoverPoints, live_connection_segments, set_map_cursor,
};
#[cfg(target_arch = "wasm32")]
use mission_creator_state::scale_math;
#[cfg(target_arch = "wasm32")]
use mission_creator_state::transform;
/// Builds the editor workspace and installs its page-level signals.
#[cfg(target_arch = "wasm32")]
#[component]
pub fn MissionEditorPage() -> impl IntoView {
    // The lower layers' hooks are filled before anything can fire them: the canvas mount (the
    // history context and the gestures) and the top strip (the export row) both come later.
    {
        use crate::ui::docks::context_menu;
        use crate::ui::inspector::validation_panel;
        use mission_creator_session::persist;
        persist::register_edit_persist();
        context_menu::register_canvas_context_menu();
        validation_panel::register_compile_findings_publisher();
    }
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
    let scale_mpp = RwSignal::new(scale_math::m_per_px(-2.0));
    let tool_mode = RwSignal::new(map_editing_tools::ruler::EditorTool::Select);
    let los_mode = RwSignal::new(LosMode::default());
    let ruler_status = RwSignal::new(None::<String>);
    let ruler_tick = RwSignal::new(0u64);
    let los_tick = RwSignal::new(0u64);
    let snap = RwSignal::new(mission_creator_state::transform::SnapState::default());
    let widget_variant = RwSignal::new(mission_creator_state::transform::WidgetVariant::default());
    let widget_tick = RwSignal::new(0u64);
    let boot = RwSignal::new(BootPhase::Hydrating);
    let map_disabled = RwSignal::new(None::<String>);
    let progress = RwSignal::new(boot_progress::BootProgress::new());
    page_effects::track_mission_size(obj_count, sz_bytes);
    page_effects::install_arrange_chords();

    let outliner_nodes =
        RwSignal::new(Vec::<mission_creator_state::outliner_model::OutlinerNode>::new());
    let orbat_nodes =
        RwSignal::new(Vec::<mission_creator_state::outliner_model::OutlinerNode>::new());
    let selected_ids = RwSignal::new(Vec::<String>::new());
    page_effects::update_widget_tick(selected_ids, widget_tick);
    let active_layer = RwSignal::new(None::<String>);
    let active_side = RwSignal::new(String::from("BLUFOR"));
    let objects_mode = RwSignal::new(false);
    let catalog = RwSignal::new(mission_creator_state::asset_catalog::CatalogState::Loading);
    let vehicle_catalog =
        RwSignal::new(mission_creator_state::asset_catalog::CatalogState::Loading);
    let attrs_open = RwSignal::new(None::<String>);
    let attrs_tab = RwSignal::new(1usize);
    let doc_tick = RwSignal::new(0u64);
    let selected_connection = RwSignal::new(None::<String>);
    let settings_open = RwSignal::new(false);
    let fm_open = RwSignal::new(false);
    let orbat_open = RwSignal::new(false);
    let context_menu = RwSignal::new(None::<crate::ui::docks::context_menu::MenuState>);
    let asset_picker = RwSignal::new(None::<AssetPickerState>);
    let comment_editor = RwSignal::new(None::<String>);
    let connections_panel = RwSignal::new(false);
    let chrome_hidden = RwSignal::new(false);
    let dock_left_collapsed = RwSignal::new(false);
    let dock_right_collapsed = RwSignal::new(false);
    let registry_items = RwSignal::new(None::<Vec<frontend_api_dtos::RegistryItem>>);
    let registry_failed = RwSignal::new(false);
    let registry_fetch_gen = RwSignal::new(0u64);

    page_effects::update_catalog(active_side, registry_items, catalog);
    let compat = RwSignal::new(mission_creator_state::arsenal_rules::CompatFeed::default());
    let dirty = RwSignal::new(false);
    let conflict = RwSignal::new(None::<ConflictInfo>);
    let current_semver = RwSignal::new(None::<String>);

    let mission_id: mission_model::ids::MissionId = {
        use leptos_router::hooks::use_params_map;
        use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string())
            .into()
    };

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
                    let strip_title = mission_id.to_string();
                    move || (!chrome_hidden.get()).then(|| view! {
                    <div class="absolute inset-x-0 top-0 z-30 h-12">
                        <crate::ui::docks::top_strip::TopCommandStrip
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
                        mission_creator_state::layout::DOCK_LEFT_MOUNT_COLLAPSED
                    } else {
                        mission_creator_state::layout::DOCK_LEFT_MOUNT
                    }>
                        <crate::ui::docks::dock_left::DockLeft
                            nodes=outliner_nodes
                            selected=selected_ids
                            active_layer
                            collapsed=dock_left_collapsed
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_right_collapsed.get() {
                        mission_creator_state::layout::DOCK_RIGHT_MOUNT_COLLAPSED
                    } else {
                        mission_creator_state::layout::DOCK_RIGHT_MOUNT
                    }>
                        <crate::ui::docks::dock_right::DockRight
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
                    <crate::ui::docks::toolbelt::ModeToolbar tool_mode los_mode />
                </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                <div class="absolute inset-x-0 bottom-0">
                    <crate::ui::docks::toolbelt::StatusBar
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
                {move || (!chrome_hidden.get()).then(|| view! { <crate::ui::docks::toolbelt::MapGridRefs cursor debug_hud=Some(debug_hud) /> })}
                <div class="pointer-events-auto">
                    <crate::ui::inspector::attributes_modal::AttributesModal attrs_open attrs_tab doc_tick registry_items compat />
                </div>
                <div class="pointer-events-auto">
                    <crate::ui::modals::settings_modal::MissionSettingsDialog open=settings_open doc_tick />
                    <crate::ui::modals::faction_manager::FactionManagerDialog open=fm_open registry=registry_items />
                    <crate::ui::modals::orbat_manager::OrbatManagerDialog
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
                    <mission_creator_session::tab_lock::TabLockBanner />
                </div>
                <div class="pointer-events-auto">
                    <crate::ui::docks::context_menu::ContextMenuOverlay menu=context_menu />
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
                <mission_creator_engine_bridge::input::tools::ruler_tool::RulerOverlay cursor debug_hud=Some(debug_hud) tick=ruler_tick />
                <mission_creator_engine_bridge::input::tools::los_tool::LosOverlay cursor debug_hud=Some(debug_hud) tick=los_tick />
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
#[path = "tests/mod.rs"]
mod tests;
