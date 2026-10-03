//! Mounting the mortar map view in the browser.
//!
//! **Role:** once the container and canvas exist, attaches the marker drag, mounts the map view
//! with the terrain-and-imagery preferences and the page's heights, places a clicked position,
//! and redraws the fire-mission overlay when the drafts, the solution or the zoom change.
//! **Position:** called by `super::map_canvas` in the browser build only.
//! **Signals & state:** the mount status signal; a local [`StoredValue`] holding the view's
//! [`MapViewHandles`], disposed on cleanup; a redraw counter; whether the marker atlas is up.
//! **Invariants:** the view mounts at most once per canvas; the drag listeners attach before the
//! mount attaches the navigation; the heights handle is the page's; a click writes the current
//! placement only.

use super::engine_overlay::{attach_marker_drag, upload_overlay, DragTargets};
use super::marks::overlay_scene;
use super::picking::apply_placement;
use super::MapStatus;
use crate::foundation::map_view::engine_mount::force_webgl_from_location;
use crate::foundation::map_view::handles::MapViewHandles;
use crate::foundation::map_view::mount::{mount_map_view, MapViewError, MapViewMount};
use crate::foundation::map_view::navigation::MapClick;
use crate::foundation::map_view::terrain_height::TerrainHeights;
use crate::foundation::map_view::terrain_preferences::terrain_and_imagery_preferences;
use crate::pages::field_tools::mortar::inputs::positions::MortarTerrain;
use crate::pages::field_tools::mortar::solve_bridge::SolvedMission;
use leptos::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// The DOM and page state one mount needs.
pub(super) struct MountParts {
    /// The terrain to boot.
    pub(super) terrain: MortarTerrain,
    /// The element the canvas fills.
    pub(super) container: NodeRef<leptos::html::Div>,
    /// The canvas the engine draws on.
    pub(super) canvas: NodeRef<leptos::html::Canvas>,
    /// The mount status the card shows.
    pub(super) status: RwSignal<MapStatus>,
    /// The page's heights reader, which the view's terrain boot fills.
    pub(super) heights: StoredValue<TerrainHeights, LocalStorage>,
}

/// The sentence for a mount failure.
fn failure_reason(error: &MapViewError) -> String {
    match error {
        MapViewError::ManifestUnavailable => "the terrain manifest is unavailable".to_string(),
        MapViewError::EngineFailed(reason) => format!("the renderer did not start: {reason}"),
    }
}

/// Mounts the view once both nodes exist, and keeps the overlay in step with the page.
pub(super) fn start(
    parts: MountParts,
    targets: DragTargets,
    solved: Signal<Option<SolvedMission>>,
) {
    let MountParts {
        terrain,
        container,
        canvas,
        status,
        heights,
    } = parts;
    let mut handles = MapViewHandles::new();
    handles.heights = heights.get_value();
    let stored = StoredValue::new_local(handles);
    let atlas_ready = StoredValue::new_local(Rc::new(Cell::new(false)));
    let redraw = RwSignal::new(0_u32);
    let started = StoredValue::new_local(false);
    on_cleanup(move || stored.with_value(MapViewHandles::dispose));

    Effect::new(move |_| {
        let (Some(div), Some(canvas_el)) = (container.get(), canvas.get()) else {
            return;
        };
        if started.get_value() {
            return;
        }
        started.set_value(true);
        let element: web_sys::HtmlElement = div.unchecked_into();
        let handles = stored.get_value();
        let bump: Rc<dyn Fn()> = Rc::new(move || redraw.update(|n| *n = n.wrapping_add(1)));
        attach_marker_drag(&element, &handles, targets, bump.clone());
        let on_click: Rc<dyn Fn(MapClick)> = Rc::new(move |click: MapClick| {
            let placing = targets.placing.get_untracked();
            targets.guns.update(|guns| {
                targets.target.update(|target| {
                    apply_placement(target, guns, placing, click.x, click.y);
                });
            });
        });
        let mount = MapViewMount {
            container: element,
            canvas: canvas_el,
            terrain: terrain.terrain_id().to_string(),
            force_webgl: force_webgl_from_location(),
            preferences: terrain_and_imagery_preferences(),
            report: Rc::new(|_| {}),
            on_click,
        };
        leptos::task::spawn_local(async move {
            let outcome = mount_map_view(mount, handles).await;
            status.set(match outcome {
                Ok(()) => MapStatus::Ready,
                Err(error) => MapStatus::Failed(failure_reason(&error)),
            });
            bump();
        });
    });

    Effect::new(move |_| {
        redraw.track();
        let scene = targets.guns.with(|guns| {
            targets
                .target
                .with(|target| overlay_scene(target, guns, solved.get().as_ref()))
        });
        let Some(div) = container.get_untracked() else {
            return;
        };
        let element: web_sys::HtmlElement = div.unchecked_into();
        stored.with_value(|handles| {
            atlas_ready.with_value(|ready| upload_overlay(handles, &element, ready, &scene));
        });
    });
}
