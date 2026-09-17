//! Role: the Line-of-Sight overlay — the DOM/SVG surface the engine's LoS tool draws through.
//! Position: `editor/input/tools` in the frontend editor.
//! Signals & state: three host heartbeats (pan cursor, zoom sampler, state tick) and the leaked
//! tool state this module hands to the engine's registry.
//! Invariants: every decidable thing — the occlusion rule, the capture machine, the projection and
//! the chart geometry — belongs to `website_map_engine::editing::tools::line_of_sight`. What lives
//! here is the drawing and the ownership: an absolutely positioned, `pointer-events-none` SVG plus
//! one inline profile panel anchored by the target, and the mount-scoped installs that make the
//! engine's registry answer "nothing here" once this surface is gone.
//!
//! The overlay re-runs off the same cursor / zoom / tick heartbeats the ruler and scale bar use —
//! never a frame loop of its own. Native builds render nothing (no engine, no `window`); the
//! geometry is proven engine-side.

// The wasm host wires the live path; on native the overlay renders nothing and the installs have
// no caller, so every item below reads as dead there.
#![allow(dead_code)]

use leptos::prelude::*;
use website_map_engine::editing::tools::selection;

use website_map_engine::editing::tools::line_of_sight::capture::{LosState, ViewshedState};
use website_map_engine::editing::tools::line_of_sight::host_registry::{
    read_registered_state, LOS_SAMPLER, LOS_STATE, VIEWSHED_STATE,
};
use website_map_engine::editing::tools::line_of_sight::object_verdict;
use website_map_engine::editing::tools::line_of_sight::projection::{ProfileChart, ProjectedShot};
use website_map_engine::editing::tools::line_of_sight::terrain_verdict::LosVerdict;

use crate::v2::apps::editor::input::tools::ruler_tool::install_seam;

/// The inline profile panel's chart box in CSS px. Width and height of the elevation curve area,
/// excluding the header text. A drawing dimension, so it lives with the drawing.
pub const PANEL_W_PX: f64 = 180.0;

/// The profile panel's chart height in CSS px.
pub const PANEL_H_PX: f64 = 64.0;

/// Hand the host's leaked LoS state to the engine's registry so [`LosOverlay`] can read it.
///
/// **This is an INSTALL** ([`install_seam`]): the state is unregistered at the registering owner's
/// cleanup, and a remount's newer state is not clobbered by the old owner's cleanup. Without it the
/// registry keeps returning a dead page's shot as though the tool were live.
pub fn register_los_state(state: std::rc::Rc<std::cell::RefCell<LosState>>) {
    install_seam(&LOS_STATE, state);
}

/// Hand the host's DEM point-sampler (world x, y → ground metres, `None` off coverage) to the
/// engine's registry, so a profile can be rebuilt as the camera pans.
///
/// **This is an INSTALL** ([`install_seam`]), and here it is load-bearing for correctness rather
/// than only for freshness: a sampler closing over a dropped page's DEM would keep answering with
/// elevations from a terrain that is no longer open. After unmount the registry reports `None` — the
/// honest "no DEM here" the engine already handles — instead of `Some` over stale ground.
pub fn register_los_sampler(sampler: std::rc::Rc<dyn Fn(f64, f64) -> Option<f64>>) {
    install_seam(&LOS_SAMPLER, sampler);
}

/// Hand the host's leaked viewshed state to the engine's registry.
///
/// **This is an INSTALL** ([`install_seam`]), for the same reason as its peers above: after unmount
/// the registry must report the empty state rather than a dead page's observer disc.
pub fn register_viewshed_state(state: std::rc::Rc<std::cell::RefCell<ViewshedState>>) {
    install_seam(&VIEWSHED_STATE, state);
}

