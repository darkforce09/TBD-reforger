//! Role: the ruler overlay — the DOM/SVG surface the engine's ruler tool draws through — and the
//! install that ties every tool seam in this cluster to its mount.
//! Position: `editor/tools` in the frontend editor.
//! Signals & state: two host heartbeats (pan cursor, zoom sampler) plus a mutation tick, and the
//! leaked chain this module hands to the engine's registry.
//! Invariants: every decidable thing — the leg quantities, the readout shapes, the capture machine
//! and the projection — belongs to `website_map_engine::editing::tools::ruler`. What lives here is
//! the drawing and the ownership.
//!
//! The overlay re-runs off the same cursor and zoom heartbeats the LoS overlay and scale bar use —
//! never a frame loop of its own. Native builds render an empty overlay (no engine, no `window`).
#![allow(dead_code)] // the wasm host wires the live path; on native this overlay draws nothing.

use leptos::prelude::*;

use website_map_engine::editing::tools::ruler::chain::RulerChain;
use website_map_engine::editing::tools::ruler::host_registry::{
    read_registered_chain, RULER_CHAIN,
};
use website_map_engine::editing::tools::ruler::projection::{
    project_legs, project_vertices, ProjectedLeg, ProjectedVertex,
};
use website_map_engine::editing::tools::selection;

/* ═══════ the seam idiom for this editor-tool cluster — install at mount, unregister at unmount ═════
 *
 * The lifecycle contract for the five seams this cluster owns (`RULER_CHAIN`; `los_tool`'s
 * `LOS_STATE` / `LOS_SAMPLER` / `VIEWSHED_STATE`; `world_assets`'s `RENDER_CTX`) is not written here.
 * **It is `validation_panel`'s, and there is exactly one of it** — the same body that serves that
 * file's four seams, sitting beside the `SeamRegistration` identity trait it depends on.
 *
 * T-778 shipped a COPY of the six-line mechanism at this spot, because those two functions were
 * module-private and `validation_panel` was not that slice's to widen. Only the identity vocabulary
 * was shared, so the crate carried one definition of "is the value in the cell the very registration
 * I put there" and two mechanisms asking it. T-783 widened them to `pub(crate)` and deleted the copy.
 *
 * The `use` below is a RE-EXPORT, not a second definition: `los_tool` and `world_assets` import
 * `crate::editor::tools::ruler_tool::install_seam`, and that path still resolves — to `validation_panel`'s body.
 * `world_assets` is `#[cfg(target_arch = "wasm32")]` while this file and `validation_panel` are
 * declared unconditionally in `main.rs`, so the single definition is reachable from every consumer on
 * BOTH targets.
 *
 * THE DEFECT it guards (wave-129 F2/F5, third recurrence — T-778). A seam registered at mount and
 * never unregistered stays READABLE after the surface that owns it is gone: Backspace hide-chrome
 * unmounts panels while dialogs deliberately survive, and SPA navigation drops the whole editor page.
 * The stale handle then reports SUCCESS — a non-empty chain, a `Some` sampler — while every `set`
 * behind it lands on a DISPOSED signal, which `reactive_graph` 0.2.14 makes a silent no-op. The
 * operator sees a click that "worked" and nothing happened.
 *
 * The naive fix closes only half of it. An UNCONDITIONAL unregister at cleanup introduces the mirror
 * defect: leptos does not guarantee that a dying owner's cleanup runs before the REMOUNT registers, so
 * an old cleanup can delete the LIVE surface's seam and leave it dead again. Hence the identity guard
 * in `validation_panel::unregister_seam` — only the LOSING registration is cleared. Only the entry
 * point is re-exported; the guard is that function's private business and no caller here names it.
 */
pub(crate) use crate::editor::panels::validation_panel::install_seam;

