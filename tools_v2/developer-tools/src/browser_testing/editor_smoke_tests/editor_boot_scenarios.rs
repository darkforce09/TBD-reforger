use super::*;

/// smoke_editor.mjs — T-159.15: canvas mounts + engine renders + wheel-zoom changes the view.
pub async fn smoke_editor(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5299, 9359, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        // Wait for engine + `__editorCam` (T-166 host bootstrap can outlast a fixed 1.2s sleep).
        h.page
            .wait_for("typeof window.__editorCam==='function'", 80, 250)
            .await?;
        cdp::sleep_ms(400).await;

        // `__editorCam()` returns a JSON *string* `{"tx","ty","z","backend"}` (not an object).
        let cam_z = "(()=>{try{const raw=window.__editorCam&&window.__editorCam();const c=typeof raw==='string'?JSON.parse(raw):raw;return String(c&&c.z!=null?c.z:NaN)}catch(e){return 'NaN'}})()";
        let z_before: f64 = eval_str(&h.page, cam_z).await?.parse().unwrap_or(f64::NAN);
        // Dispatch on the canvas (gesture host), not the first `div.relative` (chrome shell) —
        // a target outside the capture listener never reaches `zoom_at`.
        eval(
            &h.page,
            "(()=>{const c=document.querySelector('canvas');if(!c)return 0;const r=c.getBoundingClientRect();c.dispatchEvent(new WheelEvent('wheel',{deltaY:-600,clientX:r.left+r.width/2,clientY:r.top+r.height/2,bubbles:true,cancelable:true}));return 1})()",
        )
        .await?;
        cdp::sleep_ms(400).await;
        let z_after: f64 = eval_str(&h.page, cam_z).await?.parse().unwrap_or(f64::NAN);
        let info = eval_str(
            &h.page,
            "(()=>{const c=document.querySelector('canvas');return JSON.stringify({w:c?.width||0,h:c?.height||0})})()",
        )
        .await?;
        let canvas: Value = serde_json::from_str(&info).unwrap_or(json!({}));
        let changed =
            z_before.is_finite() && z_after.is_finite() && (z_after - z_before).abs() > 1e-6;
        let pass = h.no_panics() && changed;
        print_verdict(&json!({
            "gate": "editor-smoke", "path": path, "canvas": canvas,
            "zoomBefore": z_before, "zoomAfter": z_after,
            "viewChangedOnWheel": changed, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// selfcheck_editor.mjs — T-159.15.1: byte-exact GPU readback self-checks (?force=webgl).
pub async fn smoke_selfcheck(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5300, 9360, None, None, &[]).await?;
    let run = async {
        let nav = force_webgl(path);
        h.page.navigate(&h.url(&nav)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for(
                "!!(window.__selfChecks && window.__selfChecks.calibration)",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let mut backend = "unknown".to_string();
        let mut all_pass = ready;
        if !ready {
            eprintln!("selfcheck_editor: window.__selfChecks never appeared");
        } else {
            for name in ["calibration", "texture"] {
                match h
                    .page
                    .evaluate(&format!("window.__selfChecks[{name:?}]()"), true)
                    .await
                {
                    Ok(raw) => {
                        let parsed: Value = serde_json::from_str(raw.as_str().unwrap_or_default())
                            .unwrap_or(Value::Null);
                        let ok = parsed["pass"] == json!(true);
                        checks.insert(
                            name.to_string(),
                            json!({ "pass": ok, "backend": parsed["backend"] }),
                        );
                        if let Some(b) = parsed["backend"].as_str() {
                            backend = b.to_string();
                        }
                        all_pass = all_pass && ok;
                    }
                    Err(err) => {
                        checks.insert(
                            name.to_string(),
                            json!({ "pass": false, "error": err.to_string() }),
                        );
                        all_pass = false;
                    }
                }
            }
        }
        let pass = all_pass && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-selfcheck", "path": path, "backend": backend, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
