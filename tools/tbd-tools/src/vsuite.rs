//! T-165.5 — the V-suite verify/accept gate (port of `driver/gate_v_suite.mjs`).
//!
//! Captures the normalized DOM (the injected dom.js serializer — see `inject.rs` provenance)
//! plus a PNG for every leaf route and diffs against the frozen goldens under
//! `tools/tbd-tools/fixtures/t159/oracle-freeze/`. `verify` is the permanent V regression gate
//! (the React oracle is deleted); `accept` re-sources one route's golden from the current
//! Leptos dist with a recorded note. `freeze` was retired at T-171: the React dist it captured
//! from is gone, and `apps/website/frontend/dist` is the LIVE Leptos dist — a re-freeze would
//! overwrite the non-regenerable React oracle.
//!
//! Readiness = the freeze.js clock + fixture-intercepted fetches, then a stability loop:
//! serialize until two consecutive captures are byte-identical. Viewport pinned 1440×900.
//! Exit 0 = all routes green; 1 = any diff/missing; 3 = driver error (mapped in the bin).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::cdp::{self, Browser};
use crate::inject::{DOM_SERIALIZER_SRC, FREEZE_SRC};
use crate::serve::{ServeConfig, repo_root, start_server};

// The committed seed golden ids (memory/fixtures): mission / event / event-mission.
const MISSION: &str = "512d8658-7025-4a70-94e9-a1b44a7aa155";
const EVENT: &str = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";
const EM: &str = "89b1b731-37a8-4926-901a-3c7ff7de5eb3";

pub struct Route {
    pub slug: &'static str,
    pub path: String,
    pub authed: bool,
}

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

fn sha_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn gold_dir() -> PathBuf {
    repo_root().join("tools/tbd-tools/fixtures/t159/oracle-freeze")
}

fn fixtures_dir() -> PathBuf {
    repo_root().join("apps/website/frontend/tests/fixtures/api")
}

/// `/api/v1/<path>[?…]` → fixture file (gate_v's mapping): `GET__` + path with the trailing
/// slash stripped and `/` → `__`, `.json`.
fn fixture_for(url: &str) -> Option<PathBuf> {
    let idx = url.find("/api/v1/")?;
    let rest = &url[idx + "/api/v1/".len()..];
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    let slug = format!(
        "GET__{}.json",
        rest[..end].trim_end_matches('/').replace('/', "__")
    );
    let f = fixtures_dir().join(slug);
    f.exists().then_some(f)
}

/// The localStorage auth seed — the stored VALUE is built with the same key order as the
/// Node harness's object literal (serde_json preserve_order), so the app boots identically.
/// pub(crate): render-check's `--seed-auth` (T-172 behavioral probes) injects the same seed.
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

const SETTLE: &str = "(async()=>{await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()";

pub struct Capture {
    pub dom: String,
    pub png: Vec<u8>,
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
            map_assets_dir: None,
        },
        port,
    )
    .await?;
    let result = capture_inner(browser, srv.port, route).await;
    srv.close().await;
    result
}

