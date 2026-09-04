//! `/debug/world-los` — T-090.12.5: the world-occluder bench. Loads the committed catalogue
//! around a map point straight off `/map-assets/everon/` (manifest, prefabs, the chunks covering
//! the radius, then every descriptor + BLAS those chunks place — the same `OccluderHost` path
//! the mission editor uses), draws the placed objects as plan footprints on the T-090.11.6
//! architectural lanes (buildings with their eye-height section cuts; proxies amber while their
//! geometry is still loading), and probes one A → B segment through `WorldOccluder::evaluate_los`
//! — hits, verdict, concealment, coverage — as the reproducible-by-URL instrument for the
//! object LOS layer.
//!
//! URL: `?x=9363&y=285&r=150` (map metres; default = the farmhouse village), `&a=x,y,z` /
//! `&b=x,y,z` (engine-frame ray ends: x, y_up, z_north — default A/B 40 m either side of the
//! centre at the mean row elevation + 1.8 m), `&eye=1.8` (cut plane above the mean elevation),
//! `&force=webgl` (the editor's headless-backend convention). LMB click sets A, the next click B;
//! drag pans; wheel zooms.

use leptos::prelude::*;

/// Default centre: the FarmHouse_E_1L01_Wood the T-090.11 program was built on (chunk 18_0).
pub const DEFAULT_CENTER: [f64; 2] = [9363.0, 285.0];
pub const DEFAULT_RADIUS_M: f64 = 150.0;
pub const DEFAULT_EYE_M: f64 = 1.8;
/// Buildings cut at eye height, at most this many (a village), the rest keep their footprint.
pub const MAX_CUT_BUILDINGS: usize = 96;

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
mod live {
    use super::super::building_interior::InteriorLanes;
    use super::super::building_viewer::geom::screen_to_world;
    use super::super::world_los_scene::{build_bench_lanes, ray_strip, Footprint};
    use super::{DEFAULT_CENTER, DEFAULT_EYE_M, DEFAULT_RADIUS_M, MAX_CUT_BUILDINGS};
    use crate::editor::world_assets::{fetch_bytes, fetch_text, OccluderHost};
    use leptos::prelude::*;
    use map_engine_core::building_section::section_at;
    use map_engine_core::world::occluder::{WorldOccluder, WorldVerdict};
    use map_engine_core::world::WorldResidency;
    use map_engine_render::draw_order::role_id;
    use map_engine_render::RenderEngine;
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;
    type RafSlot = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

    pub struct Signals {
        pub status: RwSignal<String>,
        pub verdict: RwSignal<String>,
        pub hits: RwSignal<Vec<String>>,
        pub coverage: RwSignal<String>,
        pub stats: RwSignal<String>,
        pub engine_err: RwSignal<Option<String>>,
        pub canvas_ref: NodeRef<leptos::html::Canvas>,
    }

    /// The loaded world around the centre.
    struct Bench {
        host: OccluderHost,
        _residency: WorldResidency,
        center: [f64; 2],
        radius: f64,
        /// Mean row elevation (engine y) inside the radius — the bench's stand-in for ground.
        ground_y: f64,
        eye_m: f64,
    }

