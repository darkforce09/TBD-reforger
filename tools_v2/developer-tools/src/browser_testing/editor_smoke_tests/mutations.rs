use super::*;
use crate::browser_testing::session_tokens::gate_access_token;
use crate::repository_layout::MapAssetMounts;
use std::path::Path;

/// Live suite-mutation gate (TOKEN/REFRESH envs, backend on :8080).
pub async fn smoke_mutations(dist: &str) -> Result<u8> {
    let Ok(_token) = std::env::var("TOKEN") else {
        eprintln!("smoke_mutations: set TOKEN + REFRESH (dev-login tokens)");
        return Ok(2);
    };
    let Ok(refresh) = std::env::var("REFRESH") else {
        eprintln!("smoke_mutations: set TOKEN + REFRESH (dev-login tokens)");
        return Ok(2);
    };
    let http = reqwest::Client::new();
    if !cdp::wait_http(&http, &format!("{BACKEND}/healthz"), 60).await {
        eprintln!("smoke_mutations: backend not reachable on :8080");
        return Ok(2);
    }

    let auth_blob = serde_json::to_string(&json!({
        "state": {
            "refreshToken": refresh,
            "user": {
                "discord_id": "00000000000000001", "username": "Dev Operator", "discord_handle": "dev#0001",
                "avatar_url": "", "arma_id": null, "arma_character": "", "role": "admin",
                "is_banned": false, "total_deployments": 0, "attendance_rate": 0,
                "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
            },
            "expiresAt": "2030-01-01T00:00:00Z",
        },
        "version": 0,
    }))?;

    let h = Harness::new(dist, 5320, 9380, None, Some(BACKEND.to_string()), &[]).await?;
    let run = async {
        h.page
            .send(
                "Page.addScriptToEvaluateOnNewDocument",
                json!({ "source": format!("localStorage.setItem('tbd-auth', {});", serde_json::to_string(&auth_blob)?) }),
            )
            .await?;
        h.page.navigate(&h.url("/settings")).await?;
        let ready = h.page
            .wait_for("[...document.querySelectorAll('button')].some(b => b.textContent.includes('Generate Link Code'))", 160, 250)
            .await?;

        let mut checks = Map::new();
        checks.insert("authedRender".into(), json!(ready));
        if ready {
            checks.insert(
                "noCodeBefore".into(),
                json!(
                    eval_bool(&h.page, "!document.body.textContent.includes('Link code:')").await?
                ),
            );
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.textContent.includes('Generate Link Code')).click()").await?;
            checks.insert(
                "codePanelAfter".into(),
                json!(
                    h.page
                        .wait_for("document.body.textContent.includes('Link code:')", 80, 250)
                        .await?
                ),
            );
            checks.insert(
                "toastShown".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('[role=status]')].some(n => /Link code generated/i.test(n.textContent))", 40, 250).await?),
            );
        }
        let pass = ready && h.no_panics() && checks.values().all(|v| *v == json!(true));
        print_verdict(
            &json!({ "gate": "suite-mutations-smoke", "checks": checks, "panics": h.panics_head(), "pass": pass }),
        );
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// The R-auth single-flight refresh gate (no backend; Fetch-mocked).
/// Exit map: 0 pass · 1 fail · 2 no dist · 3 driver error (mapped by the bin).
pub async fn r_auth(dist_override: Option<String>) -> Result<u8> {
    let dist = dist_override
        .or_else(|| std::env::var("LEPTOS_DIST").ok())
        .unwrap_or_else(|| {
            repo_root()
                .join(DIST_DEFAULT)
                .to_string_lossy()
                .into_owned()
        });
    if !PathBuf::from(&dist).join("index.html").exists() {
        eprintln!("gate_r_auth: no Leptos dist at {dist} (run `trunk build`)");
        return Ok(2);
    }
    const SEED: &str = "localStorage.setItem('tbd-auth', JSON.stringify({state:{refreshToken:'rt-seed',user:null,expiresAt:'2026-01-01T00:00:00Z'},version:0}));";
    let sample_user = json!({
        "discord_id": "1", "username": "cpl-authed", "discord_handle": "cpl#0001",
        "avatar_url": "", "arma_id": null, "arma_character": "", "role": "enlisted",
        "is_banned": false, "total_deployments": 0, "attendance_rate": 0.0,
        "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-01T00:00:00Z",
    });

    let h = Harness::new(&dist, 5193, 9341, None, None, &[SEED]).await?;
    let run = async {
        let refresh_count = Arc::new(StdMutex::new(0u64));
        let me_count = Arc::new(StdMutex::new(0u64));
        h.page
            .send(
                "Fetch.enable",
                json!({ "patterns": [{ "urlPattern": "*" }] }),
            )
            .await?;
        let mut paused = h.page.on_event("Fetch.requestPaused").await;
        let rp = Arc::clone(&h.page);
        let (rc_task, mc_task) = (Arc::clone(&refresh_count), Arc::clone(&me_count));
        let user_task = sample_user.clone();
        tokio::spawn(async move {
            while let Some(p) = paused.recv().await {
                let Some(request_id) = p["requestId"].as_str() else {
                    continue;
                };
                let u = p["request"]["url"].as_str().unwrap_or_default();
                let res = if u.contains("/api/v1/auth/refresh") {
                    *rc_task.lock().unwrap() += 1;
                    rp.fulfill_json(
                        request_id,
                        200,
                        &json!({
                            "access_token": gate_access_token("auth-refresh"), "refresh_token": "new-rt",
                            "expires_at": "2026-01-01T01:00:00Z",
                        }),
                    )
                    .await
                } else if u.contains("/api/v1/me") {
                    let n = {
                        let mut m = mc_task.lock().unwrap();
                        *m += 1;
                        *m
                    };
                    if n == 1 {
                        rp.fulfill_json(request_id, 401, &json!({ "error": "unauthorized" }))
                            .await
                    } else {
                        rp.fulfill_json(
                            request_id,
                            200,
                            &json!({ "user": user_task, "arma_linked": false }),
                        )
                        .await
                    }
                } else if u.contains("/api/v1/") {
                    // 200 {} — a 401 catch-all would loop every post-boot dashboard query
                    // through refresh (the gate pins the BOOTSTRAP single-flight; /me above
                    // still 401s exactly once).
                    rp.fulfill_json(request_id, 200, &json!({})).await
                } else {
                    rp.continue_request(request_id).await
                };
                let _ = res;
            }
        });

        h.page.navigate(&h.url("/")).await?;
        let ok = h.page
            .wait_for("(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user != null } catch { return false } })()", 80, 250)
            .await?;
        cdp::sleep_ms(100).await;
        let username = eval(&h.page, "(() => { try { return JSON.parse(localStorage.getItem('tbd-auth')||'{}').state?.user?.username || null } catch { return null } })()").await?;

        let rc = *refresh_count.lock().unwrap();
        let mc = *me_count.lock().unwrap();
        let pass = ok && rc == 1 && username == json!("cpl-authed");
        print_verdict(&json!({
            "gate": "R-auth", "pass": pass, "refreshCount": rc, "meCount": mc,
            "authedUsername": username, "expected": "cpl-authed",
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

pub async fn render_check(a: &RenderCheckArgs) -> Result<u8> {
    let seed = if a.seed_auth {
        Some(crate::browser_testing::dom_oracle::seed_script()?)
    } else {
        None
    };
    let mut injects: Vec<&str> = if a.no_freeze {
        Vec::new()
    } else {
        vec![crate::browser_testing::fixture_injection::FREEZE_SRC]
    };
    if let Some(s) = seed.as_deref() {
        injects.push(s);
    }
    let extra = match &a.inject_js {
        Some(p) => Some(
            std::fs::read_to_string(p)
                .map_err(|e| anyhow!("read --inject-js {}: {e}", p.display()))?,
        ),
        None => None,
    };
    if let Some(s) = extra.as_deref() {
        injects.push(s);
    }
    // Thread a real proxy (CLI `--api-proxy`, else the standard :8080 backend).
    let api_proxy = a.api_proxy.clone().or_else(|| Some(BACKEND.to_string()));
    let h = Harness::new(
        &a.dir,
        a.port,
        a.debug_port,
        a.map_assets.clone().map(MapAssetMounts::beside_terrains),
        api_proxy,
        &injects,
    )
    .await?;
    let run = async {
        let url = h.url(&a.path);
        h.page.navigate(&url).await?;
        let ready = h
            .page
            .wait_for(
                "!!document.body && document.body.innerText.trim().length > 0",
                80,
                250,
            )
            .await?;
        cdp::sleep_ms(150).await;
        let text = eval_str(&h.page, "document.body.innerText").await?;
        let html = eval_str(&h.page, "document.body.innerHTML").await?;
        // awaitPromise so async-IIFE probes can settle reactive updates between steps
        // (behavioral probes); plain values pass through unchanged. The raw value is
        // echoed in the verdict so a diagnostic probe can return a JSON string — but a string
        // (or any object without a boolean `pass`) is NOT a pass.
        let assert_value = match &a.assert_js {
            Some(js) => Some(h.page.evaluate(js, true).await?),
            None => None,
        };
        let assert_ok = assert_value.as_ref().map(assert_js_ok);
        if let Some(path) = &a.shot {
            let png = h.page.screenshot().await?;
            std::fs::write(path, &png)
                .map_err(|e| anyhow!("write screenshot {}: {e}", path.display()))?;
        }
        h.page.close().await;

        let found = if a.expect.is_empty() {
            Value::Null
        } else {
            json!(text.contains(&a.expect))
        };
        let pass = ready
            && (a.expect.is_empty() || text.contains(&a.expect))
            && a.assert_js.as_ref().is_none_or(|_| assert_ok == Some(true));
        print_verdict(&json!({
            "url": url, "ready": ready, "expect": a.expect, "found": found,
            "assertJs": a.assert_js, "assertOk": assert_ok, "assertValue": assert_value,
            "textPreview": text.chars().take(200).collect::<String>(),
            "htmlBytes": crate::browser_testing::dom_oracle::js_len(&html),
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Perf smoke — boots the editor on the full map-assets set, runs `PERF_PROBE` +
/// `PERF_PROBE_NOBLUR_PAN`, prints every metric. `strict` turns the deterministic rows into
/// gates (Phase-3 targets): duplicate/idle chunk fetches must be 0 and a steady pan must not
/// move upload/recompose counters. FPS rows stay report-only (SwiftShader variance) except the
/// G-B floor: bench fps_equiv ≥ 60 on every camera (checked only in strict mode).
pub async fn smoke_perf(dist: &str, strict: bool) -> Result<u8> {
    let path = EDIT_PATH;
    let h = Harness::new(
        dist,
        5321,
        9381,
        // The harness resolves a relative serving directory against the gate's working directory.
        Some(MapAssetMounts::from_root(Path::new(""))),
        None,
        &[],
    )
    .await?;
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
        let mut report = json!({});
        if ready {
            let settled = h
                .page
                .wait_for(
                    "typeof window.__mapAssets === 'object' && window.__mapAssets.hillshadeW > 0 && window.__mapAssets.world_chunks_drawn > 0 && (window.__mapAssets.atlas_bytes > 0 || window.__mapAssets.glyphAtlas === true)",
                    400,
                    250,
                )
                .await?;
            checks.insert("settled".into(), json!(settled));
            if settled {
                // awaitPromise=true — both probes are async IIFEs resolving a JSON string.
                let raw = h
                    .page
                    .evaluate(PERF_PROBE, true)
                    .await?
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let parsed: Value = serde_json::from_str(&raw).unwrap_or(json!({"error": raw}));
                let noblur_raw = h
                    .page
                    .evaluate(PERF_PROBE_NOBLUR_PAN, true)
                    .await?
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let noblur: Value =
                    serde_json::from_str(&noblur_raw).unwrap_or(json!({"error": noblur_raw}));
                checks.insert("probe_ok".into(), json!(parsed.get("error").is_none()));
                if strict {
                    // G-D: the chunk-fetch thrash gates. A camera that jumps around and then idles
                    // must never re-request a chunk it already has, nor fetch anything while idle.
                    let dupes = parsed["thrash"]["duplicate_fetches"].as_i64().unwrap_or(-1);
                    let idle = parsed["thrash"]["idle_fetches"].as_i64().unwrap_or(-1);
                    checks.insert("S_dup_fetches_zero".into(), json!(dupes == 0));
                    checks.insert("S_idle_fetches_zero".into(), json!(idle == 0));
                    // G-B (potato proxy): CPU-encode fps-equivalent ≥ 60 for every camera. This is
                    // the backend-independent encode ceiling (submit/raster is excluded — on the CI
                    // SwiftShader path submit is software rasterization, not representative of even a
                    // weak real GPU; the operator measures true GPU throughput on the 3070, G-A).
                    let bench_ok = ["town", "forest", "mid", "max"]
                        .iter()
                        .all(|k| parsed["bench"][*k]["fps_equiv"].as_f64().unwrap_or(0.0) >= 60.0);
                    checks.insert("S_bench_encode_60_floor".into(), json!(bench_ok));
                }
                report = json!({ "main": parsed, "noblur_pan": noblur });
            }
        }
        checks.insert("panic_free".into(), json!(h.no_panics()));
        let pass = ready
            && h.no_panics()
            && checks.values().all(|v| *v == json!(true))
            && !checks.is_empty();
        print_verdict(&json!({
            "gate": if strict { "editor-perf-smoke-strict" } else { "editor-perf-smoke" },
            "path": path,
            "checks": checks,
            "report": report,
            "panics": h.panics_head(),
            "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Explicit `--assert-js` verdict. No truthiness.
///
/// Recognised **pass**: literal boolean `true`, or a JSON object whose `"pass"` field is
/// boolean `true`. Recognised **fail**: literal `false` / `null` / `0` / `""`, an object with
/// `"pass": false`, an object/string/array/number with no recognised verdict — including a
/// diagnostic string echoed in `assertValue`. Truthiness is never a verdict: a probe
/// returning `{"pass":false,...}` must exit nonzero.
pub(crate) fn assert_js_ok(v: &Value) -> bool {
    if let Some(b) = v.as_bool() {
        return b;
    }
    if let Some(obj) = v.as_object() {
        return obj.get("pass").and_then(Value::as_bool) == Some(true);
    }
    false
}
