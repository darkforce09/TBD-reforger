use super::*;

pub(super) async fn vehicle_snap_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
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

pub(super) async fn outside_drop_cases(
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

pub(super) async fn z_lifecycle_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
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

pub(super) async fn orbat_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
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

pub(super) async fn mixed_orbat_cases(page: &Page, checks: &mut Map<String, Value>) -> Result<()> {
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

pub(super) async fn orbat_cancel_cases(h: &Harness, checks: &mut Map<String, Value>) -> Result<()> {
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
