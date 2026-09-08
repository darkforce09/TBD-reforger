//! Real editor handlers over a controlled hydrated mission. Coordinates come from the rendered
//! widget; assertions read the production compile payload and history. No copied gesture model.
use super::*;

const ID: &str = "94686000-0000-4000-a000-000000000001";
const DUP_ID: &str = "94686000-0000-4000-a000-000000000002";
const PAYLOAD: &str = "JSON.parse(window.__editorCommands.compile_save_json())";

fn mission(duplicate: bool) -> Value {
    let ids: Vec<String> = (0..5).map(|i| format!("roof-{i}")).collect();
    let mut squad_ids = ids.clone();
    if duplicate { squad_ids.push(ids[0].clone()); }
    let slots: Vec<Value> = (0..5).map(|i| json!({
        "id": ids[i], "squadId":"sq", "role":"Rifleman", "tag":"", "index":i,
        "stance":"stand", "assetId":"", "position":{
            "x":6400.0 + i as f64 * 2.0, "y":6400.0, "z":50.0 + i as f64 * 10.0, "rotation":15.0
        }
    })).collect();
    let payload = json!({
        "schemaVersion":1,"map":{"terrain":"everon","bounds":[0,0,12800,12800]},
        "environment":{"time":"12:00","weather":"clear"},"loadouts":{},"objectives":[],"markers":[],
        "vehicles":[{"id":"vehicle-roof","resourceName":"Vehicle.et","position":{"x":6440.0,"y":6400.0,"z":81.5,"rotation":90.0}}],
        "editor":{
            "factions":[{"id":"f","name":"BLUFOR","side":"BLUFOR","squadIds":["sq"]}],
            "squads":[{"id":"sq","factionId":"f","name":"Alpha","callsign":"Alpha","slotIds":squad_ids,"vehicleIds":[]}],
            "editorLayers":[
                {"id":"source","name":"Recovery source","parentId":null,"entityIds":ids},
                {"id":"destination","name":"Recovery destination","parentId":null,"entityIds":[]}
            ],"slots":slots
        }
    });
    let id = if duplicate {DUP_ID} else {ID};
    json!({
        "id":id,"title":"T-946.86 regression","terrain":"everon","game_mode":"pve_coop",
        "weather":"clear","time_of_day":"12:00","max_players":32,"status":"draft",
        "author_id":"00000000000000001","author_name":"Dev","author_avatar":"", "bookmarked":false,
        "armory":[],"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z",
        "current_version":{"id":"v1","mission_id":id,"semver":"0.1.0","created_by":"00000000000000001",
            "created_at":"2026-01-01T00:00:00Z","json_payload":payload}
    })
}

async fn intercept(page: &Arc<Page>) -> Result<Arc<StdMutex<u64>>> {
    let posts = Arc::new(StdMutex::new(0));
    let me: Value = serde_json::from_str(&std::fs::read_to_string(repo_root().join("apps/website/frontend/tests/fixtures/api/GET__me.json"))?)?;
    let registry: Value = serde_json::from_str(&std::fs::read_to_string(repo_root().join("apps/website/frontend/tests/fixtures/api/GET__registry.json"))?)?;
    page.send("Fetch.enable", json!({"patterns":[{"urlPattern":"*/api/v1/*"}]})).await?;
    let mut paused = page.on_event("Fetch.requestPaused").await;
    let page = page.clone();
    let counter = posts.clone();
    tokio::spawn(async move {
        while let Some(event) = paused.recv().await {
            let Some(id) = event["requestId"].as_str() else { continue; };
            let url = event["request"]["url"].as_str().unwrap_or("");
            let method = event["request"]["method"].as_str().unwrap_or("");
            let (status, body) = if method == "POST" && url.contains("/versions") {
                *counter.lock().unwrap() += 1;
                (400, json!({"error":"smoke observed a save request"}))
            } else if url.contains("/auth/refresh") {
                (200,json!({"access_token":"recovery-access","refresh_token":"rt-seed","expires_at":"2030-01-01T00:00:00Z"}))
            } else if url.ends_with("/me") { (200,me.clone())
            } else if url.contains("/registry") { (200,registry.clone())
            } else if url.contains(DUP_ID) { (200,mission(true))
            } else if url.contains(ID) { (200,mission(false))
            } else { (200,json!({"data":[],"total":0,"limit":50,"offset":0})) };
            let _ = page.fulfill_json(id,status,&body).await;
        }
    });
    Ok(posts)
}

