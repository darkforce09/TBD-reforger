//! The world-occluder bench served at `/debug/world-los`.
//!
//! **Role:** loads the committed object catalogue around a map point straight off
//! `/map-assets/everon/` — manifest, prefabs, the chunks covering the radius, then every
//! descriptor and BLAS those chunks place, over the same `OccluderHost` path the mission editor
//! uses — draws the placed objects as plan footprints on the architectural lanes (buildings with
//! their eye-height section cuts; proxies amber while their geometry is still loading), and probes
//! one A → B segment through `WorldOccluder::evaluate_los`, reporting hits, verdict, concealment
//! and coverage.
//! **Position:** a routed workspace under `v2::apps::debug`, mounted by `app_routes.rs`. Its pure
//! geometry lives in [`super::world_los_scene`]; it reaches the map engine directly and never
//! touches the editor.
//! **Signals & state:** one `Signals` bundle of Leptos `RwSignal`s carries the status line, the
//! verdict, the hit list, coverage and engine errors from the wasm host back to the view. The
//! `RenderEngine` itself is held in an `Rc<RefCell<…>>` handle owned by the mount.
//! **Invariants:** every parameter of a run is in the URL, so a reading reproduces exactly:
//! `?x=9363&y=285&r=150` (map metres; the default is the farmhouse village), `&a=x,y,z` /
//! `&b=x,y,z` (engine-frame ray ends: x, y_up, z_north — by default A and B sit 40 m either side
//! of the centre at the mean row elevation + 1.8 m), `&eye=1.8` (cut plane above the mean
//! elevation) and `&force=webgl` (the headless-backend convention the editor shares). The bench is
//! URL-only and never appears in the navigation. Left-click sets A, the next click sets B; drag
//! pans; the wheel zooms.

use leptos::prelude::*;

/// Default centre: the wooden farmhouse the architectural lanes were built against (chunk 18_0).
pub const DEFAULT_CENTER: [f64; 2] = [9363.0, 285.0];
/// Default radius in map metres of the catalogue window loaded around [`DEFAULT_CENTER`].
pub const DEFAULT_RADIUS_M: f64 = 150.0;
/// Default height in metres above the mean row elevation at which buildings are section-cut and
/// the probe ray is flown.
pub const DEFAULT_EYE_M: f64 = 1.8;
/// Buildings cut at eye height, at most this many (a village), the rest keep their footprint.
pub const MAX_CUT_BUILDINGS: usize = 96;

/// The bench's view: the canvas the engine mounts on, the probe read-outs, and the status line.
#[component]
pub fn WorldLosPage() -> impl IntoView {
    let status = RwSignal::new(String::from("booting…"));
    let verdict = RwSignal::new(String::new());
    let hits = RwSignal::new(Vec::<String>::new());
    let coverage = RwSignal::new(String::new());
    let stats = RwSignal::new(String::new());
    let engine_err = RwSignal::new(Option::<String>::None);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();
    #[cfg(target_arch = "wasm32")]
    live::mount(live::Signals {
        status,
        verdict,
        hits,
        coverage,
        stats,
        engine_err,
        canvas_ref,
    });
    view! {
        <div class="relative h-full w-full bg-surface-container-lowest">
            <canvas node_ref=canvas_ref class="absolute inset-0 h-full w-full touch-none"></canvas>
            <div class="pointer-events-none absolute left-3 top-3 z-10 max-w-[520px] rounded-lg border border-white/10 bg-surface-container-lowest/85 px-3 py-2 text-xs text-on-surface shadow-xl backdrop-blur-xl">
                <div class="font-semibold">"world-los bench — T-090.12.5"</div>
                <div data-world-los-status class="text-on-surface-variant">{move || status.get()}</div>
                <div data-world-los-verdict class="mt-1 font-mono">{move || verdict.get()}</div>
                <div data-world-los-coverage class="text-on-surface-variant">{move || coverage.get()}</div>
                <ul data-world-los-hits class="mt-1 font-mono text-[10px]">
                    <For each=move || hits.get() key=|h| h.clone() let:h>
                        <li>{h}</li>
                    </For>
                </ul>
                <div data-world-los-stats class="mt-1 text-[10px] text-on-surface-variant">{move || stats.get()}</div>
                <div class="mt-1 text-[10px] text-on-surface-variant">"legend: slab = building · white strip = eye-height cut · grey = prop · green = tree · brown = rock · amber outline = proxy (loading) · ray green clear / cyan glass / yellow-green canopy / red blocked / amber provisional"</div>
                <div class="text-[10px] text-on-surface-variant">"click sets A then B · drag pans · wheel zooms · ?x&y&r&a&b&eye&force=webgl"</div>
                {move || engine_err.get().map(|e| view! { <div class="mt-1 text-error">{e}</div> })}
            </div>
        </div>
    }
}

#[cfg(target_arch = "wasm32")]
mod live;
