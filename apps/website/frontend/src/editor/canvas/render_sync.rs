//! T-934.10 — the Mission Creator canvas's PURE render-sync helper belt, split out of
//! `mission_editor.rs` (it sat between the panel views and `MissionEditorPage`). Everything here
//! is a deterministic function of its arguments — document JSON in, lane arrays / hit answers /
//! id universes out — so it compiles and unit-tests on the NATIVE shell, which is the property
//! the T-780/T-784/T-802/T-754/wave-145 blocks below argue for one lane at a time.
//!
//! The wasm-side wrappers that bind these helpers to the live document and DOM stay in
//! `mission_editor.rs` (`live_connection_segments`, `set_map_cursor`, `HoverPoints`/`hover_hit`,
//! `SubjectResolver`): each one reads `editor_ops`' OPS_CTX or `web_sys`, which is exactly the
//! line this split is drawn on. `mission_editor` re-exports every name below `pub(crate)`, so
//! call sites, `mission_editor::…` paths and the evacuated test pins kept their exact spelling.
// The same gate `mission_editor.rs` carries, for the same reason: half of this belt's callers are
// `#[cfg(target_arch = "wasm32")]` closures and the other half are `#[cfg(test)]` pins, so the
// native non-test build reaches almost none of it.
#![allow(dead_code)]

/* ══════════════ T-780 — the CONNECTION line: the map artifact `CONN-DEL-001` needs ══════════════
 *
 * T-672 shipped the connection graph and said so plainly: "a connection has no map glyph in this
 * slice, so this panel is the ONLY place an operator can observe the graph". T-768 then finished the
 * connect GESTURE — an edge can be started and completed with the pointer — and the gap became the
 * defect: an author draws an edge on the map, nothing appears, and the only way to remove it is the
 * panel's per-row Delete. This block is the line.
 *
 * THREE PIECES, and the split is deliberate:
 *   1. [`connection_segments`] — document rows + endpoint positions → world-space segments. Pure.
 *   2. [`connection_lane_verts`] — segments (+ which one is selected) → the flat
 *      `[x,y,r,g,b,a]…` LineList `RenderEngine::connections_bind` takes. Pure.
 *   3. [`pick_connection`] — a world point + a world tolerance → the edge under the cursor. Pure.
 *
 * Pure and native, so all three are unit-tested off-target: the wasm feed (the `doc_tick` Effect in
 * `on_load`) and the wasm pick (the `LG::Pending` sub-threshold arm) are thin wrappers that supply
 * the document and the camera and nothing else.
 *
 * **THE FEED IS FROM THE DOCUMENT, and that is the whole point of the lane.** T-069 and T-672 each
 * established the same failure independently: a lane fed only from its own authoring call sites goes
 * STALE after undo / redo / an IDB restore, because those paths replace the document without ever
 * re-entering the code that drew. So nothing here caches; the Effect re-reads `MissionDocCore` on
 * every `doc_tick`, and `doc_tick` is bumped by `editor_ops::refresh_docks`, which
 * `mission_history::refresh_signals` calls at the END of `after_doc_change` (every committed edit,
 * undo and redo) AND of `refresh_hud` / `rebind_engine_from_doc` (the mount seed, the server hydrate
 * and the IDB restore swap). One channel, every path.
 *
 * No z is read or written anywhere in this block (wave-127): an edge is a 2-D map-plane line between
 * two authored positions, so there is no `update_slot_position` / `move_entities_and_vehicles` call
 * to hand a `None` z to, and no `zs` vector to mis-zip against an `ids` vector.
 */

/// One connection edge reduced to what the map needs: its id and its two endpoints in world metres.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ConnSegment {
    /// The connection id — the SAME id `editor_ops::delete_connection` (the panel's verb) takes.
    pub id: String,
    pub ax: f64,
    pub ay: f64,
    pub bx: f64,
    pub by: f64,
}

/// The unselected edge hairline: the Aegis primary at a low alpha, so a dense graph reads as
/// structure rather than as a wall. Deliberately dimmer than `SquadLinks` — the ORBAT hairlines are
/// structural truth and an editor-only relation must not compete with them.
pub(crate) const CONN_LINE_RGBA: [f32; 4] = [173.0 / 255.0, 198.0 / 255.0, 1.0, 0.62];
/// The SELECTED edge: opaque amber. A hue no other lane uses, because the only thing this colour has
/// to communicate is "Delete will remove THIS one".
pub(crate) const CONN_LINE_SELECTED_RGBA: [f32; 4] = [1.0, 0.78, 0.30, 1.0];
/// Click tolerance for [`pick_connection`], in SCREEN pixels — converted to world metres by the
/// caller through the frozen press camera, so the tolerance is constant on screen at every zoom.
/// Matches the slot pick's feel: a hairline is 1 px, and nobody can click a 1 px target.
pub(crate) const CONN_PICK_PX: f64 = 6.0;

/// Build the drawable edges from `rows_json` ([`map_engine_core::doc::MissionDocCore::connection_rows_json`],
/// the SAME stable-ordered listing the panel renders) and a map of entity id → world position.
///
/// An edge whose endpoint has no position is **skipped**, not drawn to the origin: a dangling edge is
/// a `CONN-DANGLING` finding, and the panel is where a finding is reported. A line to (0,0) would be
/// a second, wordless report that also happens to be wrong about where the entity is.
///
/// Self-links are skipped too — `add_connection` refuses them and `CONN-SELF` flags a hydrated one,
/// and a zero-length segment is not a clickable artifact in any case.
#[must_use]
pub(crate) fn connection_segments(
    rows_json: &str,
    positions: &std::collections::HashMap<String, (f64, f64)>,
) -> Vec<ConnSegment> {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(rows_json) else {
        return Vec::new();
    };
    let Some(arr) = rows.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(arr.len());
    for r in arr {
        let s = |k: &str| r.get(k).and_then(serde_json::Value::as_str).unwrap_or("");
        let (id, from, to) = (s("id"), s("from"), s("to"));
        if id.is_empty() || from == to {
            continue;
        }
        let (Some(&(ax, ay)), Some(&(bx, by))) = (positions.get(from), positions.get(to)) else {
            continue;
        };
        out.push(ConnSegment {
            id: id.to_string(),
            ax,
            ay,
            bx,
            by,
        });
    }
    out
}

