//! T-090.12.5 — the wasm half of the LOS object layer: the closures that reach the live world
//! occluder (`world_assets::with_occluder`) for the shot verdict and for the viewshed's object
//! wash, and the rAF-stepped wash itself (started on placement, ticked by `canvas/viewport.rs`,
//! cancelled with the viewshed). The pure logic lives in [`super::los_world`].

use std::cell::{Cell, RefCell};

use map_engine_core::building_compound_los::Owner;
use map_engine_core::bvh::SurfaceKind;
use map_engine_core::dem::sample::Viewshed;
use map_engine_core::world::occluder::{WorldLos, WorldVerdict};
use map_engine_render::RenderEngine;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

use super::los_tool::{
    pack_rgba_256, read_registered_sampler, read_registered_viewshed, LosShot,
    EYE_HEIGHT_OBSERVER_M, EYE_HEIGHT_TARGET_M,
};
use super::los_world::{
    encode_viewshed_rgba_merged, map_to_engine, ObjectCell, ObjectPass, ObjectVerdict,
    OBJECT_PASS_BUDGET_MS, OBJECT_UPLOAD_INTERVAL_MS,
};
use crate::editor::world_assets::{with_occluder, with_occluder_host};

/// The live wash: the pass, the raster it runs over (cloned once at start), the observer eye.
struct Wash {
    pass: ObjectPass,
    vs: Viewshed,
    eye_z: f64,
}

thread_local! {
    static WASH: RefCell<Option<Wash>> = const { RefCell::new(None) };
    static GENERATION: Cell<u32> = const { Cell::new(0) };
    static LAST_UPLOAD_MS: Cell<f64> = const { Cell::new(0.0) };
    /// `(chunks, expanded prefabs, BLAS)` of the occluder when the wash last finished: a change
    /// with provisional cells left means BLAS landed — those cells are retested.
    static WASH_SIG: Cell<(usize, usize, usize)> = const { Cell::new((0, 0, 0)) };
}

fn ground_at(x: f64, y: f64) -> Option<f64> {
    read_registered_sampler().and_then(|s| s(x, y))
}

fn to_object_verdict(
    los: &WorldLos,
    total_m: f64,
    label: Option<(String, String)>,
    glass_panes: u32,
) -> ObjectVerdict {
    let dist = |t: f64| t * total_m;
    match (los.verdict, &los.blocker) {
        (WorldVerdict::Blocked, Some(b)) => {
            let (label, kind) =
                label.unwrap_or_else(|| (format!("pid {}", b.pid), "object".into()));
            ObjectVerdict::Blocked {
                dist_m: dist(b.t),
                label,
                kind,
            }
        }
        (WorldVerdict::Provisional, Some(b)) => ObjectVerdict::Provisional {
            dist_m: dist(b.t),
            label: label.map_or_else(|| format!("pid {}", b.pid), |(l, _)| l),
        },
        (WorldVerdict::Provisional, None) | (WorldVerdict::Blocked, None) => {
            ObjectVerdict::NotLoaded
        }
        (WorldVerdict::Clear, _) => ObjectVerdict::Clear {
            concealment: los.concealment,
            glass_panes,
        },
    }
}

/// The object layer's verdict for a placed shot, through the live occluder. `NotLoaded` before
/// the engine mounts, while the host is taken mid-settle, or when the DEM has no ground for an
/// endpoint.
#[must_use]
pub fn object_verdict(shot: &LosShot) -> ObjectVerdict {
    let oz = shot.obs_z.or_else(|| ground_at(shot.obs_x, shot.obs_y));
    let tz = shot.tgt_z.or_else(|| ground_at(shot.tgt_x, shot.tgt_y));
    let (Some(oz), Some(tz)) = (oz, tz) else {
        return ObjectVerdict::NotLoaded;
    };
    let obs = map_to_engine(shot.obs_x, shot.obs_y, oz + EYE_HEIGHT_OBSERVER_M);
    let tgt = map_to_engine(shot.tgt_x, shot.tgt_y, tz + EYE_HEIGHT_TARGET_M);
    let total = shot.distance_m();
    with_occluder(|occ| {
        let los = occ.evaluate_los(obs, tgt);
        let label = los.blocker.as_ref().map(|b| {
            (
                occ.label_of(b.pid).unwrap_or("object").to_string(),
                occ.kind_of(b.pid).unwrap_or("object").to_string(),
            )
        });
        // A clear shot names its panes: one cheap trace (µs) only when something conceals.
        let glass_panes = if los.verdict == WorldVerdict::Clear && los.concealment >= 0.005 {
            #[allow(clippy::cast_possible_truncation)]
            // Both faces of a pane report a Glass event: count panes, not faces.
            let panes: std::collections::HashSet<(String, u32, Owner)> = occ
                .trace(obs, tgt)
                .0
                .iter()
                .filter(|e| e.kind == SurfaceKind::Glass)
                .map(|e| (e.chunk.clone(), e.row, e.inner))
                .collect();
            panes.len() as u32
        } else {
            0
        };
        to_object_verdict(&los, total, label, glass_panes)
    })
    .unwrap_or(ObjectVerdict::NotLoaded)
}

