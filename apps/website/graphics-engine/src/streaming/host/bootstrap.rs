//! Role: bootstrap.
//! Position: `streaming/host` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use super::*;

/// Canonical world init files value.
pub(super) const WORLD_INIT_FILES: u64 = 7;

/// Canonical world label files value.
pub(crate) const WORLD_LABEL_FILES: u64 = 2;

/// Mount-time bootstrap: hillshade + sat + DEM vectors + world + forest, then first settle.
pub async fn bootstrap(
    engine: EngineHandle,
    terrain: String,
    host: HostHandle,
    dem_out: DemGridHandle,
    report: ProgressFn,
    preferences: crate::streaming::bridge::host_preferences::HostPreferences,
) {
    use crate::streaming::bridge::progress::BootEvent;
    use crate::streaming::bridge::progress::BootSeg;

    report(BootEvent::Files(
        BootSeg::World,
        WORLD_INIT_FILES
            + WORLD_LABEL_FILES
            + crate::environment::vegetation::loader::planned_density_bins(),
    ));
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
        crate::streaming::memory::budget::hold(crate::streaming::memory::budget::Asset::Dem, bytes);
        crate::streaming::memory::budget::hold(
            crate::streaming::memory::budget::Asset::Hillshade,
            bytes,
        );
    }

    let dem_fut = async {
        let r = async {
            load_dem_and_hillshade(&engine, &base, manifest.as_ref()?, report.as_ref()).await
        }
        .await;
        report(BootEvent::Finish(BootSeg::Terrain));
        r
    };
    let sat_fut = async {
        let out = async {
            let (url, tw, th) = sat_url_from(manifest.as_ref()?, &base)?;
            crate::terrain::satellite::quadtree::load_satellite(
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
    if let Some((meters, w, h, hs_w, hs_h)) = dem_res {
        {
            let mut b = bridge.borrow_mut();
            b.hillshade_w = hs_w;
            b.hillshade_h = hs_h;
        }
        publish(&bridge);
        mh.dem.ensure_grid(&meters, w, h);
        *dem_out.borrow_mut() = mh.dem.grid();
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        mh.dem.sync(&engine, zoom);
        dem_kept = Some((meters, w, h));
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

    let mark = crate::streaming::memory::budget::heap_mark();
    let _ = mh.world.init(&terrain, report.as_ref()).await;
    crate::streaming::memory::budget::observe_since(
        crate::streaming::memory::budget::Asset::World,
        mark,
    );
    let mark = crate::streaming::memory::budget::heap_mark();
    mh.forest.init(&terrain);
    crate::streaming::memory::budget::observe_since(
        crate::streaming::memory::budget::Asset::Forest,
        mark,
    );

    if let Some(m) = manifest.as_ref() {
        let mark = crate::streaming::memory::budget::heap_mark();
        mh.water
            .init(&base, m.water.as_ref(), m.world_bounds, report.as_ref())
            .await;
        crate::streaming::memory::budget::observe_since(
            crate::streaming::memory::budget::Asset::Water,
            mark,
        );
    }

    if let Some(grid) = mh.dem.grid() {
        let show = (preferences.world_layers)().airfield;
        mh.world.upload_airfield_apron(&engine, &grid, show);
    }

    if let Some((meters, w, h)) = dem_kept {
        let roads = mh.world.road_segments_clone();
        let mark = crate::streaming::memory::budget::heap_mark();
        mh.labels
            .init(&base, &meters, w, h, roads, report.as_ref())
            .await;
        crate::streaming::memory::budget::observe_since(
            crate::streaming::memory::budget::Asset::Labels,
            mark,
        );
        let zoom = engine.borrow().as_ref().map(|e| e.zoom()).unwrap_or(-2.0);
        let prefs = (preferences.world_layers)();
        mh.labels.push(&engine, zoom, &prefs);
    } else {
        report(BootEvent::Done(BootSeg::World, WORLD_LABEL_FILES));
    }

    for _ in 0..12 {
        let mark = crate::streaming::memory::budget::heap_mark();
        let w = mh
            .world
            .run_viewport(&engine, &bridge, report.as_ref())
            .await;

        crate::streaming::memory::budget::observe_since(
            crate::streaming::memory::budget::Asset::World,
            mark,
        );
        let mark = crate::streaming::memory::budget::heap_mark();
        let f = mh
            .forest
            .run_viewport(&engine, &bridge, report.as_ref())
            .await;
        crate::streaming::memory::budget::observe_since(
            crate::streaming::memory::budget::Asset::Forest,
            mark,
        );
        if !w && !f {
            break;
        }
    }
    crate::streaming::memory::budget::publish();

    if let Some(e) = engine.borrow().as_ref() {
        publish_engine(&bridge, e);
    } else {
        publish(&bridge);
    }

    report(BootEvent::Finish(BootSeg::World));

    *host.borrow_mut() = Some(mh);
}
