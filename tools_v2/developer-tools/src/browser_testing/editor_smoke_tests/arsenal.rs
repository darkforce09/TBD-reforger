use super::*;

/// smoke_arsenal_editor.mjs — T-159.27 Arsenal loadout tab (R1–R5, registry golden intercepted).
pub async fn smoke_arsenal(dist: &str, path: &str) -> Result<u8> {
    const M16A2: &str = "{3E413771E1834D2F}Prefabs/Weapons/Rifles/M16/Rifle_M16A2.et";
    let h = Harness::new(dist, 5314, 9374, None, None, &[]).await?;
    let run = async {
        let (hits, compat_hits, post_hits) = serve_arsenal_golden(&h.page, M16A2).await?;
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__missionDoc === 'object' && typeof window.__editorSelection === 'object' && typeof window.__editorHistory === 'object' && typeof window.__editorCommands === 'object' && typeof window.__missionPersist === 'object'",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        if ready {
            h.page.wait_for("(() => { try { return JSON.parse(window.__editorSelection.probe()).hit !== null } catch (e) { return false } })()", 80, 250).await?;
            let (hx, hy) = probe_hit(&h.page).await?;

            // R1 — dbl-click seed slot → modal; then the Arsenal tab.
            dbl_click(&h.page, hx, hy).await?;
            checks.insert(
                "r1_open".into(),
                json!(h.page.wait_for(MODAL_OPEN, 40, 250).await?),
            );
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Arsenal').click()").await?;

            // R2 (T-172 B10) — registry resolved → the Forge layout: 14-region rail + the item
            // list for the default-active Primary region.
            checks.insert(
                "r2_registryFetched".into(),
                json!(*hits.lock().unwrap() >= 1),
            );
            checks.insert(
                "r2_railRendered".into(),
                json!(
                    h.page
                        .wait_for(
                            "document.querySelectorAll('[data-arsenal-rail]').length >= 14",
                            40,
                            250
                        )
                        .await?
                ),
            );

            let depth0 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;

            // R3 — the Primary item list carries the golden's M16A2; click-pick it.
            let m16_json = serde_json::to_string(M16A2)?;
            checks.insert(
                "r3_m16Listed".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!(
                                "!![...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {m16_json})"
                            ),
                            40,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                &format!(
                    "[...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {m16_json})?.click()"
                ),
            )
            .await?;

            // R4 — compiled save payload carries the canonical SlotLoadoutV2.
            h.page.wait_for("JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout && s.loadout.weapons && s.loadout.weapons.length)", 40, 250).await?;
            let lo_json = eval_str(
                &h.page,
                "(() => {
        const p = JSON.parse(window.__editorCommands.compile_save_json());
        const s = (p.editor?.slots || []).find(s => s.loadout);
        return s ? JSON.stringify(s.loadout) : '';
      })()",
            )
            .await?;
            let lo: Value = serde_json::from_str(&lo_json).unwrap_or(Value::Null);
            checks.insert("r4_version2".into(), json!(lo["version"] == json!(2)));
            checks.insert(
                "r4_weaponSlot".into(),
                json!(
                    lo["weapons"][0]["slotIndex"] == json!(0)
                        && lo["weapons"][0]["slotType"] == json!("primary")
                ),
            );
            checks.insert(
                "r4_weaponIsPick".into(),
                json!(lo["weapons"][0]["weapon"] == json!(M16A2)),
            );

            // R5 — one undo step; real Ctrl+Z clears it.
            let depth1 = eval_i64(&h.page, "window.__editorHistory.undo_depth()").await?;
            checks.insert("r5_oneUndoStep".into(), json!(depth1 == depth0 + 1));
            key_chord(&h.page, "z", "KeyZ", 2, 90).await?;
            checks.insert(
                "r5_undoClears".into(),
                json!(h.page.wait_for("!JSON.parse(window.__editorCommands.compile_save_json()).editor.slots.some(s => s.loadout)", 20, 250).await?),
            );

            // R6 (T-167 compat / T-172 Forge) — R5's undo bumped `doc_tick`, which re-creates the
            // modal body and resets the tab to Identity; re-open the Arsenal tab, re-pick primary
            // from the item list, then the compat PANEL lists the edge's ACOG under OPTIC; click
            // it → saved weapons[0] carries `optic`.
            const OPTIC: &str = "{ARSENAL_OPTIC}Prefabs/Weapons/Attachments/Optic_ACOG.et";
            eval(&h.page, "[...document.querySelectorAll('button')].find(b => b.getAttribute('aria-label') === 'Arsenal').click()").await?;
            h.page
                .wait_for(
                    "document.querySelectorAll('[data-arsenal-rail]').length >= 14",
                    40,
                    250,
                )
                .await?;
            let pick_value = |val: &str| {
                format!(
                    "(() => {{ const b=[...document.querySelectorAll('[data-value]')].find(b=>b.getAttribute('data-value')==={val:?}); if(!b)return false; b.click(); return true }})()"
                )
            };
            h.page.wait_for(&pick_value(M16A2), 40, 250).await?;
            // Stay on Primary — the Forge RIGHT compat panel lists optic edges with `data-value`
            // (optic rail's left list is a different surface and can render empty under the same graph).
            eval(
                &h.page,
                r#"document.querySelector('[data-arsenal-rail="primary"]')?.click()"#,
            )
            .await?;
            checks.insert(
                "r6_compatFetched".into(),
                json!(*compat_hits.lock().unwrap() >= 1),
            );
            let optic_json = serde_json::to_string(OPTIC)?;
            checks.insert(
                "r6_opticListed".into(),
                json!(
                    h.page
                        .wait_for(
                            &format!(
                                "!![...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {optic_json})"
                            ),
                            40,
                            250
                        )
                        .await?
                ),
            );
            eval(
                &h.page,
                &format!(
                    "[...document.querySelectorAll('[data-value]')].find(b => b.getAttribute('data-value') === {optic_json})?.click()"
                ),
            )
            .await?;
            checks.insert(
                "r6_opticSaved".into(),
                json!(h.page.wait_for("(() => { const s=(JSON.parse(window.__editorCommands.compile_save_json()).editor?.slots||[]).find(s=>s.loadout); return !!(s && s.loadout.weapons && s.loadout.weapons[0] && s.loadout.weapons[0].optic) })()", 40, 250).await?),
            );

            // R7 (T-172 B10 — 3D doll) — the DollEngine canvas mounts (long wait: SwiftShader
            // create is slow headless); its window hooks report a live backend, the active-region
            // anchor projects, and a CPU pick at that anchor resolves a region. If create failed
            // (no GL at all), the SVG paper-doll fallback must be up instead — the T-154 contract.
            let doll_3d = h
                .page
                .wait_for(
                    "!!document.querySelector('[data-arsenal-doll] canvas') && typeof window.__arsenalDoll === 'object'",
                    120,
                    250,
                )
                .await?;
            if doll_3d {
                checks.insert(
                    "r7_dollBackend".into(),
                    json!(
                        h.page
                            .wait_for(
                                "typeof window.__arsenalDoll.backend() === 'string' && window.__arsenalDoll.backend().length > 0",
                                40,
                                250
                            )
                            .await?
                    ),
                );
                checks.insert(
                    "r7_dollAnchorPick".into(),
                    json!(
                        h.page
                            .wait_for(
                                "(() => { const a = window.__arsenalDoll.anchor(0); return a && a.length === 2 && window.__arsenalDoll.pick(a[0], a[1]) >= 0 })()",
                                40,
                                250
                            )
                            .await?
                    ),
                );
                checks.insert(
                    "r7_dollCallout".into(),
                    json!(
                        eval_bool(&h.page, "!!document.querySelector('[data-doll-callout]')")
                            .await?
                    ),
                );
            } else {
                // Fallback branch: SVG hotspots (the old R7) prove the fallback path works.
                checks.insert(
                    "r7_dollBackend".into(),
                    json!(
                        eval_i64(
                            &h.page,
                            "document.querySelectorAll('svg [role=\"button\"]').length"
                        )
                        .await?
                            >= 8
                    ),
                );
                checks.insert("r7_dollAnchorPick".into(), json!(true));
                checks.insert("r7_dollCallout".into(), json!(true));
                eprintln!("smoke_arsenal: DollEngine unavailable — verified the SVG fallback");
            }

            // R8 (weight) — the honest weight readout renders (contains a kg figure).
            checks.insert(
                "r8_weightReadout".into(),
                json!(eval_bool(&h.page, "[...document.querySelectorAll('p')].some(p => /\\bkg\\b/.test(p.textContent||''))").await?),
            );

            // R9 (Faction Manager) — close the modal, open the manager from the Factions dock, and a
            // create round-trips a POST to /factions.
            key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
            eval(&h.page, "document.querySelector('[aria-label=\"Manage factions\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true}))").await?;
            checks.insert(
                "r9_fmOpens".into(),
                json!(h.page.wait_for("[...document.querySelectorAll('h2')].some(h => h.textContent === 'Faction Manager')", 40, 250).await?),
            );
            eval(&h.page, "(() => { const i=document.querySelector('input[placeholder^=\"e.g.\"]'); if(i){ i.value='Smoke Bn'; i.dispatchEvent(new Event('input',{bubbles:true})); } document.querySelector('[aria-label=\"Save faction\"]')?.dispatchEvent(new MouseEvent('click',{bubbles:true})); })()").await?;
            cdp::sleep_ms(600).await; // let the POST round-trip through the Fetch tap
            checks.insert("r9_fmPost".into(), json!(*post_hits.lock().unwrap() >= 1));
        }
        let registry_hits = *hits.lock().unwrap();
        let pass = ready && h.no_panics() && checks_pass(&checks, 18);
        print_verdict(&json!({
            "gate": "editor-arsenal-smoke", "path": path, "registryHits": registry_hits,
            "compatHits": *compat_hits.lock().unwrap(), "factionPosts": *post_hits.lock().unwrap(),
            "checks": checks, "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

pub(super) fn set_eq(a: &Value, b: &Value) -> bool {
    match (a.as_array(), b.as_array()) {
        (Some(x), Some(y)) => {
            let mut xs: Vec<String> = x
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect();
            let mut ys: Vec<String> = y
                .iter()
                .map(|v| v.as_str().unwrap_or_default().to_string())
                .collect();
            xs.sort();
            ys.sort();
            xs.len() == ys.len() && xs == ys
        }
        _ => false,
    }
}
