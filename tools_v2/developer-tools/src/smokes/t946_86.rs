//! Real editor handlers over a controlled hydrated mission. Coordinates come from the rendered
//! widget; assertions read the production compile payload and history. No copied gesture model.
use super::*;

const ID: &str = "94686000-0000-4000-a000-000000000001";
const LARGE_ID: &str = "94686000-0000-4000-a000-000000000003";
const DUP_ID: &str = "94686000-0000-4000-a000-000000000002";
const MIXED_ID: &str = "94686000-0000-4000-a000-000000000004";
const PAYLOAD: &str = "JSON.parse(window.__editorCommands.compile_save_json())";

fn mission(duplicate: bool, large: bool) -> Value {
    let ids: Vec<String> = (0..5).map(|i| format!("roof-{i}")).collect();
    let mut squad_ids = ids.clone();
    if duplicate {
        squad_ids.push(ids[0].clone());
    }
    let slots: Vec<Value> = (0..5).map(|i| json!({
        "id": ids[i], "squadId":"sq", "role":"Rifleman", "tag":"", "index":i,
        "stance":"stand", "assetId":"", "position":{
            "x":6400.0 + i as f64 * 2.0, "y":6400.0, "z":50.0 + i as f64 * 10.0, "rotation":15.0
        }
    })).collect();
    let mut payload = json!({
        "schemaVersion":1,"map":{"terrain":"everon","bounds":[0,0,12800,12800]},
        "environment":{"time":"12:00","weather":"clear","tacticalGraphics":[]},"loadouts":{},"objectives":[],"markers":[],
        "vehicles":[{"id":"vehicle-roof","resourceName":"Vehicle.et","position":{"x":6600.0,"y":6400.0,"z":81.5,"rotation":90.0}}],
        "editor":{
            "factions":[{"id":"f","name":"BLUFOR","key":"BLUFOR","squadIds":["sq","bravo","charlie"]}],
            "squads":[{"id":"sq","factionId":"f","name":"Alpha","callsign":"Alpha","slotIds":squad_ids,"vehicleIds":[]},
                {"id":"bravo","factionId":"f","name":"Bravo","callsign":"Bravo","slotIds":[],"vehicleIds":[]},
                {"id":"charlie","factionId":"f","name":"Charlie","callsign":"Charlie","slotIds":[],"vehicleIds":[]}],
            "editorLayers":[
                {"id":"source","name":"Recovery source","parentId":null,"entityIds":ids},
                {"id":"destination","name":"Recovery destination","parentId":null,"entityIds":[]}
            ],"slots":slots
        }
    });
    if large {
        let layers = payload["editor"]["editorLayers"].as_array_mut().unwrap();
        for i in 0..60 {
            layers.push(json!({"id":format!("extra-{i}"),"name":format!("ZZ extra {i}"),"parentId":null,"entityIds":[]}));
        }
    }
    let id = if duplicate {
        DUP_ID
    } else if large {
        LARGE_ID
    } else {
        ID
    };
    json!({
        "id":id,"title":"T-946.86 regression","terrain":"everon","game_mode":"pve_coop",
        "weather":"clear","time_of_day":"12:00","max_players":32,"status":"draft",
        "author_id":"00000000000000001","author_name":"Dev","author_avatar":"", "bookmarked":false,
        "armory":[],"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z",
        "current_version":{"id":"v1","mission_id":id,"semver":"0.1.0","created_by":"00000000000000001",
            "created_at":"2026-01-01T00:00:00Z","json_payload":payload}
    })
}

fn mixed_mission() -> Value {
    let mut row = mission(false, false);
    row["id"] = json!(MIXED_ID);
    row["current_version"]["mission_id"] = json!(MIXED_ID);
    let p = &mut row["current_version"]["json_payload"];
    p["editor"]["factions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"red-f","key":"OPFOR","name":"OPFOR","squadIds":["red"]}));
    p["editor"]["squads"][0]["slotIds"] = json!(["roof-0", "roof-1", "roof-2"]);
    p["editor"]["squads"][1]["name"] = json!("Visible Bravo");
    p["editor"]["squads"].as_array_mut().unwrap().push(json!({"id":"red","factionId":"red-f","name":"Red","callsign":"Red","slotIds":["roof-3","roof-4"],"vehicleIds":[]}));
    for (i, slot) in p["editor"]["slots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        slot["role"] = json!(if i == 0 {
            "Visible Rifleman"
        } else {
            "Hidden Rifleman"
        });
        if i >= 3 {
            slot["squadId"] = json!("red");
        }
    }
    row
}

