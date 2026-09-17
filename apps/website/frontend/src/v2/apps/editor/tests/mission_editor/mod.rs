#[path = "source.rs"]
pub(super) mod source;

use super::*;

// Names only the evacuated `#[cfg(test)]` pins still reach through `super::…` (their shipping
// callers sit beside the definitions, in the engine's lanes and in the hover policy). This `use`
// lives HERE, at the file's test boundary, because the Class-R scrubber and the keymap census both
// treat the FIRST literal `#[cfg(test)]` as "everything after this is test fixture" — a test-gated
// import up top would truncate every scrub of this file to nothing.
#[cfg(test)]
pub(crate) use crate::v2::apps::editor::bridge::pointer_hover::{
    HOVER_CURSOR_PICKABLE, HOVER_CURSOR_PLAIN, HOVER_RELEASE_PX, HOVER_THROTTLE_MS,
};
#[cfg(test)]
pub(crate) use website_map_engine::editing::lanes::connections::{
    CONN_LINE_RGBA, CONN_LINE_SELECTED_RGBA,
};
#[cfg(test)]
pub(crate) use website_map_engine::editing::selection_universe::{
    crewed_slot_ids, map_render_keep_indices,
};

// Same discipline: only `t628_boot_progress` still reaches this constant through `super::…`;
// its one shipping consumer, `hand_over`, sits beside it in `bridge/boot.rs`.
#[cfg(test)]
pub(crate) use crate::v2::apps::editor::bridge::boot::BOOT_HANDOVER_MS;

#[cfg(test)]
#[path = "../t245_registry_session.rs"]
mod t245_registry_session;

/// T-750 — registry fetch Err raises a terminal failure signal the Favourites panel can read.
///
/// Wave-114 MINOR-2: Err only set `catalog`/`vehicle_catalog` to Failed and left `registry_items`
/// at None, so Favourites spun on "Resolving…" forever. Pins run on `live_code` (comments +
/// string literals blanked; test module cut) so a hollow note cannot green them. The helper is
/// host-visible on purpose — the wasm32 Err arm is scrubbed on native, but the call site in the
/// raw page still names it (asserted separately with a fragment-assembled needle).
#[cfg(test)]
#[path = "../t750_registry_fetch_failure_signal.rs"]
mod t750_registry_fetch_failure_signal;

/// T-573 — the mixed-drag preview wiring.
///
/// **Why a source pin here and a behavioural test elsewhere.** The proof that the preview moves the
/// right vehicles is a real unit test on the real function —
/// `map_engine_core::slots_gpu::pack_vehicle_drag_preview`, native, driven directly. What cannot be
/// proven that way is the *wiring*: `mod select_tool` is `#[cfg(target_arch = "wasm32")]`
/// (`main.rs`) and `editor_ops` is `#![cfg(target_arch = "wasm32")]`, so no native test can call the
/// drag path, and `RenderEngine` needs a GPU device besides. That leaves "the host hands the WHOLE
/// mixed selection to both lanes" as the one claim only source can carry — so it is carried on the
/// fail-closed scrubber (T-601 `class_r_scrub`), not a grep: `live_code` deletes comments, string
/// literals, `#[cfg(any())]` items, `if false` blocks and code after an unconditional jump, and
/// `only_body` refuses a marker that matches zero or two or more items rather than guessing.
/// `the_preview_pin_rejects_every_dead_code_wrapper` below keeps that honest.
#[cfg(test)]
#[path = "../t573_mixed_drag_preview.rs"]
mod t573_mixed_drag_preview;

/// T-427 — cold path must not depend on the unbounded dual dump.
#[cfg(test)]
#[path = "../t427_cold_registry_path.rs"]
mod t427_cold_registry_path;