/// Pack the edges into the flat `[x,y,r,g,b,a]` LineList `RenderEngine::connections_bind` takes:
/// 6 floats per vertex, 2 vertices per segment, in `segs` order.
///
/// `selected` tints exactly one edge. It is matched by id against the same ids
/// [`pick_connection`] returns and `editor_ops::delete_connection` consumes, so the highlighted line
/// and the line Delete removes are the same line by construction rather than by convention.
#[must_use]
pub(crate) fn connection_lane_verts(segs: &[ConnSegment], selected: Option<&str>) -> Vec<f32> {
    let mut v = Vec::with_capacity(segs.len() * 12);
    for s in segs {
        let c = if selected.is_some_and(|sel| sel == s.id) {
            CONN_LINE_SELECTED_RGBA
        } else {
            CONN_LINE_RGBA
        };
        #[allow(clippy::cast_possible_truncation)]
        for (x, y) in [(s.ax, s.ay), (s.bx, s.by)] {
            v.push(x as f32);
            v.push(y as f32);
            v.extend_from_slice(&c);
        }
    }
    v
}

/// The edge under a world point, or `None`. `tol_m` is the click radius in world metres (the caller
/// converts [`CONN_PICK_PX`] through the press camera). Nearest edge wins, so overlapping edges
/// resolve deterministically instead of by listing order.
///
/// Distance is point-to-SEGMENT, not point-to-line: the infinite-line form would let a click far off
/// the end of a short edge, but on its extension, select it — a hit on nothing.
#[must_use]
pub(crate) fn pick_connection(
    segs: &[ConnSegment],
    wx: f64,
    wy: f64,
    tol_m: f64,
) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for s in segs {
        let (dx, dy) = (s.bx - s.ax, s.by - s.ay);
        let len2 = dx.mul_add(dx, dy * dy);
        let t = if len2 <= 0.0 {
            0.0
        } else {
            (((wx - s.ax) * dx + (wy - s.ay) * dy) / len2).clamp(0.0, 1.0)
        };
        let d = (wx - t.mul_add(dx, s.ax)).hypot(wy - t.mul_add(dy, s.ay));
        if d <= tol_m && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, s.id.as_str()));
        }
    }
    best.map(|(_, id)| id.to_string())
}

/* ══════════ T-784 — the COMMENT GLYPH: ONE document read, drawn AND picked ══════════════════════
 *
 * T-651 gave a comment an Outliner row, T-748 gave it a map glyph and T-781 made it composable.
 * Nothing gave it a way INTO the selection, so the composition lane was unreachable by clicking:
 * the glyph had no pick path at all.
 *
 * The map half is fixed the way T-780 fixed the connection line, and deliberately NOT a second way.
 * [`comment_points`] is the document read; [`comment_lane_xy`] PACKS that read for
 * `RenderEngine::comments_bind`; [`pick_comment`] HIT-TESTS the same `Vec`. What is drawn and what
 * a click can find are one set BY CONSTRUCTION rather than two parsers kept in step by hope —
 * which is what they were, `mission_history` holding a private copy of this parse.
 *
 * WHY THEY LIVE HERE. `mission_history` is `#![cfg(target_arch = "wasm32")]` in its entirety, so a
 * function placed there can never be unit-tested — the T-748 feed pin in
 * `map-engine-render::draw_order` had to reach it through `include_str!` for exactly that reason,
 * and a pick tolerance nobody can execute is how a hit box silently stops matching its picture.
 * This module compiles natively, so both the packing and the pick are testable where they ship.
 *
 * WHICH UNIVERSE, AND WHY IT IS THE RIGHT ONE: `commentsById` in full, sorted by id — never
 * `materialize()`. A comment is editor-only and is in the slot SoA at no point, so the T-665/T-701
 * hidden-layer drop `materialize()` performs cannot apply to it and cannot be forgotten here. The
 * lane draws every comment the document holds, the pick tests every comment the lane drew, and the
 * Outliner (`editor_ops::comment_rows`, the same `comments_json` sorted the same way) lists the
 * same set — one universe across all three surfaces.
 */

/// T-784 — one comment glyph in WORLD metres: the unit of BOTH the render lane and the pick.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CommentPoint {
    /// The `commentsById` key — what a pick puts into the selection.
    pub id: String,
    /// World easting (`position.x`).
    pub x: f64,
    /// World northing — the row's `position.z`. A comment's position is `{x, z}`: the `$defs/marker`
    /// vocabulary of TWO HORIZONTALS and no height (T-781's capture reads it the same way). It is
    /// named `y` here because that is the axis it IS on the map plane; treating it as an elevation
    /// would file the note's northing as its altitude and draw it at the origin.
    pub y: f64,
}

/// **The document read the comment lane and the comment pick share** — `commentsById` parsed once,
/// sorted by id so the lane's instance order cannot depend on `serde_json`'s map type (the T-748
/// rule, kept where the parse now lives).
#[must_use]
pub(crate) fn comment_points(comments_json: &str) -> Vec<CommentPoint> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(comments_json) else {
        return Vec::new();
    };
    let Some(obj) = map.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<_> = obj.iter().collect();
    rows.sort_by(|a, b| a.0.cmp(b.0));
    rows.into_iter()
        .map(|(id, v)| {
            let axis = |k: &str| {
                v.get("position")
                    .and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            CommentPoint {
                id: id.clone(),
                x: axis("x"),
                y: axis("z"),
            }
        })
        .collect()
}

/// T-748 — flat interleaved `[x, z, …]` for `RenderEngine::comments_bind`, **packed from
/// [`comment_points`]**. The lane is a projection of the pick's own list, so a comment cannot be
/// drawn where it cannot be clicked, nor clicked where nothing is drawn. `mission_history`'s two
/// bind sites (the IDB-restore rebind and `after_doc_change`) call this.
#[must_use]
pub(crate) fn comment_lane_xy(comments_json: &str) -> Vec<f32> {
    let pts = comment_points(comments_json);
    let mut xy = Vec::with_capacity(pts.len() * 2);
    #[allow(clippy::cast_possible_truncation)]
    for p in pts {
        xy.push(p.x as f32);
        xy.push(p.y as f32);
    }
    xy
}

