use super::*;
use crate::browser_testing::session_tokens::gate_access_token;

/// Editor route requires `mission_maker`: a guest is bounced to
/// `?role_notice=mission_maker` before `__editorCam` appears.
/// Seed the v-suite admin fixture into `tbd-auth` on every new document so the suite (and
/// doctor liveness) can enter `/missions/smoke/edit`. Prefer **no** live `/api` proxy for the
/// pure UI smokes: a dead refresh against :8080 clears the seeded session (measured).
pub(super) fn editor_auth_seed() -> Result<String> {
    crate::browser_testing::dom_oracle::seed_script()
}

/// The auth-interception pattern used by the arsenal + outliner smokes: /registry →
/// the committed golden; other /api/v1/ → 401 {}; everything else continues. Returns a counter.
pub(super) async fn serve_registry_golden(page: &Arc<Page>) -> Result<Arc<StdMutex<u64>>> {
    let golden = std::fs::read_to_string(
        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__registry.json"),
    )?;
    let golden: Value = serde_json::from_str(&golden)?;
    let hits = Arc::new(StdMutex::new(0u64));
    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let rp = Arc::clone(page);
    let hits_task = Arc::clone(&hits);
    tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let u = p["request"]["url"].as_str().unwrap_or_default();
            let res = if u.contains("/api/v1/registry/compat") && u.contains("view=cargo_defaults")
            {
                rp.fulfill_json(
                    request_id,
                    200,
                    &json!({
                        "view": "cargo_defaults", "data": {},
                        "etag": "W/\"outliner-cargo\"",
                        "modpack_id": "00000000-0000-4000-a000-000000000001",
                        "modpack_version": "test", "source_edge_count": 0
                    }),
                )
                .await
            } else if u.contains("/api/v1/registry/compat") {
                rp.fulfill_json(
                    request_id,
                    200,
                    &json!({
                        "data": [], "etag": "W/\"outliner-compat\"",
                        "modpack_id": "00000000-0000-4000-a000-000000000001",
                        "modpack_version": "test"
                    }),
                )
                .await
            } else if u.contains("/api/v1/registry") {
                *hits_task.lock().unwrap() += 1;
                let mut body = golden.clone();
                if let Some(arr) = body["data"].as_array() {
                    let n = arr.len() as u64;
                    body["total"] = json!(n);
                    body["limit"] = json!(500);
                    body["offset"] = json!(0);
                }
                rp.fulfill_json(request_id, 200, &body).await
            } else if u.contains("/api/v1/auth/refresh") {
                rp.fulfill_json(
                    request_id,
                    200,
                    &json!({
                        "access_token": gate_access_token("outliner"),
                        "refresh_token": "rt-seed",
                        "expires_at": "2030-01-01T00:00:00Z"
                    }),
                )
                .await
            } else if u.contains("/api/v1/me") {
                let me: Value = serde_json::from_str(
                    &std::fs::read_to_string(
                        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__me.json"),
                    )
                    .unwrap_or_else(|_| "{}".into()),
                )
                .unwrap_or(json!({}));
                rp.fulfill_json(request_id, 200, &me).await
            } else if u.contains("/api/v1/") {
                rp.fulfill_json(request_id, 401, &json!({})).await
            } else {
                rp.continue_request(request_id).await
            };
            let _ = res;
        }
    });
    Ok(hits)
}

