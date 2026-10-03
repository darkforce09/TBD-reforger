//! The engine statistics report's JSON pin: the keys, their order and every value's spelling.
//!
//! **Role:** builds a report with distinct values and compares its JSON byte for byte with the
//! `format!` string the engine wrote before the report was built on `renderer_core`'s
//! `StatsJson`, and pins the key list on its own.
//! **Position:** test-only, mounted from `crate::engine_statistics`; native, because the
//! report is plain data and only its gathering needs a GPU.
//! **Signals & state:** none.
//! **Invariants:** the frontend's debug HUD and the browser gates read the object by key, so a
//! renamed, dropped or reordered key, or a changed decimal precision, fails here.

use crate::engine_statistics::EngineStatisticsReport;

/// A report whose every numeric field is distinct, so a swapped pair of keys shows.
fn sample(gpu_frame_ms: Option<f64>, flags: bool) -> EngineStatisticsReport<'static> {
    EngineStatisticsReport {
        backend: "webgpu",
        instances: 1001,
        chunks: 2,
        gpu_bytes: 32_160,
        staging_peak_bytes: 4096,
        gen_ms: 12.345,
        upload_ms: 0.05,
        uniform_bytes_last_frame: 80,
        gpu_frame_ms,
        basemap_mode: "pyramid",
        basemap_tiles: 21,
        basemap_bytes: 67_108_864,
        world_building_instances: 3001,
        world_building_outline_vertices: 3002,
        world_chunks_drawn: 3003,
        sea_polygons: 3004,
        landcover_polygons: 3005,
        contour_segments: 3006,
        road_segments: 3007,
        forest_polygons: 625,
        forest_outline_segments: 3008,
        forest_density_w: 3009,
        forest_density_h: 3010,
        forest_bins_ok: 3011,
        forest_mode: "density",
        tree_glyphs: 3012,
        prop_glyphs: 3013,
        badge_glyphs: 3014,
        text_labels_drawn: 3015,
        atlas_bytes: 3016,
        slot_instances: 3017,
        slot_drag_instances: 3018,
        cluster_instances: 3019,
        mission_vehicles: 3020,
        submitted_last_frame: flags,
        compute_cull: !flags,
        compute_cull_cpu_count: 3021,
        compute_cull_gpu_count: 3022,
        compute_cull_gpu_sampled: flags,
        icon_lane_uploads: 3023,
        polygon_lane_uploads: 3024,
        strip_lane_uploads: 3025,
        building_uploads: 3026,
        text_label_uploads: 3027,
        render_cpu_ms_last: 1.234_56,
        render_cpu_ms_ema: 16.666_666,
    }
}

/// The engine's statistics object as its `format!` string wrote it, kept verbatim as the oracle.
fn format_string_oracle(r: &EngineStatisticsReport<'_>) -> String {
    let gpu_frame_ms = match r.gpu_frame_ms {
        Some(ms) => format!("{ms:.3}"),
        None => "null".to_owned(),
    };
    format!(
        concat!(
            "{{\"backend\":\"{}\",\"instances\":{},\"chunks\":{},\"gpu_bytes\":{},",
            "\"staging_peak_bytes\":{},\"gen_ms\":{:.1},\"upload_ms\":{:.1},",
            "\"uniform_bytes_last_frame\":{},\"gpu_frame_ms\":{},",
            "\"basemap_mode\":\"{}\",\"basemap_tiles\":{},\"basemap_bytes\":{},",
            "\"world_building_instances\":{},\"world_building_outline_vertices\":{},",
            "\"world_chunks_drawn\":{},",
            "\"sea_polygons\":{},\"landcover_polygons\":{},\"contour_segments\":{},",
            "\"road_segments\":{},\"forest_polygons\":{},\"forest_outline_segments\":{},",
            "\"forest_density_w\":{},\"forest_density_h\":{},\"forest_bins_ok\":{},\"forest_mode\":\"{}\",",
            "\"tree_glyphs\":{},\"prop_glyphs\":{},\"badge_glyphs\":{},\"text_labels_drawn\":{},\"atlas_bytes\":{},",
            "\"slot_instances\":{},\"slot_drag_instances\":{},\"cluster_instances\":{},",
            "\"mission_vehicles\":{},",
            "\"submitted_last_frame\":{},",
            "\"compute_cull\":{},\"compute_cull_cpu_count\":{},\"compute_cull_gpu_count\":{},",
            "\"compute_cull_gpu_sampled\":{},",
            "\"icon_lane_uploads\":{},\"polygon_lane_uploads\":{},\"strip_lane_uploads\":{},",
            "\"building_uploads\":{},\"text_label_uploads\":{},",
            "\"render_cpu_ms_last\":{:.4},\"render_cpu_ms_ema\":{:.4}}}"
        ),
        r.backend,
        r.instances,
        r.chunks,
        r.gpu_bytes,
        r.staging_peak_bytes,
        r.gen_ms,
        r.upload_ms,
        r.uniform_bytes_last_frame,
        gpu_frame_ms,
        r.basemap_mode,
        r.basemap_tiles,
        r.basemap_bytes,
        r.world_building_instances,
        r.world_building_outline_vertices,
        r.world_chunks_drawn,
        r.sea_polygons,
        r.landcover_polygons,
        r.contour_segments,
        r.road_segments,
        r.forest_polygons,
        r.forest_outline_segments,
        r.forest_density_w,
        r.forest_density_h,
        r.forest_bins_ok,
        r.forest_mode,
        r.tree_glyphs,
        r.prop_glyphs,
        r.badge_glyphs,
        r.text_labels_drawn,
        r.atlas_bytes,
        r.slot_instances,
        r.slot_drag_instances,
        r.cluster_instances,
        r.mission_vehicles,
        r.submitted_last_frame,
        r.compute_cull,
        r.compute_cull_cpu_count,
        r.compute_cull_gpu_count,
        r.compute_cull_gpu_sampled,
        r.icon_lane_uploads,
        r.polygon_lane_uploads,
        r.strip_lane_uploads,
        r.building_uploads,
        r.text_label_uploads,
        r.render_cpu_ms_last,
        r.render_cpu_ms_ema,
    )
}