/// **T-808 — the lane's id column, for `RenderEngine::comments_bind_ids`.** The sibling of
/// [`comment_lane_xy`] and packed from the very same [`comment_points`] list: row *i* of this array
/// names the bubble drawn at rows `2i`/`2i+1` of that one, so the pairing is a property of the
/// shared read rather than of two feeders staying in step. `comment_drag_lane_xy` maps that list
/// one-for-one as well (it only OFFSETS the dragged rows), so these ids are aligned with the drag
/// preview's lane too — which is what keeps a note's selection ring on the note while it moves.
///
/// Without this column the engine cannot answer "is bubble *i* selected?": `comments_bind` marks
/// every row unselected when its id cache is empty, so a selected note drew the neutral bubble and
/// the T-796 selection treatment shipped invisible.
#[must_use]
pub(crate) fn comment_lane_ids(comments_json: &str) -> Vec<String> {
    comment_points(comments_json)
        .into_iter()
        .map(|p| p.id)
        .collect()
}

/// T-760 / **T-790** — the four parallel marker-lane arrays for [`RenderEngine::markers_bind`],
/// parsed ONCE from `briefing_marker_rows_json` (the sole schema-legal marker surface, emitted by
/// `MissionDocCore::briefing_marker_rows_json` with `x`/`z`/`factionId`/`icon`/`label`):
///   * `xy` — interleaved world `[x, z, …]`,
///   * `tints` — packed RGBA8 side tint per marker (`factionId` → [`slots_gpu::side_rgba`]),
///   * `icons` — each marker's authored `icon` ALIAS (the T-790 write-half: the authored icon finally
///     reaches the render lane; `markers_bind` maps the alias to its canonical glyph via the shared
///     `map_engine_render::scene::marker_glyph_for_alias` table),
///   * `captions` — each marker's `label` (the caption shown on the map).
///
/// T-790 note: before this, the feeder read only `x`/`z`/`factionId` and dropped `icon`/`label`, so
/// every marker drew as one pale disc with no caption. The parse lives HERE (not in the
/// `#![cfg(wasm32)]` `mission_history`) so it is natively unit-testable, matching the T-784/T-748
/// move of `comment_lane_xy`; `mission_history`'s two bind sites (restore rebind + `after_doc_change`)
/// call this so undo / redo / restore all share one feed — a lane fed only from authoring call sites
/// would go stale exactly the way T-760 forbids.
///
/// The alias-to-glyph MAPPING is deliberately NOT done here: it lives in `map-engine-render`
/// (`scene::marker_glyph_for_alias`, the T-806 deliverable), a wasm32-only dependency of this crate,
/// so mapping here would break the native test build. Carrying the alias string keeps this parse
/// native and the mapping in its single home.
#[must_use]
pub(crate) fn marker_lane_fields(
    marker_rows_json: &str,
) -> (Vec<f32>, Vec<u8>, Vec<String>, Vec<String>) {
    let Ok(rows) = serde_json::from_str::<serde_json::Value>(marker_rows_json) else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    let Some(arr) = rows.as_array() else {
        return (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    };
    let mut xy = Vec::with_capacity(arr.len() * 2);
    let mut tints = Vec::with_capacity(arr.len() * 4);
    let mut icons = Vec::with_capacity(arr.len());
    let mut captions = Vec::with_capacity(arr.len());
    for r in arr {
        let num = |k: &str| r.get(k).and_then(serde_json::Value::as_f64).unwrap_or(0.0);
        #[allow(clippy::cast_possible_truncation)]
        {
            xy.push(num("x") as f32);
            xy.push(num("z") as f32);
        }
        let faction = r
            .get("factionId")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let side = faction.strip_prefix("faction-").unwrap_or(faction);
        tints.extend_from_slice(&map_engine_core::slots_gpu::side_rgba(side));
        let str_field = |k: &str| {
            r.get(k)
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        icons.push(str_field("icon"));
        captions.push(str_field("label"));
    }
    (xy, tints, icons, captions)
}

/// Click tolerance for [`pick_comment`], in SCREEN pixels — the SAME radius the slot/vehicle pick
/// uses (`MissionDocCore::PICK_RADIUS_PX`). `comments_bind` draws a comment with the slot atlas's
/// ring glyph, so a tolerance of our own invention would give the note a hit box a different size
/// from its picture.
///
/// Restated as a literal rather than referenced, because that constant lives behind `map-engine-core`'s
/// `doc` feature and this crate enables it on wasm32 ONLY — while the pick and its tests are native.
/// `comment_pick_px_is_the_slot_pick_radius` reads the core's own declaration back through
/// `include_str!`, so the restatement cannot drift from the thing it restates.
pub(crate) const COMMENT_PICK_PX: f64 = 4.0;

/// The comment glyph under a world point, or `None`. `tol_m` is the click radius in world metres
/// (the caller converts [`COMMENT_PICK_PX`] through the frozen press camera, exactly as the
/// connection pick converts [`CONN_PICK_PX`]). NEAREST wins, so two notes within one click of each
/// other resolve deterministically instead of by listing order.
#[must_use]
pub(crate) fn pick_comment(
    points: &[CommentPoint],
    wx: f64,
    wy: f64,
    tol_m: f64,
) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for p in points {
        let d = (wx - p.x).hypot(wy - p.y);
        if d <= tol_m && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, p.id.as_str()));
        }
    }
    best.map(|(_, id)| id.to_string())
}

/// T-796 — which of the dragged ids are COMMENTS, paired with their AUTHORED position, asked of the
/// document's own comment map (`comment_points`) rather than a `cmt-` prefix — the same membership
/// rule `delete_selection` uses, and for the same reason: the prefix is [`mint_comment_id`]'s
/// convention, and a hydrated mission may carry comment ids that were never minted here. Order
/// follows `points` (already id-sorted), so a mixed drag's comment commit is deterministic.
///
/// Shared by the drag PREVIEW ([`comment_drag_lane_xy`], offset by the live delta) and the drag
/// COMMIT (base + delta → [`editor_ops::move_comment`]) so the glyph that follows the cursor
/// and the position finally stored are computed from ONE list — the parity the O-7 preview note asks
/// for, applied to the comment lane.
#[must_use]
pub(crate) fn dragged_comment_points(
    points: &[CommentPoint],
    drag_ids: &[String],
) -> Vec<CommentPoint> {
    points
        .iter()
        .filter(|p| drag_ids.iter().any(|d| d == &p.id))
        .cloned()
        .collect()
}