    fn query(name: &str) -> Option<String> {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                web_sys::UrlSearchParams::new_with_str(&s)
                    .ok()
                    .and_then(|p| p.get(name))
            })
            .filter(|v| !v.is_empty())
    }

    fn query_f64(name: &str) -> Option<f64> {
        query(name)
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
    }

    fn query_point(name: &str) -> Option<[f64; 3]> {
        let v = query(name)?;
        let mut it = v.split(',').map(|s| s.trim().parse::<f64>().ok());
        let (x, y, z) = (it.next()??, it.next()??, it.next()??);
        (x.is_finite() && y.is_finite() && z.is_finite()).then_some([x, y, z])
    }

    fn start_raf(engine: EngineHandle, disposed: std::sync::Arc<std::sync::atomic::AtomicBool>) {
        use std::sync::atomic::Ordering;
        let f: RafSlot = Rc::new(RefCell::new(None));
        let g = f.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
            if disposed.load(Ordering::Relaxed) {
                f.borrow_mut().take();
                return;
            }
            if let Ok(mut guard) = engine.try_borrow_mut() {
                if let Some(e) = guard.as_mut() {
                    let _ = e.render();
                    e.poll();
                }
            }
            let cb = f.borrow();
            if let (Some(cb), Some(win)) = (cb.as_ref(), web_sys::window()) {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
        }) as Box<dyn FnMut()>));
        let cb = g.borrow();
        if let (Some(cb), Some(win)) = (cb.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }

    /// Footprints (map frame) of every placed row within the radius, and the buildings' section
    /// cuts at `eye_y` (engine y) as world plan segments.
    fn scene_of(
        occ: &WorldOccluder,
        center: [f64; 2],
        radius: f64,
        eye_y: f64,
    ) -> (Vec<Footprint>, Vec<[[f64; 2]; 2]>, usize) {
        let mut fps = Vec::new();
        let mut cuts: Vec<[[f64; 2]; 2]> = Vec::new();
        let mut cut_buildings = 0usize;
        for id in occ.resident_chunk_ids() {
            let (Some(rows), Some(boxes)) = (occ.chunk_rows(&id), occ.chunk_boxes(&id)) else {
                continue;
            };
            for (row, bx) in rows.iter().zip(boxes) {
                let dx = f64::from(row.pos[0]) - center[0];
                let dz = f64::from(row.pos[2]) - center[1];
                if dx.hypot(dz) > radius {
                    continue;
                }
                if bx.0[0] > bx.1[0] {
                    continue; // NO_BOX: never blocks (blocks:false or unknown pid)
                }
                let kind = occ.kind_of(row.pid).unwrap_or("prop").to_string();
                let expanded = occ.expanded_of(row.pid);
                fps.push(Footprint {
                    pid: row.pid,
                    kind: kind.clone(),
                    min: [bx.0[0], bx.0[2]],
                    max: [bx.1[0], bx.1[2]],
                    proxy: expanded.is_none() && !occ.is_no_block(row.pid),
                });
                // Eye-height cut of an upright building's every instance (yaw-only rows).
                if kind == "building"
                    && cut_buildings < MAX_CUT_BUILDINGS
                    && row.angles_deg[0] == 0.0
                    && row.angles_deg[2] == 0.0
                {
                    if let Some(po) = expanded {
                        cut_buildings += 1;
                        let world = row.rigid();
                        let y_prefab = (eye_y - f64::from(row.pos[1])) / f64::from(row.scale);
                        for inst in &po.instances {
                            let place = inst.placement();
                            let y_blas = (y_prefab - place.t[1]) / place.scale;
                            for s in section_at(&inst.blas, y_blas, 0.9) {
                                let a = world.point(place.point([s[0][0], y_blas, s[0][1]]));
                                let b = world.point(place.point([s[1][0], y_blas, s[1][1]]));
                                cuts.push([[a[0], a[2]], [b[0], b[2]]]);
                            }
                        }
                    }
                }
            }
        }
        (fps, cuts, cut_buildings)
    }

    fn upload_lanes(e: &mut RenderEngine, l: &InteriorLanes) {
        e.upload_polygon_mesh(
            role_id::INTERIOR_SLABS,
            &l.slabs_pos,
            &l.slabs_col,
            &l.slabs_idx,
            1,
            true,
        );
        e.upload_polygon_mesh(
            role_id::INTERIOR_FURNITURE,
            &l.furniture_pos,
            &l.furniture_col,
            &l.furniture_idx,
            1,
            true,
        );
        e.upload_hairline_segments(
            role_id::INTERIOR_FURNITURE_OUTLINE,
            &l.furniture_outline,
            l.furniture_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_WALLS, &l.walls, l.wall_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_WALLS_OUTLINE,
            &l.walls_outline,
            l.walls_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_PORTALS, &l.portals, l.portal_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_PORTALS_OUTLINE,
            &l.portals_outline,
            l.portals_outline_count,
            true,
        );
        e.upload_strip_tris(role_id::INTERIOR_GLAZING, &l.glazing, l.glazing_count, true);
        e.upload_hairline_segments(
            role_id::INTERIOR_GLAZING_OUTLINE,
            &l.glazing_outline,
            l.glazing_outline_count,
            true,
        );
        e.upload_hairline_segments(role_id::INTERIOR_STAIRS, &l.stairs, l.stairs_count, true);
        e.upload_polygon_mesh(
            role_id::SCENE_VEGETATION,
            &l.vegetation_pos,
            &l.vegetation_col,
            &l.vegetation_idx,
            1,
            true,
        );
        e.upload_hairline_segments(
            role_id::SCENE_VEGETATION_OUTLINE,
            &l.vegetation_outline,
            l.vegetation_outline_count,
            true,
        );
    }

    /// Probe A → B through the occluder: HUD lines + the probe lane.
    fn probe(engine: &EngineHandle, bench: &Bench, a: [f64; 3], b: [f64; 3], s: &Signals) {
        let occ = bench.host.occluder();
        let t0 = js_sys::Date::now();
        let los = occ.evaluate_los(a, b);
        let ms = js_sys::Date::now() - t0;
        let total = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let blocker = los.blocker.as_ref().map(|bl| {
            format!(
                " — {} ({}) pid {} chunk {} row {} {:?} at {:.0} m",
                occ.label_of(bl.pid).unwrap_or("?"),
                occ.kind_of(bl.pid).unwrap_or("?"),
                bl.pid,
                bl.chunk,
                bl.row,
                bl.fidelity,
                bl.t * total
            )
        });
        s.verdict.set(format!(
            "A [{:.1}, {:.1}, {:.1}] → B [{:.1}, {:.1}, {:.1}] · {:.0} m · {:?} · concealment {:.2} · {:.1} ms{}",
            a[0], a[1], a[2], b[0], b[1], b[2], total, los.verdict, los.concealment, ms,
            blocker.unwrap_or_default()
        ));
        s.coverage.set(format!(
            "coverage: {} chunks crossed · missing {:?} · proxy pids {:?} · BLAS pending {}",
            los.coverage.chunks_crossed,
            los.coverage.chunks_missing,
            los.coverage.proxy_pids,
            los.coverage.blas_pending.len()
        ));
        s.hits.set(
            los.hits
                .iter()
                .take(16)
                .map(|h| {
                    format!(
                        "t {:.3} · {:.0} m · {:?} · {} · c {:.2}",
                        h.t,
                        h.t * total,
                        h.kind,
                        h.id,
                        h.concealment
                    )
                })
                .collect(),
        );
        let provisional = los.verdict == WorldVerdict::Provisional;
        let (packed, count) = ray_strip(
            [a[0], a[2]],
            [b[0], b[2]],
            &los.hits,
            los.verdict == WorldVerdict::Clear,
            provisional,
        );
        if let Ok(mut guard) = engine.try_borrow_mut() {
            if let Some(e) = guard.as_mut() {
                e.upload_strip_tris(role_id::INTERIOR_PROBE, &packed, count, true);
            }
        }
    }

    async fn load(
        center: [f64; 2],
        radius: f64,
        eye_m: f64,
        status: RwSignal<String>,
    ) -> Result<Bench, String> {
        let base = "/map-assets/everon".to_string();
        let manifest = fetch_text(&format!("{base}/manifest.json"))
            .await
            .ok_or("manifest.json unreachable")?;
        let mut residency = WorldResidency::new();
        residency
            .load_manifest_json(&manifest)
            .map_err(|e| format!("manifest: {e:?}"))?;
        let prefabs = fetch_bytes(&format!("{base}/objects/prefabs.json.gz"))
            .await
            .ok_or("prefabs.json.gz unreachable")?;
        residency
            .load_prefabs_gz(&prefabs)
            .map_err(|e| format!("prefabs: {e:?}"))?;
        let chunk_m = residency.chunk_size_m();
        let cells = |v: f64| ((v / chunk_m).floor() as i64).clamp(0, 63);
        let (cx0, cx1) = (cells(center[0] - radius), cells(center[0] + radius));
        let (cy0, cy1) = (cells(center[1] - radius), cells(center[1] + radius));
        let mut ids = Vec::new();
        for cx in cx0..=cx1 {
            for cy in cy0..=cy1 {
                ids.push(format!("{cx}_{cy}"));
            }
        }
        status.set(format!(
            "fetching {} chunk(s) around ({:.0}, {:.0}) r {radius:.0} m…",
            ids.len(),
            center[0],
            center[1]
        ));
        let futs = ids.iter().map(|id| {
            let url = format!("{base}/objects/chunks/{id}.json.gz");
            let id = id.clone();
            async move { (id, fetch_bytes(&url).await) }
        });
        let mut ingested = 0usize;
        for (id, bytes) in futures::future::join_all(futs).await {
            if let Some(b) = bytes {
                if residency.ingest_chunk_gz(&id, &b).is_ok() {
                    ingested += 1;
                }
            }
        }
        let mut host = OccluderHost::new();
        host.init(&base, &residency).await;
        let mut passes = 0usize;
        while host.run_viewport(&mut residency).await && passes < 64 {
            passes += 1;
            status.set(format!(
                "loading geometry… pass {passes}: {} descriptors expanded · {} BLAS",
                host.occluder().expanded_count(),
                host.occluder().blas_count()
            ));
        }
        // Ground stand-in: the mean row elevation inside the radius.
        let (mut sum, mut n) = (0.0f64, 0usize);
        for id in host.occluder().resident_chunk_ids() {
            if let Some(rows) = host.occluder().chunk_rows(&id) {
                for r in rows {
                    if (f64::from(r.pos[0]) - center[0]).hypot(f64::from(r.pos[2]) - center[1])
                        <= radius
                    {
                        sum += f64::from(r.pos[1]);
                        n += 1;
                    }
                }
            }
        }
        let ground_y = if n > 0 { sum / n as f64 } else { 0.0 };
        status.set(format!(
            "{ingested} chunk(s) ingested · {} passes · {} descriptors expanded · {} BLAS · {:.1} MB · ground ≈ {ground_y:.1} m ({n} rows in radius)",
            passes,
            host.occluder().expanded_count(),
            host.occluder().blas_count(),
            host.occluder().memory_bytes() as f64 / 1_048_576.0
        ));
        Ok(Bench {
            host,
            _residency: residency,
            center,
            radius,
            ground_y,
            eye_m,
        })
    }

    pub fn mount(s: Signals) {
        let center = [
            query_f64("x").unwrap_or(DEFAULT_CENTER[0]),
            query_f64("y").unwrap_or(DEFAULT_CENTER[1]),
        ];
        let radius = query_f64("r")
            .unwrap_or(DEFAULT_RADIUS_M)
            .clamp(20.0, 600.0);
        let eye_m = query_f64("eye").unwrap_or(DEFAULT_EYE_M);
        let force_webgl = query("force").as_deref() == Some("webgl");
        let engine: EngineHandle = Rc::new(RefCell::new(None));
        let bench: Rc<RefCell<Option<Bench>>> = Rc::new(RefCell::new(None));
        let disposed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let css = RwSignal::new((0.0f64, 0.0f64));
        let cam = RwSignal::new((center[0], center[1], 2.0f64));
        let a = RwSignal::new(query_point("a"));
        let b = RwSignal::new(query_point("b"));
        let next_is_a = RwSignal::new(true);
        let engine_ready = RwSignal::new(false);
        let loaded = RwSignal::new(false);
        let s = Rc::new(s);

        let sync_cam = move |e: &RenderEngine| cam.set((e.target_x(), e.target_y(), e.zoom()));

        // Engine mount once the canvas exists.
        Effect::new({
            let engine = engine.clone();
            let disposed = disposed.clone();
            let s = s.clone();
            move |_| {
                let Some(canvas) = s.canvas_ref.get() else {
                    return;
                };
                if engine.borrow().is_some() {
                    return;
                }
                let canvas: web_sys::HtmlCanvasElement = canvas;
                let rect = canvas.get_bounding_client_rect();
                let (cw, ch) = (rect.width().max(64.0), rect.height().max(64.0));
                let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    canvas.set_width(((cw * dpr + 0.5).floor().max(1.0)) as u32);
                    canvas.set_height(((ch * dpr + 0.5).floor().max(1.0)) as u32);
                }
                css.set((cw, ch));
                let engine = engine.clone();
                let disposed = disposed.clone();
                let s = s.clone();
                leptos::task::spawn_local(async move {
                    match RenderEngine::create(canvas, force_webgl).await {
                        Ok(mut e) => {
                            let _ = e.resize(cw, ch, dpr);
                            e.set_camera_bounds(
                                center[0] - 4.0 * radius,
                                center[1] - 4.0 * radius,
                                center[0] + 4.0 * radius,
                                center[1] + 4.0 * radius,
                            );
                            let zoom = (cw.min(ch) / (2.4 * radius))
                                .log2()
                                .min(map_engine_core::camera::MAX_ZOOM);
                            e.set_view(center[0], center[1], zoom);
                            e.hide_calibration();
                            e.disable_frame_timing();
                            e.set_continuous_render(false);
                            e.set_clear_color(0.043, 0.055, 0.075);
                            sync_cam(&e);
                            *engine.borrow_mut() = Some(e);
                            start_raf(engine.clone(), disposed.clone());
                            engine_ready.set(true);
                        }
                        Err(err) => s
                            .engine_err
                            .set(Some(format!("engine create failed: {err:?}"))),
                    }
                });
            }
        });

        // The world: chunks + descriptors + BLAS, then the static lanes and the default ray.
        {
            let bench = bench.clone();
            let s = s.clone();
            leptos::task::spawn_local(async move {
                match load(center, radius, eye_m, s.status).await {
                    Ok(b0) => {
                        let eye_y = b0.ground_y + b0.eye_m;
                        if a.get_untracked().is_none() {
                            a.set(Some([center[0] - 40.0, eye_y, center[1]]));
                        }
                        if b.get_untracked().is_none() {
                            b.set(Some([center[0] + 40.0, eye_y, center[1]]));
                        }
                        *bench.borrow_mut() = Some(b0);
                        loaded.set(true);
                    }
                    Err(e) => s.status.set(format!("load failed: {e}")),
                }
            });
        }

        // Static lanes once both the engine and the world are up.
        Effect::new({
            let engine = engine.clone();
            let bench = bench.clone();
            let s = s.clone();
            move |_| {
                if !engine_ready.get() || !loaded.get() {
                    return;
                }
                let t0 = js_sys::Date::now();
                let guard = bench.borrow();
                let Some(b0) = guard.as_ref() else {
                    return;
                };
                let eye_y = b0.ground_y + b0.eye_m;
                let (fps, cuts, cut_buildings) =
                    scene_of(b0.host.occluder(), b0.center, b0.radius, eye_y);
                let lanes = build_bench_lanes(&fps, &cuts);
                let proxies = fps.iter().filter(|f| f.proxy).count();
                if let Ok(mut g) = engine.try_borrow_mut() {
                    if let Some(e) = g.as_mut() {
                        upload_lanes(e, &lanes);
                    }
                }
                s.stats.set(format!(
                    "{} footprints ({} proxies) · {} buildings cut at y {:.1} m ({} segments) · lanes built in {:.0} ms · occluder {} chunks / {} expanded / {} BLAS / {:.1} MB",
                    fps.len(),
                    proxies,
                    cut_buildings,
                    eye_y,
                    cuts.len(),
                    js_sys::Date::now() - t0,
                    b0.host.occluder().chunk_count(),
                    b0.host.occluder().expanded_count(),
                    b0.host.occluder().blas_count(),
                    b0.host.occluder().memory_bytes() as f64 / 1_048_576.0
                ));
            }
        });

        // The ray: re-probed whenever A or B moves.
        Effect::new({
            let engine = engine.clone();
            let bench = bench.clone();
            let s = s.clone();
            move |_| {
                let (Some(pa), Some(pb)) = (a.get(), b.get()) else {
                    return;
                };
                if !engine_ready.get() || !loaded.get() {
                    return;
                }
                if let Some(b0) = bench.borrow().as_ref() {
                    probe(&engine, b0, pa, pb, &s);
                }
            }
        });

        // Pointer: drag pans, a click (no movement) places A then B; wheel zooms.
        Effect::new({
            let engine = engine.clone();
            let bench = bench.clone();
            let s = s.clone();
            move |_| {
                let Some(canvas) = s.canvas_ref.get() else {
                    return;
                };
                let canvas: web_sys::HtmlCanvasElement = canvas;
                let dragging = Rc::new(std::cell::Cell::new(false));
                let moved = Rc::new(std::cell::Cell::new(0.0f64));
                let last = Rc::new(std::cell::Cell::new((0.0f64, 0.0f64)));
                let down = {
                    let dragging = dragging.clone();
                    let moved = moved.clone();
                    let last = last.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        if ev.button() != 0 {
                            return;
                        }
                        dragging.set(true);
                        moved.set(0.0);
                        last.set((f64::from(ev.client_x()), f64::from(ev.client_y())));
                    }) as Box<dyn FnMut(_)>)
                };
                canvas.set_onpointerdown(Some(down.as_ref().unchecked_ref()));
                down.forget();
                let wheel = {
                    let engine = engine.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::WheelEvent| {
                        ev.prevent_default();
                        if let Ok(mut guard) = engine.try_borrow_mut() {
                            if let Some(e) = guard.as_mut() {
                                e.zoom_at(
                                    -ev.delta_y() * 0.0015,
                                    ev.offset_x().into(),
                                    ev.offset_y().into(),
                                );
                                sync_cam(e);
                            }
                        }
                    }) as Box<dyn FnMut(_)>)
                };
                canvas.set_onwheel(Some(wheel.as_ref().unchecked_ref()));
                wheel.forget();
                let Some(win) = web_sys::window() else { return };
                let mv = {
                    let engine = engine.clone();
                    let dragging = dragging.clone();
                    let moved = moved.clone();
                    let last = last.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        if !dragging.get() {
                            return;
                        }
                        let (px, py) = (f64::from(ev.client_x()), f64::from(ev.client_y()));
                        let (lx, ly) = last.get();
                        let (dx, dy) = (px - lx, py - ly);
                        last.set((px, py));
                        moved.set(moved.get() + dx.abs() + dy.abs());
                        if let Ok(mut guard) = engine.try_borrow_mut() {
                            if let Some(e) = guard.as_mut() {
                                e.pan(dx, dy);
                                sync_cam(e);
                            }
                        }
                    }) as Box<dyn FnMut(_)>)
                };
                let _ = win
                    .add_event_listener_with_callback("pointermove", mv.as_ref().unchecked_ref());
                mv.forget();
                let up = {
                    let dragging = dragging.clone();
                    let moved = moved.clone();
                    let bench = bench.clone();
                    Closure::wrap(Box::new(move |ev: web_sys::PointerEvent| {
                        if !dragging.get() {
                            return;
                        }
                        dragging.set(false);
                        if moved.get() > 3.0 {
                            return;
                        }
                        let (tx, ty, zoom) = cam.get_untracked();
                        let w = screen_to_world(
                            [f64::from(ev.client_x()), f64::from(ev.client_y())],
                            tx,
                            ty,
                            zoom,
                            css.get_untracked(),
                        );
                        let eye_y = bench
                            .borrow()
                            .as_ref()
                            .map_or(0.0, |b0| b0.ground_y + b0.eye_m);
                        let p = [w[0], eye_y, w[1]];
                        if next_is_a.get_untracked() {
                            a.set(Some(p));
                        } else {
                            b.set(Some(p));
                        }
                        next_is_a.update(|v| *v = !*v);
                    }) as Box<dyn FnMut(_)>)
                };
                let _ =
                    win.add_event_listener_with_callback("pointerup", up.as_ref().unchecked_ref());
                up.forget();
            }
        });
    }
}