async fn intercept(page: &Arc<Page>) -> Result<Arc<StdMutex<u64>>> {
    // Baseline failures deliberately leave dirty history. Accept its real unload prompt so
    // the following isolated fixture can load without a pending CDP navigation.
    let mut dialogs = page.on_event("Page.javascriptDialogOpening").await;
    let dialog_page = page.clone();
    tokio::spawn(async move {
        while let Some(dialog) = dialogs.recv().await {
            eprintln!("t946-86 dialog: {}", dialog["type"]);
            let _ = dialog_page
                .send("Page.handleJavaScriptDialog", json!({"accept":true}))
                .await;
        }
    });
    let posts = Arc::new(StdMutex::new(0));
    let me: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__me.json"),
    )?)?;
    let registry: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("apps/website/frontend/tests/fixtures/api/GET__registry.json"),
    )?)?;
    page.send(
        "Fetch.enable",
        json!({"patterns":[{"urlPattern":"*/api/v1/*"}]}),
    )
    .await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let page = page.clone();
    let counter = posts.clone();
    tokio::spawn(async move {
        while let Some(event) = paused.recv().await {
            let Some(id) = event["requestId"].as_str() else {
                continue;
            };
            let url = event["request"]["url"].as_str().unwrap_or("");
            let method = event["request"]["method"].as_str().unwrap_or("");
            let (status, body) = if method == "POST" && url.contains("/versions") {
                *counter.lock().unwrap() += 1;
                (400, json!({"error":"smoke observed a save request"}))
            } else if url.contains("/auth/refresh") {
                (
                    200,
                    json!({"access_token":"recovery-access","refresh_token":"rt-seed","expires_at":"2030-01-01T00:00:00Z"}),
                )
            } else if url.ends_with("/me") {
                (200, me.clone())
            } else if url.contains("/registry") {
                (200, registry.clone())
            } else if url.contains(DUP_ID) {
                (200, mission(true, false))
            } else if url.contains(LARGE_ID) {
                (200, mission(false, true))
            } else if url.contains(ID) {
                (200, mission(false, false))
            } else if url.contains(MIXED_ID) {
                (200, mixed_mission())
            } else {
                (200, json!({"data":[],"total":0,"limit":50,"offset":0}))
            };
            let _ = page.fulfill_json(id, status, &body).await;
        }
    });
    Ok(posts)
}

