use super::*;

/// T-789 (wave-203 MAJOR) — the Save Version dialog's Version input must render FULLY IN-VIEWPORT.
///
/// This is the authoritative guard the source class-pin (`is_clamped_on_screen_by_construction`,
/// renamed `dialog_carries_the_centering_classes_rect_is_smoke_proven`) could not be: the wave-203
/// bug was a `position:fixed` dialog nested under the strip's `backdrop-filter` glass root, which
/// establishes a containing block — so `top-1/2 -translate-y-1/2` centered on the 48px strip and the
/// Version input landed at y=-22 (1920×1080) / y=-184 (1366×768), OFF the top of the screen. Every
/// class was present and correct; only an ANCESTOR was wrong, which no class-string scrub can catch.
/// The fix portals the dialog to `document.body`. This smoke opens the real dialog in real Chrome at
/// BOTH shipping viewports and asserts the input's live `getBoundingClientRect` is on-screen
/// (`top>=0 && bottom<=innerHeight`), the "Version" label is visible, `activeElement` is the input
/// (T-789 focus-first), and 8 Tabs stay inside the dialog (T-789 Tab trap). Reintroducing the
/// containing block (re-nest under the glass, or add a `transform`/`filter` to an ancestor) drives
/// `v1920_inViewport` / `v1366_inViewport` false — the perturbation the class pin let through.
pub async fn smoke_save_dialog_rect(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5331, 9391, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(ATTR_READY, 120, 250).await?;

        let mut checks = Map::new();
        if ready {
            // The strip's ONE primary action — unique `title`, so this never hits the File-menu
            // "Save Version…" row (which carries an ellipsis and a different title).
            const SAVE_BTN: &str = "[...document.querySelectorAll('button')].find(b => b.getAttribute('title') === 'Save an immutable version of this mission')";
            // The dialog is open once its own <h2> and the Version input are both mounted.
            const DIALOG_OPEN: &str = "[...document.querySelectorAll('h2')].some(h => h.textContent === 'Save Version') && !!document.querySelector('input[aria-label=\"Version\"]')";

            // Both shipping viewports. `setDeviceMetricsOverride` reflows the CSS layout, so the
            // `fixed` centering is recomputed against each viewport before we read the rect.
            for (tag, w, hpx) in [("v1920", 1920u32, 1080u32), ("v1366", 1366u32, 768u32)] {
                h.page
                    .send(
                        "Emulation.setDeviceMetricsOverride",
                        json!({ "width": w, "height": hpx, "deviceScaleFactor": 1, "mobile": false }),
                    )
                    .await?;
                // Open the dialog for this viewport.
                eval(&h.page, &format!("{SAVE_BTN}.click()")).await?;
                let opened = h.page.wait_for(DIALOG_OPEN, 40, 250).await?;
                checks.insert(format!("{tag}_dialogOpen"), json!(opened));

                // Live rect of the Version input vs the live innerHeight. `inViewport` is the
                // acceptance: the whole field is on-screen (top>=0 AND bottom<=innerHeight).
                let rect_json = eval_str(
                    &h.page,
                    "(() => { const e = document.querySelector('input[aria-label=\"Version\"]');
                       if (!e) return 'null'; const b = e.getBoundingClientRect();
                       return JSON.stringify({top: b.top, bottom: b.bottom, ih: window.innerHeight}); })()",
                )
                .await?;
                let r: Value = serde_json::from_str(&rect_json).unwrap_or(Value::Null);
                let top = r["top"].as_f64().unwrap_or(-1.0);
                let bottom = r["bottom"].as_f64().unwrap_or(f64::MAX);
                let ih = r["ih"].as_f64().unwrap_or(0.0);
                let in_viewport = top >= 0.0 && bottom <= ih;
                checks.insert(format!("{tag}_inViewport"), json!(in_viewport));
                // Emit the numbers so a red run reads like the verifier's CDP proof.
                checks.insert(format!("{tag}_top"), json!(top));
                checks.insert(format!("{tag}_bottom"), json!(bottom));
                checks.insert(format!("{tag}_innerHeight"), json!(ih));

                // The "Version" label the field sits under must be on-screen too (the verifier's
                // "label visible" — the title/immutability line clipped above the edge in the bug).
                let label_visible = eval_bool(
                    &h.page,
                    "(() => { const s = [...document.querySelectorAll('span')].find(n => n.textContent === 'Version');
                       if (!s) return false; const b = s.getBoundingClientRect();
                       return b.top >= 0 && b.bottom <= window.innerHeight; })()",
                )
                .await?;
                checks.insert(format!("{tag}_labelVisible"), json!(label_visible));

                // T-789 focus-first: activeElement IS the Version input on open.
                let focused = eval_bool(
                    &h.page,
                    "document.activeElement === document.querySelector('input[aria-label=\"Version\"]')",
                )
                .await?;
                checks.insert(format!("{tag}_focusOnVersion"), json!(focused));

                // T-789 Tab trap: 8 Tabs keep focus inside the dialog subtree (its container is the
                // nearest ancestor DIV of the Version input carrying the centering classes).
                for _ in 0..8 {
                    key_chord(&h.page, "Tab", "Tab", 0, 9).await?;
                }
                let trapped = eval_bool(
                    &h.page,
                    "(() => { const inp = document.querySelector('input[aria-label=\"Version\"]');
                       const dlg = inp && inp.closest('div.fixed.top-1\\\\/2');
                       return !!dlg && dlg.contains(document.activeElement); })()",
                )
                .await?;
                checks.insert(format!("{tag}_tabTrapped"), json!(trapped));

                // Esc closes the dialog before the next viewport pass (also re-proves Esc-close).
                key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
                let closed = h
                    .page
                    .wait_for(&format!("!({DIALOG_OPEN})"), 20, 250)
                    .await?;
                checks.insert(format!("{tag}_escClosed"), json!(closed));
            }
        } else {
            eprintln!("smoke_save_dialog_rect: {ATTR_READY} never became true");
        }
        // 8 boolean checks per viewport (dialogOpen, inViewport, labelVisible, focusOnVersion,
        // tabTrapped, escClosed) — the *_top/_bottom/_innerHeight entries are numeric diagnostics,
        // not pass/fail, so they are excluded from the count below.
        let bool_checks: Map<String, Value> = checks
            .iter()
            .filter(|(_, v)| v.is_boolean())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let pass = ready && h.no_panics() && checks_pass(&bool_checks, 12);
        print_verdict(&json!({
            "gate": "editor-save-dialog-rect-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// T-794 (wave-207) — **an entrance animation must not move the surface.** This is the guard the
/// fix itself could not have: the defect lived in `aegis.css`, and CSS has no Rust test home.
///
/// What T-794 fixed: `dialog-in` carried `from { transform: translate(-50%, calc(-50% + 8px))
/// scale(0.96) }` to re-state a centered dialog's own centering translate. True under Tailwind v3,
/// where `-translate-x-1/2` compiled INTO `transform` and the animation replaced it; Tailwind v4
/// (pinned 4.3.2) emits the independent `translate:` property, which COMPOSES with `transform`
/// instead — so the keyframe stopped restating the centering and started DOUBLING it. Every surface
/// first-painted half a box from where it landed: the Save dialog 215 px, ORBAT 679 px corner-to-
/// corner (528 px of it in y, first-painting at (-118,-236) — off-screen), Mission Settings 433 px
/// of y above the viewport, the context menu 115 px off the click it answers.
///
/// A class-string scrub cannot see any of that — every class was correct, the KEYFRAME was wrong —
/// so this measures the thing itself, with the review's own probe: a `MutationObserver` on
/// `.animate-dialog-in,.animate-menu-in` reads `getBoundingClientRect()` INSIDE the callback (the
/// read forces sync layout, which is the first-paint position, animation applied) and again after
/// 700 ms (well past the 120 ms entrance). `max(|dx|, |dy|) < 8` px is the review's own ceiling and
/// `animation-duration <= 0.15 s` its snappiness bar.
///
/// PERTURBATION-PROVEN against a scratch copy of the dist carrying the EXACT pre-T-794 sheet
/// (`26d488d5^`: the `transform: translate(-50%, calc(-50% + 8px)) scale(0.96)` keyframe, the
/// `scale(0.95)` menu one, 200 ms / 150 ms). Every surface went red and reproduced the ticket's own
/// measurement to the pixel — `ctxmenu dx=115.2 dy=99.5` (the ticket's "115 px off its own click
/// anchor"), `settings dx=245.8 dy=432.6` ("433 px of y"), `save dx=215.0 dy=156.6` ("215 px"),
/// `orbat dx=528.0 dy=376.0` ("528 px") — and `*_fastEnough` went red too at `0.2s`.
///
/// The second pass is the `prefers-reduced-motion` half, which that sheet had never had at all:
/// under `Emulation.setEmulatedMedia` the delta must be EXACTLY 0 and the computed `animation-name`
/// must be `overlay-fade` — the one keyframe in the sheet that touches nothing but opacity, so a
/// reader who asked for less motion cannot be given a translation by construction.
pub async fn smoke_entrance_motion_rect(dist: &str, path: &str) -> Result<u8> {
    let h = Harness::new(dist, 5332, 9392, None, None, &[]).await?;
    let run = async {
        h.page.navigate(&h.url(path)).await?;
        h.page
            .wait_for("!!document.querySelector('canvas')", 80, 250)
            .await?;
        let ready = h.page.wait_for(ATTR_READY, 120, 250).await?;
        // The viewport T-794 measured at. A `fixed` surface's travel is viewport-relative, so the
        // number in the ticket is only reproducible at the ticket's own metrics.
        h.page
            .send(
                "Emulation.setDeviceMetricsOverride",
                json!({ "width": 1920, "height": 1080, "deviceScaleFactor": 1, "mobile": false }),
            )
            .await?;

        /// Any live entrance surface. The class stays on the node after the animation ends, so this
        /// doubles as "is one open right now?" between surfaces.
        const SURFACE: &str = ".animate-dialog-in,.animate-menu-in";
        // Arm the observer, then read `window.__t794` once `done`. Returns false if a surface is
        // ALREADY open (the previous one failed to close) — that would measure the wrong element.
        const ARM: &str = r#"(() => {
            window.__t794 = null;
            if (document.querySelector('.animate-dialog-in,.animate-menu-in')) return false;
            const grab = (el) => {
                const b = el.getBoundingClientRect();
                const cs = getComputedStyle(el);
                return { x: b.left, y: b.top, w: b.width, h: b.height,
                         name: cs.animationName, dur: cs.animationDuration };
            };
            const obs = new MutationObserver(() => {
                if (window.__t794) return;
                const el = document.querySelector('.animate-dialog-in,.animate-menu-in');
                if (!el) return;
                window.__t794 = { first: grab(el), settled: null, done: false };
                obs.disconnect();
                setTimeout(() => {
                    const e2 = document.querySelector('.animate-dialog-in,.animate-menu-in') || el;
                    window.__t794.settled = grab(e2);
                    window.__t794.done = true;
                }, 700);
            });
            obs.observe(document.body, { childList: true, subtree: true });
            return true;
        })()"#;

        // (tag, trigger). The context menu is a real `contextmenu` MouseEvent on the canvas at the
        // click it must appear AT (800, 600) — the surface whose 115 px drag was off its own anchor.
        let surfaces: [(&str, &str); 4] = [
            (
                "ctxmenu",
                r#"(() => { const c = document.querySelector('canvas'); if (!c) return false;
                     c.dispatchEvent(new MouseEvent('contextmenu',
                       { bubbles: true, cancelable: true, clientX: 800, clientY: 600 }));
                     return true; })()"#,
            ),
            (
                "settings",
                r#"(() => { const b = document.querySelector('button[aria-label="Mission settings"]');
                     if (!b || b.disabled) return false; b.click(); return true; })()"#,
            ),
            (
                "save",
                r#"(() => { const b = [...document.querySelectorAll('button')].find(
                       x => x.getAttribute('title') === 'Save an immutable version of this mission');
                     if (!b || b.disabled) return false; b.click(); return true; })()"#,
            ),
            (
                "orbat",
                r#"(() => { const b = document.querySelector('[aria-label="ORBAT Manager"]');
                     if (!b || b.disabled) return false; b.click(); return true; })()"#,
            ),
        ];

        let mut checks = Map::new();
        if ready {
            for reduced in [false, true] {
                // Pass 2 asks the page for less motion. Same surfaces, stricter acceptance.
                let features = if reduced {
                    json!([{ "name": "prefers-reduced-motion", "value": "reduce" }])
                } else {
                    json!([])
                };
                h.page
                    .send(
                        "Emulation.setEmulatedMedia",
                        json!({ "media": "screen", "features": features }),
                    )
                    .await?;
                let pfx = if reduced { "rm_" } else { "" };

                for (tag, trigger) in surfaces {
                    let armed = eval_bool(&h.page, ARM).await?;
                    let fired = eval_bool(&h.page, trigger).await?;
                    let done = armed
                        && fired
                        && h.page
                            .wait_for("!!window.__t794 && window.__t794.done === true", 30, 100)
                            .await?;
                    checks.insert(format!("{pfx}{tag}_firstPaintSeen"), json!(done));

                    let raw = eval_str(
                        &h.page,
                        "JSON.stringify(window.__t794 && window.__t794.done ? window.__t794 : null)",
                    )
                    .await?;
                    let m: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
                    let at = |k: &str, f: &str| m[k][f].as_f64().unwrap_or(f64::NAN);
                    let (dx, dy) = (
                        at("settled", "x") - at("first", "x"),
                        at("settled", "y") - at("first", "y"),
                    );
                    let name = m["first"]["name"].as_str().unwrap_or("").to_string();
                    let dur = m["first"]["dur"].as_str().unwrap_or("").to_string();
                    // Emit the raw travel so a red run reads like the ticket's own measurement.
                    checks.insert(format!("{pfx}{tag}_dx"), json!(dx));
                    checks.insert(format!("{pfx}{tag}_dy"), json!(dy));
                    checks.insert(format!("{pfx}{tag}_anim"), json!(name));
                    checks.insert(format!("{pfx}{tag}_dur"), json!(dur));

                    if reduced {
                        // Reduced motion is opacity or nothing: EXACTLY zero travel, and the
                        // computed animation must be the one keyframe that cannot translate.
                        checks.insert(
                            format!("{pfx}{tag}_zeroDelta"),
                            json!(done && dx == 0.0 && dy == 0.0),
                        );
                        checks.insert(
                            format!("{pfx}{tag}_overlayFade"),
                            json!(name == "overlay-fade"),
                        );
                    } else {
                        checks.insert(
                            format!("{pfx}{tag}_settles"),
                            json!(done && dx.abs().max(dy.abs()) < 8.0),
                        );
                        checks.insert(
                            format!("{pfx}{tag}_fastEnough"),
                            json!(parse_seconds(&dur).is_some_and(|s| s <= 0.15)),
                        );
                    }

                    // Close it before the next surface — the ARM above refuses on a stale one.
                    key_chord(&h.page, "Escape", "Escape", 0, 27).await?;
                    h.page
                        .wait_for(&format!("!document.querySelector('{SURFACE}')"), 20, 100)
                        .await?;
                }
            }
        } else {
            eprintln!("smoke_entrance_motion_rect: {ATTR_READY} never became true");
        }
        // 3 booleans × 4 surfaces × 2 passes; the dx/dy/anim/dur entries are diagnostics.
        let bool_checks: Map<String, Value> = checks
            .iter()
            .filter(|(_, v)| v.is_boolean())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let pass = ready && h.no_panics() && checks_pass(&bool_checks, 24);
        print_verdict(&json!({
            "gate": "editor-entrance-motion-rect-smoke", "path": path, "checks": checks,
            "panics": h.panics_head(), "pass": pass,
        }));
        Ok::<u8, anyhow::Error>(to_code(pass))
    };
    let code = run.await;
    h.shutdown().await;
    code
}

/// A CSS `<time>` as seconds — `"0.12s"` / `"120ms"`. `None` for anything unparseable (including
/// the empty string a missing element yields), so an unread duration FAILS the ceiling rather than
/// defaulting to zero and passing.
pub(super) fn parse_seconds(v: &str) -> Option<f64> {
    let t = v.split(',').next()?.trim();
    if let Some(ms) = t.strip_suffix("ms") {
        return ms.trim().parse::<f64>().ok().map(|n| n / 1000.0);
    }
    t.strip_suffix('s')?.trim().parse::<f64>().ok()
}
