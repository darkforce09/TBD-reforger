//! Reactive building viewer controls and presentation.

use super::*;

/// Building-blueprint viewer + interactive LOS bench (BVH raycast, blueprint-attributed).
#[component]
pub fn BuildingViewerPage() -> impl IntoView {
    let blueprint = RwSignal::new(None::<BuildingBlueprint>);
    // The `.bvh` occlusion sidecar fetched beside the JSON — `Arc` because the parsed sidecar
    // is `!Clone` (`RwSignal::get` clones) and the signal store wants `Send + Sync`.
    let sidecar = RwSignal::new(None::<Arc<BvhSidecar>>);
    let sidecar_err = RwSignal::new(None::<String>);
    // Per-level visibility rasters while the viewshed is on (one `LevelWash` per level, level
    // order); `Arc` for the same reason as the sidecar. `None` = off / nothing to trace.
    let wash = RwSignal::new(None::<Arc<LevelWash>>);
    // The mesh's 2D drawing (section cuts, floor / roof faces, void coverage) — computed once
    // per (blueprint, sidecar); `None` = no sidecar → the blueprint draws everything.
    let drawing = RwSignal::new(None::<Arc<BuildingDrawing>>);
    let load_err = RwSignal::new(None::<String>);
    let engine_err = RwSignal::new(None::<String>);
    let view_floor = RwSignal::new(ViewFloor::Level(0));
    let floors_open = RwSignal::new(false);
    let viewshed_on = RwSignal::new(false);
    let obs = RwSignal::new(RayEnd {
        x: -3.8,
        y: 1.4,
        z: -8.0,
    });
    let tgt = RwSignal::new(RayEnd {
        x: -3.5,
        y: 1.2,
        z: -1.2,
    });
    let los = RwSignal::new(None::<LosResult>);
    let cam = RwSignal::new(Cam {
        tx: geom::ANCHOR[0],
        ty: geom::ANCHOR[1],
        zoom: 4.5,
    });
    let css = RwSignal::new((1200.0f64, 800.0f64));
    let drag = RwSignal::new(Drag::None);
    // The furnished compound (shell + instances, door states inside), the
    // per-level owned cuts of its flattened mesh (recomputed on every door toggle) and the
    // instances-load error line. `None` = the shell-only path (blueprint annotations).
    let compound = RwSignal::new(None::<CompoundBuilding>);
    let compound_err = RwSignal::new(None::<String>);
    let cuts = RwSignal::new(None::<Arc<Vec<LevelCuts>>>);
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // Pure LOS evaluation — reruns on any ray/blueprint/sidecar change; the wasm host mirrors
    // `los` into the ray lane. Ungated: harmless on native (never mounted there). No sidecar →
    // no verdict: the blueprint alone cannot block a ray.
    Effect::new(move |_| {
        let (o, t) = (obs.get(), tgt.get());
        let sc = sidecar.get();
        blueprint.with(|bp| {
            los.set(match (bp.as_ref(), sc.as_deref()) {
                (Some(bp), Some(occl)) => Some(compound.with(|c| match c.as_ref() {
                    // The compound walks doors, glass, foliage, and props.
                    Some(c) => c.evaluate_los(Some(bp), [o.x, o.y, o.z], [t.x, t.y, t.z]),
                    None => bp.evaluate_los(occl, [o.x, o.y, o.z], [t.x, t.y, t.z]),
                })),
                _ => None,
            });
        });
    });

    // Multi-floor viewshed — the VIEWED level's visibility disc follows the observer live while
    // the viewshed is on (Alt+click / Alt+drag on A); a floor-rail change recomputes for that
    // level (the Roof view has no eye plane). Radius follows the footprint diagonal
    // + 5 m. Pure core compute, native-safe; the wasm host uploads it to the texture lane.
    // No sidecar → no wash: the blueprint alone cannot stop a ray.
    Effect::new(move |_| {
        if !viewshed_on.get() {
            wash.set(None);
            return;
        }
        let o = obs.get();
        let sc = sidecar.get();
        let view = view_floor.get();
        blueprint.with(|bp| {
            wash.set(match (bp.as_ref(), sc.as_deref(), view) {
                (Some(bp), Some(occl), ViewFloor::Level(i)) => {
                    let bb = &bp.overall_footprint.bounding_box2_d;
                    let p = WashParams {
                        radius_m: bb.width_m.hypot(bb.depth_m) + 5.0,
                        ..WashParams::default()
                    };
                    compound
                        .with(|c| match c.as_ref() {
                            Some(c) => level_wash_compound(bp, c, [o.x, o.y, o.z], i, &p),
                            None => level_wash(bp, occl, [o.x, o.y, o.z], i, &p),
                        })
                        .map(Arc::new)
                }
                _ => None,
            });
        });
    });

    // The mesh drawing follows the blueprint + sidecar pair (pure core compute, native-safe).
    Effect::new(move |_| {
        let sc = sidecar.get();
        blueprint.with(|bp| {
            drawing.set(match (bp.as_ref(), sc.as_deref()) {
                (Some(bp), Some(occl)) => Some(Arc::new(building_drawing(bp, occl))),
                _ => None,
            });
        });
    });

    // The flattened compound owns section cuts per level: door leaves, frames
    // and panes routed to their own lanes. Reruns on every door toggle (the compound signal).
    Effect::new(move |_| {
        let d = drawing.get();
        cuts.set(compound.with(|c| match (c.as_ref(), d.as_deref()) {
            (Some(c), Some(d)) => Some(Arc::new(LevelCuts::for_drawing(c, d))),
            _ => None,
        }));
    });

    #[cfg(target_arch = "wasm32")]
    live::wire(
        canvas_ref,
        blueprint,
        sidecar,
        sidecar_err,
        wash,
        drawing,
        load_err,
        engine_err,
        view_floor,
        floors_open,
        viewshed_on,
        obs,
        tgt,
        los,
        cam,
        css,
        drag,
        compound,
        compound_err,
        cuts,
    );

    // ── DOM overlay derivations ─────────────────────────────────────────────────────────────
    let marker_px = move |end: RayEnd| {
        let c = cam.get();
        geom::world_to_screen(
            geom::to_world([end.x, end.z]),
            c.tx,
            c.ty,
            c.zoom,
            css.get(),
        )
    };
    let obs_px = move || marker_px(obs.get());
    let tgt_px = move || marker_px(tgt.get());
    // Off-floor markers dim to half opacity — the point exists, just not on the viewed plan.
    let on_floor = move |y: f64| {
        blueprint.with(|bp| {
            bp.as_ref().is_none_or(|bp| {
                let (band, _) = view_floor.get().band(bp);
                y >= band[0] && y <= band[1]
            })
        })
    };
    let marker_wrap = move |y: f64| {
        if on_floor(y) {
            "pointer-events-auto absolute z-20 -translate-x-1/2 -translate-y-1/2 cursor-grab"
        } else {
            "pointer-events-auto absolute z-20 -translate-x-1/2 -translate-y-1/2 cursor-grab opacity-50"
        }
    };

    let verdict_view = move || {
        los.get().map(|r| {
            let badge = if r.is_clear {
                view! { <span class="rounded bg-emerald-500/20 px-2 py-0.5 font-bold text-emerald-400">"CLEAR"</span> }.into_any()
            } else {
                view! { <span class="rounded bg-red-500/20 px-2 py-0.5 font-bold text-red-400">"BLOCKED"</span> }.into_any()
            };
            let pct = (r.concealment * 100.0).round();
            let windows = r.window_ids_traversed.join(", ");
            let doors = r.door_ids_traversed.join(", ");
            let canopy = r
                .hits
                .iter()
                .filter(|h| h.kind == LosHitKind::Foliage)
                .map(|h| format!("{} ({:.0}%)", h.id, h.concealment * 100.0))
                .collect::<Vec<_>>()
                .join(", ");
            let blocker = r.hits.last().and_then(|h| match h.kind {
                LosHitKind::DoorLeaf => Some(format!("door leaf {}", h.id)),
                LosHitKind::DoorFrame => Some(format!("door frame {}", h.id)),
                LosHitKind::WindowFrame => Some(format!("window frame {}", h.id)),
                LosHitKind::Prop => Some(format!("prop {}", h.id)),
                _ => None,
            });
            view! {
                <div class="space-y-1">
                    <div class="flex items-center gap-2">{badge}
                        <span class="text-on-surface-variant">{format!("concealment {pct:.0}%")}</span>
                    </div>
                    {(!windows.is_empty()).then(|| view! { <div>"through glass: "<span class="text-cyan-300">{windows.clone()}</span></div> })}
                    {(!doors.is_empty()).then(|| view! { <div>"through door: "<span class="text-emerald-300">{doors.clone()}</span></div> })}
                    {r.blocked_by_wall_id.clone().map(|w| view! { <div>"blocked by "<span class="text-red-300">{w}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Roof).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("roof @ {:.1} m", h.pos[1])}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Solid).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("solid @ {:.1} m", h.pos[1])}</span></div> })}
                    {r.hits.last().filter(|h| h.kind == LosHitKind::Window && h.concealment >= 1.0).map(|h| view! { <div>"blocked by "<span class="text-red-300">{format!("frame of {}", h.id)}</span></div> })}
                    {r.cover_furniture_id.clone().map(|f| view! { <div>"cover: "<span class="text-yellow-300">{f}</span></div> })}
                    {(!canopy.is_empty()).then(|| view! { <div>"through canopy: "<span class="text-lime-300">{canopy.clone()}</span></div> })}
                    {blocker.map(|b| view! { <div>"blocked by "<span class="text-red-300">{b}</span></div> })}
                </div>
            }
        })
    };

    // Floor rail — slides out at the building's RIGHT side on building click (stays once open).
    // Vertical stack, bottom→top = Ground..N with Roof topmost; anchored to the building's
    // screen bbox so it follows pan/zoom.
    let floor_rail = move || {
        if !floors_open.get() {
            return None;
        }
        let c = cam.get();
        let size = css.get();
        blueprint.with(|bp| {
            bp.as_ref().map(|bp| {
                let bb = &bp.overall_footprint.bounding_box2_d;
                let anchor = geom::world_to_screen(
                    geom::to_world([bb.max[0] + 1.0, (bb.min[1] + bb.max[1]) * 0.5]),
                    c.tx,
                    c.ty,
                    c.zoom,
                    size,
                );
                let style = format!("left:{}px;top:{}px", anchor[0] + 12.0, anchor[1]);
                // Top→bottom DOM order = Roof, then floors highest→ground.
                let mut rows: Vec<(ViewFloor, String)> = vec![(ViewFloor::Roof, "Roof".to_string())];
                rows.extend(
                    bp.levels
                        .iter()
                        .rev()
                        .map(|l| (ViewFloor::Level(l.level_index), l.name.clone())),
                );
                view! {
                    <div
                        class="pointer-events-auto absolute z-20 flex -translate-y-1/2 flex-col gap-1 rounded-lg border border-border-subtle bg-surface-container/90 p-1 backdrop-blur"
                        style=style
                    >
                        {rows
                            .into_iter()
                            .map(|(vf, name)| {
                                let is_active = move || view_floor.get() == vf;
                                view! {
                                    <button
                                        type="button"
                                        class=move || {
                                            if is_active() {
                                                "rounded-md bg-primary px-3 py-1 text-left text-sm font-medium text-on-primary"
                                            } else {
                                                "rounded-md px-3 py-1 text-left text-sm text-on-surface-variant hover:text-primary"
                                            }
                                        }
                                        on:click=move |_| view_floor.set(vf)
                                    >
                                        {name}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                }
            })
        })
    };

    // Roof view: profile chip (the only roof data the blueprint carries today).
    let roof_chip = move || {
        if view_floor.get() != ViewFloor::Roof {
            return None;
        }
        blueprint.with(|bp| {
            bp.as_ref().map(|bp| {
                let vp = &bp.vertical_profile;
                let chimney = vp
                    .chimney_height_m
                    .map_or(String::new(), |h| format!(" · chimney {h:.1} m"));
                let label = format!(
                    "{} · eave {:.1} m · ridge {:.1} m{}",
                    vp.roof_type, vp.eave_height_m, vp.ridge_height_m, chimney
                );
                view! {
                    <div class="pointer-events-none absolute left-1/2 top-3 z-20 -translate-x-1/2 rounded-lg border border-border-subtle bg-surface-container/90 px-3 py-1.5 text-xs text-on-surface-variant backdrop-blur">
                        {label}
                    </div>
                }
            })
        })
    };

    // Sill/height badges for the active floor's windows (visible once zoomed past ~16 px/m).
    let window_badges = move || {
        let c = cam.get();
        if c.zoom < 4.0 {
            return Vec::new();
        }
        let size = css.get();
        blueprint.with(|bp| {
            let Some(bp) = bp.as_ref() else { return Vec::new() };
            let ViewFloor::Level(i) = view_floor.get() else { return Vec::new() };
            let Some(lvl) = bp.levels.get(i) else { return Vec::new() };
            lvl.windows
                .iter()
                .map(|w| {
                    let p = geom::world_to_screen(
                        geom::to_world([w.pos2_d[0] + w.normal[0] * 1.3, w.pos2_d[1] + w.normal[1] * 1.3]),
                        c.tx,
                        c.ty,
                        c.zoom,
                        size,
                    );
                    let label = format!(
                        "sill {:.2} · h {:.2}",
                        w.sill_height_m, w.window_height_m
                    );
                    view! {
                        <div
                            class="pointer-events-none absolute z-10 -translate-x-1/2 -translate-y-1/2 rounded bg-cyan-950/80 px-1.5 py-0.5 text-[10px] text-cyan-300"
                            style=move || format!("left:{}px;top:{}px", p[0], p[1])
                        >
                            {label}
                        </div>
                    }
                })
                .collect::<Vec<_>>()
        })
    };

    let slider = move |label: &'static str, end: RwSignal<RayEnd>| {
        view! {
            <label class="flex items-center gap-2 text-xs text-on-surface-variant">
                <span class="w-24">{label}" "{move || format!("{:.2} m", end.get().y)}</span>
                <input
                    type="range"
                    min="0"
                    max="10"
                    step="0.05"
                    class="w-40 accent-primary"
                    prop:value=move || end.get().y.to_string()
                    on:input=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                            end.update(|e| e.y = v);
                        }
                    }
                />
            </label>
        }
    };

    view! {
        <div class="relative h-full w-full select-none overflow-hidden bg-[#0b0e13]">
            <canvas node_ref=canvas_ref class="absolute inset-0 h-full w-full touch-none"></canvas>

            {floor_rail}
            {roof_chip}
            {window_badges}

            // Observer / target markers.
            <div
                class=move || marker_wrap(obs.get().y)
                style=move || { let p = obs_px(); format!("left:{}px;top:{}px", p[0], p[1]) }
                on:pointerdown=move |ev| { ev.prevent_default(); drag.set(Drag::Observer); }
            >
                <div class="flex h-7 w-7 items-center justify-center rounded-full border-2 border-emerald-400 bg-emerald-500/30 text-[11px] font-bold text-emerald-200">"A"</div>
                <div class="mt-0.5 rounded bg-black/60 px-1 text-center text-[10px] text-emerald-300">{move || format!("{:.1}m", obs.get().y)}</div>
            </div>
            <div
                class=move || marker_wrap(tgt.get().y)
                style=move || { let p = tgt_px(); format!("left:{}px;top:{}px", p[0], p[1]) }
                on:pointerdown=move |ev| { ev.prevent_default(); drag.set(Drag::Target); }
            >
                <div class="flex h-7 w-7 items-center justify-center rounded-full border-2 border-sky-400 bg-sky-500/30 text-[11px] font-bold text-sky-200">"B"</div>
                <div class="mt-0.5 rounded bg-black/60 px-1 text-center text-[10px] text-sky-300">{move || format!("{:.1}m", tgt.get().y)}</div>
            </div>

            // Header / controls.
            <div class="pointer-events-auto absolute left-3 top-3 z-20 max-w-sm space-y-2 rounded-lg border border-border-subtle bg-surface-container/90 p-3 backdrop-blur">
                <div class="text-sm font-bold">"Building Viewer "<span class="text-on-surface-variant">"(debug bench)"</span></div>
                <div class="text-xs text-on-surface-variant">
                    {move || blueprint.with(|bp| bp.as_ref().map(|b| b.label.clone().unwrap_or_else(|| b.prefab_id.clone())).unwrap_or_else(|| "loading…".into()))}
                </div>
                {slider("Observer Y", obs)}
                {slider("Target Y", tgt)}
                <div class="text-[10px] text-on-surface-variant">"drag A/B markers · drag canvas to pan · wheel zooms · click the building for floors · click a door to swing it · Alt+click moves A and fills its viewshed"</div>
                {move || compound.with(|c| c.as_ref().map(|c| {
                    let (open, closed) = c.doors().fold((0usize, 0usize), |(o, k), d| if d.state.is_open() { (o + 1, k) } else { (o, k + 1) });
                    view! { <div class="text-xs text-on-surface-variant">{format!("furnished: {} instances · doors {open} open · {closed} closed", c.instances.len())}</div> }
                }))}
                {move || compound_err.get().map(|e| view! { <div class="rounded bg-amber-500/15 p-2 text-xs text-amber-300">{e}</div> })}
                {move || viewshed_on.get().then(|| {
                    // Which level's wash is on screen — the floor rail swaps it; the roof
                    // view has no eye plane and shows none.
                    let shown = match view_floor.get() {
                        ViewFloor::Level(i) => blueprint.with(|bp| {
                            bp.as_ref()
                                .and_then(|b| b.levels.iter().find(|l| l.level_index == i))
                                .map_or_else(|| format!("level {i}"), |l| l.name.clone())
                        }),
                        ViewFloor::Roof => "no wash on the roof view".to_string(),
                    };
                    view! {
                    <div class="flex items-center gap-2 rounded bg-emerald-500/10 px-2 py-1 text-xs text-emerald-300">
                        {format!("viewshed from A · {shown}")}
                        <button
                            type="button"
                            class="rounded px-1 text-on-surface-variant hover:text-red-300"
                            on:click=move |_| viewshed_on.set(false)
                        >
                            "✕ clear"
                        </button>
                    </div>
                    }
                })}
                {move || load_err.get().map(|e| view! { <div class="rounded bg-red-500/15 p-2 text-xs text-red-300">{e}</div> })}
                {move || sidecar_err.get().map(|e| view! { <div class="rounded bg-amber-500/15 p-2 text-xs text-amber-300">{e}</div> })}
                {move || engine_err.get().map(|e| view! { <div class="rounded bg-red-500/15 p-2 text-xs text-red-300">{e}</div> })}
            </div>

            // Verdict.
            <div class="pointer-events-none absolute bottom-3 right-3 z-20 min-w-56 rounded-lg border border-border-subtle bg-surface-container/90 p-3 text-xs backdrop-blur">
                {verdict_view}
            </div>

            // Legend.
            <div class="pointer-events-none absolute bottom-3 left-3 z-20 space-y-0.5 rounded-lg border border-border-subtle bg-surface-container/90 p-2 text-[10px] text-on-surface-variant backdrop-blur">
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#cdd4e0]"></span>"wall (exterior)"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#34c7f2]"></span>"window / glass"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#4dd973]"></span>"open door · ray clear"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#f2a133]"></span>"closed door"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#9ed940]"></span>"canopy · ray through leaves"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#8c6640]"></span>"tree trunk"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#808c9e]"></span>"prop footprint"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#ebcc40]"></span>"low cover · ray past cover"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#e6574a]"></span>"full cover · ray blocked"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-[#7059b3]"></span>"stairs (transparent treads)"</div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-gradient-to-r from-[#2a3854] to-[#d1dbf2]"></span>"roof height (eave → ridge) · "<span class="text-[#f2a133]">"chimney"</span></div>
                <div><span class="mr-1 inline-block h-2 w-4 bg-gradient-to-r from-[#1a212e] to-[#3d4c66]"></span>"floor plate (scanned cells) · "<span class="text-[#739eb8]">"ring edge"</span></div>
            </div>
        </div>
    }
}