/// Start the object wash over the raster the viewshed state holds (called right after
/// `place_viewshed`). A previous wash is replaced; its generation is retired.
pub fn start_object_wash() {
    let st = read_registered_viewshed();
    let (Some(vs), Some((ox, oy, oz))) = (st.raster, st.observer) else {
        cancel_object_wash();
        return;
    };
    let Some(ground) = oz.or_else(|| ground_at(ox, oy)) else {
        cancel_object_wash();
        return;
    };
    let generation = GENERATION.with(|g| {
        g.set(g.get().wrapping_add(1));
        g.get()
    });
    let pass = ObjectPass::new(&vs, generation);
    WASH.with(|w| {
        *w.borrow_mut() = Some(Wash {
            pass,
            vs,
            eye_z: ground + EYE_HEIGHT_OBSERVER_M,
        });
    });
    LAST_UPLOAD_MS.with(|l| l.set(0.0));
}

/// Drop the wash (Esc / sub-mode or tool switch — the viewshed lane is cleared by the caller).
pub fn cancel_object_wash() {
    GENERATION.with(|g| g.set(g.get().wrapping_add(1)));
    WASH.with(|w| *w.borrow_mut() = None);
}

fn cell_test(obs: [f64; 3], tgt: [f64; 3]) -> Option<ObjectCell> {
    with_occluder(|occ| {
        let r = occ.evaluate_los(obs, tgt);
        match r.verdict {
            WorldVerdict::Blocked => ObjectCell::Hidden,
            WorldVerdict::Provisional => ObjectCell::Provisional,
            WorldVerdict::Clear => {
                if r.concealment >= 0.02 {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    ObjectCell::Concealed((r.concealment.clamp(0.0, 1.0) * 255.0) as u8)
                } else {
                    ObjectCell::Clear
                }
            }
        }
    })
}

/// One rAF tick: advance the wash under its budget and re-upload the merged wash when it
/// progressed (at most every [`OBJECT_UPLOAD_INTERVAL_MS`], and once more when it completes).
pub fn tick_object_wash(e: &mut RenderEngine) {
    WASH.with(|w| {
        let mut guard = w.borrow_mut();
        let Some(wash) = guard.as_mut() else {
            return;
        };
        if wash.pass.done {
            // T-090.12.5 — a finished wash retests its provisional cells when the occluder's
            // residency signature moves (descriptors / BLAS fetched after the pass ran).
            let sig = with_occluder(|o| (o.chunk_count(), o.expanded_count(), o.blas_count()));
            let Some(sig) = sig else {
                return;
            };
            if WASH_SIG.with(|w| w.get()) == sig {
                return;
            }
            WASH_SIG.with(|w| w.set(sig));
            if wash.pass.requeue_provisional(&wash.vs) == 0 {
                return;
            }
        }
        let now = js_sys::Date::now;
        let ground = |x: f64, y: f64| ground_at(x, y);
        let changed = wash.pass.step(
            &wash.vs,
            wash.eye_z,
            &ground,
            &cell_test,
            OBJECT_PASS_BUDGET_MS,
            &now,
        );
        let t = now();
        let due = LAST_UPLOAD_MS.with(|l| t - l.get() >= OBJECT_UPLOAD_INTERVAL_MS);
        if changed && (due || wash.pass.done) {
            let tight = encode_viewshed_rgba_merged(&wash.vs, &wash.pass);
            let (rgba, stride) = pack_rgba_256(&tight, wash.vs.cols, wash.vs.rows);
            #[allow(clippy::cast_possible_truncation)]
            let _ = e.viewshed_upload(
                wash.vs.min_x,
                wash.vs.min_y,
                wash.vs.max_x,
                wash.vs.max_y,
                wash.vs.cols as u32,
                wash.vs.rows as u32,
                &rgba,
                stride,
            );
            LAST_UPLOAD_MS.with(|l| l.set(t));
        }
        if wash.pass.done {
            let sig = with_occluder(|o| (o.chunk_count(), o.expanded_count(), o.blas_count()));
            if let Some(sig) = sig {
                WASH_SIG.with(|w| w.set(sig));
            }
        }
    });
}