/// T-627/T-628 — the Mission Creator boot bar: one 0→100% journey over four measured segments, and
/// a satellite fetch that is concurrent without being unordered.
///
/// Everything the loader itself does is `#[cfg(target_arch = "wasm32")]` — `fetch_range` is
/// `gloo-net` over `web_sys`, and `mod world_assets` does not exist on the host at all — so no test
/// here fetches anything, and none pretends to. What these pins do cover is the whole class of bug
/// a host test *can* catch cheaply and a browser cannot catch at all until the operator is already
/// watching the wrong bar: the arithmetic that turns four differently-sized, differently-metered
/// segments into one number that never rewinds, and the reassembly that decides whether tile 3's
/// pixels land at tile 3's coordinates. The source pins at the bottom hold the wasm side to actually
/// routing through the code proved here, so it cannot drift back to a sweep, to a whole-file DEM
/// GET, or to a batch that discovers its own size after the fact, while these stay green.
#[cfg(test)]
#[path = "../t628_boot_progress.rs"]
mod t628_boot_progress;

/// T-631 — the boot overlay cannot fail SILENTLY. The engine-init failure itself is wasm-side
/// (`RenderEngine::create` needs a real GPU), but the state machine the overlay reads —
/// `BootPhase` and its `advance` fold — is pure and drives entirely here, which is exactly what
/// the acceptance clause allows ("a native test can still drive `BootPhase`/`BootEvent`
/// transitions directly"). These tests inject the failure the way the engine task does, assert the
/// overlay reaches `Failed { seg, reason }` carrying the ORIGINAL reason, and — the part that made
/// the real bug so nasty — assert that the concurrent doc-hydrate task's later, misleading
/// transitions (`LoadingMap`, then `Ready` via the hand-over) do NOT overwrite that reason.
#[cfg(test)]
#[path = "../t631_boot_failure_state.rs"]
mod t631_boot_failure_state;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::entity_selection;
/// Exercise the graphics crate's satellite arithmetic directly from native UI regression tests.
#[cfg(all(test, not(target_arch = "wasm32")))]
use website_map_engine::world::terrain::satellite::streamer as tbd_sat_pure;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../t629_satellite_resolution.rs"]
mod t629_satellite_resolution;

/// T-662 — the two input traps that gated the editor program: RMB eaten by pan, and Backspace
/// aliased to Delete.
///
/// **Why source pins.** Both traps live in code no native test can execute: the pointer/keydown
/// closures and the whole Eden view are `#[cfg(target_arch = "wasm32")]` (they call `RenderEngine`,
/// `web_sys` events, and `editor_ops`, all wasm-only), so the proof that the trap is gone is the
/// proof that the *wiring* changed. String-literal match arms (`"Backspace"`, `"Delete"`) are pinned
/// on the raw file because `live_code` blanks string literals; structural/no-comment claims (the pan
/// button guard, the contextmenu body, the mount gating) are pinned on `live_code`, which blanks
/// comments and dead code so a stale note or an `if false` wrapper cannot satisfy them.
#[cfg(test)]
#[path = "../t662_input_traps.rs"]
mod t662_input_traps;

/// T-635 — the debug HUD (telemetry) must (a) toggle behind Ctrl+Alt+D in the editor keydown,
/// honouring the editable-field guard; (b) default HIDDEN; (c) NO LONGER live inside the toolbelt's
/// `bottom-5 left-1/2` wrapper (so it cannot paint over the CUR/OBJ readouts) and stay gated so an
/// overlap is impossible; (d) keep the telemetry-vs-diagnostics distinction explicit in a comment.
#[cfg(test)]
#[path = "../t635_debug_hud.rs"]
mod t635_debug_hud;

/// T-647 — placement interactions: the Ctrl state machine (multi-place ↔ regroup), Alt = empty
/// vehicle, the double-click asset picker on empty ground, and the double-click→Attributes swap that
/// now reaches vehicles. All six ids are pinned on live source (comments stripped / string literals
/// blanked), because the doc-mutating half is wasm-only (`editor_ops` runs no native test) — the
/// wiring is the thing a native pin can prove, exactly as the T-573 / T-662 / T-635 modules do.
#[cfg(test)]
#[path = "../t647_placement_interactions.rs"]
mod t647_placement_interactions;