async fn payload(page: &Page) -> Result<Value> {
    eval(page, PAYLOAD).await
}
async fn depth(page: &Page) -> Result<i64> {
    eval_i64(page, "window.__editorHistory.undo_depth()").await
}
async fn settle() {
    cdp::sleep_ms(180).await;
}
// Keep CDP's held button consistent with its buttons mask. With button:none Chromium emits a
// gotpointercapture carrying buttons=0 and drops capture on the next move, before any real release.
async fn drag(page: &Page, x: f64, y: f64, end_x: f64, end_y: f64) -> Result<()> {
    mouse(
        page,
        "mousePressed",
        x,
        y,
        json!({"button":"left","buttons":1,"clickCount":1}),
    )
    .await?;
    for n in 1..=6 {
        let t = f64::from(n) / 6.0;
        mouse(
            page,
            "mouseMoved",
            x + (end_x - x) * t,
            y + (end_y - y) * t,
            json!({"button":"left","buttons":1}),
        )
        .await?;
    }
    mouse(
        page,
        "mouseReleased",
        end_x,
        end_y,
        json!({"button":"left","buttons":0,"clickCount":1}),
    )
    .await?;
    Ok(())
}
async fn row_drag(page: &Page, source: &str, destination: &str) -> Result<bool> {
    let points=eval(page,&format!("(() => {{const a={source};const b={destination};return [a,b].map(e=>{{if(!e)return null;const r=e.getBoundingClientRect();const x=r.left+r.width/2,y=r.top+r.height/2;return {{x,y,hit:e.contains(document.elementFromPoint(x,y))}};}});}})()")).await?;
    if !points
        .as_array()
        .is_some_and(|a| a.iter().all(|p| p["hit"] == true))
    {
        return Ok(false);
    }
    drag(
        page,
        points[0]["x"].as_f64().unwrap(),
        points[0]["y"].as_f64().unwrap(),
        points[1]["x"].as_f64().unwrap(),
        points[1]["y"].as_f64().unwrap(),
    )
    .await?;
    Ok(true)
}
async fn click_selector(page: &Page, selector: &str) -> Result<bool> {
    let hit=eval_bool(page,&format!("(() => {{const e=document.querySelector({});if(!e)return false;const r=e.getBoundingClientRect();return e.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2));}})()",json!(selector))).await?;
    anyhow::ensure!(hit, "visible UI control is not hit-testable: {selector}");
    super::click_selector(page, selector).await
}
async fn canvas_hit(page: &Page, x: f64, y: f64) -> Result<()> {
    let hit = eval_bool(
        page,
        &format!("document.elementFromPoint({x},{y}) === document.querySelector('canvas')"),
    )
    .await?;
    anyhow::ensure!(hit, "canvas point is obscured: ({x},{y})");
    Ok(())
}
async fn click_at(page: &Page, x: f64, y: f64, ctrl: bool) -> Result<()> {
    canvas_hit(page, x, y).await?;
    super::click_at(page, x, y, ctrl).await
}
async fn fixture_ready(page: &Page, expression: &str) -> Result<()> {
    for _ in 0..480 {
        if page
            .evaluate_with_timeout(
                &format!("({expression}) && !document.querySelector('.mc-load-fill')"),
                false,
                std::time::Duration::from_secs(5),
            )
            .await?
            .as_bool()
            == Some(true)
        {
            page.send("Page.bringToFront", json!({})).await?;
            return Ok(());
        }
        cdp::sleep_ms(250).await;
    }
    anyhow::bail!("fixture hydration failed: {expression}")
}
async fn undo(page: &Page) -> Result<()> {
    key_chord(page, "z", "KeyZ", 2, 90).await?;
    settle().await;
    Ok(())
}
async fn select_five(page: &Page) -> Result<()> {
    click_selector(page, "[aria-label='No widget']").await?;
    eval(page, "window.__editorCamSet(6404,6400,0)").await?;
    settle().await;
    let probe = eval(page, "JSON.parse(window.__editorSelection.probe_marquee())").await?;
    let r = &probe["rect"];
    drag(
        page,
        r[0].as_f64().unwrap(),
        r[1].as_f64().unwrap(),
        r[2].as_f64().unwrap(),
        r[3].as_f64().unwrap(),
    )
    .await?;
    settle().await;
    anyhow::ensure!(
        eval_i64(page, "window.__editorSelection.count()").await? == 5,
        "fixture must select exactly five slots"
    );
    click_selector(page, "[aria-label='Translate widget']").await?;
    settle().await;
    Ok(())
}
async fn widget(page: &Page) -> Result<(f64, f64)> {
    let p = eval(page,"(() => { const s=document.querySelector('[data-transform-widget]'); const l=s?.querySelector('line'); if(!l) throw Error('no translate widget'); const r=s.getBoundingClientRect(); return [r.left+Number(l.getAttribute('x1')),r.top+Number(l.getAttribute('y1'))]; })()").await?;
    Ok((p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
}
async fn z_start(page: &Page, x: f64, y: f64) -> Result<()> {
    canvas_hit(page, x, y).await?;
    mouse(
        page,
        "mousePressed",
        x,
        y,
        json!({"button":"left","buttons":1,"clickCount":1}),
    )
    .await?;
    mouse(
        page,
        "mouseMoved",
        x,
        y - 8.0,
        json!({"button":"left","buttons":1}),
    )
    .await?;
    mouse(
        page,
        "mouseMoved",
        x,
        y - 12.0,
        json!({"button":"left","buttons":1}),
    )
    .await?;
    settle().await;
    Ok(())
}
fn same_positions(a: &Value, b: &Value) -> bool {
    a["editor"]["slots"] == b["editor"]["slots"] && a["vehicles"] == b["vehicles"]
}

async fn vehicle_point(page: &Page) -> Result<(f64, f64)> {
    let p=eval(page,"(() => {const c=JSON.parse(window.__editorCam()); const r=document.querySelector('canvas').getBoundingClientRect(); return [r.left+r.width/2+(6600-c.tx)*2**c.z,r.top+r.height/2-(6400-c.ty)*2**c.z];})()").await?;
    Ok((p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
}
async fn vehicle_snap_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
    click_at(page, 1000.0, 700.0, false).await?;
    let (vx, vy) = vehicle_point(page).await?;
    click_at(page, vx, vy, false).await?;
    settle().await;
    let selected=eval_bool(page,"JSON.parse(window.__editorSelection.ids()).length===1 && JSON.parse(window.__editorSelection.ids())[0]==='vehicle-roof'").await?;
    click_selector(page, "[aria-label='Toggle snap grid']").await?;
    for _ in 0..2 {
        click_selector(page, "[aria-label='Increase snap step']").await?;
    }
    for (name, modifier, expected) in [
        ("vehicle_snap", 0, 10.0),
        ("vehicle_shift_suspends_snap", 8, 12.0),
    ] {
        let before = payload(page).await?;
        let d = depth(page).await?;
        let (x, cy) = widget(page).await?;
        let y = cy - 45.0;
        canvas_hit(page, x, y).await?;
        mouse(
            page,
            "mousePressed",
            x,
            y,
            json!({"button":"left","buttons":1,"clickCount":1,"modifiers":modifier}),
        )
        .await?;
        for offset in [8.0, 12.0] {
            mouse(
                page,
                "mouseMoved",
                x,
                y - offset,
                json!({"button":"left","buttons":1,"modifiers":modifier}),
            )
            .await?;
        }
        eprintln!("t946-86 {name} before release: {}",eval(page,"({chip:document.querySelector('[data-transform-widget] text')?.textContent,capture:document.querySelector('canvas').parentElement.hasPointerCapture(1),ids:JSON.parse(window.__editorSelection.ids()),events:window.__t94686Events})").await?);
        mouse(
            page,
            "mouseReleased",
            x,
            y - 12.0,
            json!({"button":"left","buttons":0,"clickCount":1,"modifiers":modifier}),
        )
        .await?;
        settle().await;
        let after = payload(page).await?;
        let z = |v: &Value| {
            v["vehicles"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == "vehicle-roof")
                .unwrap()["position"]["z"]
                .as_f64()
                .unwrap()
        };
        eprintln!(
            "t946-86 {name}: {}",
            json!({"selected":selected,"before_z":z(&before),"after_z":z(&after),"expected_delta":expected,"before_depth":d,"after_depth":depth(page).await?,"status":eval_str(page,"document.body.innerText.slice(-1500)").await?})
        );
        checks.insert(
            name.into(),
            json!(
                selected
                    && (z(&after) - z(&before) - expected).abs() < 1e-9
                    && depth(page).await? == d + 1
            ),
        );
        undo(page).await?;
        checks.insert(
            format!("{name}_undo"),
            json!(same_positions(&before, &payload(page).await?)),
        );
    }
    click_selector(page, "[aria-label='Toggle snap grid']").await?;
    Ok(())
}

// A release outside the tree and a cancellation must clear both pending representations.
async fn outside_drop_cases(
    page: &Page,
    checks: &mut Map<String, Value>,
    prefix: &str,
) -> Result<()> {
    for event in ["pointerup", "pointercancel"] {
        eval(page,"(() => {const e=document.querySelector('[data-testid=outliner-window-scroller]');if(e){e.scrollTop=e.scrollHeight;e.dispatchEvent(new Event('scroll'));}})()").await?;
        settle().await;
        let before = payload(page).await?;
        let d = depth(page).await?;
        let armed=eval_bool(page,"(() => { const b=document.querySelector('aside button[aria-label=Rifleman]'); if(!b)return false; const r=b.getBoundingClientRect();if(!b.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2)))return false; b.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:1,button:0,buttons:1})); return true; })()").await?;
        eval(page,&format!("document.body.dispatchEvent(new PointerEvent('{event}',{{bubbles:true,pointerId:1,button:0}}))")).await?;
        settle().await;
        eval(page,"(() => {const e=document.querySelector('[data-testid=outliner-window-scroller]');if(e){e.scrollTop=0;e.dispatchEvent(new Event('scroll'));}})()").await?;
        settle().await;
        let released=eval_bool(page,"(() => { const b=[...document.querySelectorAll('aside button')].find(b=>(b.getAttribute('aria-label')||'').includes('Recovery destination')); if(!b)return false; b.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:1,button:0})); return true; })()").await?;
        settle().await;
        eprintln!(
            "t946-86 {prefix} {event}: {}",
            json!({"armed":armed,"released":released,"depth_before":d,"depth_after":depth(page).await?,"layers_same":payload(page).await?["editor"]["editorLayers"]==before["editor"]["editorLayers"],"buttons":eval(page,"[...document.querySelectorAll('aside button')].map(b=>b.getAttribute('aria-label')).filter(Boolean)").await?})
        );
        checks.insert(
            format!("{prefix}_{event}_clears_outliner_latches"),
            json!(
                armed
                    && released
                    && payload(page).await?["editor"]["editorLayers"]
                        == before["editor"]["editorLayers"]
                    && depth(page).await? == d
            ),
        );
    }
    Ok(())
}

async fn z_lifecycle_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
    for event in ["lostpointercapture", "blur", "Escape", "widget", "tool"] {
        select_five(page).await?;
        let before = payload(page).await?;
        let d = depth(page).await?;
        let (x, cy) = widget(page).await?;
        z_start(page, x, cy - 45.0).await?;
        let armed = eval_bool(page,"!!document.querySelector('[data-transform-widget] text') && document.querySelector('canvas').parentElement.hasPointerCapture(1)").await?;
        // An unrelated release must leave the real arm live, before cancellation is tested.
        eval(page,"document.querySelector('canvas').parentElement.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:99,clientX:700,clientY:300,button:0}))").await?;
        let unrelated_ignored =
            same_positions(&before, &payload(page).await?) && depth(page).await? == d;
        match event {
            "lostpointercapture" => {
                eval(
                    page,
                    "(() => {const c=document.querySelector('canvas').parentElement;if(c.hasPointerCapture(1))c.releasePointerCapture(1);})()",
                )
                .await?;
            }
            "blur" => {
                eval(page, "window.dispatchEvent(new Event('blur'))").await?;
            }
            "Escape" => {
                key_chord(page, "Escape", "Escape", 0, 27).await?;
            }
            "widget" => {
                eval(
                    page,
                    "document.querySelector('[aria-label=\"No widget\"]').click()",
                )
                .await?;
            }
            "tool" => {
                eval(page,"document.querySelector('button[title^=Ruler]').dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:2,button:0}))").await?;
            }
            _ => unreachable!(),
        }
        settle().await;
        eprintln!(
            "t946-86 lifecycle {event}: {}",
            json!({"armed":armed,"unrelated_ignored":unrelated_ignored,"depth_before":d,"depth_after":depth(page).await?,"positions_unchanged":same_positions(&before,&payload(page).await?),"dom":eval(page,"({chip:document.querySelector('[data-transform-widget] text')?.textContent,capture:document.querySelector('canvas').parentElement.hasPointerCapture(1)})").await?})
        );
        mouse(
            page,
            "mouseMoved",
            x,
            cy - 75.0,
            json!({"button":"left","buttons":1}),
        )
        .await?;
        mouse(
            page,
            "mouseReleased",
            x,
            cy - 75.0,
            json!({"button":"left","buttons":0,"clickCount":1}),
        )
        .await?;
        settle().await;
        checks.insert(format!("z_{event}_cancels_real_capture"),json!(armed && unrelated_ignored && same_positions(&before,&payload(page).await?) && depth(page).await?==d && eval_bool(page,"!document.querySelector('[data-transform-widget] text') && !document.querySelector('canvas').parentElement.hasPointerCapture(1)").await?));
        if event == "tool" {
            eval(page,"document.querySelector('button[title=Select]').dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:2,button:0}))").await?;
            settle().await;
        }
    }
    Ok(())
}

async fn orbat_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
    select_five(page).await?;
    let before = payload(page).await?;
    let opened = click_selector(page, "[aria-label='ORBAT Manager']").await?;
    settle().await;
    eprintln!("t946-86 ORBAT DOM: {}",eval(page,"({rows:[...document.querySelectorAll('[role=button]')].map(e=>e.getAttribute('aria-label')),names:[...document.querySelectorAll('span')].filter(e=>/Bravo|Charlie/.test(e.textContent)).map(e=>e.textContent)})").await?);
    let d = depth(page).await?;
    let drop=row_drag(page,"document.querySelector('[role=button][aria-label=Rifleman]')","[...document.querySelectorAll('span')].find(e=>e.textContent.startsWith('Bravo ('))?.parentElement").await?;
    settle().await;
    let moved = payload(page).await?;
    let n = moved["editor"]["squads"]
        .as_array()
        .and_then(|a| a.iter().find(|s| s["id"] == "bravo"))
        .and_then(|s| s["slotIds"].as_array())
        .map_or(0, Vec::len);
    eprintln!(
        "t946-86 ORBAT drop: {}",
        json!({"opened":opened,"dispatched":drop,"moved":n,"depth_before":d,"depth_after":depth(page).await?,"squads":moved["editor"]["squads"]})
    );
    checks.insert(
        "orbat_five_row_one_undo".into(),
        json!(opened && drop && n == 5 && depth(page).await? == d + 1),
    );
    let click=eval_bool(page,"(() => { const e=[...document.querySelectorAll('span')].find(e=>/^Charlie \\(/.test(e.textContent))?.parentElement; if(!e)return false; e.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:1,button:0})); return true; })()").await?;
    settle().await;
    checks.insert(
        "orbat_later_squad_click_cannot_move_anchor".into(),
        json!(
            click
                && payload(page).await?["editor"]["squads"] == moved["editor"]["squads"]
                && depth(page).await? == d + 1
        ),
    );
    // Cancel a fresh ORBAT row arm, then release a different squad.
    let armed=eval_bool(page,"(() => {const row=document.querySelector('[role=button][aria-label=Rifleman]'); if(!row)return false; row.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:1,button:0,buttons:1})); window.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:1})); return true; })()").await?;
    eval(page,"(() => { const e=[...document.querySelectorAll('span')].find(e=>/^Charlie \\(/.test(e.textContent))?.parentElement; e?.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:1,button:0})); return !!e; })()").await?;
    settle().await;
    checks.insert(
        "orbat_cancel_clears_legacy_refile".into(),
        json!(armed && payload(page).await?["editor"]["squads"] == moved["editor"]["squads"]),
    );
    key_chord(page, "Escape", "Escape", 0, 27).await?;
    settle().await;
    undo(page).await?;
    checks.insert(
        "orbat_one_undo_restores_authored_structure".into(),
        json!(payload(page).await?["editor"] == before["editor"]),
    );
    Ok(())
}

async fn mixed_orbat_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
    for query in ["", "Visible"] {
        select_five(page).await?;
        let before = payload(page).await?;
        let d = depth(page).await?;
        click_selector(page, "[aria-label='ORBAT Manager']").await?;
        settle().await;
        eval(page,&format!("(() => {{const e=document.querySelector('input[placeholder=\"Search entities...\"]');e.focus();e.value={};e.dispatchEvent(new Event('input',{{bubbles:true}}));}})()",json!(query))).await?;
        settle().await;
        let input_focused=eval_bool(page,"(() => {const e=document.querySelector('input[placeholder=\"Search entities...\"]');e.focus();return document.activeElement===e;})()").await?;
        let points=eval(page,"(() => {const a=document.querySelector('[role=button][aria-label=\"Visible Rifleman\"]');const b=[...document.querySelectorAll('span')].find(e=>e.textContent.startsWith('Visible Bravo ('))?.parentElement;return [a,b].map(e=>{if(!e)return null;const r=e.getBoundingClientRect();const x=r.left+r.width/2,y=r.top+r.height/2;return {x,y,hit:e.contains(document.elementFromPoint(x,y))};});})()").await?;
        let reachable = points
            .as_array()
            .is_some_and(|a| a.iter().all(|p| p["hit"] == true));
        eval(page,"window.__mixedEvents=[];for(const type of ['pointerdown','pointermove','pointerup','pointercancel','dragstart','dragend','blur','focus'])window.addEventListener(type,e=>{window.__mixedEvents.push({type,id:e.pointerId,target:e.target.outerHTML?.slice(0,180),phase:e.eventPhase,trusted:e.isTrusted,active:document.activeElement?.outerHTML?.slice(0,180)});if(window.__mixedEvents.length>24)window.__mixedEvents.shift();},true)").await?;
        if reachable {
            drag(
                page,
                points[0]["x"].as_f64().unwrap(),
                points[0]["y"].as_f64().unwrap(),
                points[1]["x"].as_f64().unwrap(),
                points[1]["y"].as_f64().unwrap(),
            )
            .await?;
        }
        settle().await;
        let after = payload(page).await?;
        let moved = after["editor"]["squads"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["id"] == "bravo"))
            .and_then(|s| s["slotIds"].as_array())
            .map_or(0, Vec::len);
        let name = if query.is_empty() {
            "mixed_faction"
        } else {
            "search_hidden"
        };
        checks.insert(
            format!("orbat_{name}_moves_five"),
            json!(input_focused && reachable && moved == 5 && depth(page).await? == d + 1),
        );
        eprintln!(
            "t946-86 {name}: {}",
            json!({"points":points,"moved":moved,"before":before["editor"]["squads"],"after":after["editor"]["squads"],"events":eval(page,"window.__mixedEvents").await?})
        );
        key_chord(page, "Escape", "Escape", 0, 27).await?;
        settle().await;
        undo(page).await?;
        checks.insert(
            format!("orbat_{name}_one_undo_restores_all_structure"),
            json!(payload(page).await?["editor"] == before["editor"] && depth(page).await? == d),
        );
    }
    Ok(())
}