/// T-796 — the `MissionComments` lane re-packed for a live drag: EVERY comment the document holds
/// (so the notes not being dragged stay drawn where they are), with the dragged ones translated by
/// the world delta `(dx, dy)`. Feeds `RenderEngine::comments_bind` mid-drag exactly as
/// [`comment_lane_xy`] feeds it at rest — a comment has its own lane (not the slot overlay or the
/// vehicle re-pack `push_drag_preview` drives), so its preview is this lane re-bound, and dropping
/// the drag re-binds `comment_lane_xy` (the authored positions) the same way a committed move does
/// through `after_doc_change`.
#[must_use]
pub(crate) fn comment_drag_lane_xy(
    comments_json: &str,
    drag_ids: &[String],
    dx: f64,
    dy: f64,
) -> Vec<f32> {
    let pts = comment_points(comments_json);
    let mut xy = Vec::with_capacity(pts.len() * 2);
    #[allow(clippy::cast_possible_truncation)]
    for p in pts {
        let dragged = drag_ids.iter().any(|d| d == &p.id);
        let (ox, oy) = if dragged { (dx, dy) } else { (0.0, 0.0) };
        xy.push((p.x + ox) as f32);
        xy.push((p.y + oy) as f32);
    }
    xy
}

/* ══════════ T-802 (O-8) — THE HOVER CURSOR: the pointer names what is grabbable ═════════════════
 *
 * Until this ticket the map canvas wore `cursor: auto` over a unit, over a note and over bare
 * ground alike: nothing on the surface said "this pixel is pickable". Every OTHER clickable thing
 * in the editor already says so — the outliner rows, the toolbelt chips and the validation findings
 * all wear `cursor-pointer`, and T-754 / the wave-115 MAJOR were both filed for the INVERSE lie (a
 * `cursor-pointer` over a click that could not land). The map was the one surface making no claim
 * at all, which is the same defect with the sign flipped.
 *
 * ── WHY THIS IS NOT A CSS ONE-LINER ─────────────────────────────────────────────────────────────
 * Hover picking was DELIBERATELY REMOVED for performance at T-057 (`7adc34596`, the React era):
 * "Removed `onHover`; cursor unprojected from the mouse on `onPointerMove`. Picking only on
 * click/dbl-click/marquee/drag-start", with the trade recorded in the roadmap as **"the pointer no
 * longer changes to a 'pointer' glyph over an icon (no hover pick)"**. Deck's `onHover` ran a GPU
 * pick pass over every icon on EVERY pointer move and took the editor to ~9 fps at 200 slots.
 * So the cursor is not the hard part; paying for it every frame is. Four things keep this cheap:
 *
 *   1. THROTTLE — at most one hit-test per [`HOVER_THROTTLE_MS`]. A pointer streams 60–125 moves a
 *      second; this answers ~25 of them and drops the rest on the floor.
 *   2. NO NEW GEOMETRY — the test is `select_tool::pick_slot_or_vehicle` (a radius query over
 *      `PointIndex`, the spatial hash) followed by the SAME comment fold the click and drag paths
 *      run, with the SAME tolerance derivation. There is no second point set, no second transform
 *      and no second notion of "what is under this pixel": if the cursor says pickable, the click
 *      that follows picks — and `the_hover_reuses_the_click_paths_pick_and_camera` holds them so.
 *   3. ONE DOCUMENT READ PER GENERATION — [`HoverPoints`] materialises the slot SoA, the vehicle
 *      points and the comment points ONCE per `doc_tick` and reuses them across every hit-test in
 *      between. `doc_tick` is the exact channel `mission_history::after_doc_change` bumps in the
 *      same tail that re-binds the glyph lanes (`refresh_signals` → `editor_ops::refresh_docks`,
 *      pinned by `t780_connection_line`), so the hover cache is never staler than the picture the
 *      operator is looking at. A per-move `materialize()` would be a full Y.Doc read at 25 Hz —
 *      the T-057 cost in a new coat.
 *   4. WRITE ONLY ON CHANGE — the DOM `style.cursor` write happens on a TRANSITION, never per tick.
 *
 * ── HYSTERESIS ──────────────────────────────────────────────────────────────────────────────────
 * A bare boundary test flickers: the pick radius is 4 px, so a hand resting on the rim of a glyph
 * crosses in and out several times a second and the cursor strobes. [`hover_next`] holds the
 * "pickable" claim through a miss while the pointer is still within [`HOVER_RELEASE_PX`] of the
 * pixel that last hit — a dead-band, not a timer, so a decisive move off the glyph drops it at once
 * while jitter cannot. The band is derived from [`COMMENT_PICK_PX`] (this file's pinned restatement
 * of `MissionDocCore::PICK_RADIUS_PX`) and NOT from a fresh pixel guess, so when T-808 changes what
 * the glyphs look like the affordance follows the pick radius instead of drifting away from it.
 *
 * ── READ-ONLY WITH RESPECT TO THE GESTURE MACHINE ───────────────────────────────────────────────
 * This runs in the same `pointermove` as T-723's arm, T-795's rotate ring and T-796's comment drag.
 * It takes NOTHING and consumes NOTHING: it reads `left` through a shared borrow purely to ask "is
 * a gesture in flight?", and it is SUPPRESSED (state reset, cursor back to plain) whenever one is —
 * see [`hover_suppressed`]. A hover test that mutated or consumed gesture state would be a defect,
 * so the only state it owns is its own [`HoverState`] cell.
 */

/// T-802 — the CSS `cursor` over a pickable entity. `pointer` (not `grab`) because it is the
/// vocabulary the rest of this editor already speaks: every clickable chrome row wears
/// `cursor-pointer`, and T-754 made "wears pointer" mean exactly "a click here resolves".
pub(crate) const HOVER_CURSOR_PICKABLE: &str = "pointer";

/// T-802 — the CSS `cursor` everywhere else on the map. Written EXPLICITLY rather than left as the
/// UA default `auto`, so the resting state is a value the surface asserts (and a scripted read can
/// distinguish "decided: nothing here" from "never asked").
pub(crate) const HOVER_CURSOR_PLAIN: &str = "default";

/// T-802 — the hover hit-test throttle floor, in milliseconds. 40 ms ⇒ at most 25 tests a second
/// against a pointer that fires two to five times that often. Inside the 30–60 ms band the ticket
/// names: fast enough that the cursor changes within one frame of arriving over a glyph, slow
/// enough that the pick is not on the pointer's hot path.
pub(crate) const HOVER_THROTTLE_MS: f64 = 40.0;