/// T-642 — source pins for the RULER click-chain wiring in the wasm pointer/keydown/dblclick
/// handlers (which a native test cannot execute; the pure state machine + math are event-tested in
/// `ruler_tool`). These pin the binding constraints the wave-106 verifier flagged (T-723) plus the
/// mount + the tool-mode arbitration entry, on scrubbed code (comments + strings blanked) so a
/// needle is real code, never a comment. The scrubber KEEPS `#[cfg(target_arch="wasm32")]` blocks
/// (undecided cfg), so the handler tokens are visible — the same reason t662 can pin inside them.
#[cfg(test)]
#[path = "../t642_ruler_wiring.rs"]
mod t642_ruler_wiring;

/// T-643 — source pins for the LINE-OF-SIGHT click-capture wiring in the wasm pointer/keydown/
/// dblclick handlers (which a native test cannot execute; the pure state machine + occlusion math are
/// unit-tested in `los_tool`). LoS deliberately REUSES the ruler's `LG::Ruler` gesture arm + Esc seam
/// (the "mode field on the ruler arm" the ticket sanctions, so no third `LeftGesture` variant is
/// added to the un-owned `select_tool`), so these pins prove that reuse is disciplined: the commit
/// routes by `tool_mode`, the Esc is the SHARED arm (not a second window listener — T-726), and the
/// overlay/state/sampler are mounted + registered. Scrubbed code (comments + strings blanked) so a
/// needle is real code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
#[path = "../t643_los_wiring.rs"]
mod t643_los_wiring;

/// T-644 (wave 110) — source pins for the VIEWSHED live entry point: the sub-mode is threaded through
/// the LoS button (toggle) and the pointer commit (route), a viewshed click computes + uploads the
/// wash to the engine lane, and the clear seams (Esc + tool/sub-mode switch) drop BOTH the state and
/// the GPU lane through the EXISTING shared seams — no new window listener (T-726 pending). The pure
/// `LosMode`/`ViewshedState`/`place_viewshed` core is unit-tested in `los_tool`; these prove the wasm
/// wiring a native test cannot execute. Scrubbed code (comments + strings blanked) so a needle is real
/// code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
#[path = "../t644_viewshed_wiring.rs"]
mod t644_viewshed_wiring;
// T-090.12.5 — the LOS object layer wiring (object wash start / tick / cancel, overlay verdict).
#[cfg(test)]
#[path = "../t090_12_world_los_wiring.rs"]
mod t090_12_world_los_wiring;

/// T-644 (wave 110) — source pins for the LoS button's SUB-MODE TOGGLE in `eden_toolbelt`: the ONE
/// LoS button re-click toggles Ray ⇆ Viewshed (`LosMode::toggled`) while LoS is already active, and
/// the button's title/label reflect the live sub-mode. The toolbar is a Leptos view (structural), so
/// this is pinned by SOURCE INSPECTION on scrubbed `eden_toolbelt.rs`, mirroring `t643`/`t668`.
#[cfg(test)]
#[path = "../t644_los_button_submode.rs"]
mod t644_los_button_submode;

/// T-648 — Transform: Shift-rotate, snap grid, transform widget + the Space collision decision.
///
/// The pure primitives (`transform` module) are proved BEHAVIOURALLY here — it is an ungated module
/// like `boot_progress`, so a native `cargo test -p website-frontend` (the command CI/the wave gate
/// runs) compiles and executes these, unlike a test placed beside `drag_delta` in the wasm-only
/// `select_tool`. The wasm wiring (the Shift-rotate gesture arm, the widget mount, the keydown
/// bindings, the included comment fix) is proved by SOURCE PINS on `live_code` (comments + dead code
/// stripped, so a stale note or an `if false` wrapper cannot satisfy them). The keydown CENSUS reads
/// all fifteen window-level editor keydowns across ten modules, including `input/pointer_gestures`,
/// as raw text.
#[cfg(test)]
#[path = "../t648_transform.rs"]
mod t648_transform;

/// T-655 — the validation panel wiring pins: the mount exists, its payload source is registered, it
/// re-evaluates off the `doc_tick` channel, it is ALWAYS ON (no debug flag), and it SURVIVES
/// hide-chrome (mounted OUTSIDE every `chrome_hidden` gate — the diagnostics doctrine). These scan
/// the comment-stripped page source (`live_code`) so the doc prose that mentions `chrome_hidden`
/// cannot false-match the gate check.
#[cfg(test)]
#[path = "../t655_validation_panel_wiring.rs"]
mod t655_validation_panel_wiring;

