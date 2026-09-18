use super::*;

/// smoke_cur_editor.mjs — T-159.22: CUR toolbelt read-out (C0 camera + C1/C2 math + C3 em dash).
/// MUST NOT call probe() (it re-centres the camera and would invalidate the arithmetic).
pub async fn smoke_cur(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5310, 9370, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 200, 250)
            .await?;

        let cell = |axis: &'static str| {
            let page = Arc::clone(&h.page);
            async move {
                let expr = format!(
                    "(() => {{ const e = document.querySelector('[title=\"Cursor {axis}\"]');
      return e ? (e.textContent || '').replace(/^\\s*{axis}\\s*/, '').trim() : null; }})()"
                );
                page.evaluate(&expr, false).await
            }
        };
        let read = || async {
            Ok::<Value, anyhow::Error>(json!({ "x": cell("X").await?, "y": cell("Y").await? }))
        };
        let mv = |x: f64, y: f64| {
            mouse(
                &h.page,
                "mouseMoved",
                x,
                y,
                json!({ "button": "none", "buttons": 0 }),
            )
        };

        let mut checks = Map::new();
        let (mut cam, mut boot_r, mut centre, mut offset) =
            (Value::Null, Value::Null, Value::Null, Value::Null);
        if ready {
            // C3 — off-map BEFORE any pointer move.
            boot_r = read().await?;
            checks.insert(
                "c3_offMapEmDash".into(),
                json!(boot_r["x"] == json!("—") && boot_r["y"] == json!("—")),
            );
            // C0 — pin the camera this math rests on.
            cam = serde_json::from_str(&eval_str(&h.page, "window.__editorCam()").await?)
                .unwrap_or(Value::Null);
            checks.insert(
                "c0_defaultCamera".into(),
                json!(
                    cam["tx"].as_f64() == Some(6400.0)
                        && cam["ty"].as_f64() == Some(6400.0)
                        && cam["z"].as_f64() == Some(-2.0)
                ),
            );
            // C1 — the container centre is the camera target.
            mv(720.0, 450.0).await?;
            centre = read().await?;
            // T-843 / T-793 — CUR readout carries Eden's presentation-only ` m` suffix
            // (`fmt_coord_eden`); values stay metre-exact, the unit is what the smoke must pin.
            checks.insert(
                "c1_centreIsTarget".into(),
                json!(centre["x"] == json!("6400.000 m") && centre["y"] == json!("6400.000 m")),
            );
            // C2 — the offset proof (1 px = 4 m, north-up).
            mv(600.0, 300.0).await?;
            offset = read().await?;
            checks.insert(
                "c2_offsetMath".into(),
                json!(offset["x"] == json!("5920.000 m") && offset["y"] == json!("7000.000 m")),
            );
        } else {
            eprintln!("smoke_cur_editor: window.__editorCam never appeared");
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 4);
        print_verdict(&json!({
            "gate": "editor-cur-smoke", "path": path,
            "ready": ready, "backend": cam["backend"],
            "cam": cam, "readouts": { "boot": boot_r, "centre": centre, "offset": offset },
            "expected": { "centre": ["6400.000 m", "6400.000 m"], "offset": ["5920.000 m", "7000.000 m"] },
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

pub(super) async fn probe_hit(page: &Page) -> Result<(f64, f64)> {
    let probe: Value =
        serde_json::from_str(&eval_str(page, "window.__editorSelection.probe()").await?)
            .unwrap_or(Value::Null);
    Ok((
        probe["hit"][0].as_f64().unwrap_or(0.0),
        probe["hit"][1].as_f64().unwrap_or(0.0),
    ))
}

/// smoke_attributes_editor.mjs — T-159.26 Attributes modal (A1/A2t/A2i/U/A1c).
pub async fn smoke_attributes(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5311, 9371, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(ATTR_READY, 120, 250).await?;

        let mut checks = Map::new();
        if ready {
            let (hx, hy) = probe_hit(&h.page).await?;

            // A1 — trusted dbl-click on the slot.
            dbl_click(&h.page, hx, hy).await?;
            checks.insert(
                "a1_open".into(),
                json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
            );
            checks.insert(
                "a1_selected".into(),
                json!(eval_bool(&h.page, "window.__editorSelection.count() === 1").await?),
            );

            let d0 = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;

            // A2t — Transform tab → X commit via input + blur.
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Transform').click()").await?;
            checks.insert(
                "a2t_tab".into(),
                json!(
                    h.page
                        .wait_for(
                            "document.querySelectorAll('input[type=number]').length >= 4",
                            20,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                "(() => {
      const el = document.querySelectorAll('input[type=number]')[0];
      el.focus();
      el.value = '5000';
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.blur();
    })()",
            )
            .await?;
            let d1 = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
            let depth1 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            checks.insert("a2t_digestChanged".into(), json!(d1 != d0));
            checks.insert("a2t_oneUndoStep".into(), json!(depth1 == depth0 + 1));

            // U — real Ctrl+Z restores the digest.
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            let d0_json = serde_json::to_string(&d0)?;
            checks.insert(
                "u_digestRestored".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionPersist.slots_digest() === {d0_json}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // A2i — Identity tab → Role commit via input + blur (T-785: text_field commits on
            // blur/Enter, not per keystroke — a bare `input` only writes the draft).
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Identity').click()").await?;
            checks.insert(
                "a2i_tab".into(),
                json!(
                    h.page
                        .wait_for(
                            "!!document.querySelector('input[placeholder=Rifleman]')",
                            20,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                "(() => {
      const el = document.querySelector('input[placeholder=Rifleman]');
      el.focus();
      el.value = 'Marksman';
      el.dispatchEvent(new Event('input', { bubbles: true }));
      el.blur();
    })()",
            )
            .await?;
            checks.insert(
                "a2i_digestChanged".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionPersist.slots_digest() !== {d0_json}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // A1c — Esc closes.
            for ev in ["rawKeyDown", "keyUp"] {
                h.page.send(
                    "Input.dispatchKeyEvent",
                    json!({ "type": ev, "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27 }),
                ).await?;
            }
            checks.insert(
                "a1c_closed".into(),
                json!(
                    h.page
                        .wait_for(&format!("!({MODAL_OPEN})"), 20, 250)
                        .await?
                ),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 9);
        print_verdict(&json!({
            "gate": "editor-attributes-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// smoke_keyboard_settings_editor.mjs — T-159.26: Delete/undo, copy/paste, Mission Settings.
pub async fn smoke_keyboard_settings(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5316, 9376, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__missionPersist === 'object' && typeof window.__editorCam === 'function'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        if ready {
            let (hx, hy) = probe_hit(&h.page).await?;
            // Select seed 0 with a single click (sub-threshold).
            click_at(&h.page, hx, hy, false).await?;
            checks.insert(
                "selected1".into(),
                json!(
                    h.page
                        .wait_for("window.__editorSelection.count() === 1", 20, 250)
                        .await?
                ),
            );

            // K-del — Delete removes it; one undo step; Ctrl+Z restores.
            let n0 = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            key_chord(&h.page, "Delete", "Delete", 0, 46).await?;
            checks.insert(
                "delRemoved".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {}", n0 - 1),
                            20,
                            250
                        )
                        .await?
                ),
            );
            checks.insert(
                "delOneUndo".into(),
                json!(
                    eval_i64(&h.page, "window.__editorHistory.undo_depth()").await? == depth0 + 1
                ),
            );
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            checks.insert(
                "delUndoRestored".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {n0}"),
                            20,
                            250
                        )
                        .await?
                ),
            );

            // K-cv — reselect, Ctrl+C then cursor over canvas and Ctrl+V.
            click_at(&h.page, hx, hy, false).await?;
            h.page
                .wait_for("window.__editorSelection.count() === 1", 20, 250)
                .await?;
            eval(&h.page, "window.dispatchEvent(new KeyboardEvent('keydown',{key:'c',code:'KeyC',ctrlKey:true,bubbles:true}))").await?;
            mouse(&h.page, "mouseMoved", 720.0, 470.0, json!({})).await?;
            let n_before = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
            eval(&h.page, "window.dispatchEvent(new KeyboardEvent('keydown',{key:'v',code:'KeyV',ctrlKey:true,bubbles:true}))").await?;
            checks.insert(
                "pasteAdded".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!("window.__missionDoc.slot_count() === {}", n_before + 1),
                            30,
                            250
                        )
                        .await?
                ),
            );

            // S-env — open Mission Settings, change Weather, assert the compiled env changed.
            eval(
                &h.page,
                "document.querySelector('button[aria-label=\"Mission settings\"]').click()",
            )
            .await?;
            checks.insert(
                "settingsOpen".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('h2')].some(h => h.textContent === 'Mission Settings')", 30, 250).await?),
            );
            eval(&h.page, "(() => {
      const sel = [...document.querySelectorAll('select')].find(s => [...s.options].some(o => o.value === 'overcast'));
      sel.value = 'overcast';
      sel.dispatchEvent(new Event('change', { bubbles: true }));
    })()").await?;
            checks.insert(
                "weatherCommitted".into(),
                json!(h.page.wait_for("JSON.parse(window.__editorCommands.compile_save_json()).environment.weather === 'overcast'", 20, 250).await?),
            );
        }
        let pass = ready && h.no_panics() && checks_pass(&checks, 7);
        print_verdict(&json!({
            "gate": "editor-keyboard-settings-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