/// T-802 — the hysteresis dead-band, in SCREEN pixels: how far the pointer must travel from the
/// pixel that last HIT before a miss is believed.
///
/// Derived from [`COMMENT_PICK_PX`] — this file's pinned restatement of the slot pick radius
/// (`MissionDocCore::PICK_RADIUS_PX`; see `comment_pick_px_is_the_slot_pick_radius`) — and NOT from
/// a hand-picked pixel count, so it tracks the pick radius rather than the glyph art. 1.5× the
/// radius: wide enough that hand tremor at the rim cannot strobe the cursor, narrow enough that
/// leaving the glyph reads as instant.
pub(crate) const HOVER_RELEASE_PX: f64 = COMMENT_PICK_PX * 1.5;

/// T-802 — everything the hover cursor remembers between pointer moves. `Copy`, three scalars, and
/// held in a plain `Cell` beside the gesture state it must never touch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct HoverState {
    /// Timestamp of the last hit-test (`Date::now()` ms domain) — the throttle clock. `0.0` (the
    /// `Default`) means "never tested", which [`hover_due`] treats as due.
    pub last_ms: f64,
    /// Is the cursor currently CLAIMING that the pixel under the pointer is pickable?
    pub pickable: bool,
    /// The pixel of the most recent HIT — the anchor the hysteresis dead-band is measured from.
    /// `None` whenever `pickable` is false; the two always move together.
    pub anchor: Option<(f64, f64)>,
}

/// T-802 — may the hit-test run at `now_ms`? The throttle, as a pure question.
///
/// Due when at least [`HOVER_THROTTLE_MS`] has passed, when the clock has never been read
/// (`last_ms == 0.0`), and when the clock went BACKWARDS or non-finite — a stalled throttle that
/// silently stopped answering would look exactly like the feature being off, so every degenerate
/// clock resolves to "test it".
#[must_use]
pub(crate) fn hover_due(prev: HoverState, now_ms: f64) -> bool {
    if !now_ms.is_finite() || prev.last_ms <= 0.0 {
        return true;
    }
    !(0.0..HOVER_THROTTLE_MS).contains(&(now_ms - prev.last_ms))
}

/// T-802 — fold one hit-test result into the hover state. **The only place the pointer/plain
/// decision is made**, and pure so the hysteresis is provable off-target.
///
/// A HIT always claims pickable and re-anchors. A MISS drops the claim UNLESS the pointer is still
/// inside [`HOVER_RELEASE_PX`] of the anchor — and a held miss deliberately does NOT move the
/// anchor, so continued travel in one direction always escapes the band (a re-anchoring hold would
/// let a slow drag carry the claim across the whole map).
#[must_use]
pub(crate) fn hover_next(prev: HoverState, hit: bool, px: f64, py: f64, now_ms: f64) -> HoverState {
    if hit {
        return HoverState {
            last_ms: now_ms,
            pickable: true,
            anchor: Some((px, py)),
        };
    }
    let held = prev.pickable
        && prev
            .anchor
            .is_some_and(|(ax, ay)| (px - ax).hypot(py - ay) <= HOVER_RELEASE_PX);
    HoverState {
        last_ms: now_ms,
        pickable: held,
        anchor: if held { prev.anchor } else { None },
    }
}

/// T-802 — the CSS cursor value for a hover verdict.
#[must_use]
pub(crate) fn hover_cursor_css(pickable: bool) -> &'static str {
    if pickable {
        HOVER_CURSOR_PICKABLE
    } else {
        HOVER_CURSOR_PLAIN
    }
}

/// T-802 — is the hover read suppressed right now? Pure, so the suppression set is a readable list
/// rather than a chain of early returns nobody can enumerate.
///
///   * `gesture_active` — an LMB drag / marquee / rotate / ruler capture is in flight. The pointer
///     is committed to a gesture; re-labelling it mid-drag would be noise, and the cursor must not
///     be left claiming "pickable" over whatever the drag happens to be passing over.
///   * `place_armed` — a palette place (or a multi-click zone draw: both are `editor_ops::Pending`)
///     owns the pointer, and the live affordance is the place ghost, not the cursor.
///   * `measuring` — Ruler / LoS capture points; the map's pickable entities are not the subject.
///
/// `place_armed` and the zone draw are ALSO caught by the `has_pending` early return further up the
/// handler. They are named here anyway: the predicate is the statement of intent, and a later edit
/// that reorders the handler must not silently un-suppress them.
#[must_use]
pub(crate) fn hover_suppressed(gesture_active: bool, place_armed: bool, measuring: bool) -> bool {
    gesture_active || place_armed || measuring
}

/* ═════════════ T-754 — what the click-to-select router RESOLVES, as a pure question ═════════════
 *
 * T-655 shipped ONE click-to-select router (`validation_panel::register_select_by_id`, registered
 * from this file's wasm mount). Two surfaces now draw a click affordance on top of it — the
 * validation panel's finding rows and T-688's aggregated settings rows — and the wave-115 verifier
 * found the defect that follows from the router's resolution living INSIDE the closure: a surface
 * cannot ask "would this click select anything?", so it guesses "the row names an id, so yes", and
 * every zone-owned row wore `cursor-pointer` over a click that could only produce a toast.
 *
 * So the resolution is lifted OUT of the closure into [`route_target`] — pure, `serde_json`-only,
 * natively compiled, and therefore both TESTABLE and ASKABLE. The registered closure is the only
 * ACTOR (it still owns the `!Send` doc/selection/engine `Rc`s); a view that wants to know whether a
 * click has a target asks the same function the click will ask. That is what makes "a row is
 * clickable iff the router resolves its subject" a correspondence rather than a hope.
 *
 * The ZONE arm is the T-754 widening. A zone is in neither the slot SoA nor `vehiclesById` — its
 * selection is the Zones panel's own `RwSignal` (`eden_dock_right::route_select_zone`, the seam that
 * panel now exposes for exactly this) — so before this the router returned `false` for 100% of the
 * aggregation's entity rows.
 *
 * The ENTITY arm is the wave-129 widening, and it is the reachable half: the validation engine
 * ALREADY emits `ASSET-RESOLVES` findings whose subject is a placed-object id (`placed_asset_refs`
 * walks the payload's `entities[]`, the compiled copy of `entitiesById`), so every such row wore
 * `cursor-pointer` over a click that resolved to `None`. The same defect T-754 fixed on the settings
 * surface, live on the validation panel — and fixed the same two ways: the router grew the arm, and
 * the panel stopped guessing (`validation_panel::register_route_probe`, fed by the SAME resolution
 * the click runs, so the affordance and the click cannot disagree).
 */