/* ═══════ T-761 — compile findings cleared on editor hydrate (wave-116 finding 3) ═══════════════
 *
 * The behavioural pin lives in validation_panel. This Class-R pin locks the PRODUCTION call site:
 * MissionEditorPage must clear after hydrate_from_server, or a client-side mission switch still
 * inherits the previous mission's build report.
 */
#[cfg(test)]
#[path = "../t761_compile_findings_cleared_on_hydrate.rs"]
mod t761_compile_findings_cleared_on_hydrate;

/* ═══════ T-754 — the click-to-select router resolves ZONES, and says so before it is clicked ═════
 *
 * Two families, as the ticket demands: unit tests over the pure resolution (it is `serde_json`-only,
 * so it RUNS natively — this is not a source scan pretending to be a behaviour test), and source pins
 * for the parts that are wiring (the closure is wasm-only and holds `!Send` handles).
 */
#[cfg(test)]
#[path = "../t754_router_resolves_zones.rs"]
mod t754_router_resolves_zones;

/* ════ wave 129 F6 — the affordance probe and the click may not answer different questions ════
 *
 * F1 built the probe out of `route_target`'s resolution alone; F2 made `route_select_zone` report
 * honestly that an unmounted Zones panel selects nothing. Each is right. TOGETHER they disagreed:
 * for a zone subject with the panel unmounted the probe said `true` (the row painted
 * `cursor-pointer`) and the click said `false` (nothing happened) — a dead click dressed as an
 * affordance, which is the T-754 MAJOR this wave was opened to kill.
 *
 * The fix is [`route_availability`]: ONE narrowing both seams go through. These pins defend the
 * INVARIANT, not the arm — the table below covers every target kind, so the next divergence (a new
 * arm that needs a seam, say) is red here whether or not it is a zone.
 */
#[cfg(test)]
#[path = "../wave129_f6_probe_and_click_cannot_disagree.rs"]
mod wave129_f6_probe_and_click_cannot_disagree;

// ─────────────────────── T-649 — Select All in view + Attributes multi-edit ───────────────────
/// Source pins for T-649. `map-engine-core` is linked natively with the `mission` feature ONLY
/// (`Cargo.toml`: `doc`/`camera` are `cfg(target_arch = "wasm32")` deps), and `select_tool` /
/// `editor_ops` are both wasm32-gated modules — so neither `OrthoCamera`, `SlotSoa` nor
/// `select_all_in_view` can be CALLED from a native `cargo test`. These pin the wiring the way the
/// rest of this file's editor contracts are pinned: on the live source, with string literals
/// blanked (`live_code`) wherever the shape rather than the text is the contract.
#[cfg(test)]
#[path = "../t649_select_all_and_multi_edit.rs"]
mod t649_select_all_and_multi_edit;

// ──────────────── T-669 — clipboard completion: cut + paste-at-original ───────────────────────
/// Source pins for T-669 (`ACTION-CUT-001`, `ACTION-PASTE-ORIG-001`). `editor_ops` is a wasm32-only
/// module, so neither `copy_selection` nor `paste_at_cursor` can be CALLED from a native
/// `cargo test`; these pin the WIRING the way the rest of this file's editor contracts are pinned —
/// on the live source, sliced to the keydown arm list so a needle can never self-match inside this
/// module (which lives in the same file as the arms it reads).
#[cfg(test)]
#[path = "../t669_clipboard_completion.rs"]
mod t669_clipboard_completion;

/// T-670 (`STATUS-ZOOM-001`) — the editor's half of the metres-per-pixel readout. `RenderEngine::
/// zoom()` is reachable only from the rAF sampler, so the editor owns the signal and the sampler
/// writes it. The sampler runs EVERY FRAME, which makes the write guard the load-bearing part of
/// this ticket: an unguarded `set` would dirty the status bar 60×/s and tank editor performance —
/// the exact class of regression the `rf <ms>` HUD cell exists to surface. These are Leptos view /
/// wasm-closure innards, so they are pinned by SOURCE INSPECTION on scrubbed code (the established
/// `t635`/`t636` pattern here); needles are assembled at run time so this module's own prose can
/// never satisfy them.
#[cfg(test)]
#[path = "../t670_scale_signal.rs"]
mod t670_scale_signal;