/// Hand the host's leaked ruler chain to the engine's registry so [`RulerOverlay`] can read it.
///
/// **This is an INSTALL** ([`install_seam`]): the chain is unregistered when the owner that
/// registered it is cleaned up, and a remount's newer chain is not clobbered by the old owner's
/// cleanup. Without that, the registry would keep returning a dead page's polyline as though the
/// ruler were still live.
pub fn register_ruler_chain(chain: std::rc::Rc<std::cell::RefCell<RulerChain>>) {
    install_seam(&RULER_CHAIN, chain);
}

/// T-642 — the ruler overlay. ONE absolutely-positioned, `pointer-events-none` SVG spanning the
/// viewport, drawing the placed polyline + per-leg on-line labels (Decision 1) + vertex dots + the
/// live rubber-band leg to the cursor while drawing. It reads the live camera off the registered
/// engine (`world_assets::camera_snapshot`, the same seam the scale bar / grid refs use) and re-runs
/// off the `cursor` (pan) + `debug_hud` (~1 Hz zoom) heartbeats — NO new rAF loop. `tick` is bumped
/// by the host on every chain mutation (press / Esc / dbl-click) so a click that does not move the
/// pointer still repaints.
///
/// The chain itself is read via [`read_registered_chain`] — a cheap clone of the leaked host
/// `RulerChain` (session-local overlay state — Decision 4). Native builds render an empty overlay
/// (no engine, no `window`); the geometry is proven by `project_legs`/`world_key` above.
#[component]
pub fn RulerOverlay(
    /// Pan heartbeat — the editor's pointer-move cursor write (drives the pan re-projection). Also
    /// the live cursor world point the rubber-band leg draws to while the chain is `drawing`.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat — the rAF debug sampler (a wheel-zoom with a still pointer still
    /// re-projects within a second). `Option` so the mount can forward `Some(debug_hud)`.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped by the host on every chain mutation so a click repaints even with a still pointer.
    tick: RwSignal<u64>,
) -> impl IntoView {
    // (legs, vertices, rubber-band [(x1,y1,x2,y2)]) projected through the live camera for the
    // current chain + camera state. The rubber-band is the un-committed leg from the last vertex to
    // the live cursor while `drawing` (a single-element vec, or empty).
    #[allow(clippy::type_complexity)]
    let projected =
        move || -> (Vec<ProjectedLeg>, Vec<ProjectedVertex>, Vec<(f64, f64, f64, f64)>) {
            // Subscribe to all three heartbeats so the closure re-runs on pan (cursor), zoom (hud)
            // and any chain edit (tick).
            let cur = cursor.get();
            if let Some(h) = debug_hud {
                let _ = h.get();
            }
            let _ = tick.get();
            let chain = read_registered_chain();
            if chain.is_empty() {
                return (Vec::new(), Vec::new(), Vec::new());
            }
            #[cfg(target_arch = "wasm32")]
            {
                let Some((tx, ty, zoom)) = website_map_engine::streaming::host::camera_snapshot() else {
                    return (Vec::new(), Vec::new(), Vec::new());
                };
                let Some(win) = web_sys::window() else {
                    return (Vec::new(), Vec::new(), Vec::new());
                };
                let vw = win.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                let vh = win.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
                if vw <= 0.0 || vh <= 0.0 {
                    return (Vec::new(), Vec::new(), Vec::new());
                }
                // The canvas is full-bleed (like MapGridRefs), so the camera viewport IS the whole
                // window; build it exactly as `select_tool::frozen_camera` does.
                let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
                let project = move |x: f64, y: f64| {
                    let p = cam.project([x, y, 0.0]);
                    (p[0], p[1])
                };
                // Rubber-band: last committed vertex → live cursor, only while drawing and with a
                // live on-map cursor. It previews the leg-to-be so the operator aims the next click.
                let mut rubber = Vec::new();
                if chain.drawing {
                    if let (Some(last), Some((cwx, cwy, _))) = (chain.points.last().copied(), cur) {
                        let (x1, y1) = project(last.x, last.y);
                        let (x2, y2) = project(cwx, cwy);
                        rubber.push((x1, y1, x2, y2));
                    }
                }
                (
                    project_legs(&chain, &project),
                    project_vertices(&chain, &project),
                    rubber,
                )
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (chain, cur);
                (Vec::new(), Vec::new(), Vec::new())
            }
        };
    view! {
        // Full-bleed, non-interactive SVG. z-10 sits it in the same overlay band as MapGridRefs so
        // it draws over the map but under the chrome docks; `pointer-events-none` so it never eats a
        // map gesture (the click-chain capture is the map's own pointer handlers, not this SVG).
        <svg
            data-ruler-overlay
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            // Placed polyline legs — one <line> per committed leg, keyed by world coordinate (T-727).
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <line
                    x1=move || format!("{:.1}", l.x1)
                    y1=move || format!("{:.1}", l.y1)
                    x2=move || format!("{:.1}", l.x2)
                    y2=move || format!("{:.1}", l.y2)
                    class="stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            // Rubber-band preview — the un-committed leg from the last vertex to the live cursor
            // (dashed, to read as provisional vs the solid committed legs). One or zero lines.
            <For
                each=move || projected().2
                key=|r| format!("{:.0}:{:.0}:{:.0}:{:.0}", r.0, r.1, r.2, r.3)
                let:r
            >
                <line
                    x1=move || format!("{:.1}", r.0)
                    y1=move || format!("{:.1}", r.1)
                    x2=move || format!("{:.1}", r.2)
                    y2=move || format!("{:.1}", r.3)
                    class="stroke-primary/60"
                    stroke-width="1.5"
                    stroke-dasharray="4 4"
                />
            </For>
            // Vertex dots — a small ring at each committed point, keyed by world coordinate.
            <For
                each=move || projected().1
                key=|v| v.key.clone()
                let:v
            >
                <circle
                    cx=move || format!("{:.1}", v.px)
                    cy=move || format!("{:.1}", v.py)
                    r="3"
                    class="fill-surface-container-lowest stroke-primary"
                    stroke-width="1.5"
                />
            </For>
            // On-line leg labels (Decision 1) — horizontal, centred on the leg mid-point, keyed by
            // world coordinate so an identical label string on two legs never shares a node (T-727).
            <For
                each=move || projected().0
                key=|l| l.key.clone()
                let:l
            >
                <text
                    x=move || format!("{:.1}", l.mid_x)
                    y=move || format!("{:.1}", l.mid_y - 4.0)
                    text-anchor="middle"
                    class="fill-primary font-mono text-code-md"
                    // A faint halo so the label reads over any basemap (paint-order: stroke first).
                    stroke="rgba(0,0,0,0.55)"
                    stroke-width="3"
                    style="paint-order:stroke"
                >
                    {l.label.clone()}
                </text>
            </For>
        </svg>
    }
}