#[test]
fn the_report_is_byte_identical_to_the_format_string_it_replaces() {
    for (gpu_frame_ms, flags) in [(Some(4.567_89), true), (None, false), (Some(0.0), false)] {
        let report = sample(gpu_frame_ms, flags);
        assert_eq!(report.to_json(), format_string_oracle(&report));
    }
}

#[test]
fn the_report_spells_its_values_as_the_hud_reads_them() {
    let json = sample(None, true).to_json();
    for field in [
        "{\"backend\":\"webgpu\",",
        "\"gen_ms\":12.3,",
        "\"upload_ms\":0.1,",
        "\"gpu_frame_ms\":null,",
        "\"basemap_mode\":\"pyramid\",",
        "\"forest_mode\":\"density\",",
        "\"submitted_last_frame\":true,",
        "\"compute_cull\":false,",
        "\"render_cpu_ms_last\":1.2346,",
        "\"render_cpu_ms_ema\":16.6667}",
    ] {
        assert!(json.contains(field), "missing `{field}` in {json}");
    }
    let timed = sample(Some(4.567_89), false).to_json();
    assert!(timed.contains("\"gpu_frame_ms\":4.568,"), "{timed}");
}

#[test]
fn the_report_keys_and_their_order_are_pinned() {
    let json = sample(Some(1.0), true).to_json();
    let keys: Vec<&str> = json
        .split('"')
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|w| w[1].starts_with(':'))
        .map(|w| w[0])
        .collect();
    assert_eq!(
        keys,
        [
            "backend",
            "instances",
            "chunks",
            "gpu_bytes",
            "staging_peak_bytes",
            "gen_ms",
            "upload_ms",
            "uniform_bytes_last_frame",
            "gpu_frame_ms",
            "basemap_mode",
            "basemap_tiles",
            "basemap_bytes",
            "world_building_instances",
            "world_building_outline_vertices",
            "world_chunks_drawn",
            "sea_polygons",
            "landcover_polygons",
            "contour_segments",
            "road_segments",
            "forest_polygons",
            "forest_outline_segments",
            "forest_density_w",
            "forest_density_h",
            "forest_bins_ok",
            "forest_mode",
            "tree_glyphs",
            "prop_glyphs",
            "badge_glyphs",
            "text_labels_drawn",
            "atlas_bytes",
            "slot_instances",
            "slot_drag_instances",
            "cluster_instances",
            "mission_vehicles",
            "submitted_last_frame",
            "compute_cull",
            "compute_cull_cpu_count",
            "compute_cull_gpu_count",
            "compute_cull_gpu_sampled",
            "icon_lane_uploads",
            "polygon_lane_uploads",
            "strip_lane_uploads",
            "building_uploads",
            "text_label_uploads",
            "render_cpu_ms_last",
            "render_cpu_ms_ema",
        ]
    );
}