/// T-723 — event-SEQUENCE regressions for the armed-place root (wave-106 MAJOR-1/2/3,
/// wave-108 composition tooltip + Ruler strand, wave-109 LoS strand).
///
/// These drive `armed_place::step` / `run` — the same decide_* helpers the wasm handlers call.
/// Source pins are forbidden here: they could not see any of the three defects.
#[cfg(test)]
#[path = "../t723_armed_place.rs"]
mod t723_armed_place;

/// Wave-130 F1 / T-760 — `mission_history` is `#![cfg(target_arch = "wasm32")]`, so its feed cannot
/// host a native Class-R pin. Pin the two live call sites from here via `include_str!` + scrub, the
/// same way T-573 pins `vehicles_bind` / `select_tool` from this file.
#[cfg(test)]
#[path = "../t760_markers_bind_feed.rs"]
mod t760_markers_bind_feed;

/// **T-808 — the FEEDERS, pinned to the symbology signatures.**
///
/// T-808 built the engine half (`slots_bind_symbology`, `vehicles_bind_symbology`,
/// `comments_bind_ids`) but could not wire the callers, and an engine that can draw a medic facing
/// east is worth exactly nothing while the feeder still calls `slots_bind_soa`: every slot reads as
/// a rifleman pointing north, every vehicle as an amber disc, every note as unselected. These pins
/// hold the wiring — the same Class-R shape as the T-760 marker pin above, and for the same reason:
/// `mission_history` is `#![cfg(target_arch = "wasm32")]` end to end, so a scrubbed `include_str!`
/// is a native test's only reach into it.
#[cfg(test)]
#[path = "../t808_symbology_feed.rs"]
mod t808_symbology_feed;

/// T-726 — window-Esc pile-up: every editor Esc consumer consults the modal stack.
///
/// Defect (wave106 MINOR-2 / wave108 MAJOR-2 / wave109-110): separate window keydowns all fired on
/// one Esc — stacked prefs+settings both closed, and Esc closing the context menu also cleared a
/// placed ruler/LoS/viewshed. The fix is one design: overlays `register` + `is_topmost_open`; the
/// shared measure arm yields while `any_open()`.
///
/// Hollow-pin discipline: deleting the `any_open` guard or any overlay's `is_topmost_open` gate
/// turns the matching assert RED. Needles are assembled from fragments so this module cannot
/// self-satisfy them.
#[cfg(test)]
#[path = "../t726_window_esc_stack.rs"]
mod t726_window_esc_stack;

/// T-768 — Eden CONN-START-001 LMB target pick: the missing half after T-672's RMB arm/complete.
///
/// Hollow-pin discipline: deleting `complete_connect` from the Pending click path, or
/// `cancel_connect` from the Esc arm, turns the matching assert RED. Needles are assembled from
/// fragments so this module cannot self-satisfy them.
#[cfg(test)]
#[path = "../t768_connect_lmb_complete.rs"]
mod t768_connect_lmb_complete;

/// T-780 — the connection LINE: geometry, packing, hit test, and the four wiring facts that make
/// `CONN-DEL-001` reachable from the map.
///
/// The first three are behaviour tests over the pure functions. The rest are Class-R pins, because
/// the wiring lives in `#[cfg(target_arch = "wasm32")]` closures no native test can call: they are
/// taken over `live_code` (comments AND string literals blanked, test module cut), so a needle can
/// never be satisfied by the prose that describes it or by this module's own assertion text.
#[cfg(test)]
#[path = "../t780_connection_line.rs"]
mod t780_connection_line;

// ═════ T-936.7 — the tactical lane is bound on the RESTORE path too ═══════════════════════════════
//
// Wave-255 verify BLOCKER. T-936.7 bound its lane from `after_doc_change` only — the edit half —
// so a mission whose payload carried `tacticalGraphics` drew nothing when OPENED, which is the only
// way rows can currently exist at all. Same family as t760 / t780 / t819 above.
#[cfg(test)]
#[path = "../t936_7_tactical_lane_bind.rs"]
mod t936_7_tactical_lane_bind;

