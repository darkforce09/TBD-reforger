use super::*;

/// T-166 — full W1–W5 host wiring Class-R matrix (`?force=webgl&sat=preview`).
pub async fn smoke_fullmap(dist: &str, map_assets: &str) -> Result<u8> {
    let path = "/missions/smoke/edit?force=webgl&sat=preview";
    let h = Harness::new(dist, 5318, 9378, Some(PathBuf::from(map_assets)), None, &[]).await?;
    let run = async {
        // Track whether any Network response delivered the full sat body (A_sat_bytes).
        h.page.send("Network.enable", json!({})).await?;
        let mut net = h.page.on_event("Network.responseReceived").await;
        let sat_full_hits = Arc::new(StdMutex::new(0u32));
        let hits = Arc::clone(&sat_full_hits);
        tokio::spawn(async move {
            while let Some(e) = net.recv().await {
                let url = e["response"]["url"].as_str().unwrap_or("");
                if !url.contains(".tbd-sat") {
                    continue;
                }
                let len = e["response"]["headers"]
                    .as_object()
                    .and_then(|hdrs| {
                        hdrs.iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
                    })
                    .and_then(|(_, v)| v.as_str())
                    .and_then(|s| s.parse::<u64>().ok())
                    .or_else(|| e["response"]["encodedDataLength"].as_u64())
                    .unwrap_or(0);
                if len == SAT_FULL_BYTES {
                    *hits.lock().unwrap() += 1;
                }
            }
        });

        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 160, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            // Wait for host bootstrap + residency drain (hillshade first, then world pins).
            let bridge = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.hillshadeW > 0 && window.__mapAssets.road_segments === 887 && window.__mapAssets.landcover_polygons === 0 && window.__mapAssets.sea_polygons > 0 && window.__mapAssets.contour_segments > 0 && window.__mapAssets.world_building_instances > 0 && window.__mapAssets.world_chunks_drawn > 0 && window.__mapAssets.forest_mode === 'density' && window.__mapAssets.forest_density_w === 1601 && window.__mapAssets.forest_density_h === 1601 && window.__mapAssets.forest_bins_ok === 625 && window.__mapAssets.forest_polygons === 625 && (window.__mapAssets.atlas_bytes > 0 || window.__mapAssets.glyphAtlas === true) && window.__mapAssets.tree_glyphs === 0",
                    720,
                    250,
                )
                .await?;
            checks.insert("bridgeSettled".into(), json!(bridge));

            let a_hs = eval_bool(
                &h.page,
                "!!window.__mapAssets && window.__mapAssets.hillshadeW > 0 && window.__mapAssets.hillshadeH > 0",
            )
            .await?;
            let a_sat = eval_bool(
                &h.page,
                "!!window.__mapAssets && window.__mapAssets.satW > 0 && window.__mapAssets.satH > 0 && window.__mapAssets.satMode === 'single'",
            )
            .await?;
            let a_roads = eval_bool(&h.page, "window.__mapAssets.road_segments === 887").await?;
            // T-177 — landcover lane is empty on Everon since **T-176 A2** dropped `forest`-kind
            // regions (the 32 m wash) and Everon has no `field`/`waterBody` regions, so the composed
            // mesh is 0 polygons (`world_host::push_landcover`). Was `=== 36`; that stale assertion
            // went unnoticed because the chrome-headless-shell font crash killed the suite at
            // `selfcheck` (see cdp.rs `find_chromium`). This asserts the lane stays empty (regression
            // guard: a re-added wash → non-zero → red).
            let a_lc = eval_bool(&h.page, "window.__mapAssets.landcover_polygons === 0").await?;
            let a_sea = eval_bool(&h.page, "window.__mapAssets.sea_polygons > 0").await?;
            let a_cont = eval_bool(&h.page, "window.__mapAssets.contour_segments > 0").await?;
            let a_bld = eval_bool(
                &h.page,
                "window.__mapAssets.world_building_instances > 0 && window.__mapAssets.world_chunks_drawn > 0",
            )
            .await?;
            // T-179 — density canopy: Class-R equality pins (soft >0 banned for bins/dims).
            let a_density_dims = eval_bool(
                &h.page,
                "window.__mapAssets.forest_density_w === 1601 && window.__mapAssets.forest_density_h === 1601",
            )
            .await?;
            let a_density_mode =
                eval_bool(&h.page, "window.__mapAssets.forest_mode === 'density'").await?;
            let a_bins = eval_bool(&h.page, "window.__mapAssets.forest_bins_ok === 625").await?;
            let a_forest = eval_bool(&h.page, "window.__mapAssets.forest_polygons === 625").await?;
            let a_outline_boot =
                eval_bool(&h.page, "window.__mapAssets.forest_outline_segments === 0").await?;
            let a_atlas = eval_bool(
                &h.page,
                "window.__mapAssets.atlas_bytes > 0 || window.__mapAssets.glyphAtlas === true",
            )
            .await?;
            let a_trees_off = eval_bool(&h.page, "window.__mapAssets.tree_glyphs === 0").await?;

            checks.insert("A_hs".into(), json!(a_hs));
            checks.insert("A_sat".into(), json!(a_sat));
            checks.insert("A_roads".into(), json!(a_roads));
            checks.insert("A_lc".into(), json!(a_lc));
            checks.insert("A_sea".into(), json!(a_sea));
            checks.insert("A_cont".into(), json!(a_cont));
            checks.insert("A_bld".into(), json!(a_bld));
            checks.insert("A_density_dims".into(), json!(a_density_dims));
            checks.insert("A_density_mode".into(), json!(a_density_mode));
            checks.insert("A_bins".into(), json!(a_bins));
            checks.insert("A_forest".into(), json!(a_forest));
            checks.insert("A_outline_boot".into(), json!(a_outline_boot));
            checks.insert("A_atlas".into(), json!(a_atlas));
            checks.insert("A_trees_off".into(), json!(a_trees_off));

            // T-179 — real MS outline hairlines armed at z=-1 (not fake segments===1).
            let outline_set = eval_bool(
                &h.page,
                "typeof window.__editorCamSet === 'function' && (window.__editorCamSet(6400, 6400, -1.0), true)",
            )
            .await?;
            checks.insert("A_outline_probe_set".into(), json!(outline_set));
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let a_outline_probe = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.forest_outline_segments > 0 && window.__mapAssets.forest_density_w === 1601",
                    80,
                    250,
                )
                .await?;
            let outline_segs_at_probe =
                eval_i64(&h.page, "window.__mapAssets.forest_outline_segments || 0")
                    .await
                    .unwrap_or(0);
            // T-179 floor from this checkout fullmap: 99374 segments @ z=-1 (MS hairlines).
            // Soft `> 0` alone can false-green a stub flag; require a real polyline count.
            const OUTLINE_SEGS_FLOOR: i64 = 50_000;
            let a_outline_probe = a_outline_probe && outline_segs_at_probe >= OUTLINE_SEGS_FLOOR;
            checks.insert("A_outline_probe".into(), json!(a_outline_probe));
            // Restore island zoom before tree probe.
            let _ = eval_bool(
                &h.page,
                "typeof window.__editorCamSet === 'function' && (window.__editorCamSet(6400, 6400, -2.0), true)",
            )
            .await?;
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;

            // Zoom probe ≥ 0 → tree glyphs on. Use z=2 (not 0): at island-center z=0 the
            // exact-count heatmap rung (INSTANCE_BUDGET) clears tree glyphs by design — smoke would
            // false-red on a correct LOD ladder. z=2 shrinks the draw-set under budget.
            let set_ok = eval_bool(
                &h.page,
                "typeof window.__editorCamSet === 'function' && (window.__editorCamSet(6400, 6400, 2), true)",
            )
            .await?;
            checks.insert("A_trees_probe".into(), json!(set_ok));
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            let a_trees_on = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.tree_glyphs > 0",
                    160,
                    250,
                )
                .await?;
            checks.insert("A_trees_on".into(), json!(a_trees_on));
            let sat_hits = *sat_full_hits.lock().unwrap();
            checks.insert("A_sat_bytes".into(), json!(sat_hits == 0));
            checks.insert("A_panic".into(), json!(h.no_panics()));

            let pass = ready
                && h.no_panics()
                && checks.values().all(|v| *v == json!(true))
                && checks.len() >= 16;
            print_verdict(&json!({
                "gate": "editor-fullmap-smoke",
                "path": path,
                "pins": {
                    "sat_full_bytes": SAT_FULL_BYTES,
                    "roads": 887,
                    "landcover": 0,
                    "forest_density": 1601,
                    "forest_bins": 625,
                    "forest_outline_segments_at_z_neg1": outline_segs_at_probe,
                    "default_zoom": -2.0,
                    "tree_glyph_min_zoom": 0.0,
                },
                "checks": checks,
                "panics": h.panics_head(),
                "pass": pass,
            }));
            return Ok::<u8, anyhow::Error>(to_code(pass));
        }

        let pass = false;
        print_verdict(&json!({
            "gate": "editor-fullmap-smoke",
            "path": path,
            "pins": {
                "sat_full_bytes": SAT_FULL_BYTES,
                "roads": 887,
                "landcover": 36,
                "default_zoom": -2.0,
                "tree_glyph_min_zoom": 0.0,
            },
            "checks": checks,
            "panics": h.panics_head(),
            "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_hillshade_editor.mjs — T-159.28: DEM fetched + Rust-decoded + hillshade uploaded.
pub async fn smoke_hillshade(dist: &str, map_assets: &str) -> Result<u8> {
    let path = "/missions/smoke/edit?force=webgl&sat=preview";
    let h = Harness::new(dist, 5317, 9377, Some(PathBuf::from(map_assets)), None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 160, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            let uploaded = h.page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.hillshadeH > 0 && window.__mapAssets.hillshadeW > 0",
                    200,
                    250,
                )
                .await?;
            checks.insert("hillshadeUploaded".into(), json!(uploaded));
            if uploaded {
                let dims = eval_str(&h.page, "JSON.stringify([window.__mapAssets.hillshadeW, window.__mapAssets.hillshadeH])").await?;
                let dims: Vec<i64> = serde_json::from_str(&dims).unwrap_or_default();
                checks.insert("dimsPositive".into(), json!(dims.iter().all(|d| *d > 0)));
            }
            checks.insert(
                "laneDrawn".into(),
                json!(checks["hillshadeUploaded"] == json!(true)),
            );
        }
        let pass = ready
            && h.no_panics()
            && checks.values().all(|v| *v == json!(true))
            && checks.len() >= 2;
        print_verdict(&json!({
            "gate": "editor-hillshade-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_doc_editor.mjs — T-159.16: hosted MissionDocCore live + seeded + round-trips.
pub async fn smoke_doc(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5302, 9362, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(DOC_READY, 120, 250).await?;

        let (mut slot_count, mut roundtrip_ok, mut encode_stable) = (-1i64, false, false);
        let (mut hex_len, mut hex_head) = (0usize, String::new());
        if ready {
            slot_count = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            roundtrip_ok = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
            let h1 = eval_str(&h.page, "window.__missionDoc.encode_hex()").await?;
            let h2 = eval_str(&h.page, "window.__missionDoc.encode_hex()").await?;
            encode_stable = !h1.is_empty() && h1 == h2;
            hex_len = h1.len();
            hex_head = h1.chars().take(48).collect();
        } else {
            eprintln!("smoke_doc_editor: window.__missionDoc never appeared");
        }
        // T-172 B4 — the slot glyph lane must be live: atlas uploaded at mount and the seeded
        // SoA bound (the pre-T-172 editor never called ensure_slot_atlas → invisible slots).
        let slot_stats: Value = {
            let engine_up = h
                .page
                .wait_for("typeof window.__wgpuSlotStats === 'function'", 120, 250)
                .await?;
            if engine_up {
                let raw = eval_str(&h.page, "window.__wgpuSlotStats()").await?;
                serde_json::from_str(&raw).unwrap_or(Value::Null)
            } else {
                Value::Null
            }
        };
        let atlas_ready = slot_stats["atlas_ready"].as_bool() == Some(true);
        let lane_bound = slot_stats["slot_len"].as_i64() == Some(SEED_N);
        let seeded = slot_count == SEED_N;
        let pass = ready
            && h.no_panics()
            && seeded
            && roundtrip_ok
            && encode_stable
            && atlas_ready
            && lane_bound;
        print_verdict(&json!({
            "gate": "editor-doc-smoke", "path": path,
            "slotCount": slot_count, "seeded": seeded, "roundtripOk": roundtrip_ok,
            "encodeStable": encode_stable, "encodeHexLen": hex_len, "encodeHexHead": hex_head,
            "slotAtlasReady": atlas_ready, "slotLaneBound": lane_bound,
            "slotStats": slot_stats,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