/// What [`route_target`] resolved a subject id to — i.e. WHICH selection surface owns it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RouteTarget {
    /// A slot: the caller's SoA predicate matched. The position comes from the SoA row (the caller
    /// already has it), so no coordinates ride this arm.
    Slot,
    /// A `vehiclesById` row, at its authored `position`.
    Vehicle { x: f64, y: f64 },
    /// Wave 129 — an `entitiesById` row (a placed world object, T-254) at its authored `position`.
    ///
    /// Rides the SAME path as [`RouteTarget::Vehicle`] (editor selection + camera centre), because a
    /// placed object is off the slot SoA in exactly the way a vehicle is: neither is tinted by
    /// `slots_bind_soa`'s lane, both are real members of the editor selection the attributes /
    /// history mirrors read. This arm is the one the ASSET-RESOLVES rule needs — `placed_asset_refs`
    /// emits a finding per `entities[]` row with that row's id as the subject, so before this arm
    /// every placed-object finding resolved to `None`.
    Entity { x: f64, y: f64 },
    /// T-754 — a `zonesById` row, at its geometric centre. Selected in the Zones panel, not in the
    /// slot selection (a zone id in `select_tool`'s selection would read `SEL 1` with nothing
    /// highlighted — see `eden_dock_right`'s `zone_selected`).
    Zone { x: f64, y: f64 },
    /// T-784 — a `commentsById` row: the editor-only annotation (T-651), at its authored position.
    ///
    /// Rides the SAME path as [`RouteTarget::Vehicle`] / [`RouteTarget::Entity`] (editor selection +
    /// camera centre) because it belongs in the same place they do: `editor_ops`'s selection `Vec`
    /// is what the T-781 composition capture reads, and that capture classifies each selected id as
    /// slot / vehicle / object / COMMENT off ONE vector. A comment in a lane of its own would be a
    /// selection the composition could never see.
    ///
    /// Without this arm `validation_panel::subject_id_routes` answered `false` for every comment, so
    /// the dock-left document-search hit rendered inert (`eden_dock_left::hit_is_routable`) and the
    /// Outliner row could not honestly paint an affordance either. The resolver learning comments is
    /// what makes both surfaces live — neither of them grew a kind list.
    Comment { x: f64, y: f64 },
}

/// **Where a `subject_id` would go if it were clicked**, over the document's `small_maps_json()`
/// root plus `is_slot` (slot ids live in the SoA, which is not in that root, so the one fact this
/// function cannot read is supplied by the caller).
///
/// `None` means NOTHING would be selected — a stale id whose entity was deleted, or a kind no
/// selection surface owns. A view MUST NOT paint a click affordance on a row this returns `None`
/// for; that is the T-754 defect, stated as a rule.
///
/// Order is slot → vehicle → entity → zone, the order the shipped router already tried with the
/// wave-129 entity lookup appended to the by-id maps, so neither widening can change what an
/// existing id resolves to (the id spaces are disjoint — each map is keyed by its own minted id).
pub(crate) fn route_target(
    root: &serde_json::Value,
    subject_id: &str,
    is_slot: &dyn Fn(&str) -> bool,
) -> Option<RouteTarget> {
    if is_slot(subject_id) {
        return Some(RouteTarget::Slot);
    }
    if let Some(p) = root
        .get("vehiclesById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("y").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Vehicle { x, y });
        }
    }
    // Wave 129 — placed world objects. `entitiesById` rows carry the SAME `position {x, y, z,
    // rotation}` shape `vehiclesById` rows do (`doc/store.rs::add_entity`), so this is the vehicle
    // lookup over a second map, not a second kind of resolution.
    if let Some(p) = root
        .get("entitiesById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("y").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Entity { x, y });
        }
    }
    if let Some(zone) = root.get("zonesById").and_then(|m| m.get(subject_id)) {
        if let Some((x, y)) = zone_centre(zone) {
            return Some(RouteTarget::Zone { x, y });
        }
    }
    // T-784 — the editor-only annotation, appended LAST for the reason stated above: a new arm may
    // not change what an already-resolving id resolves to, and appending is the only placement that
    // guarantees it without reasoning about id-space disjointness a second time.
    //
    // The axes are `{x, z}`, not `{x, y}`: a comment row carries TWO HORIZONTALS and no height (the
    // `$defs/marker` vocabulary — see [`CommentPoint`]). Reading `y` here would find nothing, return
    // `None`, and leave the row inert under an affordance that had already been painted.
    if let Some(p) = root
        .get("commentsById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("z").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Comment { x, y });
        }
    }
    None
}

/// **Wave 129 F6 — AVAILABILITY: what a click on this subject would actually REACH.**
///
/// [`route_target`] answers "which surface owns this id?" over the document alone. That is not the
/// whole of "would this click do something", because one arm needs a SEAM as well as a row: a
/// [`RouteTarget::Zone`] is selected through `eden_dock_right::route_select_zone`, and that reports
/// `false` when the Zones panel is not mounted (wave-129 F2 made it honest — it used to answer
/// `true` while writing to disposed signals). So a zone whose panel is gone RESOLVES but is not
/// AVAILABLE, and a click on it returns `false`.
///
/// F6 is the divergence that fell out of F1 + F2 being fixed independently: the affordance probe was
/// built from the resolution alone (`… .is_some()`) while the click also consulted the seam, so a
/// zone row painted `cursor-pointer` over a click that could not land — the T-754 MAJOR, re-created
/// by two correct fixes disagreeing. This function is the fix's shape: **ONE narrowing that the
/// probe and the click both go through**, so "a row is clickable IFF clicking it does something" is
/// a single decision rather than a condition written twice and kept in step by hope.
///
/// `resolved` is [`route_target`]'s answer with the centre the caller already computed;
/// `zone_panel_live` is the one fact the document cannot carry — "is the Zones panel mounted?".
/// Every non-`Zone` arm passes straight through: they select into the editor selection, which exists
/// for as long as the router itself does.
pub(crate) fn route_availability(
    resolved: Option<(RouteTarget, f64, f64)>,
    zone_panel_live: &dyn Fn() -> bool,
) -> Option<(RouteTarget, f64, f64)> {
    let (target, x, y) = resolved?;
    if matches!(target, RouteTarget::Zone { .. }) && !zone_panel_live() {
        return None;
    }
    Some((target, x, y))
}