// ═════ T-784 — the comment GLYPH is pickable, and the pick is the lane's own list ════════════════
//
// The defect was total: the glyph had no pick path at all, the Outliner comment row was `ROW_STATIC`
// with no route to the selection, the T-697 selection filter can only NARROW an existing selection
// (so it can never introduce a comment that was not already selected), and `route_target` had no
// comment arm — so even the document-search hit rendered inert. Nothing could put a comment id into
// the selection by clicking, which left T-781's composable-comment lane unreachable.
//
// These pins hold the three properties the fix rests on:
//   1. the lane and the pick are ONE document read (T-780's construction, applied to the glyph);
//   2. a comment RESOLVES, so `subject_id_routes` — the affordance behind the Outliner row and the
//      dock-left search hit — can honestly say yes, and the existing kinds still resolve as before;
//   3. the map click folds the comment into `hit`, so the selection it lands in is the one the
//      composition capture reads, and Delete removes the comment rather than reporting success over
//      an unchanged document.
#[cfg(test)]
#[path = "../t784_comment_glyph.rs"]
mod t784_comment_glyph;

// ═════ T-796 — a comment can be DRAGGED: pick → preview → one-txn move ════════════════════════════
//
// O-6 pixel-verified two defects. The DRAG half is fixed here (the glyph RESTYLE — neutral shape +
// non-selection colour + selection-on-top — is the `comments_bind` glyph in `map-engine-render`'s
// `engine.rs`, a co-owned symbology surface (T-808/T-790) outside this slice's one file; it is
// reported found-not-fixed, not touched from here).
//
// Before this ticket a drag STARTING on a note resolved `pick_slot_or_vehicle` → None → LG::Marquee,
// so the note never entered a move; and even if a comment id HAD reached the LG::Move commit it would
// have fallen into `slot_ids` and been handed to `move_entities`, which reads the slot SoA, finds no
// such row, and moves it nowhere — a 90px drag left the stored position unchanged (verified twice).
//
// These pins + the two pure-function tests hold the four properties the fix rests on:
//   1. the drag-start FOLDS the comment pick into `hit` (the T-784 click precedent, not a fork), so a
//      note grabs like a slot — after slot/vehicle, before the marquee fallthrough;
//   2. the commit PARTITIONS comments out by asking the document's own map (`comment_details`, the
//      `delete_selection` rule) and routes them to `move_comment` — never to `move_entities`;
//   3. each note is base + delta, ONE txn per note, so a single-note drag is one Ctrl+Z (the ticket's
//      "ONE step"; multi-note inherits `delete_selection`'s accepted per-txn class);
//   4. the mid-drag preview re-binds the note's own lane so the glyph follows the cursor, and every
//      non-commit exit (zero delta, wrong button, cancel) re-binds it to the authored positions.
#[cfg(test)]
#[path = "../t796_comment_drag.rs"]
mod t796_comment_drag;

// ══ T-790 — the authored icon + caption reach the marker lane (F-03 write-half) ══════════════════
//
// Before this ticket the feeder read only x/z/factionId and dropped `icon`/`label`, so every placed
// marker drew as one pale disc with no caption. `marker_lane_fields` now parses all four parallel
// arrays from `briefing_marker_rows_json`, mapping `icon` → the canonical glyph (so DIFFERENT icons
// draw DIFFERENT shapes) and carrying `label` as the on-map caption. These pins hold that; the last
// is a Class-R pin binding `mission_history`'s two feed sites to the widened `markers_bind` shape
// (that module is `#![cfg(wasm32)]`, so a source pin is a native test's only reach into it).
#[cfg(test)]
#[path = "../t790_marker_glyph_caption.rs"]
mod t790_marker_glyph_caption;