async fn orbat_cancel_cases(h: &Harness, checks: &mut Map<String, Value>) -> Result<()> {
    // Positive control and cancellations use the same trusted row press and direct squad
    // release. No outside release or modal close can independently clear the latch in between.
    for mode in [
        "positive_control",
        "window_targeted_blur_cleanup",
        "pointercancel",
    ] {
        let page = &h.page;
        select_five(page).await?;
        let before = payload(page).await?;
        let d = depth(page).await?;
        click_selector(page, "[aria-label='ORBAT Manager']").await?;
        settle().await;
        eval(page,r#"window.__r86BlurSeen=null;window.addEventListener('blur',e=>{if(e.target===window)window.__r86BlurSeen={targetWindow:e.target===window,currentWindow:e.currentTarget===window,phase:e.eventPhase,bubbles:e.bubbles,cancelable:e.cancelable,trusted:e.isTrusted};});document.querySelector('input[placeholder="Search entities..."]').focus()"#).await?;
        let points=eval(page,"(() => {const a=document.querySelector('[role=button][aria-label=Rifleman]');const b=[...document.querySelectorAll('span')].find(e=>e.textContent.startsWith('Charlie ('))?.parentElement;return [a,b].map(e=>{const r=e.getBoundingClientRect();const x=r.left+r.width/2,y=r.top+r.height/2;return {x,y,hit:e.contains(document.elementFromPoint(x,y))};});})()").await?;
        anyhow::ensure!(
            points
                .as_array()
                .is_some_and(|a| a.iter().all(|p| p["hit"] == true)),
            "ORBAT cancellation targets obscured"
        );
        mouse(
            page,
            "mousePressed",
            points[0]["x"].as_f64().unwrap(),
            points[0]["y"].as_f64().unwrap(),
            json!({"button":"left","buttons":1,"clickCount":1}),
        )
        .await?;
        let mut event_valid = true;
        if mode == "window_targeted_blur_cleanup" {
            // Explicitly synthetic Window-targeted event. The production closure ignores its
            // Event argument; headless tab activation does not reliably emit native window blur.
            eval(page,"window.dispatchEvent(new FocusEvent('blur',{bubbles:false,cancelable:false,relatedTarget:null}))").await?;
            event_valid = eval_bool(page,"(() => {const e=window.__r86BlurSeen;return e?.targetWindow && e.currentWindow && e.phase===Event.AT_TARGET && !e.bubbles && !e.cancelable && !e.trusted;})()").await?;
        } else if mode == "pointercancel" {
            eval(page,"window.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:1}))").await?;
        }
        // The FIRST event after cancellation is the valid destination release. Its absence
        // of a refile proves the synchronous production cleanup consumed the armed state.
        mouse(
            page,
            "mouseReleased",
            points[1]["x"].as_f64().unwrap(),
            points[1]["y"].as_f64().unwrap(),
            json!({"button":"left","buttons":0,"clickCount":1}),
        )
        .await?;
        settle().await;
        let after = payload(page).await?;
        let direct_inert = after["editor"] == before["editor"] && depth(page).await? == d;
        let moved = after["editor"]["squads"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "charlie")
            .unwrap()["slotIds"]
            .as_array()
            .unwrap()
            .len();
        if mode == "positive_control" {
            checks.insert(
                "orbat_cancel_probe_positive_control_moves_five".into(),
                json!(moved == 5 && depth(page).await? == d + 1),
            );
        } else {
            mouse(
                page,
                "mouseReleased",
                points[1]["x"].as_f64().unwrap(),
                points[1]["y"].as_f64().unwrap(),
                json!({"button":"left","buttons":0,"clickCount":1}),
            )
            .await?;
            settle().await;
            checks.insert(
                format!("orbat_{mode}_trusted_arm_releases_inert"),
                json!(
                    event_valid
                        && direct_inert
                        && payload(page).await?["editor"] == before["editor"]
                        && depth(page).await? == d
                ),
            );
        }
        eprintln!(
            "t946-86 ORBAT {mode}: {}",
            json!({"blur_observer":eval(page,"window.__r86BlurSeen").await?,"direct_release_inert":direct_inert,"moved":moved,"depth":depth(page).await?})
        );
        key_chord(page, "Escape", "Escape", 0, 27).await?;
        settle().await;
        if depth(page).await? > d {
            undo(page).await?;
        }
    }
    Ok(())
}

pub(super) async fn run(dist: &str) -> Result<u8> {
    let h = Harness::new(
        dist,
        5396,
        9496,
        Some(repo_root().join("packages/map-assets")),
        None,
        &[],
    )
    .await?;
    let run = async {
        let posts = intercept(&h.page).await?;
        h.page.navigate(&h.url(&format!("/missions/{ID}/edit?force=webgl&sat=preview"))).await?;
        fixture_ready(&h.page,&format!("{SEL_READY} && {HIST_READY} && typeof window.__editorCommands === 'object' && window.__missionDoc.slot_count() === 5")).await?;
        eval(&h.page,"window.__t94686Events=[]; for(const type of ['pointerdown','pointermove','pointerup','gotpointercapture','lostpointercapture']) window.addEventListener(type,e=>{window.__t94686Events.push({type,id:e.pointerId,buttons:e.buttons,x:e.clientX,y:e.clientY,target:e.target.tagName,capture:e.target.hasPointerCapture?.(e.pointerId)});if(window.__t94686Events.length>12)window.__t94686Events.shift();},true)").await?;
        settle().await;
        let mut checks = Map::new();
        checks.insert("document_focused_and_boot_settled".into(),json!(eval_bool(&h.page,"document.hasFocus() && !document.querySelector('.mc-load-fill')").await?));
        select_five(&h.page).await?;
        let (vx,vy)=vehicle_point(&h.page).await?;
        click_at(&h.page,vx,vy,true).await?;
        settle().await;
        checks.insert("mixed_slots_vehicle_selection".into(),json!(eval_i64(&h.page,"window.__editorSelection.count()").await?==6));
        let initial = payload(&h.page).await?;
        let d0 = depth(&h.page).await?;
        let (x,cy) = widget(&h.page).await?;
        let y=cy-45.0;
        z_start(&h.page,x,y).await?;
        let chip = eval_str(&h.page,"document.querySelector('[data-transform-widget] text')?.textContent || ''").await?;
        checks.insert("z_preview_authored_height".into(),json!(chip.ends_with(" m") && chip.trim_end_matches(" m").parse::<f64>().unwrap_or(0.0)>50.0));
        checks.insert("z_preview_no_write".into(),json!(same_positions(&initial,&payload(&h.page).await?)));
        mouse(&h.page,"mouseReleased",x,y-12.0,json!({"button":"left","buttons":0,"clickCount":1})).await?;
        settle().await;
        let raised=payload(&h.page).await?;
        let delta=raised["editor"]["slots"][0]["position"]["z"].as_f64().unwrap()-initial["editor"]["slots"][0]["position"]["z"].as_f64().unwrap();
        let all = raised["editor"]["slots"].as_array().unwrap().iter().all(|s| {
            let old=initial["editor"]["slots"].as_array().unwrap().iter().find(|o|o["id"]==s["id"]).unwrap();
            (s["position"]["z"].as_f64().unwrap()-old["position"]["z"].as_f64().unwrap()-delta).abs()<1e-9
        });
        checks.insert("z_mixed_heights_preserved".into(),json!(delta>0.0 && all));
        let vehicle_z=|v:&Value|v["vehicles"].as_array().unwrap().iter().find(|r|r["id"]=="vehicle-roof").unwrap()["position"]["z"].as_f64().unwrap();
        checks.insert("mixed_vehicle_has_same_z_delta".into(),json!((vehicle_z(&raised)-vehicle_z(&initial)-delta).abs()<1e-9));
        checks.insert("z_single_undo".into(),json!(depth(&h.page).await?==d0+1));
        checks.insert("z_capture_released".into(),json!(eval_bool(&h.page,"!document.querySelector('canvas').parentElement.hasPointerCapture(1)").await?));
        undo(&h.page).await?;
        checks.insert("z_undo_restores".into(),json!(same_positions(&initial,&payload(&h.page).await?)));

        vehicle_snap_cases(&h.page,&mut checks).await?;

        // Cancel a REAL captured pointer, then move/release another id and the original id.
        select_five(&h.page).await?;
        let before=payload(&h.page).await?;
        let before_depth=depth(&h.page).await?;
        let (x,cy)=widget(&h.page).await?;
        z_start(&h.page,x,cy-45.0).await?;
        eval(&h.page,"(() => { const c=document.querySelector('canvas').parentElement; c.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:1})); for(const pointerId of [99,1]) {c.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,pointerId,clientX:700,clientY:300,buttons:1})); c.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId,clientX:700,clientY:300,button:0}));} return true; })()").await?;
        mouse(&h.page,"mouseReleased",x,cy-57.0,json!({"button":"left","buttons":0,"clickCount":1})).await?;
        settle().await;
        checks.insert("cancel_then_unrelated_release_never_commits".into(),json!(same_positions(&before,&payload(&h.page).await?) && depth(&h.page).await?==before_depth));
        checks.insert("cancel_clears_preview_and_capture".into(),json!(eval_bool(&h.page,"!document.querySelector('[data-transform-widget] text') && !document.querySelector('canvas').parentElement.hasPointerCapture(1)").await?));
        z_lifecycle_cases(&h.page,&mut checks).await?;

        // A press on an already selected entity center must remain an ordinary XY drag.
        let probe=eval(&h.page,"JSON.parse(window.__editorSelection.probe())").await?;
        let (px,py)=(probe["hit"][0].as_f64().unwrap(),probe["hit"][1].as_f64().unwrap());
        click_at(&h.page,px,py,false).await?;
        settle().await;
        let xy_before=payload(&h.page).await?;
        drag(&h.page,px,py,px+24.0,py).await?;
        settle().await;
        checks.insert("selected_center_xy_drag_survives".into(),json!(!same_positions(&xy_before,&payload(&h.page).await?)));
        undo(&h.page).await?;

        // Five selected layer rows go through the production pending set and one batch.
        select_five(&h.page).await?;
        let before=payload(&h.page).await?;
        let d=depth(&h.page).await?;
        let dropped=row_drag(&h.page,"document.querySelector('aside button[aria-label=Rifleman]')","document.querySelector('aside button[aria-label=\"Recovery destination\"]')").await?;
        settle().await;
        let after=payload(&h.page).await?;
        let moved=after["editor"]["editorLayers"].as_array().and_then(|a|a.iter().find(|r|r["id"]=="destination")).and_then(|r|r["entityIds"].as_array()).map_or(0,Vec::len);
        checks.insert("five_row_drop_one_undo".into(),json!(dropped && moved==5 && depth(&h.page).await?==d+1));
        undo(&h.page).await?;
        checks.insert("five_row_undo_restores_all".into(),json!(payload(&h.page).await?["editor"]["editorLayers"]==before["editor"]["editorLayers"]));

        outside_drop_cases(&h.page,&mut checks,"eager").await?;
        orbat_cases(&h.page,&mut checks).await?;
        orbat_cancel_cases(&h,&mut checks).await?;

        // UI trigger → canvas vertices → finish → canvas pick → keyboard delete.
        click_selector(&h.page,"[aria-label='Zones']").await?;
        settle().await;
        let armed=click_selector(&h.page,"[data-testid='tactical-draw-arm']").await?;
        click_at(&h.page,700.0,550.0,false).await?;
        click_at(&h.page,800.0,550.0,false).await?;
        eval(&h.page,"(() => {const b=[...document.querySelectorAll('[data-testid=tactical-draw-draft] button')].find(b=>b.textContent==='Finish'); if(!b)return false;const r=b.getBoundingClientRect();if(!b.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2)))return false;b.click();return true;})()").await?;
        settle().await;
        let tg=payload(&h.page).await?;
        let count=tg["environment"]["tacticalGraphics"].as_array().map_or(0,Vec::len);
        let tg_depth=depth(&h.page).await?;
        mouse(&h.page,"mousePressed",700.0,550.0,json!({"button":"left","buttons":1,"clickCount":1})).await?;
        mouse(&h.page,"mouseMoved",680.0,520.0,json!({"button":"left","buttons":1})).await?;
        eval(&h.page,"(() => {const c=document.querySelector('canvas'); c.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:1})); c.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:99,clientX:600,clientY:400,button:0})); return true;})()").await?;
        mouse(&h.page,"mouseReleased",680.0,520.0,json!({"button":"left","buttons":0,"clickCount":1})).await?;
        settle().await;
        checks.insert("tactical_vertex_cancel_never_commits".into(),json!(count==1 && payload(&h.page).await?["environment"]["tacticalGraphics"]==tg["environment"]["tacticalGraphics"] && depth(&h.page).await?==tg_depth));
        click_at(&h.page,750.0,550.0,false).await?;
        key_chord(&h.page,"Delete","Delete",0,46).await?;
        settle().await;
        let deleted=payload(&h.page).await?["environment"]["tacticalGraphics"].as_array().map_or(0,Vec::len)==0;
        checks.insert("tactical_ui_draw_pick_delete".into(),json!(armed && count==1 && deleted));

        // Undo deletion, then undo the creation: a stale tactical selection must not consume
        // Delete ahead of the currently selected ordinary slot.
        undo(&h.page).await?;
        click_at(&h.page,750.0,550.0,false).await?;
        undo(&h.page).await?;
        click_selector(&h.page,"aside button[aria-label=Rifleman]").await?;
        key_chord(&h.page,"Delete","Delete",0,46).await?;
        settle().await;
        checks.insert("stale_tactical_selection_does_not_eat_delete".into(),json!(eval_i64(&h.page,"window.__missionDoc.slot_count()").await?==4));

        eprintln!("t946-86 primary fixture: {}",json!({"checks":checks,"preview":chip,"z_delta":delta,"five_moved":moved,"tactical_count":count}));

        h.page.navigate(&h.url(&format!("/missions/{MIXED_ID}/edit?force=webgl&sat=preview"))).await?;
        fixture_ready(&h.page,&format!("{SEL_READY} && {HIST_READY} && window.__missionDoc.slot_count()===5")).await?;
        mixed_orbat_cases(&h.page,&mut checks).await?;

        h.page.navigate(&h.url(&format!("/missions/{DUP_ID}/edit?force=webgl&sat=preview"))).await?;
        fixture_ready(&h.page,&format!("{SEL_READY} && typeof window.__editorCommands === 'object' && window.__missionDoc.slot_count()===5")).await?;
        click_selector(&h.page,"button[title=\"Save an immutable version of this mission\"]").await?;
        settle().await;
        eval(&h.page,"(() => {const b=[...document.querySelectorAll('button')].find(b=>b.textContent?.trim()==='Save'); b?.click(); return !!b;})()").await?;
        settle().await;
        let text=eval_str(&h.page,"document.body.textContent").await?;
        checks.insert("duplicate_save_refused_before_request".into(),json!(*posts.lock().unwrap()==0 && text.contains("Alpha") && text.contains("roof-0") && text.contains("more than once")));
        eprintln!("t946-86 duplicate fixture: {}",json!({"checks":checks,"posts":*posts.lock().unwrap(),"text":text}));

        h.page.navigate(&h.url(&format!("/missions/{LARGE_ID}/edit?force=webgl&sat=preview"))).await?;
        fixture_ready(&h.page,&format!("{SEL_READY} && window.__missionDoc.slot_count()===5 && !!document.querySelector('[data-testid=outliner-window-scroller]')")).await?;
        settle().await;
        outside_drop_cases(&h.page,&mut checks,"windowed").await?;
        eprintln!("t946-86 windowed fixture: {}",json!({"checks":checks}));

        checks.insert("no_browser_panics".into(),json!(h.no_panics()));
        let pass=checks.values().all(|v|v==true);
        println!("{}",json!({"smoke":"t946-86","pass":pass,"checks":checks,"panics":h.panics_head(),"z_delta":delta,"preview":chip,"five_moved":moved,"tactical_count":count}));
        Ok::<u8,anyhow::Error>(if pass {0} else {1})
    }.await;
    h.shutdown().await;
    run
}
