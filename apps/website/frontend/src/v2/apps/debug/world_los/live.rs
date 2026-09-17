//! Browser host for the world line-of-sight diagnostics bench.

use super::super::building_interior::InteriorLanes;
use super::super::building_viewer::geom::screen_to_world;
use super::super::world_los_scene::{build_bench_lanes, ray_strip, Footprint};
use super::{DEFAULT_CENTER, DEFAULT_EYE_M, DEFAULT_RADIUS_M, MAX_CUT_BUILDINGS};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::frame::engine::RenderEngine;
use website_map_engine::frame::RafPump;
use website_map_engine::overlay::lanes::role_id;
use website_map_engine::spatial::los::world::coverage_1::WorldVerdict;
use website_map_engine::spatial::los::world::state::WorldOccluder;
use website_map_engine::streaming::loaders::fetch::fetch_bytes;
use website_map_engine::streaming::loaders::fetch::fetch_text;
use website_map_engine::streaming::loaders::occluder_loader::OccluderHost;
use website_map_engine::streaming::scheduler::state::WorldResidency;
use website_map_engine::world::architecture::section::cutter::section_at;

type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

/// The reactive cells the wasm host writes as the catalogue loads and the probe resolves.
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

mod mount;
pub use mount::mount;
