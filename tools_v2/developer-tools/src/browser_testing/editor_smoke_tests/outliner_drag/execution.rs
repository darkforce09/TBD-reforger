use super::*;
use crate::repository_layout::MapAssetMounts;

pub(crate) async fn run(dist: &str) -> Result<u8> {
    let h = Harness::new(
        dist,
        5396,
        9496,
        Some(MapAssetMounts::from_root(&repo_root())),
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

        eprintln!("outliner-drag primary fixture: {}",json!({"checks":checks,"preview":chip,"z_delta":delta,"five_moved":moved,"tactical_count":count}));

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
        eprintln!("outliner-drag duplicate fixture: {}",json!({"checks":checks,"posts":*posts.lock().unwrap(),"text":text}));

        h.page.navigate(&h.url(&format!("/missions/{LARGE_ID}/edit?force=webgl&sat=preview"))).await?;
        fixture_ready(&h.page,&format!("{SEL_READY} && window.__missionDoc.slot_count()===5 && !!document.querySelector('[data-testid=outliner-window-scroller]')")).await?;
        settle().await;
        outside_drop_cases(&h.page,&mut checks,"windowed").await?;
        eprintln!("outliner-drag windowed fixture: {}",json!({"checks":checks}));

        checks.insert("no_browser_panics".into(),json!(h.no_panics()));
        let pass=checks.values().all(|v|v==true);
        println!("{}",json!({"smoke":"outliner-drag","pass":pass,"checks":checks,"panics":h.panics_head(),"z_delta":delta,"preview":chip,"five_moved":moved,"tactical_count":count}));
        Ok::<u8,anyhow::Error>(if pass {0} else {1})
    }.await;
    h.shutdown().await;
    run
}