async fn capture_inner(browser: &Browser, port: u16, route: &Route) -> Result<Capture> {
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
    let router = tokio::spawn(async move {
        while let Some(p) = paused.recv().await {
            let Some(request_id) = p["requestId"].as_str() else {
                continue;
            };
            let url = p["request"]["url"].as_str().unwrap_or_default();
            let reply: Option<Value> = if url.contains("/api/v1/auth/refresh") {
                Some(
                    json!({ "access_token": "acc-v", "refresh_token": "rt-v2", "expires_at": "2026-01-01T01:00:00Z" }),
                )
            } else if url.contains("/api/v1/auth/logout") {
                Some(json!({}))
            } else if let Some(f) = fixture_for(url) {
                std::fs::read_to_string(&f)
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            } else if url.contains("/api/v1/") {
                Some(json!({}))
            } else {
                None
            };
            let res = match reply {
                Some(body) => router_page.fulfill_json(request_id, 200, &body).await,
                None => router_page.continue_request(request_id).await,
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
            // Scope = the app root's first child (see gate_v_suite.mjs for the toaster note).
            let v = page
                .evaluate("__t159SerializeDom('#root>:first-child', null)", false)
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

pub struct VSuiteArgs {
    pub mode: String,
    pub leptos_dir: PathBuf,
    pub only: String,
    pub note: String,
}

/// Run the suite. Returns the process exit code (0 green, 1 diff/missing, 2 usage).
pub async fn run(args: &VSuiteArgs) -> Result<u8> {
    if args.mode == "freeze" {
        eprintln!(
            "v-suite freeze retired (T-171): the React oracle under {} is non-regenerable — \
             the source dist was deleted at T-159.29.3, and a capture from the live Leptos dist \
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

    let mut browser = cdp::launch(9341, &[]).await?;
    let result = run_modes(&browser, &gold, args, &selected).await;
    browser.kill();
    result
}

async fn run_modes(
    browser: &Browser,
    gold: &Path,
    args: &VSuiteArgs,
    routes: &[&Route],
) -> Result<u8> {
    let mut rows: Vec<Value> = Vec::new();
    let mut fail = 0usize;
    for route in routes {
        match args.mode.as_str() {
            "accept" => {
                // Preserve the React capture as the historical reference, then re-golden.
                let gold_file = gold.join(format!("{}.dom.json", route.slug));
                let react_ref = gold.join(format!("{}.react.dom.json", route.slug));
                if gold_file.exists() && !react_ref.exists() {
                    std::fs::copy(&gold_file, &react_ref)?;
                }
                let cap = capture_route(browser, &args.leptos_dir, 5197, route).await?;
                std::fs::write(&gold_file, &cap.dom)?;
                std::fs::write(gold.join(format!("{}.png", route.slug)), &cap.png)?;
                let manifest_path = gold.join("manifest.json");
                let mut manifest: Value =
                    serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
                let digest = sha_hex(cap.dom.as_bytes());
                if let Some(row) = manifest["routes"]
                    .as_array_mut()
                    .and_then(|a| a.iter_mut().find(|r| r["slug"] == route.slug))
                    && let Some(obj) = row.as_object_mut()
                {
                    obj.insert("goldenSource".into(), json!("leptos"));
                    obj.insert("bytes".into(), json!(js_len(&cap.dom)));
                    obj.insert("sha256".into(), json!(digest));
                    obj.insert("acceptedDelta".into(), json!(args.note));
                }
                std::fs::write(
                    &manifest_path,
                    serde_json::to_string_pretty(&manifest)? + "\n",
                )?;
                println!(
                    "accept {:<14} {:>8} B  {}  (react ref kept)",
                    route.slug,
                    js_len(&cap.dom),
                    &digest[..12]
                );
            }
            _ => {
                let gold_file = gold.join(format!("{}.dom.json", route.slug));
                if !gold_file.exists() {
                    rows.push(
                        json!({ "slug": route.slug, "pass": false, "error": "missing golden" }),
                    );
                    fail += 1;
                    continue;
                }
                let golden = std::fs::read_to_string(&gold_file)?;
                let cap = capture_route(browser, &args.leptos_dir, 5196, route).await?;
                let mut out = Vec::new();
                diff_node(
                    &serde_json::from_str(&golden)?,
                    &serde_json::from_str(&cap.dom)?,
                    "approot",
                    &mut out,
                    40,
                );
                let pass = out.is_empty();
                if !pass {
                    fail += 1;
                }
                println!(
                    "{}   {:<14} diffs={}  {}→{} B",
                    if pass { "PASS" } else { "FAIL" },
                    route.slug,
                    out.len(),
                    js_len(&golden),
                    js_len(&cap.dom)
                );
                rows.push(json!({
                    "slug": route.slug, "path": route.path, "pass": pass, "diffs": out.len(),
                    "goldenBytes": js_len(&golden), "leptosBytes": js_len(&cap.dom),
                    "first": out.iter().take(5).collect::<Vec<_>>(),
                }));
            }
        }
    }

    match args.mode.as_str() {
        "accept" => {
            println!(
                "\naccepted {} route(s) — golden re-sourced from Leptos, React reference kept",
                routes.len()
            );
        }
        _ => {
            println!(
                "\n{}/{} routes match the frozen oracle",
                rows.len() - fail,
                rows.len()
            );
            if fail > 0 {
                for r in rows.iter().filter(|x| x["pass"] == false) {
                    println!("{}", serde_json::to_string_pretty(r)?);
                }
            }
        }
    }
    Ok(u8::from(fail > 0))
}
