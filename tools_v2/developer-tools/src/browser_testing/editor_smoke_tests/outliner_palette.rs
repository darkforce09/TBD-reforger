use super::*;

/// Dock gate (P1/O1/O2/D1/D2/D3/W1).
/// MUST NOT call probe() before the D2 read (it would re-centre the camera).
pub async fn smoke_outliner_palette(dist: &str, path: &str) -> Result<u8> {
    const RIFLEMAN_LABEL: &str = "[aria-label=\"US Rifleman\"]";
    let expect_x_bits = 6320.0f32.to_bits();
    let expect_y_bits = 6200.0f32.to_bits();
    let h = Harness::new(dist, 5309, 9369, None, None, &[]).await?;
    let run = async {
        let hits = serve_registry_golden(&h.page).await?;
        let url = h.url(path);
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
        let slot_count = || async { eval_i64(&h.page, "window.__missionDoc.slot_count()").await };
        let edit_count =
            || async { eval_i64(&h.page, "window.__missionPersist.edit_persist_count()").await };
        let cam = || async {
            Ok::<Value, anyhow::Error>(
                serde_json::from_str(&eval_str(&h.page, "window.__editorCam()").await?)
                    .unwrap_or(Value::Null),
            )
        };
        let dock_text = || async {
            eval_str(&h.page, "(() => [...document.querySelectorAll('aside')].map((e) => e.textContent || '').join('|'))()").await
        };
        let obj_text = || async {
            eval(&h.page, "(() => { const e = document.querySelector('[title^=\"Placed slots\"]');\n    const m = (e?.textContent || '').match(/OBJ\\s*(\\d+)/); return m ? m[1] : null; })()").await
        };
        // slots_digest rows: id|x_bits|y_bits|z_bits|rot_bits|stance|role|tag|squad|layer
        let row_of = |d: &str, id: &str| -> Option<Vec<String>> {
            d.split('\n')
                .map(|r| r.split('|').map(str::to_string).collect::<Vec<_>>())
                .find(|c| c.first().map(String::as_str) == Some(id))
        };

        // boot 0 + reset; boot 1 COLD.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;
        let ready = boot_to(format!("{SEL_READY} && {PERSIST_READY} && {DOC_READY}")).await?;
        // Palette folders below depth 0 boot collapsed (`default_expanded` rule 3:
        // only faction roots open). Expand US_Army before waiting on its leaves.
        let us_army = h
            .page
            .wait_for(
                "!!document.querySelector('aside [aria-label=\"US_Army\"]')",
                200,
                250,
            )
            .await?;
        if us_army {
            h.page
                .evaluate(
                    "document.querySelector('aside [aria-label=\"US_Army\"]').click()",
                    true,
                )
                .await?;
        }
        let palette_ready = h
            .page
            .wait_for(
                &format!("!!document.querySelector('{RIFLEMAN_LABEL}')"),
                200,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let (mut count0, mut count1) = (-1i64, -1i64);
        let (mut obj0, mut obj1) = (Value::Null, Value::Null);
        let (mut ec0, mut ec1) = (-1i64, -1i64);
        let mut first_row_ids = Value::Null;
        let mut placed_row: Option<Vec<String>> = None;
        let (mut cam_before, mut cam_dock, mut cam_canvas) =
            (Value::Null, Value::Null, Value::Null);

        if ready && palette_ready {
            // P1 — the palette tree from the golden.
            let docks0 = dock_text().await?;
            checks.insert(
                "p1_paletteTree".into(),
                // Factions tab is icon-only (`aria-label`); tree text still has NATO/US_Army.
                json!(
                    eval_bool(
                        &h.page,
                        "!!document.querySelector('[aria-label=\"Factions\"]')"
                    )
                    .await?
                        && docks0.contains("NATO")
                        && docks0.contains("US_Army")
                ),
            );
            checks.insert(
                "p1_eightLeaves".into(),
                json!(
                    eval_i64(
                        &h.page,
                        "document.querySelectorAll('aside [aria-label^=\"US \"]').length"
                    )
                    .await?
                        == 8
                ),
            );
            checks.insert(
                "p1_registryFetched".into(),
                json!(*hits.lock().unwrap() >= 1),
            );

            // O1 — the seed's 8 slots are listed, unfiled.
            count0 = slot_count().await?;
            checks.insert(
                "o1_unfiledRoot".into(),
                json!(docks0.contains("Unfiled (10)") && count0 == 8),
            );

            // Guide click toggles expand/collapse. Slot rows carry
            // `data-guide-toggle` for Unfiled; after collapse Unfiled is depth-0 (no guide), so
            // re-expand via the chevron (`aria-expanded=false`) before later o2 row-select.
            let guide_ok = eval_bool(
                &h.page,
                "(() => { const left = [...document.querySelectorAll('aside')].find((a) => (a.textContent || '').includes('Layers') && (a.textContent || '').includes('Locations')); const g = left && left.querySelector('[data-guide-toggle]'); if (!g) return false; g.dispatchEvent(new MouseEvent('click', { bubbles: true })); return true; })()",
            )
            .await?;
            let collapsed = h
                .page
                .wait_for(
                    "(() => { const left = [...document.querySelectorAll('aside')].find((a) => (a.textContent || '').includes('Layers') && (a.textContent || '').includes('Locations')); return !!left && left.querySelectorAll('[aria-label=\"Rifleman\"]').length === 0; })()",
                    40,
                    50,
                )
                .await?;
            let _ = eval_bool(
                &h.page,
                "(() => { const left = [...document.querySelectorAll('aside')].find((a) => (a.textContent || '').includes('Layers') && (a.textContent || '').includes('Locations')); const chev = left && left.querySelector('[aria-expanded=\"false\"]'); if (chev) chev.dispatchEvent(new MouseEvent('click', { bubbles: true })); return !!chev; })()",
            )
            .await?;
            let restored = h
                .page
                .wait_for(
                    "(() => { const left = [...document.querySelectorAll('aside')].find((a) => (a.textContent || '').includes('Layers') && (a.textContent || '').includes('Locations')); return !!left && left.querySelectorAll('[aria-label=\"Rifleman\"]').length >= 8; })()",
                    40,
                    50,
                )
                .await?;
            checks.insert(
                "a4_guideToggle".into(),
                json!(guide_ok && collapsed && restored),
            );

            // O2 — clicking the first Unfiled row selects exactly s0.
            click_selector(&h.page, "aside [aria-label=\"Rifleman\"]").await?;
            first_row_ids = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
            checks.insert(
                "o2_rowSelectsS0".into(),
                json!(
                    first_row_ids
                        .as_array()
                        .map(|a| a.len() == 1 && a[0] == json!("s0"))
                        == Some(true)
                ),
            );

            // D1/D2/D3 — drag the palette leaf onto the canvas at (700, 500).
            let d0 = digest().await?;
            ec0 = edit_count().await?;
            obj0 = obj_text().await?;
            if let Some((lx, ly)) = rect_of(&h.page, RIFLEMAN_LABEL).await? {
                drag(&h.page, lx, ly, 700.0, 500.0).await?;
                count1 = slot_count().await?;
                let d1 = digest().await?;
                ec1 = edit_count().await?;
                obj1 = obj_text().await?;
                let docks1 = dock_text().await?;

                checks.insert("d1_slotAdded".into(), json!(count0 == 8 && count1 == 9));
                checks.insert(
                    "d1_objReadout".into(),
                    json!(obj0 == json!("8") && obj1 == json!("9")),
                );
                checks.insert("d1_digestChanged".into(), json!(!d0.is_empty() && d1 != d0));
                checks.insert("d1_persistArmed".into(), json!(ec1 > ec0));

                // The placed slot is the one row present in d1 but not d0.
                let ids0: std::collections::HashSet<&str> = d0
                    .split('\n')
                    .map(|r| r.split('|').next().unwrap_or(""))
                    .collect();
                let new_id = d1
                    .split('\n')
                    .map(|r| r.split('|').next().unwrap_or(""))
                    .find(|id| !ids0.contains(id))
                    .map(str::to_string);
                placed_row = new_id.as_deref().and_then(|id| row_of(&d1, id));
                let bits = |row: &Option<Vec<String>>, i: usize| {
                    row.as_ref()
                        .and_then(|r| r.get(i))
                        .and_then(|v| v.parse::<u32>().ok())
                };
                checks.insert(
                    "d2_positionBitExact".into(),
                    json!(
                        placed_row.is_some()
                            && bits(&placed_row, 1) == Some(expect_x_bits)
                            && bits(&placed_row, 2) == Some(expect_y_bits)
                    ),
                );
                checks.insert(
                    "d2_roleFromPalette".into(),
                    json!(
                        placed_row
                            .as_ref()
                            .and_then(|r| r.get(6))
                            .map(String::as_str)
                            == Some("US Rifleman")
                    ),
                );
                checks.insert(
                    "d3_filedInDefaultLayer".into(),
                    json!(
                        placed_row
                            .as_ref()
                            .and_then(|r| r.get(9))
                            .map(String::as_str)
                            == Some("layer-1")
                    ),
                );
                checks.insert(
                    "d3_layerInOutliner".into(),
                    json!(docks1.contains("Layer 1") && docks1.contains("Unfiled (10)")),
                );

                // O3/O4/O5 — the place minted a default squad. The ORBAT tree
                // moved from the left dock into the top-strip ORBAT Manager modal, so open it, then
                // assert the squad shows, its slot leaf selects, and dbl-click opens Attributes
                // (SEL-ORBAT-DBL-001). Keep the modal open through o5.
                eval(
                    &h.page,
                    "document.querySelector('[aria-label=\"ORBAT Manager\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true}))",
                )
                .await?;
                // The modal's title h2 marks it mounted (avoids a read/leaf-lookup race).
                const ORBAT_MODAL_OPEN: &str = "[...document.querySelectorAll('h2')].some(h => h.textContent === 'ORBAT Manager')";
                h.page.wait_for(ORBAT_MODAL_OPEN, 40, 250).await?;
                // The popup carries `glass`; scope the text read + leaf lookup to it.
                let orbat_popup_text = eval_str(
                    &h.page,
                    "(() => { const h=[...document.querySelectorAll('h2')].find(h=>h.textContent==='ORBAT Manager'); const p=h&&h.closest('.glass'); return p?(p.textContent||''):''; })()",
                )
                .await?;
                checks.insert(
                    "o3_orbatSquadMinted".into(),
                    json!(
                        // Place mints faction-BLUFOR / "Squad N"; side tabs show BLUFOR.
                        orbat_popup_text.contains("Squad 1") && orbat_popup_text.contains("BLUFOR")
                    ),
                );
                // The ORBAT slot leaf = role aria-label inside the ORBAT Manager popup (div[role=button] OK).
                const ORBAT_LEAF: &str = "(() => { const h=[...document.querySelectorAll('h2')].find(h=>h.textContent==='ORBAT Manager'); const p=h&&h.closest('.glass'); return p?p.querySelector('[aria-label=\"US Rifleman\"]'):null; })()";
                eval(
                    &h.page,
                    &format!(
                        "{ORBAT_LEAF}?.dispatchEvent(new MouseEvent('click',{{bubbles:true}}))"
                    ),
                )
                .await?;
                let orbat_sel = eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await?;
                checks.insert(
                    "o4_orbatLeafSelects".into(),
                    json!(
                        orbat_sel
                            .as_array()
                            .map(|a| a.len() == 1 && a[0].as_str() == new_id.as_deref())
                            == Some(true)
                    ),
                );
                eval(
                    &h.page,
                    &format!(
                        "{ORBAT_LEAF}?.dispatchEvent(new MouseEvent('dblclick',{{bubbles:true}}))"
                    ),
                )
                .await?;
                // MODAL_OPEN matches the Attributes h2 exactly, so the ORBAT Manager modal (h2
                // "ORBAT Manager") never false-triggers this.
                checks.insert(
                    "o5_orbatDblAttributes".into(),
                    json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
                );
                // One Escape closes BOTH modals (Attributes + ui::Dialog each own a window-level Esc
                // listener); SETTLE until neither is present so neither `fixed inset-0 z-50` backdrop
                // covers the 700,500 canvas wheel point in W1.
                key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
                h.page
                    .wait_for(
                        "![...document.querySelectorAll('h2')].some(h => h.textContent === 'Attributes' || h.textContent === 'ORBAT Manager')",
                        40,
                        250,
                    )
                    .await?;
            }

            // W1 — wheel over a dock must not zoom; over the canvas it must.
            cam_before = cam().await?;
            mouse(
                &h.page,
                "mouseWheel",
                120.0,
                500.0,
                json!({ "deltaX": 0, "deltaY": -240 }),
            )
            .await?;
            cam_dock = cam().await?;
            // CDP mouseWheel at fixed 700,500 often misses the canvas after modal
            // teardown; dispatch a WheelEvent on the canvas centre (same as smoke_editor / cur).
            eval(
                &h.page,
                "(()=>{const c=document.querySelector('canvas');if(!c)return 0;const r=c.getBoundingClientRect();c.dispatchEvent(new WheelEvent('wheel',{deltaY:-600,clientX:r.left+r.width/2,clientY:r.top+r.height/2,bubbles:true,cancelable:true}));return 1})()",
            )
            .await?;
            cam_canvas = cam().await?;
            checks.insert(
                "w1_dockWheelNoZoom".into(),
                json!(cam_dock["z"] == cam_before["z"]),
            );
            checks.insert(
                "w1_canvasWheelZooms".into(),
                json!(cam_canvas["z"] != cam_before["z"]),
            );
        } else {
            eprintln!("smoke_outliner_palette_editor: bridges/palette never appeared");
        }

        let registry_hits = *hits.lock().unwrap();
        let pass = ready0 && ready && palette_ready && h.no_panics() && checks_pass(&checks, 19);
        let placed = placed_row.as_ref().map(|r| {
            json!({
                "id": r.first(), "xBits": r.get(1).and_then(|v| v.parse::<u32>().ok()),
                "yBits": r.get(2).and_then(|v| v.parse::<u32>().ok()),
                "role": r.get(6), "layer": r.get(9),
            })
        });
        print_verdict(&json!({
            "gate": "editor-outliner-palette-smoke", "path": path,
            "ready0": ready0, "ready": ready, "paletteReady": palette_ready, "registryHits": registry_hits,
            "counts": { "slots": [count0, count1], "obj": [obj0, obj1], "editPersist": [ec0, ec1] },
            "selectedFirstRow": first_row_ids,
            "placed": placed,
            "expectedBits": { "x": expect_x_bits, "y": expect_y_bits },
            "cam": { "before": cam_before, "afterDockWheel": cam_dock, "afterCanvasWheel": cam_canvas },
            "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