/* ═════ Wave 145 F-1 — WHAT THE SELECTION PRUNE MAY KEEP: the WHOLE selectable universe ══════════
 *
 * `mission_history`'s post-change tail prunes `ctx.selection` so a document that changed under the
 * app cannot leave the selection naming ids the document no longer holds. That guarantee is not
 * decoration: undoing an *add* deletes rows, an IDB restore swaps the whole document, and a Delete
 * over an id that no longer exists is the wave-129 / wave-142 MAJOR — a success report over a
 * document nothing was written to, and a stale id coexisting with an edge selection so Delete took
 * the wrong object. The prune is what has been closing that route.
 *
 * It was sourced from `materialize()`'s slot SoA, which is the SLOT universe and a lossy view of
 * even that. Every other selectable kind is absent from it, so `retain` was not pruning STALE ids —
 * it was deleting LIVE ones. A selected comment, vehicle or placed object fell out of the selection
 * on the next drag commit, undo, layer toggle or loadout write, silently, with the Outliner
 * highlight clearing under the operator's hands and no message.
 *
 * [`selectable_ids`] is the universe instead: the ids the document ACTUALLY HOLDS, across every map
 * a selection can name. The two directions the prune has to get right, and where each comes from:
 *
 *   * A LIVE id SURVIVES — because its key is in one of the four maps read below.
 *   * A GONE id STILL FALLS OUT — because both halves are read from the POST-change document. A
 *     comment removed by Delete, by the comment panel, or by an undo is out of `commentsById`
 *     BEFORE the prune runs, so `retain` drops it exactly as it always did. Widening the universe
 *     does not weaken the staleness guarantee; sourcing it from a snapshot would.
 *
 * Membership is by KEY PRESENCE, not by resolvability. [`route_target`] additionally demands a
 * `position` before it will centre a click on a row, but "does this row exist?" and "can a click fly
 * to it?" are different questions and the prune asks the first — a vehicle row whose position was
 * never authored still EXISTS, and deleting it from the selection would be the same silent loss in
 * a smaller costume. `editor_ops::delete_selection` partitions on exactly this key-presence test.
 *
 * THE SLOT HALF COMES OFF `slots_json`, NOT THE SoA. That is the wave-144 rule for id universes
 * (`editor_ops::live_slot_ids`, pinned by `eden_dock_right`'s
 * `both_id_minters_prove_uniqueness_against_hidden_slots_too`), and it is load-bearing here for a
 * second reason. `materialize()` DROPS slots on a hidden layer (T-665) and slots carrying
 * `editorHidden` (T-701), so an SoA-sourced prune deselects a slot for being HIDDEN rather than for
 * being GONE — a visibility policy nobody wrote, contradicted by the verbs that ship. `editor_ops::
 * toggle_hidden` and `show_selection` act on the LIVE selection, and the hide they perform ends in
 * `after_local_edit` → the prune → the very rows just hidden leave the selection: the toggle can
 * never toggle back, and Show-selection is unreachable by the only route that reaches it. Existence
 * is the question the prune asks; `slots_json` is the only complete answer to it.
 *
 * ZONES AND MARKERS ARE DELIBERATELY ABSENT. A zone is selected in the Zones panel through
 * `eden_dock_right::route_select_zone` and never enters `ctx.selection` — the router's own `Zone`
 * arm is written around exactly that — and a marker has no selection route at all. Adding either
 * key would widen the universe past anything that can appear in the set it prunes, which is how a
 * prune stops being able to say "this id is gone".
 */

/// **The ids the live document holds that a selection may name.** Slots off the raw `slots_json` key
/// set (hidden rows INCLUDED), plus the `vehiclesById` / `entitiesById` / `commentsById` key sets off
/// `small_maps_json`. The block above argues each half.
///
/// It lives HERE, beside [`route_target`], rather than in the module that calls it: `mission_history`
/// is `#![cfg(target_arch = "wasm32")]` from line one, so a function placed there can never be
/// unit-tested — the same reason [`comment_points`] lives here. `the_selection_prune_runs_over_the_
/// whole_selectable_universe` holds `mission_history`'s single prune site to this function through
/// `include_str!`, because an `include_str!` pin is the only reach a native test has into that file.
#[must_use]
pub(crate) fn selectable_ids(
    slots_json: &str,
    small_maps_json: &str,
) -> std::collections::HashSet<String> {
    let mut live: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(slots_json)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    if let Ok(small) = serde_json::from_str::<serde_json::Value>(small_maps_json) {
        for key in ["vehiclesById", "entitiesById", "commentsById"] {
            if let Some(obj) = small.get(key).and_then(|v| v.as_object()) {
                live.extend(obj.keys().cloned());
            }
        }
    }
    live
}

/// **T-819 — slot ids currently referenced by any placed vehicle's crew map.**
///
/// Derived state only: the crew assignment IS the hide. There is no `editorHidden` (or any other)
/// document flag for "this slot is crewed" — unassigning a seat, deleting the vehicle, or undoing
/// the board removes the id from this set and the figure returns by itself. Read off
/// `vehiclesById.*.crew` in `small_maps_json`, never off a filtered SoA (wave-144: id universes
/// come from the raw maps).
#[must_use]
pub(crate) fn crewed_slot_ids(small_maps_json: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Ok(small) = serde_json::from_str::<serde_json::Value>(small_maps_json) else {
        return out;
    };
    let Some(vehicles) = small.get("vehiclesById").and_then(|v| v.as_object()) else {
        return out;
    };
    for v in vehicles.values() {
        let Some(crew) = v.get("crew").and_then(|c| c.as_object()) else {
            continue;
        };
        for slot in crew.values() {
            if let Some(id) = slot.as_str().filter(|s| !s.is_empty()) {
                out.insert(id.to_string());
            }
        }
    }
    out
}

/// **T-819 — which SoA rows stay on the map** after the derived crew hide.
///
/// Returns the KEEP indices into `ids` (and every parallel column). A crewed slot leaves the map
/// render SoA (figure + label) the way T-701 `editorHidden` leaves it — but this filter is a VIEW
/// over the crew assignment, not a `materialize()` drop: compile, outliner, and selection still see
/// every slot. Empty `crewed` keeps every row.
#[must_use]
pub(crate) fn map_render_keep_indices(
    ids: &[String],
    crewed: &std::collections::HashSet<String>,
) -> Vec<usize> {
    if crewed.is_empty() {
        return (0..ids.len()).collect();
    }
    ids.iter()
        .enumerate()
        .filter(|(_, id)| !crewed.contains(id.as_str()))
        .map(|(i, _)| i)
        .collect()
}