/// HUD suffix: the occluder's residency + the wash progress (empty when neither is live).
#[must_use]
pub fn hud_suffix() -> String {
    let occ = with_occluder(|occ| {
        format!(
            " · occl {}ch/{}bvh {:.0}MB",
            occ.chunk_count(),
            occ.blas_count(),
            occ.memory_bytes() as f64 / 1_048_576.0
        )
    })
    .unwrap_or_default();
    let wash = WASH
        .with(|w| {
            w.borrow().as_ref().map(|wash| {
                let (tested, queued, cursor) = wash.pass.progress();
                if wash.pass.done {
                    format!(" · wash done ({tested})")
                } else {
                    format!(" · wash {:.0}m {cursor}/{queued}", wash.pass.level_m())
                }
            })
        })
        .unwrap_or_default();
    format!("{occ}{wash}")
}

/// JSON snapshot of the live wash for the headless probes — `null` when no wash is live.
#[must_use]
pub fn wash_status_json() -> String {
    // The occluder's residency alongside the census: `pendingBlas` / `pendingDesc` separate
    // "BLAS still in flight" from "segment crossed a chunk residency never loaded" (both are
    // provisional cells; only the first resolves without a camera move).
    let occ = with_occluder(|o| {
        let ids = o.resident_chunk_ids();
        let want = o.wanted(&ids, usize::MAX);
        (
            o.chunk_count(),
            o.expanded_count(),
            o.blas_count(),
            want.descriptors.len(),
            want.blas.len(),
        )
    })
    .unwrap_or((0, 0, 0, 0, 0));
    let (failed, failed_sample) = with_occluder_host(|h| h.failed_summary()).unwrap_or_default();
    let failed_sample = serde_json::to_string(&failed_sample).unwrap_or_else(|_| "[]".into());
    WASH.with(|w| {
        w.borrow().as_ref().map_or_else(
            || "null".to_string(),
            |wash| {
                let (tested, queued, cursor) = wash.pass.progress();
                let (clear, hidden, concealed, provisional, untested) = wash.pass.counts();
                // Why is the first provisional cell provisional? Its coverage, verbatim.
                let sample = wash
                    .pass
                    .cells
                    .iter()
                    .position(|c| *c == ObjectCell::Provisional)
                    .and_then(|i| {
                        let (col, row) = (i % wash.pass.cols, i / wash.pass.cols);
                        let (x, y) = wash.pass.cell_center(col, row);
                        let g = ground_at(x, y)?;
                        let obs = map_to_engine(wash.pass.obs_x, wash.pass.obs_y, wash.eye_z);
                        let tgt = map_to_engine(x, y, g + EYE_HEIGHT_TARGET_M);
                        with_occluder(|o| {
                            let r = o.evaluate_los(obs, tgt);
                            format!(
                                "{{\"x\":{x},\"y\":{y},\"verdict\":\"{:?}\",\"crossed\":{},\"missing\":{},\"proxyPids\":{},\"blasPending\":{}}}",
                                r.verdict,
                                r.coverage.chunks_crossed,
                                serde_json::to_string(&r.coverage.chunks_missing).unwrap_or_default(),
                                serde_json::to_string(&r.coverage.proxy_pids).unwrap_or_default(),
                                serde_json::to_string(&r.coverage.blas_pending).unwrap_or_default(),
                            )
                        })
                    })
                    .unwrap_or_else(|| "null".into());
                format!(
                    "{{\"done\":{},\"generation\":{},\"levelM\":{},\"tested\":{tested},\"queued\":{queued},\"cursor\":{cursor},\"clear\":{clear},\"hidden\":{hidden},\"concealed\":{concealed},\"provisional\":{provisional},\"untested\":{untested},\"chunks\":{},\"expanded\":{},\"blas\":{},\"pendingDesc\":{},\"pendingBlas\":{},\"failed\":{failed},\"failedSample\":{failed_sample},\"sample\":{sample}}}",
                    wash.pass.done,
                    wash.pass.generation,
                    wash.pass.level_m(),
                    occ.0,
                    occ.1,
                    occ.2,
                    occ.3,
                    occ.4,
                )
            },
        )
    })
}

/// Install `window.__editorObjectWash()` — a read-only smoke bridge (peer of `__editorCam`)
/// returning [`wash_status_json`], so a headless probe can wait for the object wash to finish
/// and read its census + wall time. Leaks one closure for the page lifetime like its peers.
pub fn register_object_wash_hook() {
    let Some(win) = web_sys::window() else {
        return;
    };
    let f = Closure::<dyn Fn() -> JsValue>::new(|| JsValue::from_str(&wash_status_json()));
    let _ = js_sys::Reflect::set(
        &win,
        &JsValue::from_str("__editorObjectWash"),
        f.as_ref().unchecked_ref(),
    );
    f.forget();
}