async fn payload(page: &Page) -> Result<Value> { eval(page, PAYLOAD).await }
async fn depth(page: &Page) -> Result<i64> { eval_i64(page,"window.__editorHistory.undo_depth()").await }
async fn settle() { cdp::sleep_ms(180).await; }
async fn undo(page: &Page) -> Result<()> {
    key_chord(page,"z","KeyZ",2,90).await?;
    settle().await;
    Ok(())
}
async fn select_five(page: &Page) -> Result<()> {
    click_selector(page,"[aria-label='No widget']").await?;
    let probe = eval(page,"JSON.parse(window.__editorSelection.probe_marquee())").await?;
    let r = &probe["rect"];
    drag(page,r[0].as_f64().unwrap(),r[1].as_f64().unwrap(),r[2].as_f64().unwrap(),r[3].as_f64().unwrap()).await?;
    settle().await;
    anyhow::ensure!(eval_i64(page,"window.__editorSelection.count()").await? == 5,"fixture must select exactly five slots");
    click_selector(page,"[aria-label='Translate widget']").await?;
    settle().await;
    Ok(())
}
async fn widget(page: &Page) -> Result<(f64,f64)> {
    let p = eval(page,"(() => { const s=document.querySelector('[data-transform-widget]'); const l=s?.querySelector('line'); if(!l) throw Error('no translate widget'); const r=s.getBoundingClientRect(); return [r.left+Number(l.getAttribute('x1')),r.top+Number(l.getAttribute('y1'))]; })()").await?;
    Ok((p[0].as_f64().unwrap(),p[1].as_f64().unwrap()))
}
async fn z_start(page: &Page, x: f64, y: f64) -> Result<()> {
    mouse(page,"mousePressed",x,y,json!({"button":"left","buttons":1,"clickCount":1})).await?;
    mouse(page,"mouseMoved",x,y-8.0,json!({"button":"none","buttons":1})).await?;
    mouse(page,"mouseMoved",x,y-12.0,json!({"button":"none","buttons":1})).await?;
    settle().await;
    Ok(())
}
fn same_positions(a: &Value,b: &Value) -> bool { a["editor"]["slots"]==b["editor"]["slots"] && a["vehicles"]==b["vehicles"] }

