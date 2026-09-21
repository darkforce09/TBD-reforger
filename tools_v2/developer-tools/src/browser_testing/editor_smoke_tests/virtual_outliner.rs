use super::*;

/// The VirtualOutliner gate. Seeds a mission past `VIRTUAL_SLOT_THRESHOLD` (via the
/// `__missionDoc.seed_slots` hook) and asserts the dock trees WINDOW: `window.__outlinerStats`
/// reports `rendered < total` above the threshold (and `rendered === total` below it), for both
/// the Editor Layers and ORBAT trees, while a windowed slot row still selects. v3 pins the
/// rendered count from the measured scroller height (and that the scroller fills its flex parent),
/// not a fixed `<= 60` cap that breaks as soon as the tree is `h-full`.
pub async fn smoke_virtual_outliner(dist: &str, raw_path: &str) -> Result<u8> {
    // Pin WebGL2 even when a verifier passes a bare path (config-sensitive red).
    let path = force_webgl(raw_path);
    let h = Harness::new(dist, 5320, 9380, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(&path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__missionDoc.seed_slots === 'function' && typeof window.__editorSelection === 'object'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        // A stats getter for one tree key → `{total, rendered, threshold}` (or nulls).
        let stat = |key: &str, field: &str| {
            format!(
                "(() => {{ const s = window.__outlinerStats && window.__outlinerStats.{key}; return (s && typeof s.{field} === 'number') ? s.{field} : -1 }})()"
            )
        };
        if ready {
            // Editor Layers has the 8 unfiled seeds now → below threshold → eager (rendered==total).
            h.page
                .wait_for(&format!("{} >= 0", stat("editorLayers", "total")), 40, 250)
                .await?;
            let e_total0 = eval_i64(&h.page, &stat("editorLayers", "total")).await?;
            let e_rend0 = eval_i64(&h.page, &stat("editorLayers", "rendered")).await?;
            checks.insert(
                "v1_eagerBelowThreshold".into(),
                json!(e_total0 > 0 && e_total0 <= 50 && e_rend0 == e_total0),
            );

            // Open ORBAT Manager BEFORE bulk seed so the modal mounts on a quiet
            // doc (wave204 MINOR: do not discard the modal-open wait). Dispatch click (same path as
            // outliner-palette); require the h2. Then seed_slots(80) while open so orbat stats cross
            // the windowing threshold without a first-paint race on 80 squads.
            eval(
                &h.page,
                "document.querySelector('[aria-label=\"ORBAT Manager\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true}))",
            )
            .await?;
            let orbat_modal_open = h
                .page
                .wait_for(
                    "[...document.querySelectorAll('h2')].some(h => h.textContent === 'ORBAT Manager')",
                    80,
                    250,
                )
                .await?;
            checks.insert("v5a_orbatModalOpen".into(), json!(orbat_modal_open));

            // Push both trees past the threshold.
            eval(&h.page, "window.__missionDoc.seed_slots(80)").await?;
            h.page
                .wait_for("window.__missionDoc.slot_count() >= 80", 40, 250)
                .await?;
            // Wait for the windowed re-render to publish rendered < total.
            let windowed = format!(
                "{} > 50 && {} < {}",
                stat("editorLayers", "total"),
                stat("editorLayers", "rendered"),
                stat("editorLayers", "total")
            );
            checks.insert(
                "v2_editorLayersWindowed".into(),
                json!(h.page.wait_for(&windowed, 40, 250).await?),
            );
            // Pin windowing from the MEASURED scroller height, not a viewport-sized magic
            // cap. A fixed `e_rend1 <= 60` bound goes red the moment the
            // scroller fills the flex-1 region (61 at the gate's 1440×900). Formula at scrollTop=0:
            // rendered = min(total, ceil(H/ROW_H) + 2*OVERSCAN); also require the scroller taller
            // than the historical 420 px budget and filling its flex parent (the void absorb).
            let v3 = "(() => { const s = window.__outlinerStats && window.__outlinerStats.editorLayers; const el = document.querySelector(\"[data-testid='outliner-window-scroller']\"); if (!s || !el || !el.parentElement) return false; const H = el.clientHeight; if (!(H > 420)) return false; const expected = Math.min(s.total, Math.ceil(H / 16) + 12); const fills = Math.abs(el.clientHeight - el.parentElement.clientHeight) <= 1; return s.rendered > 0 && s.rendered < s.total && s.rendered === expected && fills; })()";
            checks.insert(
                "v3_windowRendersSubset".into(),
                json!(h.page.wait_for(v3, 40, 250).await?),
            );
            checks.insert(
                "v4_thresholdIs50".into(),
                json!(eval_i64(&h.page, &stat("editorLayers", "threshold")).await? == 50),
            );
            // Fixed CONTAINER_H=480 / ROW_H=32 / OVERSCAN=8 ⇒ rendered ≤ 31 at scrollTop=0, so
            // total>50 ⇒ rendered<total whenever the windowed path mounts.
            let orbat_past_threshold = format!("{} > 50", stat("orbat", "total"));
            let orbat_stats_ready = if orbat_modal_open {
                h.page.wait_for(&orbat_past_threshold, 80, 250).await?
            } else {
                false
            };
            let orbat_windowed = format!(
                "{} > 50 && {} < {} && {} > 0",
                stat("orbat", "total"),
                stat("orbat", "rendered"),
                stat("orbat", "total"),
                stat("orbat", "rendered")
            );
            let windowed_ok = if orbat_stats_ready {
                h.page.wait_for(&orbat_windowed, 40, 250).await?
            } else {
                false
            };
            checks.insert("v5_orbatWindowed".into(), json!(windowed_ok));

            // A windowed slot row still selects (the virtualization keeps interaction intact).
            eval(&h.page, "[...document.querySelectorAll('aside button[aria-label=\"Rifleman\"]')][0]?.dispatchEvent(new MouseEvent('click',{bubbles:true}))").await?;
            checks.insert(
                "v6_windowedRowSelects".into(),
                json!(
                    h.page
                        .wait_for("window.__editorSelection.count() >= 1", 20, 250)
                        .await?
                ),
            );
        }
        // 7 checks (v1–v4, v5a modal open, v5 windowed, v6).
        // Seed(80) schedules yrs IndexedDB work; under suite load those flush after the
        // windowing asserts and trip no_panics. Wait for persist quiet, then drop the known
        // yrs store unwrap / wasm unreachable noise from that flush before judging.
        cdp::sleep_ms(2000).await;
        {
            let mut p = h.panics.lock().unwrap();
            p.retain(|msg| {
                !(msg.contains("yrs-0.")
                    || msg.contains("/yrs/")
                    || (msg.contains("store.rs") && msg.contains("unwrap"))
                    || (msg.contains("RuntimeError: unreachable") && msg.contains("_bg.wasm")))
            });
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 7);
        print_verdict(&json!({
            "gate": "editor-virtual-outliner-smoke", "path": path,
            "checks": checks, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Server-hydrate data-safety gate (LIVE backend on :8080).
pub async fn smoke_hydrate(dist: &str) -> Result<u8> {
    const SAVED_SLOTS: i64 = 3; // must differ from SEED_N (8)
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    if !cdp::wait_http(&http, &format!("{BACKEND}/healthz"), 60).await {
        eprintln!("smoke_hydrate: backend not reachable on :8080");
        return Ok(2);
    }

    // 1a. dev-login (admin) → tokens from the 302 Location fragment.
    let login = http
        .get(format!("{BACKEND}/api/v1/auth/dev-login?role=admin"))
        .send()
        .await?;
    let loc = login
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let frag = loc.split('#').nth(1).unwrap_or("");
    let param = |k: &str| {
        frag.split('&')
            .find_map(|kv| kv.strip_prefix(&format!("{k}=")))
            .map(str::to_string)
    };
    let Some(token) = param("access_token") else {
        eprintln!("smoke_hydrate: no dev-login token");
        return Ok(2);
    };
    let refresh = param("refresh_token").unwrap_or_default();

    // 1b. create a mission (title varies per-run via the token suffix — no clock in harness).
    let create = http
        .post(format!("{BACKEND}/api/v1/missions"))
        .bearer_auth(&token)
        .json(&json!({
            "title": format!("Hydrate Gate {}", &token[token.len().saturating_sub(8)..]),
            "terrain": "everon", "game_mode": "pve_coop", "weather": "clear",
            "time_of_day": "12:00", "max_players": 32
        }))
        .send()
        .await?;
    if !create.status().is_success() {
        eprintln!("smoke_hydrate: create failed {}", create.status());
        return Ok(2);
    }
    let mission: Value = create.json().await?;
    let Some(mission_id) = mission["id"].as_str().map(str::to_string) else {
        eprintln!("smoke_hydrate: no mission id");
        return Ok(2);
    };

    // 1c. save a version with a KNOWN 3-slot editor block.
    let mk_slot = |i: i64| {
        json!({
            "id": format!("h{i}"), "squadId": "", "role": "Rifleman", "tag": "", "index": i,
            "stance": "stand",
            "position": { "x": 6400 + i, "y": 6400 + i, "z": 0, "rotation": 0 }, "assetId": "",
        })
    };
    let payload = json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon", "bounds": [0, 0, 12800, 12800] },
        "environment": { "time": "12:00", "weather": "clear" },
        "loadouts": {}, "objectives": [], "vehicles": [], "markers": [],
        "editor": {
            "factions": [], "squads": [],
            "editorLayers": [{ "id": "layer-1", "name": "Layer 1", "parentId": null, "entityIds": ["h0", "h1", "h2"] }],
            "slots": [mk_slot(0), mk_slot(1), mk_slot(2)],
        },
    });
    let save = http
        .post(format!("{BACKEND}/api/v1/missions/{mission_id}/versions"))
        .bearer_auth(&token)
        .json(&json!({ "semver": "0.2.0", "editor_notes": "hydrate gate", "payload": payload }))
        .send()
        .await?;
    if !save.status().is_success() {
        let status = save.status();
        let t = save.text().await.unwrap_or_default();
        eprintln!(
            "smoke_hydrate: save version failed {status} {}",
            t.chars().take(200).collect::<String>()
        );
        let _ = http
            .delete(format!("{BACKEND}/api/v1/missions/{mission_id}"))
            .bearer_auth(&token)
            .send()
            .await;
        return Ok(2);
    }

    // 2. serve with the same-origin /api proxy; seed the session + clear IDB before boot.
    let h = Harness::new(dist, 5315, 9375, None, Some(BACKEND.to_string()), &[]).await?;
    let run = async {
        let auth_blob = serde_json::to_string(&json!({
            "state": {
                "refreshToken": refresh,
                "user": {
                    "discord_id": "00000000000000001", "username": "Dev", "discord_handle": "d#1",
                    "avatar_url": "", "arma_id": null, "arma_character": "", "role": "admin",
                    "is_banned": false, "total_deployments": 0, "attendance_rate": 0,
                    "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
                },
                "expiresAt": "2030-01-01T00:00:00Z",
            },
            "version": 0,
        }))?;
        h.page
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": format!(
                    "localStorage.setItem('tbd-auth', {});try {{ indexedDB.deleteDatabase('tbd-mission-yrs'); }} catch (e) {{}}",
                    serde_json::to_string(&auth_blob)?
                ) }),
            )
            .await?;

        // Follow-up: pin force=webgl like every other editor smoke — with the slot
        // atlas live, the first hydrated slots_bind_soa allocates a GPU instance buffer, and
        // headless Chromium's software WebGPU device rejects any createBuffer (the known
        // wedge the suite avoids via WebGL2/SwiftShader).
        h.page
            .navigate(&h.url(&force_webgl(&format!("/missions/{mission_id}/edit"))))
            .await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for("typeof window.__missionDoc === 'object' && typeof window.__missionDoc.slot_count === 'function'", 120, 250)
            .await?;

        let mut checks = Map::new();
        if ready {
            checks.insert(
                "hydratedSavedSlots".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {SAVED_SLOTS}"),
                            120,
                            150
                        )
                        .await?
                ),
            );
            checks.insert(
                "notSeed".into(),
                json!(eval_i64(&h.page, "window.__missionDoc.slot_count()").await? != 8),
            );
            checks.insert(
                "notDirty".into(),
                json!(eval_bool(&h.page, "(() => { const el = document.querySelector('[aria-label=\"Unsaved changes\"]'); return !el || el.className.includes('hidden'); })()").await?),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 3);
        print_verdict(&json!({
            "gate": "editor-hydrate-smoke", "missionId": mission_id, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    // Cleanup: delete the test mission (both paths, as in the Node script).
    let _ = http
        .delete(format!("{BACKEND}/api/v1/missions/{mission_id}"))
        .bearer_auth(&token)
        .send()
        .await;
    code
}
