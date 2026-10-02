//! **Role:** the mount-time terrain boot: manifest, elevation model and hillshade, satellite
//! imagery, basemap tiles and grid, then (in the full scope) world objects, forest, water and
//! labels, and the first viewport settle.
//! **Position:** `streaming/host`. The embedding frontend calls [`bootstrap`] once per mount with
//! its [`crate::streaming::bridge::host_preferences::HostPreferences`]; the finished
//! [`MapHost`] lands in the caller's [`HostHandle`] for camera-settle refreshes.
//! **Signals & state:** fills the caller's vector-grid handle and, when the scope keeps it, the
//! full-resolution elevation handle; reports boot progress through the caller's callback.
//! **Invariants:** the scope decides what is fetched, never what is drawn afterwards;
//! `TerrainAndImagery` issues no world-object, forest, water or label request; the host is
//! published only after the last load finished.

use super::*;
use crate::streaming::bridge::host_preferences::HostPreferences;
use crate::streaming::bridge::progress::{BootEvent, BootSeg};
use crate::streaming::memory::budget::{self, Asset};
use crate::world::terrain::dem::full_resolution::FullResolutionDemHandle;
use browser_platform::fetch::fetch_bytes;

/// Canonical world init files value.
pub(super) const WORLD_INIT_FILES: u64 = 7;

/// Canonical world label files value.
pub(crate) const WORLD_LABEL_FILES: u64 = 2;

/// Viewport passes the first world and forest settle may take.
const FIRST_SETTLE_PASSES: usize = 12;

/// Mount-time boot of the layers `preferences.scope` names, then the first settle.
///
/// `dem_out` receives the box-averaged vector grid in every scope; `full_dem_out` receives the
/// full-resolution raster only in
/// [`crate::streaming::bridge::host_preferences::BootstrapScope::TerrainAndImagery`].
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
    full_dem_out: FullResolutionDemHandle,
    report: ProgressFn,
    preferences: HostPreferences,
) {
    let scope = preferences.scope;
    if scope.loads_world_content() {
        report(BootEvent::Files(
            BootSeg::World,
            WORLD_INIT_FILES
                + WORLD_LABEL_FILES
                + crate::world::environment::vegetation::loader::planned_density_bins(),
        ));
    }
    let mut mh = MapHost::new(preferences);
    mh.terrain = terrain.clone();
    let bridge = mh.bridge.clone();
    let base = format!("/map-assets/{terrain}");

    let manifest: Option<ManifestDem> = match fetch_bytes(&format!("{base}/manifest.json")).await {
        Some(bytes) => serde_json::from_slice(&bytes).ok(),
        None => None,
    };

    let terrain_forecast = manifest
        .as_ref()
        .and_then(|m| Some(u64::from(m.dem.width_px?) * u64::from(m.dem.height_px?)))
        .map(|px| px * 4);
    if let Some(bytes) = terrain_forecast {
        budget::hold(Asset::Dem, bytes);
        budget::hold(Asset::Hillshade, bytes);
    }

    let keep_full = scope.keeps_full_resolution_dem();
    let dem_fut = async {
        let r = async {
            load_dem_and_hillshade(
                &engine,
                &base,
                manifest.as_ref()?,
                report.as_ref(),
                keep_full,
            )
            .await
        }
        .await;
        report(BootEvent::Finish(BootSeg::Terrain));
        r
    };
    let sat_fut = async {
        let out = async {
            let (url, tw, th) = sat_url_from(manifest.as_ref()?, &base)?;
            crate::world::terrain::satellite::quadtree::load_satellite(
                engine.clone(),
                &base,
                &url,
                tw,
                th,
                bridge.clone(),
                report.as_ref(),
            )
            .await;
            Some(())
        }
        .await;

        report(BootEvent::Finish(BootSeg::Satellite));
        out
    };
    let (dem_res, _sat) = futures::join!(dem_fut, sat_fut);

    let mut dem_kept: Option<(Vec<f32>, u32, u32)> = None;
    if let Some(loaded) = dem_res {
        {
            let mut b = bridge.borrow_mut();
            b.hillshade_w = loaded.hillshade_w;
            b.hillshade_h = loaded.hillshade_h;
        }
        publish(&bridge);
        mh.dem
            .ensure_grid(&loaded.meters, loaded.width, loaded.height);
        *dem_out.borrow_mut() = mh.dem.grid();
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        mh.dem.sync(&engine, zoom);
        if let Some(full) = loaded.full_resolution {
            budget::set_held(Asset::Dem, full.resident_bytes());
            *full_dem_out.borrow_mut() = Some(Rc::new(full));
        }
        dem_kept = Some((loaded.meters, loaded.width, loaded.height));
    }

    if let Some(e) = engine.borrow_mut().as_mut() {
        e.set_grid(TERRAIN_M, TERRAIN_M, true, true);
    }

    {
        let env = (preferences.render)();
        if let Some(e) = engine.borrow_mut().as_mut() {
            #[allow(clippy::cast_possible_truncation)]
            e.set_lane_opacity(1, env.hillshade_opacity as f32, env.show_hillshade);
            e.set_grid(TERRAIN_M, TERRAIN_M, true, env.show_grid);
        }
        let view = (preferences.basemap)();
        if view == "map" {
            mh.set_basemap_view(&engine, &view).await;
        }
    }

    if scope.loads_world_content() {
        let mark = budget::heap_mark();
        let _ = mh.world.init(&terrain, report.as_ref()).await;
        budget::observe_since(Asset::World, mark);
        let mark = budget::heap_mark();
        mh.forest.init(&terrain);
        budget::observe_since(Asset::Forest, mark);

        if let Some(m) = manifest.as_ref() {
            let mark = budget::heap_mark();
            mh.water
                .init(&base, m.water.as_ref(), m.world_bounds, report.as_ref())
                .await;
            budget::observe_since(Asset::Water, mark);
        }

        if let Some(grid) = mh.dem.grid() {
            let show = (preferences.world_layers)().airfield;
            mh.world.upload_airfield_apron(&engine, &grid, show);
        }

        if let Some((meters, w, h)) = dem_kept {
            let roads = mh.world.road_segments_clone();
            let mark = budget::heap_mark();
            mh.labels
                .init(&base, &meters, w, h, roads, report.as_ref())
                .await;
            budget::observe_since(Asset::Labels, mark);
            let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
            let prefs = (preferences.world_layers)();
            mh.labels.push(&engine, zoom, &prefs);
        } else {
            report(BootEvent::Done(BootSeg::World, WORLD_LABEL_FILES));
        }

        first_settle(&mut mh, &engine, &bridge, &report).await;
    }
    budget::publish();

    if let Some(e) = engine.borrow().as_ref() {
        publish_engine(&bridge, e);
    } else {
        publish(&bridge);
    }

    report(BootEvent::Finish(BootSeg::World));

    *host.borrow_mut() = Some(mh);
}

/// Up to [`FIRST_SETTLE_PASSES`] world and forest viewport passes, stopping at the first pass
/// that loads nothing new.
async fn first_settle(
    mh: &mut MapHost,
    engine: &EngineHandle,
    bridge: &BridgeHandle,
    report: &ProgressFn,
) {
    for _ in 0..FIRST_SETTLE_PASSES {
        let mark = budget::heap_mark();
        let w = mh.world.run_viewport(engine, bridge, report.as_ref()).await;
        budget::observe_since(Asset::World, mark);
        let mark = budget::heap_mark();
        let f = mh
            .forest
            .run_viewport(engine, bridge, report.as_ref())
            .await;
        budget::observe_since(Asset::Forest, mark);
        if !w && !f {
            break;
        }
    }
}