// ── Tests: the chain state machine, per-leg math goldens, formatter goldens, label keying, and the
//    fired bearing rule. Native (`cargo test -p website-frontend`) — no browser/engine. ───────────

/* ══ T-778 — every editor-tool seam is unregistered at unmount, and no remount is clobbered ═════════
 *
 * The lifecycle half of the dead click (wave-129 F2/F5), pinned across the FOUR natively-compiled
 * thread_local seams of this tool cluster at once: `RULER_CHAIN` here, plus `los_tool`'s `LOS_STATE`,
 * `LOS_SAMPLER` and `VIEWSHED_STATE`. Table-driven for the same reason
 * `validation_panel::f5_seam_lifecycle` is: this defect has now been fixed FOUR times in three files,
 * and each time a nearby seam shipped without the fix. A fifth seam that forgets [`install_seam`]
 * joins this table and goes red, rather than being found by the next reader.
 *
 * These drive real `Owner`s and call `Owner::cleanup` — the code path leptos runs at unmount — in the
 * three shapes that matter:
 *   1. never installed                    -> the seam reports HONEST FAILURE (the baseline, so a
 *                                            green elsewhere cannot be "it was already empty");
 *   2. install -> cleanup                 -> FAILURE, and the STALE registration is not read at all;
 *   3. install(A) -> install(B) -> A's cleanup -> B SURVIVES and still answers (the identity guard's
 *      entire reason for existing: leptos does not guarantee that a dying owner's cleanup runs before
 *      the remount registers).
 *
 * The two owners in shape 3 are SIBLINGS (`root.child()` twice), never parent/child — a child would
 * be cleaned up BY its parent and the test would measure the tree instead of the guard.
 *
 * Perturbation RED, and they redden DIFFERENTLY, which is the point: drop the `on_cleanup` from
 * [`install_seam`] and shape 2 goes red; keep the cleanup but make it unconditional (delete the
 * `is_same_registration` guard from `unregister_seam`) and shape 3 goes red ALONE — that is the
 * failure a naive fix ships. Since T-783 both live in `validation_panel`, so either perturbation is
 * one edit that reddens this table AND `validation_panel`'s together — they share the body now.
 *
 * The FIFTH seam, `world_assets::RENDER_CTX`, cannot join this table: `world_assets` is declared
 * `#[cfg(target_arch = "wasm32")]` in `main.rs` and its handles wrap a live `RenderEngine`/`MapHost`,
 * so a native `cargo test` never compiles it. It is covered by [`the_render_ctx_seam_is_installed`]
 * below — a Class-R pin over the SCRUBBED production half of that file, so neither the prose in its
 * comments nor a test module can satisfy it.
 */
