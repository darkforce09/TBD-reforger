//! The mortar calculator's map: the terrain with the guns, the target and the solution drawn on
//! it, where a click or a drag places a position.
//!
//! **Role:** mounts a terrain-and-imagery map view through
//! [`crate::foundation::map_view::mount::mount_map_view`], offers the placement picker (the target
//! or one gun), writes clicked and dragged positions into the drafts, and redraws the
//! fire-mission overlay whenever the drafts, the solution or the zoom change. `profile` samples
//! the crest profile the solve uses.
//! **Position:** a card of the `/tools/mortar` page between the inputs and Calculate; the drafts
//! and the heights reader are the page's, so the grid fields, the map and the solve always agree.
//! **Signals & state:** the placement signal; per mounted view, the [`MapViewHandles`] (disposed
//! when the view unmounts), the mount status and a redraw counter the wheel bumps.
//! **Invariants:** only a terrain with a served elevation model shows a map (Arland is typed by
//! hand); the view's heights are the page's [`TerrainHeights`], so terrain heights resolve once the
//! raster loads; a placement never names a gun the battery no longer holds; the picker and its map
//! exist in the browser build only.

#[cfg(target_arch = "wasm32")]
mod engine_overlay;
pub(crate) mod marks;
#[cfg(target_arch = "wasm32")]
mod mount;
pub(crate) mod picking;
pub(crate) mod profile;

#[cfg(target_arch = "wasm32")]
use super::inputs::battery::GunDraft;
#[cfg(target_arch = "wasm32")]
use super::inputs::positions::MortarTerrain;
#[cfg(target_arch = "wasm32")]
use super::inputs::positions::PositionDraft;
#[cfg(target_arch = "wasm32")]
use super::inputs::INPUT_CLASS;
#[cfg(target_arch = "wasm32")]
use super::solve_bridge::SolvedMission;
#[cfg(target_arch = "wasm32")]
use crate::foundation::map_view::terrain_height::TerrainHeights;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use picking::{placement_options, valid_placement, Placement};

/// The map card's class.
#[cfg(target_arch = "wasm32")]
const CARD_MAP: &str = "flex flex-col gap-3 rounded-xl p-4 glass";

/// The map picker for `terrain`, placing into `target` and `guns`, drawing `solved`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn map_picker(
    terrain: RwSignal<MortarTerrain>,
    target: RwSignal<PositionDraft>,
    guns: RwSignal<Vec<GunDraft>>,
    solved: Signal<Option<SolvedMission>>,
    heights: StoredValue<TerrainHeights, LocalStorage>,
) -> impl IntoView {
    let placing = RwSignal::new(Placement::Target);
    Effect::new(move |_| {
        let current = placing.get_untracked();
        let valid = guns.with(|all| valid_placement(current, all));
        if valid != current {
            placing.set(valid);
        }
    });
    view! {
        <section class=CARD_MAP data-mortar-map="">
            <div class="flex flex-wrap items-end gap-3">
                <label class="text-sm">
                    "Place on the map"
                    <select
                        prop:value=move || placing.get().option_value()
                        on:change=move |ev| {
                            if let Some(p) = Placement::from_option_value(&event_target_value(&ev)) {
                                placing.set(p);
                            }
                        }
                        class=INPUT_CLASS
                        data-mortar-input="placement"
                    >
                        {move || {
                            guns.with(|all| placement_options(all))
                                .into_iter()
                                .map(|(p, label)| view! { <option value=p.option_value()>{label}</option> })
                                .collect_view()
                        }}
                    </select>
                </label>
                <p class="text-xs text-on-surface-variant">
                    "Click the map to place the chosen position; drag a marker to move it."
                </p>
            </div>
            {move || match terrain.get() {
                MortarTerrain::Arland => view! {
                    <p class="text-sm text-on-surface-variant" data-mortar-map-state="no-map">
                        "Arland has no served elevation model: type the grid references and the heights."
                    </p>
                }
                .into_any(),
                MortarTerrain::Everon => {
                    map_canvas(MortarTerrain::Everon, target, guns, placing, solved, heights).into_any()
                }
            }}
        </section>
    }
}

/// The mounted view of `terrain`: the canvas, its mount status and, in the browser build, the
/// mount itself.
#[cfg(target_arch = "wasm32")]
fn map_canvas(
    terrain: MortarTerrain,
    target: RwSignal<PositionDraft>,
    guns: RwSignal<Vec<GunDraft>>,
    placing: RwSignal<Placement>,
    solved: Signal<Option<SolvedMission>>,
    heights: StoredValue<TerrainHeights, LocalStorage>,
) -> impl IntoView {
    let container = NodeRef::<leptos::html::Div>::new();
    let canvas = NodeRef::<leptos::html::Canvas>::new();
    let status = RwSignal::new(MapStatus::Loading);
    mount::start(
        mount::MountParts {
            terrain,
            container,
            canvas,
            status,
            heights,
        },
        engine_overlay::DragTargets {
            target,
            guns,
            placing,
        },
        solved,
    );
    view! {
        <div
            node_ref=container
            class="relative h-[28rem] w-full touch-none overflow-hidden rounded-lg bg-surface-container"
            data-mortar-map-state=move || status.get().attribute_value()
        >
            <canvas node_ref=canvas class="absolute inset-0 h-full w-full"></canvas>
            {move || {
                status
                    .get()
                    .message()
                    .map(|text| {
                        view! {
                            <p class="absolute left-3 top-3 rounded bg-surface-glass px-2 py-1 text-xs text-on-surface-variant">
                                {text}
                            </p>
                        }
                    })
            }}
        </div>
    }
}

/// Where the map mount stands.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MapStatus {
    /// The engine or the terrain is still loading.
    Loading,
    /// The terrain is booted.
    Ready,
    /// The mount failed; the reason.
    Failed(String),
}

#[cfg(any(target_arch = "wasm32", test))]
impl MapStatus {
    /// The value of the map's `data-mortar-map-state` attribute.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn attribute_value(&self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Ready => "ready",
            Self::Failed(_) => "failed",
        }
    }

    /// The line shown over the map; `None` once it is ready.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn message(&self) -> Option<String> {
        match self {
            Self::Loading => Some("Loading the map…".to_string()),
            Self::Ready => None,
            Self::Failed(reason) => Some(format!(
                "The map could not load ({reason}); type the grid references instead."
            )),
        }
    }
}

#[cfg(test)]
#[path = "../tests/map_picker.rs"]
mod tests;
