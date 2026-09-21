use super::*;

/// RMB pan + mid-pan wheel rebase via __editorCam.
pub async fn smoke_pan(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5301, 9361, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h
            .page
            .wait_for("typeof window.__editorCam === 'function'", 120, 250)
            .await?;

        let cam = || async {
            let raw = eval_str(&h.page, "window.__editorCam()").await?;
            Ok::<Value, anyhow::Error>(serde_json::from_str(&raw).unwrap_or(json!({})))
        };
        // Map pan is MMB (button 1); RMB opens the context menu (mission_editor host).
        let mmb = json!({ "button": "middle", "buttons": 4, "clickCount": 1 });
        let held = json!({ "button": "none", "buttons": 4 });

        let zero = json!({ "tx": 0, "ty": 0, "z": 0, "backend": "unknown" });
        let (mut cam0, mut cam1) = (zero.clone(), zero.clone());
        let (mut cam_b1, mut cam_b2, mut cam_b3) = (zero.clone(), zero.clone(), zero.clone());
        let (mut pan_moved, mut zoom_changed, mut pan_continued) = (false, false, false);
        let f = |v: &Value, k: &str| v[k].as_f64().unwrap_or(0.0);

        if ready {
            cam0 = cam().await?;
            // Test A: MMB drag left → target moves east.
            mouse(&h.page, "mousePressed", 720.0, 450.0, mmb.clone()).await?;
            mouse(&h.page, "mouseMoved", 620.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseMoved", 520.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseReleased", 520.0, 450.0, mmb.clone()).await?;
            cam1 = cam().await?;
            pan_moved = (f(&cam1, "tx") - f(&cam0, "tx")).abs() > 1e-6;

            // Test B: mid-pan wheel rebase — pan continues after a mid-drag zoom, no re-press.
            mouse(&h.page, "mousePressed", 720.0, 450.0, mmb.clone()).await?;
            mouse(&h.page, "mouseMoved", 680.0, 450.0, held.clone()).await?;
            cam_b1 = cam().await?;
            // Mid-pan zoom via synthetic WheelEvent on the canvas (same delivery path as
            // `smoke_editor`). CDP `mouseWheel` while MMB is held does not reliably reach the
            // container capture listener on the pinned Chrome — `zoomChanged` stays false.
            eval(
                &h.page,
                "(()=>{const c=document.querySelector('canvas');if(!c)return 0;const r=c.getBoundingClientRect();c.dispatchEvent(new WheelEvent('wheel',{deltaY:-600,clientX:r.left+r.width/2,clientY:r.top+r.height/2,bubbles:true,cancelable:true}));return 1})()",
            )
            .await?;
            cam_b2 = cam().await?;
            mouse(&h.page, "mouseMoved", 620.0, 450.0, held.clone()).await?;
            mouse(&h.page, "mouseReleased", 620.0, 450.0, mmb).await?;
            cam_b3 = cam().await?;
            zoom_changed = (f(&cam_b2, "z") - f(&cam_b1, "z")).abs() > 1e-6;
            pan_continued = (f(&cam_b3, "tx") - f(&cam_b2, "tx")).abs() > 1e-6;
        } else {
            eprintln!("smoke_pan_editor: window.__editorCam never appeared");
        }
        let pass = ready && h.no_panics() && pan_moved && zoom_changed && pan_continued;
        print_verdict(&json!({
            "gate": "editor-pan-smoke", "path": path, "backend": cam0["backend"],
            "cam0": cam0, "cam1": cam1, "camB1": cam_b1, "camB2": cam_b2, "camB3": cam_b3,
            "panMoved": pan_moved, "zoomChanged": zoom_changed, "panContinued": pan_continued,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// IDB persist across reload (COLD seed → WARM restore).
pub async fn smoke_persist(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5303, 9363, None, None, &[]).await?;
    let run = async {
        let url = h.url(path);
        let boot_to = |ready_expr: String| {
            let page = Arc::clone(&h.page);
            let url = url.clone();
            async move {
                page.navigate(&url).await?;
                page.wait_for("!!document.querySelector('canvas')", 80, 250)
                    .await?;
                let ready = page.wait_for(&ready_expr, 200, 250).await?;
                page.wait_for(DOC_READY, 120, 250).await?;
                Ok::<bool, anyhow::Error>(ready)
            }
        };

        // boot 0: reach a live editor, then hard-reset for a deterministic COLD start.
        let ready0 = boot_to(PERSIST_READY.to_string()).await?;
        h.page
            .evaluate("window.__missionPersist.clear()", true)
            .await?;

        // boot 1 (COLD): no blob → seed.
        let ready_cold = boot_to(PERSIST_READY.to_string()).await?;
        let cold_loaded = eval(&h.page, "window.__missionPersist.loaded_from_storage()").await?;
        let cold_slots = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
        let cold_doc_rt = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
        let cold_digest = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
        let encode_hex_len = eval_str(&h.page, "window.__missionDoc.encode_hex()")
            .await?
            .len();
        h.page
            .evaluate("window.__missionPersist.flush()", true)
            .await?;

        // boot 2 (WARM): blob present → SWAP restore.
        let ready_warm = boot_to(format!(
            "{PERSIST_READY} && window.__missionPersist.loaded_from_storage() === true"
        ))
        .await?;
        let warm_loaded = eval(&h.page, "window.__missionPersist.loaded_from_storage()").await?;
        let warm_slots = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;
        let warm_doc_rt = eval_bool(&h.page, "window.__missionDoc.roundtrip_ok()").await?;
        let warm_digest = eval_str(&h.page, "window.__missionPersist.slots_digest()").await?;
        let warm_json = eval_str(&h.page, "window.__missionPersist.warm()").await?;
        let warm: Value = serde_json::from_str(&warm_json).unwrap_or(Value::Null);

        let digest_match = !cold_digest.is_empty() && warm_digest == cold_digest;
        let cold_ok = ready0
            && ready_cold
            && cold_loaded == json!(false)
            && cold_slots == SEED_N
            && cold_doc_rt
            && !cold_digest.is_empty();
        let warm_ok = ready_warm
            && warm_loaded == json!(true)
            && warm_slots == SEED_N
            && warm_doc_rt
            && digest_match
            && !warm.is_null()
            && warm["missionId"] == json!("smoke")
            && warm["slotCount"] == json!(SEED_N);
        let pass = cold_ok && warm_ok && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-persist-smoke", "path": path,
            "coldLoaded": cold_loaded, "coldSlots": cold_slots, "coldDocRt": cold_doc_rt,
            "warmLoaded": warm_loaded, "warmSlots": warm_slots, "warmDocRt": warm_doc_rt,
            "digestMatch": digest_match, "digestLen": cold_digest.len(), "encodeHexLen": encode_hex_len,
            "warm": warm, "coldOk": cold_ok, "warmOk": warm_ok,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// LMB pick foundation (selfcheck + click/toggle battery).
pub async fn smoke_select(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5304, 9364, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(SEL_READY, 200, 250).await?;

        let ids = || async { eval(&h.page, "JSON.parse(window.__editorSelection.ids())").await };
        let count = || async { eval_i64(&h.page, "window.__editorSelection.count()").await };

        let mut selfcheck = false;
        let mut probe = Value::Null;
        let mut probe_ok = false;
        let (mut t1, mut t2, mut t3, mut t4) = (false, false, false, false);
        let mut sel_ids = Value::Null;
        let (mut sel_count, mut clr_count, mut on_count, mut off_count) =
            (-1i64, -1i64, -1i64, -1i64);

        if ready {
            selfcheck = eval_bool(&h.page, "window.__editorSelection.pick_selfcheck()").await?;
            probe =
                serde_json::from_str(&eval_str(&h.page, "window.__editorSelection.probe()").await?)
                    .unwrap_or(Value::Null);
            probe_ok =
                probe["id"].is_string() && probe["hit"].is_array() && probe["empty"].is_array();
            if probe_ok {
                let (hx, hy) = (
                    probe["hit"][0].as_f64().unwrap_or(0.0),
                    probe["hit"][1].as_f64().unwrap_or(0.0),
                );
                let (ex, ey) = (
                    probe["empty"][0].as_f64().unwrap_or(0.0),
                    probe["empty"][1].as_f64().unwrap_or(0.0),
                );

                click_at(&h.page, hx, hy, false).await?;
                sel_ids = ids().await?;
                sel_count = count().await?;
                t1 = sel_count == 1
                    && sel_ids
                        .as_array()
                        .map(|a| a.len() == 1 && a[0] == probe["id"])
                        == Some(true);

                click_at(&h.page, ex, ey, false).await?;
                clr_count = count().await?;
                t2 = clr_count == 0;

                click_at(&h.page, hx, hy, true).await?;
                on_count = count().await?;
                t3 = on_count == 1;

                click_at(&h.page, hx, hy, true).await?;
                off_count = count().await?;
                t4 = off_count == 0;
            }
        } else {
            eprintln!("smoke_select_editor: window.__editorSelection / __editorCam never appeared");
        }
        let pass = ready && selfcheck && probe_ok && t1 && t2 && t3 && t4 && h.no_panics();
        print_verdict(&json!({
            "gate": "editor-select-smoke", "path": path,
            "ready": ready, "selfcheck": selfcheck, "probeOk": probe_ok, "probeId": probe["id"],
            "hit": probe["hit"], "empty": probe["empty"],
            "selIds": sel_ids, "selCount": sel_count, "clrCount": clr_count,
            "onCount": on_count, "offCount": off_count,
            "t1_select": t1, "t2_clear": t2, "t3_toggleOn": t3, "t4_toggleOff": t4,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// Rust compile bridges produce the schema payloads.
pub async fn smoke_save_export(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5307, 9367, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page
            .wait_for(
                "typeof window.__editorCommands === 'object' && window.__editorCommands !== null && typeof window.__missionDoc === 'object' && window.__missionDoc !== null",
                120,
                250,
            )
            .await?;

        let mut checks = Map::new();
        let (mut save_len, mut export_len, mut slot_count) = (0usize, 0usize, -1i64);
        if ready {
            let s1 = eval_str(&h.page, "window.__editorCommands.compile_save_json()").await?;
            let s2 = eval_str(&h.page, "window.__editorCommands.compile_save_json()").await?;
            let e1 = eval_str(&h.page, "window.__editorCommands.compile_export_json()").await?;
            let e2 = eval_str(&h.page, "window.__editorCommands.compile_export_json()").await?;
            save_len = s1.len();
            export_len = e1.len();
            slot_count = eval_i64(&h.page, "window.__missionDoc.slot_count()").await?;

            // Editor slot maps iterate in hash order; sort slots by id before compare so
            // consecutive compiles are judged for content stability, not Map walk order.
            let canon = |raw: &str| -> Value {
                let mut v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
                let sort_slots = |root: &mut Value, path: &[&str]| {
                    let mut cur = root;
                    for p in path {
                        match cur {
                            Value::Object(m) => {
                                cur = m.entry((*p).to_string()).or_insert(Value::Null)
                            }
                            _ => return,
                        }
                    }
                    if let Value::Array(arr) = cur {
                        arr.sort_by(|a, b| {
                            a.get("id")
                                .and_then(|x| x.as_str())
                                .cmp(&b.get("id").and_then(|x| x.as_str()))
                        });
                    }
                };
                sort_slots(&mut v, &["editor", "slots"]);
                sort_slots(&mut v, &["payload", "editor", "slots"]);
                v
            };
            let save_same = !s1.is_empty() && canon(&s1) == canon(&s2);
            let export_same = !e1.is_empty() && canon(&e1) == canon(&e2);
            checks.insert("saveDeterministic".into(), json!(save_same));
            checks.insert("exportDeterministic".into(), json!(export_same));

            let save: Value = serde_json::from_str(&s1).unwrap_or(Value::Null);
            let exp: Value = serde_json::from_str(&e1).unwrap_or(Value::Null);
            let is_obj = |v: &Value| v.is_object();
            checks.insert("saveParsed".into(), json!(is_obj(&save)));
            checks.insert("exportParsed".into(), json!(is_obj(&exp)));
            if is_obj(&save) {
                let schema_version_re = regex::Regex::new(r#""schemaVersion":1[,}]"#).unwrap();
                checks.insert(
                    "schemaVersionInt".into(),
                    json!(save["schemaVersion"] == json!(1) && schema_version_re.is_match(&s1)),
                );
                checks.insert(
                    "terrainEveron".into(),
                    json!(save["map"]["terrain"] == json!("everon")),
                );
                checks.insert(
                    "boundsExact".into(),
                    json!(save["map"]["bounds"] == json!([0, 0, 12800, 12800])),
                );
                checks.insert("saveOmitsOrbat".into(), json!(save.get("orbat").is_none()));
                checks.insert("editorObj".into(), json!(save["editor"].is_object()));
                checks.insert(
                    "slotsMatchDoc".into(),
                    json!(
                        save["editor"]["slots"].as_array().map(|a| a.len() as i64)
                            == Some(slot_count)
                    ),
                );
                let empty_arr = |v: &Value| v.as_array().map(Vec::len) == Some(0);
                checks.insert(
                    "emptyGraph".into(),
                    json!(
                        empty_arr(&save["editor"]["factions"])
                            && empty_arr(&save["editor"]["squads"])
                            && empty_arr(&save["editor"]["editorLayers"])
                    ),
                );
                checks.insert(
                    "objectShapes".into(),
                    json!(save["loadouts"].is_object() && save["environment"].is_object()),
                );
                checks.insert(
                    "arrayShapes".into(),
                    json!(
                        save["objectives"].is_array()
                            && save["vehicles"].is_array()
                            && save["markers"].is_array()
                    ),
                );
            }
            if is_obj(&exp) {
                checks.insert(
                    "exportFormatVersion".into(),
                    json!(exp["exportFormatVersion"] == json!(1)),
                );
                checks.insert(
                    "exportOrbatEmpty".into(),
                    json!(
                        exp["payload"].is_object()
                            && exp["payload"]["orbat"].as_array().map(Vec::len) == Some(0)
                    ),
                );
                checks.insert(
                    "exportWrapsPayload".into(),
                    json!(
                        exp["payload"].is_object() && exp["payload"]["schemaVersion"] == json!(1)
                    ),
                );
            }
        } else {
            eprintln!("smoke_save_export_editor: window.__editorCommands never appeared");
        }
        let seeded = slot_count == SEED_N;
        let pass = ready && h.no_panics() && seeded && checks_pass(&checks, 16);
        print_verdict(&json!({
            "gate": "editor-save-export-smoke", "path": path,
            "slotCount": slot_count, "seeded": seeded, "checks": checks,
            "saveLen": save_len, "exportLen": export_len,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}