#[cfg(test)]
mod t778_seam_lifecycle {
    use super::register_ruler_chain;
    use crate::editor::tools::los_tool::{
        register_los_sampler, register_los_state, register_viewshed_state,
    };
    use leptos::prelude::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use website_map_engine::editing::tools::line_of_sight::capture::{LosState, ViewshedState};
    use website_map_engine::editing::tools::line_of_sight::host_registry::{
        read_registered_sampler, read_registered_state, read_registered_viewshed,
    };
    use website_map_engine::editing::tools::ruler::{
        read_registered_chain, RulerChain, RulerPoint,
    };

    thread_local! {
        /// Every tag that ANSWERED a seam's question, in call order. "Did anything actually happen"
        /// is answered by WHICH registration answered, not only by the seam's boolean — a seam that
        /// reports failure while still reading a dead handle has not been fixed.
        static ANSWERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn note(tag: &str) {
        ANSWERED.with(|l| l.borrow_mut().push(tag.to_string()));
    }
    fn answered() -> Vec<String> {
        ANSWERED.with(|l| l.borrow().clone())
    }
    fn forget_answers() {
        ANSWERED.with(|l| l.borrow_mut().clear());
    }

    /// The world X a given mount registers. Both are NON-ZERO and distinct, so "the `Default` empty
    /// state answered" (the honest failure) can never be mistaken for "a registration answered", and
    /// the two mounts can never be mistaken for each other.
    fn mark(tag: &str) -> f64 {
        match tag {
            "A" => 1000.0,
            "B" => 2000.0,
            other => unreachable!("unknown mount tag {other}"),
        }
    }

    /// Decode a read-back world X into the mount that registered it, and record that it ANSWERED.
    fn note_mark(x: f64) {
        let tag = ["A", "B"]
            .into_iter()
            .find(|t| (mark(t) - x).abs() < f64::EPSILON)
            .unwrap_or("<unregistered value>");
        note(tag);
    }

    /// One seam, reduced to the two operations its lifecycle turns on.
    struct Seam {
        /// The thread_local's name, so a failure names the seam rather than a row index.
        name: &'static str,
        /// Register a `tag`-marked value under the CURRENT reactive owner.
        install: fn(&'static str),
        /// Ask the seam its OWN question. `true` = a live registration answered (and `note`d itself).
        ask: fn() -> bool,
    }

    /// Every natively-compiled seam in this cluster. A new one belongs here.
    fn seams() -> [Seam; 4] {
        [
            Seam {
                name: "RULER_CHAIN",
                install: |tag| {
                    register_ruler_chain(Rc::new(RefCell::new(RulerChain {
                        points: vec![RulerPoint::new(mark(tag), 0.0, None)],
                        ..RulerChain::default()
                    })));
                },
                ask: || match read_registered_chain().points.first() {
                    Some(p) => {
                        note_mark(p.x);
                        true
                    }
                    None => false,
                },
            },
            Seam {
                name: "LOS_STATE",
                install: |tag| {
                    register_los_state(Rc::new(RefCell::new(LosState {
                        pending_obs: Some((mark(tag), 0.0, None)),
                        ..LosState::default()
                    })));
                },
                ask: || match read_registered_state().pending_obs {
                    Some((x, _, _)) => {
                        note_mark(x);
                        true
                    }
                    None => false,
                },
            },
            Seam {
                name: "LOS_SAMPLER",
                // The one seam that is a real closure: it notes its own tag when CALLED, so shape 2
                // can distinguish "the seam reported failure" from "the stale closure still ran".
                install: |tag| {
                    register_los_sampler(Rc::new(move |_x, _y| {
                        note(tag);
                        Some(mark(tag))
                    }));
                },
                ask: || match read_registered_sampler() {
                    Some(f) => f(0.0, 0.0).is_some(),
                    None => false,
                },
            },
            Seam {
                name: "VIEWSHED_STATE",
                install: |tag| {
                    register_viewshed_state(Rc::new(RefCell::new(ViewshedState {
                        observer: Some((mark(tag), 0.0, None)),
                        ..ViewshedState::default()
                    })));
                },
                ask: || match read_registered_viewshed().observer {
                    Some((x, _, _)) => {
                        note_mark(x);
                        true
                    }
                    None => false,
                },
            },
        ]
    }

    /// Shape 1 — never installed. The baseline: without it, a green in shape 2 could just mean the
    /// seam never worked in the first place.
    #[test]
    fn an_uninstalled_seam_reports_honest_failure() {
        let _root = Owner::new();
        for seam in seams() {
            forget_answers();
            assert!(
                !(seam.ask)(),
                "T-778 {}: a seam nothing ever registered must report FAILURE",
                seam.name
            );
            assert!(
                answered().is_empty(),
                "T-778 {}: nothing may answer when nothing is installed — got {:?}",
                seam.name,
                answered()
            );
        }
    }

    /// Shape 2 — install then unmount. The seam must report failure AND not read the dead handle.
    #[test]
    fn a_seam_is_unregistered_when_its_owner_is_cleaned_up() {
        let root = Owner::new();
        for seam in seams() {
            let mounted = root.child();
            mounted.with(|| (seam.install)("A"));

            forget_answers();
            assert!(
                (seam.ask)(),
                "T-778 {} precondition: while mounted the seam really does answer",
                seam.name
            );
            assert_eq!(
                answered(),
                vec!["A".to_string()],
                "T-778 {} precondition: the LIVE registration is the one that answered",
                seam.name
            );

            mounted.cleanup();

            forget_answers();
            assert!(
                !(seam.ask)(),
                "T-778 {}: the installing owner is gone, so the seam must report FAILURE rather \
                 than success over state whose every write is a disposed no-op",
                seam.name
            );
            assert!(
                answered().is_empty(),
                "T-778 {}: the stale registration must not be read at all after unmount — got {:?}",
                seam.name,
                answered()
            );
        }
    }

    /// Shape 3 — the identity guard. A remount installs its NEWER value before the old owner's
    /// cleanup runs. The losing cleanup must recognise it is no longer the live registration and
    /// leave the new one alone — otherwise the fix for a stale seam becomes a fresh way to kill a
    /// live one, and the click is dead again. This is the case an unconditional unregister fails.
    #[test]
    fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
        let root = Owner::new();
        for seam in seams() {
            // Siblings, not parent/child: two successive mounts under the page owner. A child would
            // be cleaned up BY the parent and would prove nothing about the guard.
            let old = root.child();
            let new = root.child();
            old.with(|| (seam.install)("A"));
            new.with(|| (seam.install)("B"));

            old.cleanup();

            forget_answers();
            assert!(
                (seam.ask)(),
                "T-778 {}: the NEW mount is live — the superseded owner's cleanup must not \
                 unregister it",
                seam.name
            );
            assert_eq!(
                answered(),
                vec!["B".to_string()],
                "T-778 {}: the surviving registration must be the NEWER one, not a leftover that \
                 merely happens to answer",
                seam.name
            );

            new.cleanup();

            forget_answers();
            assert!(
                !(seam.ask)(),
                "T-778 {}: the live mount's OWN cleanup does clear it — the guard skips losers, \
                 not everyone",
                seam.name
            );
        }
    }

    /// Class-R — the fifth seam, `world_assets::RENDER_CTX`, which is wasm-only and so invisible to
    /// every test above. Pinned over `live_code`, which cuts the test module, the comments AND the
    /// string literals: the needles below therefore have to be real calls in the production body,
    /// not the prose two lines above them nor a decoy in a string.
    ///
    /// [wave 142 F-3] TIGHTENED. The pin reddened on the honest regression (the pre-T-778 direct
    /// write) but GREENED on a decoy: rename a local to `install_seam_later` and write the cell with
    /// `RefCell::replace` instead of `borrow_mut`, and both needles were satisfied while the
    /// un-unregisterable registration was back. `install_seam` was a bare substring, and the negative
    /// named ONE of the several ways to write a `RefCell`. Two changes close it:
    ///
    /// * the positive names the CALL and its ARGUMENT — the seam has to be installed on THIS cell,
    ///   which no local's name can satisfy;
    /// * the negative forbids reaching the cell at all from this body. `install_seam(&RENDER_CTX, …)`
    ///   passes the cell; it never opens it. So `RENDER_CTX.with` inside `register_render_ctx` means
    ///   a hand-rolled registration by definition, whatever mutator it then reaches for — and the
    ///   three write shapes are forbidden by name as well, so the failure message says which one.
    ///
    /// This seam has no behavioural test anywhere (`world_assets` is wasm32-only with no
    /// wasm-bindgen-test target), so this pin is its only guarantee. That is exactly the case where
    /// the standard Class-R substring ceiling is worth paying to raise.
    #[test]
    fn the_render_ctx_seam_is_installed() {
        use crate::v2::core::test_support::class_r_scrub::{live_code, only_body, only_item};
        let src = live_code(include_str!("../world_assets/mod.rs"));

        let body = only_body(&src, "pub fn register_render_ctx(");
        // The call AND the cell it installs on — `install_seam_later = ()` does not contain this.
        let install = ["install_seam(&", "RENDER_CTX"].concat();
        assert!(
            body.contains(&install),
            "T-778: register_render_ctx must INSTALL on RENDER_CTX (register + guarded unregister at \
             the owner's cleanup), not write the cell directly; got:\n{body}"
        );
        // `install_seam` takes the cell; it never opens it. Opening it here is a hand-rolled
        // registration whichever mutator follows — `borrow_mut`, `replace`, `take`, `set`.
        let opens_cell = ["RENDER_CTX", ".with"].concat();
        for forbidden in [opens_cell.as_str(), "borrow_mut", ".replace(", ".take("] {
            assert!(
                !body.contains(forbidden),
                "T-778: register_render_ctx must not reach into RENDER_CTX behind install_seam's \
                 back (`{forbidden}`) — a bare write is the un-unregisterable registration this \
                 ticket removes; got:\n{body}"
            );
        }

        // The tuple-valued seam needs its own identity, and it must be Rc IDENTITY on both handles —
        // never a `usize` address (ABA) and never `||`, which would let a half-matching cleanup clear
        // a live remount.
        let ident = only_item(&src, "fn is_same_registration(");
        assert!(
            ident.contains("Rc::ptr_eq") && ident.contains("&&"),
            "T-778: RENDER_CTX identity must be Rc::ptr_eq on BOTH leaked handles; got:\n{ident}"
        );
    }
}