/// T-819 — the map-render SoA: `materialize()` minus every slot referenced by a vehicle crew list.
///
/// **Not** a `materialize()` change. T-665 / T-701 drop operator-hidden rows inside the core; crewed
/// slots must still materialize and compile. This is the RENDER shape only — feed
/// `slots_bind_symbology` and map picks with it; leave outliner / selection / id minting on the raw
/// maps and the unfiltered SoA.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub(crate) fn map_render_slot_soa(
    core: &map_engine_core::doc::MissionDocCore,
) -> map_engine_core::doc::SlotSoa {
    let soa = core.materialize();
    let crewed = crewed_slot_ids(&core.small_maps_json());
    filter_slot_soa_excluding(&soa, &crewed)
}

/// Drop SoA rows whose ids are in `exclude`, keeping dictionaries so remaining `*_idx` values stay
/// valid. Pure column filter — does not touch the document.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub(crate) fn filter_slot_soa_excluding(
    soa: &map_engine_core::doc::SlotSoa,
    exclude: &std::collections::HashSet<String>,
) -> map_engine_core::doc::SlotSoa {
    use map_engine_core::doc::SlotSoa;
    let keep = map_render_keep_indices(&soa.ids, exclude);
    if keep.len() == soa.ids.len() {
        return soa.clone();
    }
    let mut out = SlotSoa {
        roles: soa.roles.clone(),
        tags: soa.tags.clone(),
        squads: soa.squads.clone(),
        layers: soa.layers.clone(),
        ..SlotSoa::default()
    };
    out.ids.reserve(keep.len());
    out.xs.reserve(keep.len());
    out.ys.reserve(keep.len());
    out.xy.reserve(keep.len() * 2);
    out.zs.reserve(keep.len());
    out.rotations.reserve(keep.len());
    out.stance.reserve(keep.len());
    out.role_idx.reserve(keep.len());
    out.tag_idx.reserve(keep.len());
    out.squad_idx.reserve(keep.len());
    out.layer_idx.reserve(keep.len());
    out.side_keys.reserve(keep.len());
    for i in keep {
        out.ids.push(soa.ids[i].clone());
        out.xs.push(soa.xs[i]);
        out.ys.push(soa.ys[i]);
        out.xy.push(soa.xy[i * 2]);
        out.xy.push(soa.xy[i * 2 + 1]);
        out.zs.push(soa.zs[i]);
        out.rotations.push(soa.rotations[i]);
        out.stance.push(soa.stance[i]);
        out.role_idx.push(soa.role_idx[i]);
        out.tag_idx.push(soa.tag_idx[i]);
        out.squad_idx.push(soa.squad_idx[i]);
        out.layer_idx.push(soa.layer_idx[i]);
        out.side_keys.push(soa.side_keys[i].clone());
    }
    out
}

/// A zone's geometric centre in world metres — a circle's centre, or a polygon's vertex mean.
/// `None` for a shapeless row (a draw that was never committed), which is exactly the row a click
/// cannot centre on and therefore must not advertise.
///
/// The shape vocabulary is the document's, read the same way `editor_ops::zone_rows` reads it:
/// `shape.circle {x, z, r}` / `shape.polygon [[x, z], …]`, where **the map's world `y` IS that `z`**
/// (`eden_zones::circle_from_clicks`' note). Read here off the JSON rather than through `ZoneRow`
/// because `editor_ops` is a wasm32-only module and this must run on the native test shell.
fn zone_centre(zone: &serde_json::Value) -> Option<(f64, f64)> {
    let shape = zone.get("shape")?;
    if let Some(c) = shape.get("circle") {
        if let (Some(x), Some(z)) = (
            c.get("x").and_then(serde_json::Value::as_f64),
            c.get("z").and_then(serde_json::Value::as_f64),
        ) {
            return Some((x, z));
        }
    }
    let ring = shape.get("polygon")?.as_array()?;
    let verts: Vec<(f64, f64)> = ring
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
        })
        .collect();
    if verts.is_empty() {
        return None;
    }
    let n = verts.len() as f64;
    let (sx, sz) = verts
        .iter()
        .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
    Some((sx / n, sz / n))
}

/// T-743 — where a PLAIN `Ctrl/Cmd+V` anchors when the map cursor is not available.
///
/// **Why this function exists at all.** `MissionDocCore::paste_slots` takes an OPTIONAL anchor, and
/// "no anchor" now means one thing only: paste every slot on its source coordinates
/// (`Ctrl/Cmd+Shift+V`). Before T-743 the same no-anchor branch was doing double duty — it also
/// absorbed the plain paste whose cursor was off-map, and paid for that with a 20 m nudge applied to
/// BOTH callers. Splitting the intents means the plain arm has to answer the off-map question here,
/// on its own, rather than by falling through to a branch named for something else.
///
/// **The decision.** An off-map plain paste anchors on the CENTRE OF THE VISIBLE MAP. It is not an
/// exotic path — `cursor` is `None` whenever the pointer sits over any chrome panel (the hierarchy
/// tree, the Attributes dock), which is exactly where it is after a click-then-Ctrl+V — so making it
/// a silent no-op would strand a real, frequent gesture. Pasting into the middle of the view keeps
/// the promise plain paste actually makes ("put the copy where I am looking"), lands it inside the
/// terrain clamp, and leaves it selected and visible. What it deliberately does NOT do is quietly
/// become paste-at-original: that is a separate command with a separate chord, and two chords that
/// do the same thing under a condition the operator cannot see is the defect this ticket fixes.
///
/// `None` out means "do not paste" and is reachable only when there is no camera to take a centre
/// from — the engine has not booted, or its matrix is singular (NaN, read as off-map by the same
/// `is_finite` rule the CUR read-out uses). There is nowhere to put the slots and no view to put
/// them in; the keypress falls through unhandled rather than inventing a coordinate.
pub(crate) fn plain_paste_anchor(
    cursor: Option<(f64, f64)>,
    view_centre: Option<(f64, f64)>,
) -> Option<(f64, f64)> {
    cursor.or(view_centre)
}