// ══ Wave 145 F-1 — the selection prune keeps what EXISTS and drops what is GONE, both ways ═══════
//
// The prune in `mission_history` had one job — drop ids the document no longer holds — and was
// sourced from the slot SoA, a universe that contains no vehicle, no placed object and no comment.
// So it did the opposite of its job for three of the four selectable kinds: every document change
// (a drag commit, an undo, a layer toggle, a loadout write, an IDB restore) silently deleted them
// from the selection and cleared the Outliner highlight.
//
// The correction is only half a correction if it stops there. The prune is ALSO what has been
// guaranteeing that a stale id can never reach Delete — the wave-129 and wave-142 MAJORs both grew
// out of an id outliving the thing it named. So these pins fire BOTH directions against
// `selectable_ids`, the widened universe:
//
//   * a comment / vehicle / object / hidden slot that IS in the document SURVIVES a change;
//   * one that is NOT in the document — deleted, undone away, or never minted — still FALLS OUT.
//
// Plus the two the widening could have got wrong: a HIDDEN slot is in the universe (it exists; the
// SoA's T-665/T-701 drop is a visibility view, not an existence test), and a ZONE or MARKER is NOT
// (neither has a route into `ctx.selection`, so admitting them would widen the universe past the
// set being pruned). The Class-R pin at the end binds `mission_history`'s single prune site to this
// function through `include_str!` — that module is `#![cfg(target_arch = "wasm32")]` end to end, so
// no test placed in it would ever execute, and a source pin is the only reach a native test has.
#[cfg(test)]
#[path = "../w145_selection_prune.rs"]
mod w145_selection_prune;

// ── T-802 (O-8) — the hover cursor ───────────────────────────────────────────────────────────────
//
// Two halves, and both are load-bearing for a different reason.
//
// The STATE MACHINE (`hover_due` / `hover_next` / `hover_cursor_css`) is pure, so the throttle and
// the hysteresis are proved here rather than eyeballed in a browser — including the acceptance's
// own churn property, which is a statement about a SEQUENCE of hit-tests and therefore exactly the
// kind of thing a screenshot cannot check.
//
// The SOURCE PINS are the T-057 half. The reason hover picking was removed in the first place was
// cost, and every one of the four things that keeps this cheap (throttle first, no second point
// set, one document read per `doc_tick`, write only on change) is a property of the CALL SITE, not
// of any function these tests can call. So the call site is read back out of the file: an edit that
// hoists the pick above the throttle, or re-derives its own geometry, or writes the style every
// tick, breaks a named assertion instead of quietly re-creating the regression that deleted this
// feature once already.
#[cfg(test)]
#[path = "../t802_hover_cursor.rs"]
mod t802_hover_cursor;

/* ═══════════════════════ T-819 — crewed slots leave the map render SoA ═══════════════════════
 *
 * Eden hides a unit that is boarded into a vehicle. The hide is DERIVED from `vehicle.crew` —
 * never a new document flag, never a `materialize()` drop (those are T-665/T-701 for operator
 * hide). Figures + labels leave the map; outliner / selection / compile still see the slots.
 */

#[cfg(test)]
#[path = "../t819_crewed_render_hide.rs"]
mod t819_crewed_render_hide;

#[cfg(test)]
#[path = "arrange_chords.rs"]
mod arrange_chords;
/// T-930 — vehicle first paint is a silhouette, not the yellow disc. Native Class-R pins:
/// engine-mount first bind + catalog place-time invalidate. Needles split so this module
/// cannot green itself.

/* ═════════ T-939.4 — the Arrange chords, on the editor's own keydown (the defect, as a test) ════
 *
 * T-645 gave the Arrange tools a menu and no keys. The census in `ui/modals/help_modal.rs` proves it:
 * `KeyL` / `KeyT` / `KeyB` / `KeyH` are bound by nothing in the whole editor surface, and `KeyR` /
 * `KeyV` are bound only bare and only under Ctrl/Cmd — so `Alt` + any of the six reaches no arm and
 * the operator's keypress does nothing at all.
 *
 * Source pins rather than behaviour, for the reason every keydown pin in this programme is one: the
 * listener is a `#[cfg(target_arch = "wasm32")]` closure over `web_sys::KeyboardEvent`, so no native
 * test can press a key at it — the arm list IS the binding. Read off `live_source`, which cuts the
 * whole test half of this file first, so these needles can never match themselves.
 */

#[cfg(test)]
#[path = "vehicle_first_paint.rs"]
mod vehicle_first_paint;
