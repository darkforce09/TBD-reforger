use super::*;

/// Marquee select + drag-move (?force=webgl).
pub async fn smoke_marquee_drag(dist: &str, raw_path: &str) -> Result<u8> {
    let path = force_webgl(raw_path);
    let h = Harness::new(dist, 5305, 9365, None, None, &[]).await?;
    let run = async {
        let url = h.url(&path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                page.wait_for(&ready_expr, 200, 250).await
            }
        };
        let digest = || async { eval_str(&h.page, "window.__missionPersist.slots_digest()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let sel_ids =
            || async { eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await };

        // boot 0: hard-reset IDB for a deterministic COLD start.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        // boot 1 (COLD).
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY}")).await?;

        let mut marquee_selfcheck = false;
        let mut pm = Value::Null;
        let (mut marquee_ok, mut marquee_count) = (false, -1i64);
        let mut marquee_ids = Value::Null;
        let (mut c0, mut c1) = (-1i64, -1i64);
        let mut mv = Value::Null;
        let (mut move_digest_changed, mut move_selected, mut edit_persist_fired) =
            (false, false, false);

        if ready {
            marquee_selfcheck =
                eval_bool(&h.page, "window.__editorSelection.marquee_selfcheck()").await?;

            // Marquee — drag the probe box; selection must equal the oracle's expect set.
            pm = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_marquee()").await?,
            )
            .unwrap_or(Value::Null);
            if pm["rect"].is_array() && pm["expect_ids"].is_array() {
                let r = &pm["rect"];
                drag(
                    &h.page,
                    r[0].as_f64().unwrap_or(0.0),
                    r[1].as_f64().unwrap_or(0.0),
                    r[2].as_f64().unwrap_or(0.0),
                    r[3].as_f64().unwrap_or(0.0),
                )
                .await?;
                marquee_count = eval_i64(&h.page, "window.__editorSelection.count()").await?;
                marquee_ids = sel_ids().await?;
                marquee_ok = pm["expect_count"].as_i64().unwrap_or(0) >= 1
                    && marquee_count == pm["expect_count"].as_i64().unwrap_or(-2)
                    && set_eq(&marquee_ids, &pm["expect_ids"]);
            }

            // Reset the selection to none (plain click on a guaranteed-empty px).
            let probe: Value =
                serde_json::from_str(&eval_str(&h.page, "window.__editorSelection.probe()").await?)
                    .unwrap_or(Value::Null);
            if probe["empty"].is_array() {
                click_at(
                    &h.page,
                    probe["empty"][0].as_f64().unwrap_or(0.0),
                    probe["empty"][1].as_f64().unwrap_or(0.0),
                    false,
                )
                .await?;
            }

            // Move (Class R + M5).
            let d0 = digest().await?;
            c0 = edit_count().await?;
            mv = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
            )
            .unwrap_or(Value::Null);
            if mv["id"].is_string() && mv["from"].is_array() && mv["to"].is_array() {
                drag(
                    &h.page,
                    mv["from"][0].as_f64().unwrap_or(0.0),
                    mv["from"][1].as_f64().unwrap_or(0.0),
                    mv["to"][0].as_f64().unwrap_or(0.0),
                    mv["to"][1].as_f64().unwrap_or(0.0),
                )
                .await?;
                let d1 = digest().await?;
                c1 = edit_count().await?;
                let move_ids = sel_ids().await?;
                move_digest_changed = !d0.is_empty() && !d1.is_empty() && d1 != d0;
                move_selected = move_ids.as_array().map(|a| a.contains(&mv["id"])) == Some(true);
                edit_persist_fired = c1 > c0;
            }
        } else {
            eprintln!("smoke_marquee_drag_editor: bridges never appeared");
        }

        let pass = ready0
            && ready
            && marquee_selfcheck
            && marquee_ok
            && move_digest_changed
            && move_selected
            && edit_persist_fired
            && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-marquee-drag-smoke", "path": path,
            "ready0": ready0, "ready": ready, "marqueeSelfcheck": marquee_selfcheck,
            "marquee": { "rect": pm["rect"], "expectCount": pm["expect_count"], "count": marquee_count, "ids": marquee_ids, "ok": marquee_ok },
            "move": { "id": mv["id"], "from": mv["from"], "to": mv["to"], "digestChanged": move_digest_changed, "selected": move_selected, "editPersistFired": edit_persist_fired, "c0": c0, "c1": c1 },
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Two drags, undo boundary, redo button, A7 keydown guard.
pub async fn smoke_undo(dist: &str, raw_path: &str) -> Result<u8> {
    let path = force_webgl(raw_path);
    let h = Harness::new(dist, 5308, 9368, None, None, &[]).await?;
    let run = async {
        let url = h.url(&path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                page.wait_for(&ready_expr, 200, 250).await
            }
        };
        let digest = || async { eval_str(&h.page, "window.__missionPersist.slots_digest()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let can_undo = || async { eval_bool(&h.page, "window.__editorHistory.can_undo()").await };
        let can_redo = || async { eval_bool(&h.page, "window.__editorHistory.can_redo()").await };
        let undo_depth =
            || async { eval_i64(&h.page, "window.__editorHistory.undo_depth()").await };

        // boot 0 + hard reset; boot 1 COLD.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY} && {HIST_READY}")).await?;

        let mut checks = Map::new();
        let (mut d0, mut d1, mut d2) = (String::new(), String::new(), String::new());
        let (mut du1, mut du2, mut d3) = (String::new(), String::new(), String::new());
        let (mut cu, mut cr, mut depth) = (Vec::new(), Vec::new(), Vec::new());
        let (mut c_after_drag, mut c_after_undo) = (-1i64, -1i64);
        let (mut mv, mut mv2) = (Value::Null, Value::Null);
        let mut undo_ids = Value::Null;
        let mut kd = -1i64;

        if ready {
            eval(&h.page, "window.__kd = 0; window.addEventListener('keydown', () => { window.__kd++ }, true); 'ok'").await?;
            // A0 — the INIT-origin seed is not an undo step.
            cu.push(can_undo().await?);
            cr.push(can_redo().await?);
            depth.push(undo_depth().await?);
            checks.insert("a0_seedNotUndoable".into(), json!(!cu[0] && !cr[0]));
            checks.insert("a0_depthZero".into(), json!(depth[0] == 0));

            // A6 — the chrome scaffold is mounted.
            checks.insert(
                "a6_chromeMounted".into(),
                json!(eval_bool(&h.page, "!!document.querySelector('[aria-label=\"Undo\"]') &&\n        !!document.querySelector('[aria-label=\"Redo\"]')").await?),
            );
            // Left dock is Editor Layers only (no Outliner label, no ORBAT);
            // palette has Factions; ORBAT Manager on the strip.
            // Left dock header is "Layers"+"Locations" (not "Editor Layers");
            // right dock Factions tab is icon-only (name lives on aria-label/title, not textContent).
            checks.insert(
                "a6_docksMounted".into(),
                json!(eval_bool(&h.page, "(() => { const asides = [...document.querySelectorAll('aside')].map((e) => e.textContent || ''); const left = asides.find((t) => t.includes('Layers') && t.includes('Locations')); const factionsTab = !!document.querySelector('[aria-label=\"Factions\"]'); const orbatBtn = !!document.querySelector('[aria-label=\"ORBAT Manager\"]'); return !!left && !left.includes('ORBAT') && !left.includes('Outliner') && factionsTab && orbatBtn; })()").await?),
            );

            d0 = digest().await?;
            mv = serde_json::from_str(
                &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
            )
            .unwrap_or(Value::Null);
            if mv["id"].is_string() && mv["from"].is_array() && mv["to"].is_array() {
                let dxy = |v: &Value, k: &str, i: usize| v[k][i].as_f64().unwrap_or(0.0);
                // A1 — commit a real drag-move.
                drag(
                    &h.page,
                    dxy(&mv, "from", 0),
                    dxy(&mv, "from", 1),
                    dxy(&mv, "to", 0),
                    dxy(&mv, "to", 1),
                )
                .await?;
                d1 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert(
                    "a1_moveChangedDigest".into(),
                    json!(!d0.is_empty() && d1 != d0),
                );
                checks.insert("a1_canUndoAfterMove".into(), json!(cu[1] && !cr[1]));
                checks.insert("a1_depthOne".into(), json!(depth[1] == 1));

                // A1b — a SECOND drag on the same slot.
                mv2 = serde_json::from_str(
                    &eval_str(&h.page, "window.__editorSelection.probe_move()").await?,
                )
                .unwrap_or(Value::Null);
                drag(
                    &h.page,
                    dxy(&mv2, "from", 0),
                    dxy(&mv2, "from", 1),
                    dxy(&mv2, "to", 0),
                    dxy(&mv2, "to", 1),
                )
                .await?;
                d2 = digest().await?;
                depth.push(undo_depth().await?);
                c_after_drag = edit_count().await?;
                checks.insert("a1b_move2ChangedDigest".into(), json!(d2 != d1));
                checks.insert("a1b_depthTwo".into(), json!(depth[2] == 2));

                // A2 — THE BOUNDARY: one Ctrl+Z reverts ONLY the 2nd drag.
                key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
                du1 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                undo_ids = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
                c_after_undo = edit_count().await?;
                checks.insert("a2_undoLandsOnD1".into(), json!(du1 == d1));
                checks.insert("a2_undoDidNotLandOnD0".into(), json!(du1 != d0));
                checks.insert("a2_depthOne".into(), json!(depth[3] == 1));
                checks.insert("a2_stillUndoable".into(), json!(cu[2] && cr[2]));

                // A2b — a second Ctrl+Z empties the stack.
                key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
                du2 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert("a2b_undoRestoredDigest".into(), json!(du2 == d0));
                checks.insert(
                    "a2b_stackEmptied".into(),
                    json!(!cu[3] && cr[3] && depth[4] == 0),
                );

                // A4 — undo of a move keeps the seed selected.
                checks.insert(
                    "a4_selectionKept".into(),
                    json!(undo_ids.as_array().map(|a| a.contains(&mv["id"])) == Some(true)),
                );
                // A5 — the undo re-armed the debounced IDB writer.
                checks.insert(
                    "a5_undoPersisted".into(),
                    json!(c_after_undo > c_after_drag),
                );

                // A3 — the Redo BUTTON re-applies one step.
                let redo_clicked = click_selector(&h.page, "[aria-label=\"Redo\"]").await?;
                d3 = digest().await?;
                cu.push(can_undo().await?);
                cr.push(can_redo().await?);
                depth.push(undo_depth().await?);
                checks.insert("a3_redoClicked".into(), json!(redo_clicked));
                checks.insert("a3_redoRestoredMove".into(), json!(d3 == d1));
                checks.insert(
                    "a3_oneStepBack".into(),
                    json!(cu[4] && cr[4] && depth[5] == 1),
                );

                // A7 — exactly one keydown per chord (2 chords above → 2).
                kd = eval_i64(&h.page, "window.__kd").await?;
                checks.insert("a7_oneKeydownPerChord".into(), json!(kd == 2));
            }
        } else {
            eprintln!(
                "smoke_undo_editor: window.__editorHistory / __editorSelection never appeared"
            );
        }

        let pass = ready0 && ready && h.no_panics() && checks_pass(&checks, 21);
        print_verdict(&json!({
            "gate": "editor-undo-smoke", "path": path,
            "ready0": ready0, "ready": ready,
            "moveId": mv["id"], "moveId2": mv2["id"],
            "digests": { "d0": d0, "d1": d1, "d2": d2, "du1": du1, "du2": du2, "d3": d3 },
            "canUndo": cu, "canRedo": cr, "undoDepth": depth,
            "keydownEvents": kd,
            "editPersist": { "afterDrag": c_after_drag, "afterUndo": c_after_undo },
            "undoIds": undo_ids,
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