/// the blocking point, PLUS an inline HTML profile panel anchored by the target (Decision 2). It
/// reads the live camera off the registered engine (`world_assets::camera_snapshot`, the same seam
/// the ruler / scale bar use), rebuilds the terrain profile via the registered DEM sampler, and
/// re-runs off the `cursor` (pan) + `debug_hud` (~1 Hz zoom) heartbeats + a `tick` bumped on every
/// state mutation — NO new rAF loop.
///
/// The state is read via [`read_registered_state`] — a cheap clone of the leaked host [`LosState`]
/// (session-local overlay state — Decision 4). Native builds render nothing (no engine, no
/// `window`); the geometry is proven by `project_shot` / `profile_chart` / `occlusion` above.
#[component]
pub fn LosOverlay(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-projection). Also
    /// the live cursor the rubber-band line draws to while an observer is placed but no target yet.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler (a wheel-zoom with a still pointer re-projects
    /// within a second). `Option` so the mount can forward `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped by the host on every state mutation (click / Esc / tool-switch) so a click repaints
    /// even with a still pointer.
    tick: RwSignal<u64>,
) -> impl IntoView {
    // The overlay's three draw lists, ALL as `Vec` so each `<For each>` is a plain field access (no
    // turbofish inside the view macro): the placed shot (0 or 1 `ProjectedShot`), the rubber-band
    // line while capturing (0 or 1 `[x1,y1,x2,y2]`), and the profile-panel pairs (0 or 1
    // `(ProjectedShot, ProfileChart)`). All derived from the current state + camera + live DEM
    // sampler. One `derived()` call feeds three thin accessor closures below so the heavy compute
    // runs once per reactive tick.
    #[allow(clippy::type_complexity)]
    let derived = move || -> (
        Vec<ProjectedShot>,
        Vec<(f64, f64, f64, f64)>,
        Vec<(ProjectedShot, ProfileChart)>,
    ) {
        // Subscribe to all three heartbeats so the closure re-runs on pan (cursor), zoom (hud) and
        // any state edit (tick).
        let cur = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        let _ = tick.get();
        let state = read_registered_state();
        if state.is_empty() {
            return (Vec::new(), Vec::new(), Vec::new());
        }
        #[cfg(target_arch = "wasm32")]
        {
            use website_map_engine::editing::tools::line_of_sight::projection::{
                profile_chart, project_shot,
            };
            use website_map_engine::editing::tools::line_of_sight::terrain_survey::build_profile;
            use website_map_engine::editing::tools::line_of_sight::terrain_verdict::{
                EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M,
            };

            let Some((tx, ty, zoom)) = website_map_engine::streaming::host::camera_snapshot() else {
                return (Vec::new(), Vec::new(), Vec::new());
            };
            let Some(win) = web_sys::window() else {
                return (Vec::new(), Vec::new(), Vec::new());
            };
            let vw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
            if vw <= 0.0 || vh <= 0.0 {
                return (Vec::new(), Vec::new(), Vec::new());
            }
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
            let project = move |x: f64, y: f64| {
                let p = cam.project([x, y, 0.0]);
                (p[0], p[1])
            };
            // Rubber-band: a placed observer awaiting its target → line from the observer to the
            // live cursor, so the operator aims the target click.
            let mut rubber = Vec::new();
            if let (Some((ox, oy, _)), None) = (state.pending_obs, state.shot) {
                if let Some((cwx, cwy, _)) = cur {
                    let (x1, y1) = project(ox, oy);
                    let (x2, y2) = project(cwx, cwy);
                    rubber.push((x1, y1, x2, y2));
                }
            }
            // A completed shot → project it + build the profile panel from the live DEM sampler.
            let (mut shots, mut panels) = (Vec::new(), Vec::new());
            if let Some(shot) = state.shot {
                let profile = build_profile(&shot);
                let mut proj = project_shot(
                    &shot,
                    &profile,
                    EYE_HEIGHT_OBSERVER_M,
                    EYE_HEIGHT_TARGET_M,
                    &project,
                );
                // The object layer: terrain and objects, marker at the nearer block.
                object_verdict::apply_objects(
                    &mut proj,
                    crate::v2::apps::editor::input::tools::los_world_wasm::object_verdict(&shot),
                );
                let chart = profile_chart(
                    &profile,
                    EYE_HEIGHT_OBSERVER_M,
                    EYE_HEIGHT_TARGET_M,
                    PANEL_W_PX,
                    PANEL_H_PX,
                );
                shots.push(proj.clone());
                panels.push((proj, chart));
            }
            (shots, rubber, panels)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = cur;
            (Vec::new(), Vec::new(), Vec::new())
        }
    };

    view! {
        // Full-bleed, non-interactive overlay. z-10 sits it in the same band as the ruler overlay /
        // MapGridRefs — over the map, under the chrome docks. `pointer-events-none` so it never eats
        // a map gesture (the click capture is the map's own pointer handlers, not this SVG).
        <div data-los-overlay class="pointer-events-none absolute inset-0 z-10">
            <svg class="absolute inset-0" width="100%" height="100%">
                // Rubber-band preview — observer → live cursor while awaiting the target (dashed).
                <For
                    each=move || derived().1
                    key=|rb| format!("{:.0}:{:.0}:{:.0}:{:.0}", rb.0, rb.1, rb.2, rb.3)
                    let:rb
                >
                    <line
                        x1=move || format!("{:.1}", rb.0)
                        y1=move || format!("{:.1}", rb.1)
                        x2=move || format!("{:.1}", rb.2)
                        y2=move || format!("{:.1}", rb.3)
                        class="stroke-primary/60"
                        stroke-width="1.5"
                        stroke-dasharray="4 4"
                    />
                </For>
                // The placed sight line + endpoint dots + blocking marker. Keyed by world coord
                // (T-727) so re-placing the target swaps the node cleanly. Colour encodes the verdict:
                // clear → success, blocked → error, unknown → neutral outline.
                <For
                    each=move || derived().0
                    key=|shot| shot.key.clone()
                    let:shot
                >
                    <line
                        x1=move || format!("{:.1}", shot.obs_px)
                        y1=move || format!("{:.1}", shot.obs_py)
                        x2=move || format!("{:.1}", shot.tgt_px)
                        y2=move || format!("{:.1}", shot.tgt_py)
                        class=los_line_class(object_verdict::styling_of(&shot))
                        stroke-width="1.5"
                    />
                    // Observer dot.
                    <circle
                        cx=move || format!("{:.1}", shot.obs_px)
                        cy=move || format!("{:.1}", shot.obs_py)
                        r="3"
                        class="fill-surface-container-lowest stroke-primary"
                        stroke-width="1.5"
                    />
                    // Target dot.
                    <circle
                        cx=move || format!("{:.1}", shot.tgt_px)
                        cy=move || format!("{:.1}", shot.tgt_py)
                        r="3"
                        class=los_dot_class(object_verdict::styling_of(&shot))
                        stroke-width="1.5"
                    />
                    // Blocking-point marker (only when blocked).
                    {shot.block_px.map(|(bx, by)| view! {
                        <circle
                            cx=format!("{bx:.1}")
                            cy=format!("{by:.1}")
                            r="4"
                            class="fill-error stroke-surface-container-lowest"
                            stroke-width="1.5"
                        />
                    })}
                </For>
            </svg>
            // The inline profile panel (Decision 2) — anchored by the TARGET point, keyed by world
            // coord. A small elevation chart (ground curve + sight line) with the verdict header.
            <For
                each=move || derived().2
                key=|(shot, _)| shot.key.clone()
                let:panel
            >
                {
                    let (shot, chart) = panel;
                    // Anchor the panel just above-right of the target; nudge so it doesn't sit on the
                    // dot. Positioned in CSS px from the projected target pixel.
                    let left = shot.tgt_px + 12.0;
                    let top = shot.tgt_py - PANEL_H_PX - 28.0;
                    // T-090.12.5 — terrain ∧ objects: the header names the nearer blocker (or the
                    // canopy concealment / "objects not loaded"); styling follows the pair.
                    let style = object_verdict::styling_of(&shot);
                    let verdict_text = object_verdict::format_combined(
                        &object_verdict::combine(shot.verdict, shot.objects.clone()),
                        shot.total_m,
                    );
                    view! {
                        <div
                            data-los-panel
                            class="absolute rounded-lg border border-white/10 bg-surface-container-lowest/85 px-2 py-1.5 shadow-xl backdrop-blur-xl"
                            style=format!("left:{left:.1}px;top:{top:.1}px;width:{:.0}px", PANEL_W_PX + 16.0)
                        >
                            <div class=los_header_class(style)>
                                {verdict_text}
                            </div>
                            <svg
                                class="mt-1 block"
                                width=format!("{PANEL_W_PX:.0}")
                                height=format!("{PANEL_H_PX:.0}")
                            >
                                // The straight sight line (drawn first, under the ground).
                                <polyline
                                    points=chart.line.clone()
                                    fill="none"
                                    class=los_line_class(style)
                                    stroke-width="1.5"
                                    stroke-dasharray="3 3"
                                />
                                // The terrain ground curve.
                                <polyline
                                    points=chart.ground.clone()
                                    fill="none"
                                    class="stroke-on-surface-variant"
                                    stroke-width="1.5"
                                />
                                // Blocking marker on the ground curve, when blocked.
                                {chart.block.map(|(bx, by)| view! {
                                    <circle
                                        cx=format!("{bx:.1}")
                                        cy=format!("{by:.1}")
                                        r="3"
                                        class="fill-error"
                                    />
                                })}
                            </svg>
                        </div>
                    }
                }
            </For>
        </div>
    }
}

/// The SVG stroke class for the sight LINE by verdict: clear → success, blocked → error, unknown →
/// neutral outline. A pure helper so the overlay closures stay terse and the mapping is one place.
#[must_use]
fn los_line_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "stroke-success",
        LosVerdict::Blocked { .. } => "stroke-error",
        LosVerdict::Unknown => "stroke-outline",
    }
}

/// The target DOT fill/stroke class by verdict (mirrors the line colour on the fill).
#[must_use]
fn los_dot_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "fill-success stroke-surface-container-lowest",
        LosVerdict::Blocked { .. } => "fill-error stroke-surface-container-lowest",
        LosVerdict::Unknown => "fill-outline stroke-surface-container-lowest",
    }
}

/// The panel HEADER text class by verdict (coloured verdict word).
#[must_use]
fn los_header_class(v: LosVerdict) -> &'static str {
    match v {
        LosVerdict::Clear => "font-mono text-code-md text-success",
        LosVerdict::Blocked { .. } => "font-mono text-code-md text-error",
        LosVerdict::Unknown => "font-mono text-code-md text-on-surface-variant",
    }
}
