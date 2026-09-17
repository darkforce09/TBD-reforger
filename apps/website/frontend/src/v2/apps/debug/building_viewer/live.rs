//! Browser render host, lane uploads, and compound asset loading.

use super::super::building_interior::{self, InteriorLanes, LevelCuts};
use super::{geom, Cam, Drag, RayEnd, ViewFloor, DEFAULT_PREFAB_PATH};
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use website_map_engine::frame::engine::RenderEngine;
use website_map_engine::frame::RafPump;
use website_map_engine::overlay::lanes::role_id;
use website_map_engine::spatial::bvh::sidecar::BvhSidecar;
use website_map_engine::spatial::los::interior::wash::LevelWash;
use website_map_engine::world::architecture::blueprint::attribution_1::LosResult;
use website_map_engine::world::architecture::blueprint::structure::BuildingBlueprint;
use website_map_engine::world::architecture::compound::assembly::CompoundBuilding;
use website_map_engine::world::architecture::compound::instances::InstancesFile;
use website_map_engine::world::architecture::section::cutter::BuildingDrawing;

type EngineHandle = Rc<RefCell<Option<RenderEngine>>>;

fn sync_cam(e: &RenderEngine, cam: RwSignal<Cam>) {
    cam.set(Cam {
        tx: e.target_x(),
        ty: e.target_y(),
        zoom: e.zoom(),
    });
}

/// Every static lane of the bench on its OWN role (`InteriorLanes::ROLES`);
/// the probe (`INTERIOR_PROBE`) is `upload_ray`'s. An empty payload drops its lane.
fn upload_static(
    e: &mut RenderEngine,
    bp: &BuildingBlueprint,
    drawing: Option<&BuildingDrawing>,
    compound: Option<&CompoundBuilding>,
    cuts: Option<&[LevelCuts]>,
    view: ViewFloor,
) {
    let l: InteriorLanes =
        building_interior::build_interior_lanes(bp, drawing, compound, cuts, view);
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
    e.mark_dirty();
}

fn upload_ray(
    e: &mut RenderEngine,
    obs: RayEnd,
    tgt: RayEnd,
    los: &LosResult,
    band: [f64; 2],
    band_last: bool,
) {
    let (packed, n) = building_interior::build_ray_lane(
        [obs.x, obs.y, obs.z],
        [tgt.x, tgt.y, tgt.z],
        &los.hits,
        los.is_clear,
        band,
        band_last,
    );
    e.upload_strip_tris(role_id::INTERIOR_PROBE, &packed, n, true);
    e.mark_dirty();
}

/// The viewed level's wash → the engine's single viewshed texture slot; `None` (viewshed
/// off, Roof view, no sidecar) clears the lane — a wash with nothing to stop it would be a
/// lie.
fn upload_wash(e: &mut RenderEngine, wash: Option<&LevelWash>) {
    match wash {
        Some(w) => {
            let t = geom::wash_texture(w);
            if let Err(err) = e.viewshed_upload(
                t.min_x,
                t.min_y,
                t.max_x,
                t.max_y,
                t.tex_w,
                t.tex_h,
                &t.rgba,
                t.stride_bytes,
            ) {
                let err: JsValue = err.into();
                web_sys::console::warn_2(
                    &"building-viewer: viewshed wash upload failed".into(),
                    &err,
                );
                e.viewshed_clear();
            }
        }
        None => e.viewshed_clear(),
    }
    e.mark_dirty();
}

fn prefab_path() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| {
            web_sys::UrlSearchParams::new_with_str(&s)
                .ok()
                .and_then(|p| p.get("prefab"))
        })
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| DEFAULT_PREFAB_PATH.to_string())
}

/// One query parameter, if present and non-empty.
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

/// `?a=x,y,z` / `?b=x,y,z` — the ray ends in building-local metres (a reproducible LOS state
/// for screenshots and bug reports).
fn ray_end_query(name: &str) -> Option<RayEnd> {
    let v = query(name)?;
    let mut it = v.split(',').map(|s| s.trim().parse::<f64>().ok());
    let (x, y, z) = (it.next()??, it.next()??, it.next()??);
    (x.is_finite() && y.is_finite() && z.is_finite()).then_some(RayEnd { x, y, z })
}

/// `?scene=1` — also load `<slug>.scene.json` (hand-placed exterior trees) into the compound.
fn scene_mode() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| {
            web_sys::UrlSearchParams::new_with_str(&s)
                .ok()
                .and_then(|p| p.get("scene"))
        })
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "on"))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    match gloo_net::http::Request::get(url).send().await {
        Ok(resp) if resp.ok() => resp
            .binary()
            .await
            .map_err(|e| format!("{url}: read failed — {e}")),
        Ok(resp) => Err(format!("{url}: HTTP {}", resp.status())),
        Err(e) => Err(format!("{url}: {e}")),
    }
}

/// `<slug>.instances.json` + every BLAS it references (paths relative to the
/// prefabs root, the parent of `buildings/`) assembled onto the shell; with `scene`, the
/// `<slug>.scene.json` trees too. Returns the compound and a non-fatal warning (scene file
/// missing).
async fn load_compound(
    json_path: &str,
    shell: Arc<BvhSidecar>,
    scene: bool,
) -> Result<(CompoundBuilding, Option<String>), String> {
    let stem = json_path
        .strip_suffix(".json")
        .ok_or_else(|| format!("{json_path}: not a .json path"))?;
    let root = stem
        .rsplit_once('/')
        .and_then(|(dir, _)| dir.rsplit_once('/'))
        .map_or_else(|| "/".to_string(), |(parent, _)| format!("{parent}/"));
    let url = format!("{stem}.instances.json");
    let bytes = fetch_bytes(&url).await?;
    let file: InstancesFile =
        serde_json::from_slice(&bytes).map_err(|e| format!("{url}: parse failed — {e}"))?;
    let mut records = file.instances;
    let mut warning = None;
    if scene {
        let surl = format!("{stem}.scene.json");
        match fetch_bytes(&surl).await {
            Ok(b) => match serde_json::from_slice::<InstancesFile>(&b) {
                Ok(sf) => records.extend(sf.instances),
                Err(e) => warning = Some(format!("{surl}: parse failed — {e} — scene off")),
            },
            Err(e) => warning = Some(format!("{e} — scene off")),
        }
    }
    let mut paths: Vec<String> = Vec::new();
    for r in &records {
        if !paths.contains(&r.blas) {
            paths.push(r.blas.clone());
        }
    }
    let fetched = futures::future::join_all(paths.iter().map(|p| {
        let url = format!("{root}{p}");
        async move {
            let res = fetch_bytes(&url).await;
            (url, res)
        }
    }))
    .await;
    let mut map: HashMap<String, Arc<BvhSidecar>> = HashMap::new();
    for (p, (url, res)) in paths.iter().zip(fetched) {
        let b = res?;
        let sc = BvhSidecar::parse(&b).map_err(|e| format!("{url}: BLAS parse failed — {e}"))?;
        map.insert(p.clone(), Arc::new(sc));
    }
    let c = CompoundBuilding::assemble(shell, &records, &map).map_err(|e| format!("{url}: {e}"))?;
    Ok((c, warning))
}

mod wiring;
pub use wiring::wire;
