use super::*;

pub(super) fn mission(duplicate: bool, large: bool) -> Value {
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

pub(super) fn mixed_mission() -> Value {
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

pub(super) async fn intercept(page: &Arc<Page>) -> Result<Arc<StdMutex<u64>>> {
    // Baseline failures deliberately leave dirty history. Accept its real unload prompt so
    // the following isolated fixture can load without a pending CDP navigation.
    let mut dialogs = page.on_event("Page.javascriptDialogOpening").await;
    let dialog_page = page.clone();
    tokio::spawn(async move {
        while let Some(dialog) = dialogs.recv().await {
            eprintln!("outliner-drag dialog: {}", dialog["type"]);
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

pub(super) async fn payload(page: &Page) -> Result<Value> {
    eval(page, PAYLOAD).await
}

pub(super) async fn depth(page: &Page) -> Result<i64> {
    eval_i64(page, "window.__editorHistory.undo_depth()").await
}

pub(super) async fn settle() {
    cdp::sleep_ms(180).await;
}

pub(super) async fn drag(page: &Page, x: f64, y: f64, end_x: f64, end_y: f64) -> Result<()> {
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

pub(super) async fn row_drag(page: &Page, source: &str, destination: &str) -> Result<bool> {
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

pub(super) async fn click_selector(page: &Page, selector: &str) -> Result<bool> {
    let hit=eval_bool(page,&format!("(() => {{const e=document.querySelector({});if(!e)return false;const r=e.getBoundingClientRect();return e.contains(document.elementFromPoint(r.left+r.width/2,r.top+r.height/2));}})()",json!(selector))).await?;
    anyhow::ensure!(hit, "visible UI control is not hit-testable: {selector}");
    super::super::click_selector(page, selector).await
}

pub(super) async fn canvas_hit(page: &Page, x: f64, y: f64) -> Result<()> {
    let hit = eval_bool(
        page,
        &format!("document.elementFromPoint({x},{y}) === document.querySelector('canvas')"),
    )
    .await?;
    anyhow::ensure!(hit, "canvas point is obscured: ({x},{y})");
    Ok(())
}

pub(super) async fn click_at(page: &Page, x: f64, y: f64, ctrl: bool) -> Result<()> {
    canvas_hit(page, x, y).await?;
    super::super::click_at(page, x, y, ctrl).await
}

pub(super) async fn fixture_ready(page: &Page, expression: &str) -> Result<()> {
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

pub(super) async fn undo(page: &Page) -> Result<()> {
    key_chord(page, "z", "KeyZ", 2, 90).await?;
    settle().await;
    Ok(())
}

pub(super) async fn select_five(page: &Page) -> Result<()> {
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

pub(super) async fn widget(page: &Page) -> Result<(f64, f64)> {
    let p = eval(page,"(() => { const s=document.querySelector('[data-transform-widget]'); const l=s?.querySelector('line'); if(!l) throw Error('no translate widget'); const r=s.getBoundingClientRect(); return [r.left+Number(l.getAttribute('x1')),r.top+Number(l.getAttribute('y1'))]; })()").await?;
    Ok((p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
}

pub(super) async fn z_start(page: &Page, x: f64, y: f64) -> Result<()> {
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

pub(super) fn same_positions(a: &Value, b: &Value) -> bool {
    a["editor"]["slots"] == b["editor"]["slots"] && a["vehicles"] == b["vehicles"]
}

pub(super) async fn vehicle_point(page: &Page) -> Result<(f64, f64)> {
    let p=eval(page,"(() => {const c=JSON.parse(window.__editorCam()); const r=document.querySelector('canvas').getBoundingClientRect(); return [r.left+r.width/2+(6600-c.tx)*2**c.z,r.top+r.height/2-(6400-c.ty)*2**c.z];})()").await?;
    Ok((p[0].as_f64().unwrap(), p[1].as_f64().unwrap()))
}
