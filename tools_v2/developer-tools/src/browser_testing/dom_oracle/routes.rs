use super::*;

use std::sync::Mutex;

#[path = "fixture_router.rs"]
pub(super) mod fixture_router;
pub use fixture_router::MissingFixture;
use fixture_router::Reply;
use fixture_router::fixtures_dir;

/// slug → { path, authed }. 25 of routes.csv's 26 rows (the editor is excluded — its
/// regression gate is the CDP editor smokes, strictly stronger than a DOM snapshot).
pub fn routes() -> Vec<Route> {
    let r = |slug: &'static str, path: String, authed: bool| Route { slug, path, authed };
    vec![
        r("notfound", "/this-route-does-not-exist".into(), true),
        r("dashboard", "/".into(), true),
        r("approvals", "/admin/approvals".into(), true),
        r("audit", "/admin/audit".into(), true),
        r("content", "/admin/content".into(), true),
        r("eventmgr", "/admin/events".into(), true),
        r("personnel", "/admin/personnel".into(), true),
        r("servercontrol", "/admin/server".into(), true),
        r("announcements", "/announcements".into(), true),
        r("callback", "/auth/callback".into(), false),
        r("deployments", "/deployments".into(), true),
        r("events", "/events".into(), true),
        r("eventhub", format!("/events/{EVENT}"), true),
        r(
            "orbat",
            format!("/events/{EVENT}/missions/{EM}/orbat"),
            true,
        ),
        r("leaderboards", "/leaderboards".into(), true),
        r("login", "/login".into(), false),
        r("missions", "/missions".into(), true),
        r("missionview", format!("/missions/{MISSION}"), true),
        r("modpacks", "/modpacks".into(), true),
        r("serverintel", "/server-intel".into(), true),
        r("settings", "/settings".into(), true),
        r("mortar", "/tools/mortar".into(), true),
        r("vehicles", "/vehicles".into(), true),
        r("wiki", "/wiki".into(), true),
        r("wikislug", "/wiki/field-manual".into(), true),
    ]
}

/// JS `String.prototype.length` semantics — UTF-16 code units, not UTF-8 bytes. The Node
/// harness printed `golden.length`/`dom.length` and stored `bytes:` in the freeze manifest
/// with these units; the port keeps the same measure.
pub fn js_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Validate a captured DOM string before `accept` may overwrite a committed golden.
/// Mirrors verify-mode's `serde_json::from_str` parse, then refuses JSON `null` and
/// undersized captures that would destroy the baseline.
pub fn validate_accept_dom(dom: &str) -> Result<()> {
    let v: Value = serde_json::from_str(dom)
        .map_err(|e| anyhow!("accept refused: captured DOM is not valid JSON ({e})"))?;
    if v.is_null() {
        return Err(anyhow!(
            "accept refused: captured DOM is JSON null \
             (SPA failed to mount — inject serializer returns literal \"null\")"
        ));
    }
    if !v.is_object() {
        return Err(anyhow!(
            "accept refused: captured DOM root must be a JSON object, got {}",
            match &v {
                Value::Bool(_) => "bool",
                Value::Number(_) => "number",
                Value::String(_) => "string",
                Value::Array(_) => "array",
                _ => "non-object",
            }
        ));
    }
    let n = js_len(dom);
    if n < MIN_ACCEPT_DOM_JS_LEN {
        return Err(anyhow!(
            "accept refused: captured DOM js_len={n} < floor {MIN_ACCEPT_DOM_JS_LEN} \
             (structurally empty / undersized)"
        ));
    }
    Ok(())
}

pub(super) fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Append one unanswered request to the capture's shared record.
///
/// A poisoned lock cannot make the capture pass: the record is only ever read to *refuse* a route,
/// so the recovered guard is used rather than propagated.
fn record_missing(record: &Arc<Mutex<Vec<MissingFixture>>>, entry: MissingFixture) {
    let mut guard = match record.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.push(entry);
}

pub(super) fn gold_dir() -> PathBuf {
    repo_root().join("tools_v2/developer-tools/fixtures/dom_oracle/oracle-freeze")
}

/// The localStorage auth seed — the stored VALUE is built with the same key order as the
/// Node harness's object literal (serde_json preserve_order), so the app boots identically.
/// pub(crate): render-check's `--seed-auth` (behavioral probes) injects the same seed.
pub(crate) fn seed_script() -> Result<String> {
    let me: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures_dir().join("GET__me.json")).context("GET__me.json")?,
    )?;
    let inner = serde_json::to_string(&json!({
        "state": {
            "refreshToken": "rt-seed",
            "user": me["user"],
            "expiresAt": "2026-01-01T00:00:00Z"
        },
        "version": 0
    }))?;
    Ok(format!(
        "localStorage.setItem('tbd-auth', {});",
        serde_json::to_string(&inner)?
    ))
}