pub(super) async fn run(dist: &str) -> Result<u8> {
    let h = Harness::new(dist,5396,9496,None,None,&[]).await?;
    let run = async {
        let posts = intercept(&h.page).await?;
        h.page.navigate(&h.url(&format!("/missions/{ID}/edit?force=webgl&sat=preview"))).await?;
        anyhow::ensure!(h.page.wait_for(&format!("{SEL_READY} && {HIST_READY} && typeof window.__editorCommands === 'object' && window.__missionDoc.slot_count() === 5"),160,250).await?,"fixture hydration failed");
        settle().await;
        let mut checks = Map::new();
        select_five(&h.page).await?;
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
        checks.insert("z_single_undo".into(),json!(depth(&h.page).await?==d0+1));
        checks.insert("z_capture_released".into(),json!(eval_bool(&h.page,"!document.querySelector('canvas').parentElement.hasPointerCapture(1)").await?));
        undo(&h.page).await?;
        checks.insert("z_undo_restores".into(),json!(same_positions(&initial,&payload(&h.page).await?)));

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
        let dropped=eval_bool(&h.page,"(() => { const buttons=[...document.querySelectorAll('aside button')]; const source=buttons.find(b=>(b.getAttribute('aria-label')||'').includes('Recovery source')); if(source?.getAttribute('aria-expanded')==='false') source.click(); const row=[...document.querySelectorAll('aside button')].find(b=>(b.getAttribute('aria-label')||'').includes('roof-0')); const dest=buttons.find(b=>(b.getAttribute('aria-label')||'').includes('Recovery destination')); if(!row||!dest)return false; row.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:1,button:0,buttons:1})); dest.dispatchEvent(new PointerEvent('pointerup',{bubbles:true,pointerId:1,button:0})); return true; })()").await?;
        settle().await;
        let after=payload(&h.page).await?;
        let moved=after["editor"]["editorLayers"].as_array().and_then(|a|a.iter().find(|r|r["id"]=="destination")).and_then(|r|r["entityIds"].as_array()).map_or(0,Vec::len);
        checks.insert("five_row_drop_one_undo".into(),json!(dropped && moved==5 && depth(&h.page).await?==d+1));
        undo(&h.page).await?;
        checks.insert("five_row_undo_restores_all".into(),json!(payload(&h.page).await?["editor"]["editorLayers"]==before["editor"]["editorLayers"]));

        // UI trigger → canvas vertices → finish → canvas pick → keyboard delete.
        click_selector(&h.page,"[aria-label='Zones']").await?;
        settle().await;
        let armed=click_selector(&h.page,"[data-testid='tactical-draw-arm']").await?;
        click_at(&h.page,700.0,550.0,false).await?;
        click_at(&h.page,800.0,550.0,false).await?;
        eval(&h.page,"(() => {const b=[...document.querySelectorAll('[data-testid=tactical-draw-draft] button')].find(b=>b.textContent==='Finish'); b?.click(); return !!b;})()").await?;
        settle().await;
        let tg=payload(&h.page).await?;
        let count=tg["environment"]["tacticalGraphics"].as_array().map_or(0,Vec::len);
        click_at(&h.page,750.0,550.0,false).await?;
        key_chord(&h.page,"Delete","Delete",0,46).await?;
        settle().await;
        let deleted=payload(&h.page).await?["environment"]["tacticalGraphics"].as_array().map_or(0,Vec::len)==0;
        checks.insert("tactical_ui_draw_pick_delete".into(),json!(armed && count==1 && deleted));

        h.page.navigate(&h.url(&format!("/missions/{DUP_ID}/edit?force=webgl&sat=preview"))).await?;
        anyhow::ensure!(h.page.wait_for(&format!("{SEL_READY} && typeof window.__editorCommands === 'object' && window.__missionDoc.slot_count()===5"),160,250).await?,"duplicate fixture hydration failed");
        key_chord(&h.page,"s","KeyS",2,83).await?;
        settle().await;
        eval(&h.page,"(() => {const d=document.querySelector('[role=dialog]'); const b=[...(d?.querySelectorAll('button')||[])].find(b=>b.textContent?.trim()==='Save'); b?.click(); return !!b;})()").await?;
        settle().await;
        let text=eval_str(&h.page,"document.body.textContent").await?;
        checks.insert("duplicate_save_refused_before_request".into(),json!(*posts.lock().unwrap()==0 && text.contains("Alpha") && text.contains("roof-0") && text.contains("more than once")));
        checks.insert("no_browser_panics".into(),json!(h.no_panics()));
        let pass=checks.values().all(|v|v==true);
        println!("{}",json!({"smoke":"t946-86","pass":pass,"checks":checks,"panics":h.panics_head(),"z_delta":delta,"preview":chip,"five_moved":moved,"tactical_count":count}));
        Ok::<u8,anyhow::Error>(if pass {0} else {1})
    }.await;
    h.shutdown().await;
    run
}