/// Smart-Arsenal tap: the committed registry golden **augmented** with a compat optic +
/// magazine (kind + weight), plus `/registry/compat` edges linking the golden's M16A2 to them and
/// `/factions` from its golden. Inline JSON so the r_api-pinned `GET__registry.json` stays byte-exact.
/// `(registry_hits, compat_hits, faction_post_hits)`.
pub(super) async fn serve_arsenal_golden(
    page: &Arc<Page>,
    m16a2: &str,
) -> Result<(Arc<StdMutex<u64>>, Arc<StdMutex<u64>>, Arc<StdMutex<u64>>)> {
    const OPTIC: &str = "{ARSENAL_OPTIC}Prefabs/Weapons/Attachments/Optic_ACOG.et";
    const MAG: &str = "{ARSENAL_MAG}Prefabs/Weapons/Magazines/Mag_STANAG_30.et";
    let root = repo_root();
    let mut registry: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("apps/website/frontend/tests/fixtures/api/GET__registry.json"),
    )?)?;
    let mp = registry["modpack_id"].clone();
    let mk = |rn: &str, name: &str, kind: &str, wkg: f64| {
        json!({
            "id": format!("arsenal-{kind}"), "modpack_id": mp.clone(),
            "resource_name": rn, "display_name": name, "category": "Weapons/Attachments",
            "kind": kind, "weight_kg": wkg, "sort_order": 900,
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
        })
    };
    if let Some(arr) = registry["data"].as_array_mut() {
        arr.push(mk(OPTIC, "ACOG", "gear_optic", 0.6));
        arr.push(mk(MAG, "STANAG 30rd", "gear_magazine", 0.45));
        let n = arr.len() as u64;
        // Paginated cold path reads `total`.
        registry["total"] = json!(n);
        registry["limit"] = json!(500);
        registry["offset"] = json!(0);
    }
    let compat = json!({
        "data": [
            { "id": "e1", "modpack_id": mp.clone(), "from_node": m16a2, "to_node": OPTIC,
              "edge_type": "optic_on_weapon", "evidence": "",
              "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
            { "id": "e2", "modpack_id": mp.clone(), "from_node": m16a2, "to_node": MAG,
              "edge_type": "mag_in_weapon", "evidence": "",
              "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z" },
        ],
        "etag": "W/\"arsenal-compat\"", "modpack_id": mp, "modpack_version": "test",
    });
    let factions: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("apps/website/frontend/tests/fixtures/api/GET__factions.json"),
    )?)?;

    let reg_hits = Arc::new(StdMutex::new(0u64));
    let compat_hits = Arc::new(StdMutex::new(0u64));
    let post_hits = Arc::new(StdMutex::new(0u64));
    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let rp = Arc::clone(page);
    let (rh, ch, ph) = (
        Arc::clone(&reg_hits),
        Arc::clone(&compat_hits),
        Arc::clone(&post_hits),
    );
    tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let u = p["request"]["url"].as_str().unwrap_or_default();
            let method = p["request"]["method"].as_str().unwrap_or("GET");
            let res = if u.contains("/api/v1/registry/compat") && u.contains("view=cargo_defaults")
            {
                // Cold path also GETs cargo_defaults; returning the edge list here
                // makes fetch_compat_cold fail deserialize → CompatStatus::Unavailable → no optics.
                rp.fulfill_json(
                    request_id,
                    200,
                    &json!({
                        "view": "cargo_defaults",
                        "data": {},
                        "etag": "W/\"arsenal-cargo\"",
                        "modpack_id": mp,
                        "modpack_version": "test",
                        "source_edge_count": 0
                    }),
                )
                .await
            } else if u.contains("/api/v1/registry/compat") {
                *ch.lock().unwrap() += 1;
                rp.fulfill_json(request_id, 200, &compat).await
            } else if u.contains("/api/v1/registry") {
                *rh.lock().unwrap() += 1;
                rp.fulfill_json(request_id, 200, &registry).await
            } else if u.contains("/api/v1/auth/refresh") {
                // Keep the seeded editor session alive under Fetch (rt-seed is not a real token).
                rp.fulfill_json(
                    request_id,
                    200,
                    &json!({
                        "access_token": gate_access_token("arsenal"),
                        "refresh_token": "rt-seed",
                        "expires_at": "2030-01-01T00:00:00Z"
                    }),
                )
                .await
            } else if u.contains("/api/v1/me") {
                let me: Value = serde_json::from_str(
                    &std::fs::read_to_string(
                        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__me.json"),
                    )
                    .unwrap_or_else(|_| "{}".into()),
                )
                .unwrap_or(json!({}));
                rp.fulfill_json(request_id, 200, &me).await
            } else if u.contains("/api/v1/factions") {
                if method == "POST" || method == "PUT" {
                    *ph.lock().unwrap() += 1;
                    rp.fulfill_json(request_id, 201, &json!({
                        "id": "fnew", "owner_id": "smoke", "side": "BLUFOR", "name": "Smoke Bn",
                        "doc": { "side": "BLUFOR", "name": "Smoke Bn", "roles": [], "vehicles": [] },
                        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z"
                    })).await
                } else {
                    rp.fulfill_json(request_id, 200, &factions).await
                }
            } else if u.contains("/api/v1/") {
                rp.fulfill_json(request_id, 401, &json!({})).await
            } else {
                rp.continue_request(request_id).await
            };
            let _ = res;
        }
    });
    Ok((reg_hits, compat_hits, post_hits))
}