/// One-route capture: fresh server + fresh page, fixture-intercepted fetches, stability loop
/// (two consecutive byte-identical serializations = settled).
pub async fn capture_route(
    browser: &Browser,
    dir: &Path,
    port: u16,
    route: &Route,
) -> Result<Capture> {
    let srv = start_server(
        ServeConfig {
            dir: dir.to_path_buf(),
            api_proxy: None,
            map_assets: None,
        },
        port,
    )
    .await?;
    let result = capture_inner(browser, srv.port, route).await;
    srv.close().await;
    result
}

pub(super) async fn capture_inner(browser: &Browser, port: u16, route: &Route) -> Result<Capture> {
    let seed;
    let mut init: Vec<&str> = vec![FREEZE_SRC, DOM_SERIALIZER_SRC];
    if route.authed {
        seed = seed_script()?;
        init.push(&seed);
    }
    let page = Arc::new(cdp::new_page(browser, None, &init).await?);
    // The harness re-applies the viewport before Fetch.enable (mirrors captureRoute).
    page.send(
        "Emulation.setDeviceMetricsOverride",
        json!({ "width": 1440, "height": 900, "deviceScaleFactor": 1, "mobile": false }),
    )
    .await?;

    page.send(
        "Fetch.enable",
        json!({ "patterns": [{ "urlPattern": "*" }] }),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let router_page = Arc::clone(&page);
    // Every API request the corpus could not answer, in the order the page asked for it. The
    // capture reads it once the DOM has settled and refuses the route rather than snapshotting a
    // page that rendered without its data.
    let missing: Arc<Mutex<Vec<MissingFixture>>> = Arc::new(Mutex::new(Vec::new()));
    let router_missing = Arc::clone(&missing);
    let authed = route.authed;
    let router = tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let url = p["request"]["url"].as_str().unwrap_or_default();
            let method = p["request"]["method"].as_str().unwrap_or("GET");
            if authed && fixture_router::refuses_without_bearer(url, &p["request"]["headers"]) {
                let unauthorized = json!({ "error": "authentication required" });
                let _ = router_page
                    .fulfill_json(request_id, 401, &unauthorized)
                    .await;
                continue;
            }
            let res = match fixture_router::route(method, url) {
                Reply::Canned(body) => router_page.fulfill_json(request_id, 200, &body).await,
                Reply::Fixture { path, content_type } => {
                    match fixture_router::body_bytes(&path, content_type) {
                        Some(bytes) => {
                            router_page
                                .fulfill_raw(request_id, 200, content_type, &bytes)
                                .await
                        }
                        // Unreadable or malformed: the file exists but answers nothing, which is
                        // the same defect as its absence and is reported the same way.
                        None => {
                            record_missing(
                                &router_missing,
                                MissingFixture {
                                    url: url.to_string(),
                                    expected_file: format!(
                                        "{} (unreadable or malformed)",
                                        path.file_name().unwrap_or_default().to_string_lossy()
                                    ),
                                },
                            );
                            router_page.continue_request(request_id).await
                        }
                    }
                }
                Reply::Missing(m) => {
                    record_missing(&router_missing, m);
                    router_page.continue_request(request_id).await
                }
                Reply::Passthrough => router_page.continue_request(request_id).await,
            };
            let _ = res; // errors swallowed, as in the Node harness
        }
    });

    let run = async {
        page.navigate(&format!("http://localhost:{port}{}", route.path))
            .await?;
        let ok = page
            .wait_for("!!document.querySelector('body')", 80, 250)
            .await?;
        if !ok {
            return Err(anyhow!("body never appeared at {}", route.path));
        }

        // Stability loop: two consecutive byte-identical serializations = settled.
        let mut dom = String::new();
        let mut prev: Option<String> = None;
        for i in 0..60 {
            page.evaluate(SETTLE, true).await?;
            // Scope = the app root's first child, so a toaster mounted at the body is excluded.
            let v = page
                .evaluate("__domOracleSerialize('#root>:first-child', null)", false)
                .await?;
            dom = v
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| v.to_string());
            if prev.as_deref() == Some(dom.as_str()) {
                break;
            }
            prev = Some(dom.clone());
            cdp::sleep_ms(300).await;
            if i == 59 {
                return Err(anyhow!("DOM never stabilized at {}", route.path));
            }
        }
        let png = page.screenshot().await?;
        Ok(Capture { dom, png })
    };
    let result = run.await;
    page.close().await;
    router.abort();

    // Checked after the settle loop rather than at the first miss: one report naming every
    // unanswered request is what repairs the corpus, where the first one only restarts the hunt.
    let unanswered = match missing.lock() {
        Ok(g) => g.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    if !unanswered.is_empty() {
        return Err(anyhow!(fixture_router::missing_fixture_error(
            &route.path,
            &unanswered
        )));
    }
    result
}

/// Structural tree-diff (gate_v's diffNode). Cap checked at entry, as in the JS.
pub fn diff_node(o: &Value, l: &Value, path: &str, out: &mut Vec<Value>, cap: usize) {
    if out.len() >= cap {
        return;
    }
    if o.is_null() || l.is_null() || !o.is_object() || !l.is_object() {
        if o != l {
            out.push(json!({ "path": path, "oracle": o, "leptos": l }));
        }
        return;
    }
    if o["tag"] != l["tag"] {
        out.push(json!({ "path": format!("{path}/tag"), "oracle": o["tag"], "leptos": l["tag"] }));
        return;
    }
    // Union in JS `new Set([...o keys, ...l keys])` insertion order — the cap-40 cutoff and
    // `first` slice depend on push order (serde preserve_order keeps document key order).
    let empty_map = serde_json::Map::new();
    fn obj<'a>(
        v: &'a Value,
        k: &str,
        empty: &'a serde_json::Map<String, Value>,
    ) -> &'a serde_json::Map<String, Value> {
        v[k].as_object().unwrap_or(empty)
    }
    for (label, key) in [("@", "attrs"), ("style.", "style")] {
        let om = obj(o, key, &empty_map);
        let lm = obj(l, key, &empty_map);
        let mut keys: Vec<&String> = om.keys().collect();
        for k in lm.keys() {
            if !om.contains_key(k) {
                keys.push(k);
            }
        }
        for k in keys {
            if om.get(k) != lm.get(k) {
                // JS `JSON.stringify` drops `undefined` keys — omit the missing side.
                let mut row = serde_json::Map::new();
                row.insert("path".into(), json!(format!("{path}/{label}{k}")));
                if let Some(v) = om.get(k) {
                    row.insert("oracle".into(), v.clone());
                }
                if let Some(v) = lm.get(k) {
                    row.insert("leptos".into(), v.clone());
                }
                out.push(Value::Object(row));
            }
        }
    }
    let empty = vec![];
    let oc = o["children"].as_array().unwrap_or(&empty);
    let lc = l["children"].as_array().unwrap_or(&empty);
    if oc.len() != lc.len() {
        out.push(json!({
            "path": format!("{path}/children.length"),
            "oracle": oc.len(),
            "leptos": lc.len(),
        }));
    }
    for i in 0..oc.len().min(lc.len()) {
        let (oi, li) = (&oc[i], &lc[i]);
        if oi.is_string() || li.is_string() {
            if oi != li {
                out.push(
                    json!({ "path": format!("{path}/text[{i}]"), "oracle": oi, "leptos": li }),
                );
            }
        } else {
            let tag = li["tag"]
                .as_str()
                .or(oi["tag"].as_str())
                .unwrap_or("?")
                .to_string();
            diff_node(oi, li, &format!("{path}/{tag}[{i}]"), out, cap);
        }
    }
}

/// Run the suite. Returns the process exit code (0 green, 1 diff/missing, 2 usage).
pub async fn run(args: &VSuiteArgs) -> Result<u8> {
    if args.mode == "freeze" {
        eprintln!(
            "v-suite freeze retired: the React oracle under {} is non-regenerable — \
             its source dist is deleted, and a capture from the live Leptos dist \
             would overwrite it. Use `verify` (regression) or `accept --only <slug> --note` \
             (intentional single-route divergence).",
            gold_dir().display()
        );
        return Ok(2);
    }
    if !["verify", "accept"].contains(&args.mode.as_str()) {
        eprintln!(
            "usage: gate v-suite <verify|accept> [--leptos-dir d] [--only slug] [--note why]"
        );
        return Ok(2);
    }
    if args.mode == "accept" && (args.only.is_empty() || args.note.is_empty()) {
        eprintln!("accept requires --only <slug> and --note \"<why the divergence is intended>\"");
        return Ok(2);
    }
    let gold = gold_dir();
    let all = routes();
    let selected: Vec<&Route> = if args.only.is_empty() {
        all.iter().collect()
    } else {
        all.iter().filter(|r| r.slug == args.only).collect()
    };

    // Fonts: `cdp::launch` pins gate-owned `XDG_CACHE_HOME` on the chromium child
    // child (`.env`); `gate` main also calls `ensure_gate_font_cache` before tokio.
    let mut browser = cdp::launch(9341, &[]).await?;
    let result = run_modes(&browser, &gold, args, &selected).await;
    browser.kill();
    result
}
