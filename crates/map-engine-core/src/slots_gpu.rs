//! T-151.6 W6 — pure slot/cluster GPU packing + cluster gates (no wgpu).
//!
//! Mirrors Deck oracles:
//! - `useIconLayer.ts` — ring size 20 / selected 28, Aegis primary + tactical yellow
//! - `useClusterIconLayer.ts` — disc size `22 + min(26, log10(count)*12)`
//! - `state/constants.ts` — `CLUSTER_SLOT_THRESHOLD=500`, `ZOOM_CLUSTER_MAX=-4`

/// Icon instance stride (pos2 + size + yaw_i16 + glyph_u16 + tint_u32).
pub const SLOT_ICON_STRIDE: usize = 20;

/// Glyph index in the dedicated slot atlas (ring).
pub const SLOT_GLYPH_RING: u16 = 0;
/// Glyph index in the dedicated slot atlas (solid disc).
pub const SLOT_GLYPH_DISC: u16 = 1;

/// Base ring size in CSS pixels (`useIconLayer` getSize).
pub const SLOT_RING_PX: f32 = 20.0;
/// Selected ring size in CSS pixels.
pub const SLOT_SELECTED_PX: f32 = 28.0;

/// T-180.3 — BLUFOR side tint (`#adc6ff`).
pub const SIDE_BLUFOR_RGBA: [u8; 4] = [173, 198, 255, 255];
/// T-180.3 — OPFOR side tint (`#f87171`).
pub const SIDE_OPFOR_RGBA: [u8; 4] = [248, 113, 113, 255];
/// T-180.3 — INDFOR side tint (`#22c55e`).
pub const SIDE_INDFOR_RGBA: [u8; 4] = [34, 197, 94, 255];
/// Aegis primary `#adc6ff` full alpha (= BLUFOR).
pub const SLOT_PRIMARY_RGBA: [u8; 4] = SIDE_BLUFOR_RGBA;
/// Tactical yellow `#facc15` full alpha.
pub const SLOT_SELECTED_RGBA: [u8; 4] = [250, 204, 21, 255];
/// Cluster disc primary with Deck alpha 235.
pub const CLUSTER_DISC_RGBA: [u8; 4] = [173, 198, 255, 235];

/// Map faction `key` → unselected ring RGBA. Unknown / empty → BLUFOR (C-L4).
#[must_use]
pub fn side_rgba(key: &str) -> [u8; 4] {
    match key {
        "BLUFOR" => SIDE_BLUFOR_RGBA,
        "OPFOR" => SIDE_OPFOR_RGBA,
        "INDFOR" => SIDE_INDFOR_RGBA,
        _ => SIDE_BLUFOR_RGBA,
    }
}

/// Flatten side keys → packed RGBA8 bytes for the engine slot bind (T-180.3).
#[must_use]
pub fn side_tints_rgba_bytes(side_keys: &[String]) -> Vec<u8> {
    let mut out = Vec::with_capacity(side_keys.len() * 4);
    for k in side_keys {
        out.extend_from_slice(&side_rgba(k));
    }
    out
}

/// T-065: cluster mode only when placed slots exceed this.
pub const CLUSTER_SLOT_THRESHOLD: u32 = 500;
/// T-065.2: cluster mode only at/below this deck zoom.
pub const ZOOM_CLUSTER_MAX: f64 = -4.0;

/// Pack RGBA8 as little-endian `u32` (r | g<<8 | b<<16 | a<<24).
#[must_use]
pub fn pack_rgba_u32(rgba: [u8; 4]) -> u32 {
    u32::from(rgba[0])
        | (u32::from(rgba[1]) << 8)
        | (u32::from(rgba[2]) << 16)
        | (u32::from(rgba[3]) << 24)
}

/// Cluster mode gate (T-065): `slot_len > 500 && zoom ≤ −4`.
#[must_use]
pub fn cluster_mode(slot_len: u32, deck_zoom: f64) -> bool {
    slot_len > CLUSTER_SLOT_THRESHOLD && deck_zoom <= ZOOM_CLUSTER_MAX
}

/// Disc pixel size from aggregated count (`useClusterIconLayer.discSize`).
#[must_use]
pub fn cluster_disc_size_px(count: u32) -> f32 {
    let c = count.max(1) as f64;
    let extra = (c.log10() * 12.0).min(26.0);
    (22.0 + extra) as f32
}

/// Pack one 20 B icon instance (WORLD meters for pos; size in **pixels** for slot atlas
/// with `px_to_m` uniform, or meters when `px_to_m = 1`).
///
/// Yaw is **0** — a symmetric glyph, so orientation is invisible. This is the T-832 defect when the
/// glyph is *not* symmetric: use [`pack_icon_instance_yaw`] for anything that must point.
pub fn pack_icon_instance(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_px: f32,
    glyph: u16,
    tint: u32,
) {
    pack_icon_instance_yaw(out, pos_x, pos_y, size_px, 0.0, glyph, tint);
}

/// T-832 — pack one 20 B icon instance carrying a real **screen-CCW yaw** in degrees.
///
/// The slot lane hardcoded `yaw = 0` from T-151.6 onward (see [`pack_icon_instance`]) while the
/// shader has always rotated by the field (`shader.wgsl` `vs_icon`: `deg = yaw/32767*180`), so
/// T-795's rotate ring moved `position.rotation` in the document and nothing on the map turned.
/// Feed [`screen_yaw_for_heading_deg`] here, not the raw document heading — the document stores a
/// COMPASS bearing and the shader wants a screen CCW angle; they differ by a sign.
pub fn pack_icon_instance_yaw(
    out: &mut Vec<u8>,
    pos_x: f32,
    pos_y: f32,
    size_px: f32,
    yaw_deg: f64,
    glyph: u16,
    tint: u32,
) {
    out.extend_from_slice(&pos_x.to_le_bytes());
    out.extend_from_slice(&pos_y.to_le_bytes());
    out.extend_from_slice(&size_px.to_le_bytes());
    out.extend_from_slice(&yaw_to_snorm16(yaw_deg).to_le_bytes());
    out.extend_from_slice(&glyph.to_le_bytes());
    out.extend_from_slice(&tint.to_le_bytes());
}

/// Encode screen-CCW degrees as the lane's `snorm16` (`angle/180` clamped to `[-1,1]` × 32767).
///
/// Byte-for-byte the encoder `world::glyph_math::yaw_to_snorm16` already applies to the tree / prop /
/// text lanes; it is re-stated here because `slots_gpu` is un-gated and `world` is behind a cargo
/// feature, so slots may not depend on it. `yaw_encoders_agree` (a `--features world` test in this
/// file) sweeps both over the same inputs so the copy cannot drift.
#[must_use]
pub fn yaw_to_snorm16(angle_deg: f64) -> i16 {
    if !angle_deg.is_finite() || angle_deg == 0.0 {
        return 0;
    }
    let n = (angle_deg / 180.0).clamp(-1.0, 1.0);
    #[allow(clippy::cast_possible_truncation)]
    {
        (n * 32767.0).round() as i16
    }
}

/// Document heading → the screen-CCW yaw the icon shader wants.
///
/// The document's `position.rotation` / wire `headingDeg` is a **compass bearing: clockwise from
/// north (+Y)** — the convention `mission_editor::bearing_to_face` authors and the spawn export
/// reads. The shader rotates the quad **counter-clockwise** in world XY. So the two differ by a
/// sign, exactly as `world::glyph_math::deck_angle_for_rotation_deg` already states for the world
/// lanes. Non-finite → 0; `0.0` returns `+0.0` (never `-0.0`, which would round-trip to a different
/// literal in a byte pin).
#[must_use]
pub fn screen_yaw_for_heading_deg(heading_deg: f64) -> f64 {
    if !heading_deg.is_finite() || heading_deg == 0.0 {
        return 0.0;
    }
    -heading_deg
}

/// Pack slot rings from interleaved `xy` (`[x0,y0,…]`, length `2·n`).
/// `selected[i]` true → yellow + 28 px, else `side_tints[i]` (or BLUFOR if short) + 20 px.
///
/// # Panics
/// Never; short `selected` / `side_tints` pad as unselected / BLUFOR.
#[must_use]
pub fn pack_slot_instances(xy: &[f32], selected: &[bool], side_tints: &[[u8; 4]]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let sel = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let (size, tint) = if is_sel {
            (SLOT_SELECTED_PX, sel)
        } else {
            let rgba = side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA);
            (SLOT_RING_PX, pack_rgba_u32(rgba))
        };
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_RING, tint);
    }
    out
}

/// Pack rings from faction side keys (T-180.3). Maps each key via [`side_rgba`].
#[must_use]
pub fn pack_rings(xy: &[f32], selected: &[bool], side_keys: &[&str]) -> Vec<u8> {
    let tints: Vec<[u8; 4]> = side_keys.iter().map(|k| side_rgba(k)).collect();
    pack_slot_instances(xy, selected, &tints)
}

/// Pack a single slot instance at world `(x,y)` with selection flag.
#[must_use]
pub fn pack_one_slot(x: f32, y: f32, selected: bool) -> [u8; SLOT_ICON_STRIDE] {
    let mut v = Vec::with_capacity(SLOT_ICON_STRIDE);
    let (size, tint) = if selected {
        (SLOT_SELECTED_PX, pack_rgba_u32(SLOT_SELECTED_RGBA))
    } else {
        (SLOT_RING_PX, pack_rgba_u32(SLOT_PRIMARY_RGBA))
    };
    pack_icon_instance(&mut v, x, y, size, SLOT_GLYPH_RING, tint);
    let mut arr = [0u8; SLOT_ICON_STRIDE];
    arr.copy_from_slice(&v);
    arr
}

/// T-180.8 — pack mission vehicle discs (tactical yellow, distinct from slot rings).
/// `xy` is interleaved `[x0,y0,…]` in world meters. Empty → empty buffer.
#[must_use]
pub fn pack_vehicle_instances(xy: &[f32]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        pack_icon_instance(&mut out, x, y, SLOT_RING_PX, SLOT_GLYPH_DISC, tint);
    }
    out
}

/// Pack cluster disc markers: parallel `xs`/`ys`/`counts` (world meters).
#[must_use]
pub fn pack_cluster_instances(xs: &[f64], ys: &[f64], counts: &[u32]) -> Vec<u8> {
    let n = xs.len().min(ys.len()).min(counts.len());
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let tint = pack_rgba_u32(CLUSTER_DISC_RGBA);
    for i in 0..n {
        #[allow(clippy::cast_possible_truncation)]
        let x = xs[i] as f32;
        #[allow(clippy::cast_possible_truncation)]
        let y = ys[i] as f32;
        let size = cluster_disc_size_px(counts[i]);
        pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_DISC, tint);
    }
    out
}

/// World-meter drag delta applied in the shader (anchor cancels: same in relative space).
#[must_use]
pub fn drag_projected(base_x: f64, base_y: f64, dx: f64, dy: f64) -> (f64, f64) {
    (base_x + dx, base_y + dy)
}

/// Meters per CSS pixel at deck zoom (`scale = 2^zoom` → m/px = `2^(-zoom)`).
#[must_use]
pub fn px_to_m_at_zoom(deck_zoom: f64) -> f32 {
    if !deck_zoom.is_finite() {
        return 1.0;
    }
    #[allow(clippy::cast_possible_truncation)]
    {
        2.0_f64.powf(-deck_zoom) as f32
    }
}

/// Drag GPU phase for the slot overlay lane (T-151.7.1 / T-151.7.3).
///
/// - `Start` / `Restart` → one overlay upload; `Delta` → `set_slot_drag_delta` only; `End` → clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragGpuPhase {
    Idle,
    Start,
    Delta,
    Restart,
    End,
}

/// Classify a drag store transition for the GPU bridge (pure; mirrors W7.1 TS helper).
#[must_use]
pub fn classify_drag_transition(
    had: bool,
    has: bool,
    ids_changed: bool,
    delta_changed: bool,
) -> DragGpuPhase {
    if !had && has {
        return DragGpuPhase::Start;
    }
    if had && !has {
        return DragGpuPhase::End;
    }
    if had && has && ids_changed {
        return DragGpuPhase::Restart;
    }
    if had && has && delta_changed {
        return DragGpuPhase::Delta;
    }
    DragGpuPhase::Idle
}

/// Pack only selected slot rings (cluster short-lane / selection-only path).
/// Full-doc row index is **not** preserved — output is dense k selected instances.
#[must_use]
pub fn pack_selection_only(xy: &[f32], selected: &[bool]) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::new();
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        if !selected.get(i).copied().unwrap_or(false) {
            continue;
        }
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        pack_icon_instance(&mut out, x, y, SLOT_SELECTED_PX, SLOT_GLYPH_RING, tint);
    }
    out
}

/// 12 B hide patch for base-lane size/yaw/glyph/tint at instance offset+8 (alpha 0 tint).
#[must_use]
pub fn hide_slot_row_patch() -> [u8; 12] {
    let mut hide = [0u8; 12];
    hide[0..4].copy_from_slice(&SLOT_SELECTED_PX.to_le_bytes());
    // yaw i16 = 0, glyph u16 = 0, tint u32 = 0 (alpha 0)
    hide
}

/// T-175 B4 — 12 B **selected** row patch for the base slot lane at instance offset+8:
/// `[size=28 px, yaw=0, glyph=SLOT_GLYPH_RING(0), tint=tactical-yellow]`. Byte-identical to what
/// `pack_slot_instances` emits for a selected row, so an O(delta) sub-row patch matches a full
/// rematerialize exactly (slot rows always carry yaw 0 / glyph 0).
#[must_use]
pub fn selected_row_patch() -> [u8; 12] {
    let mut p = [0u8; 12];
    p[0..4].copy_from_slice(&SLOT_SELECTED_PX.to_le_bytes());
    // yaw i16 = 0, glyph u16 = SLOT_GLYPH_RING (0) — left zero
    p[8..12].copy_from_slice(&pack_rgba_u32(SLOT_SELECTED_RGBA).to_le_bytes());
    p
}

/// T-175 B4 / T-180.3 — 12 B **unselected** row patch for a concrete side tint:
/// `[size=20 px, yaw=0, glyph=0, tint=rgba]`.
#[must_use]
pub fn unselected_row_patch_for(rgba: [u8; 4]) -> [u8; 12] {
    let mut p = [0u8; 12];
    p[0..4].copy_from_slice(&SLOT_RING_PX.to_le_bytes());
    p[8..12].copy_from_slice(&pack_rgba_u32(rgba).to_le_bytes());
    p
}

/// Unselected patch with BLUFOR tint (compat / default when side unknown).
#[must_use]
pub fn unselected_row_patch() -> [u8; 12] {
    unselected_row_patch_for(SIDE_BLUFOR_RGBA)
}

/// Pack drag overlay instances for the given drag ids (lookup by id → row in `ids`/`xy`).
/// Returns packed bytes + parallel full-doc row indices that were hidden (for base patches).
#[must_use]
pub fn pack_drag_overlay(drag_ids: &[String], ids: &[String], xy: &[f32]) -> (Vec<u8>, Vec<usize>) {
    let mut id_to_row: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        id_to_row.insert(id.as_str(), i);
    }
    let tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    let mut out = Vec::with_capacity(drag_ids.len() * SLOT_ICON_STRIDE);
    let mut rows = Vec::with_capacity(drag_ids.len());
    for id in drag_ids {
        let Some(&row) = id_to_row.get(id.as_str()) else {
            continue;
        };
        let x = xy.get(row * 2).copied().unwrap_or(0.0);
        let y = xy.get(row * 2 + 1).copied().unwrap_or(0.0);
        pack_icon_instance(&mut out, x, y, SLOT_SELECTED_PX, SLOT_GLYPH_RING, tint);
        rows.push(row);
    }
    (out, rows)
}

/// T-573 — the previewed **`MissionVehicles`** lane for a live drag: interleaved `[x0,y0,…]` world
/// meters ready for `RenderEngine::vehicles_bind`, with the dragged vehicles offset by `(dx, dy)`.
///
/// `drag_ids` is the **whole** dragged selection — slot ids and vehicle ids mixed, in gesture order.
/// `points` is every *placed* vehicle as `(id, world_x, world_y)`: the host's
/// `editor_ops::vehicle_points`, i.e. the same list [`crate::doc::MissionDocCore::pick_vehicle`]
/// picks against, so anything the operator can grab is by construction a row here. A `drag_ids`
/// entry that names no vehicle — the slot half of a mixed selection — matches nothing and is
/// **skipped**, never resolved to a row.
///
/// **The whole lane comes back, not just the dragged rows.** `MissionVehicles` is a dense pack with
/// no ids and no stable row identity inside the engine, and `vehicles_bind` re-uploads it wholesale,
/// so a row left out of this vector would *vanish* mid-drag rather than stay put. That property is
/// also why this cures T-573 without teaching the engine about vehicle ids and without SoA row
/// patching: the re-pack **is** the update.
///
/// Two deliberate departures from the T-573 diagnosis, both to keep `map-engine-render` untouched:
/// the parameters are the host's `(id, x, y)` triples rather than a split `ids` + `xy` (so the f64→
/// f32 truncation and the interleave are proven *here*, not in un-testable wasm-only host code),
/// and the return is the **xy lane** rather than packed instance bytes (so the existing
/// `vehicles_bind(&[f32])` consumes it as-is — no new engine entry point, no engine signature
/// change). Empty `drag_ids` therefore also means "restore": the identity re-pack of the lane.
///
/// T-574 — the example below is a **pin**, not decoration. `vehicle_drag_preview_*` in this file's
/// `tests` module is the primary behavioural proof, but it compiles under `--cfg test`, so a
/// `#[cfg(not(test))]` twin of this function could gut the shipped preview while the unit tests
/// exercised an honest `#[cfg(test)]` copy. A doctest links against the crate built **without**
/// `--cfg test`, so it is the one check here that shape cannot hide from.
///
/// ```
/// use map_engine_core::slots_gpu::pack_vehicle_drag_preview;
///
/// let points = vec![
///     ("v-parked".to_string(), 10.0, 20.0),
///     ("v-dragged".to_string(), 30.0, 40.0),
/// ];
/// // The mixed selection that was broken: one slot id (which names no vehicle) + one vehicle id.
/// let drag = vec!["slot-7".to_string(), "v-dragged".to_string()];
///
/// assert_eq!(
///     pack_vehicle_drag_preview(&drag, &points, 7.5, -3.25),
///     vec![10.0_f32, 20.0, 37.5, 36.75],
///     "the dragged vehicle moves, the parked one does not, and the slot id resolves to no row"
/// );
/// ```
#[must_use]
pub fn pack_vehicle_drag_preview(
    drag_ids: &[String],
    points: &[(String, f64, f64)],
    dx: f64,
    dy: f64,
) -> Vec<f32> {
    let dragged: std::collections::HashSet<&str> = drag_ids.iter().map(String::as_str).collect();
    let mut out = Vec::with_capacity(points.len() * 2);
    for (id, x, y) in points {
        let (wx, wy) = if dragged.contains(id.as_str()) {
            (x + dx, y + dy)
        } else {
            (*x, *y)
        };
        #[allow(clippy::cast_possible_truncation)]
        {
            out.push(wx as f32);
            out.push(wy as f32);
        }
    }
    out
}

/// Build a dense `selected[i]` mask from SoA ids + selected id set.
#[must_use]
pub fn selected_mask(ids: &[String], selected: &std::collections::HashSet<String>) -> Vec<bool> {
    ids.iter().map(|id| selected.contains(id)).collect()
}

/// Slot/cluster atlas dimensions — two 64 px cells side by side (ring | disc), the
/// `slotAtlas.ts` contract the engine's UV table + pipeline were built against.
pub const SLOT_ATLAS_W: u32 = 128;
pub const SLOT_ATLAS_H: u32 = 64;
/// Flat per-glyph UV table: minU,minV,maxU,maxV for ring (glyph 0) and disc (glyph 1).
pub const SLOT_ATLAS_UV: [f32; 8] = [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0];

/// Procedurally built slot atlas pixels (T-172 B4). The engine's `ensure_slot_atlas` takes
/// caller-built RGBA — the React app built this on a 2D canvas; the Leptos host builds it here.
pub struct SlotAtlas {
    /// `SLOT_ATLAS_W × SLOT_ATLAS_H` straight-alpha RGBA, white-on-alpha (tint multiplies).
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub uv: [f32; 8],
}

/// Build the two-glyph atlas: glyph 0 = ring (outer r 24, inner r 10), glyph 1 = solid disc
/// (r 26) — the `slotAtlas.ts` radii. 1 px analytic edge coverage stands in for canvas arc AA
/// (visually equivalent at the 20–28 px render sizes).
#[must_use]
pub fn build_slot_atlas() -> SlotAtlas {
    let (w, h) = (SLOT_ATLAS_W as usize, SLOT_ATLAS_H as usize);
    let mut rgba = vec![0u8; w * h * 4];
    // Coverage of a disc of radius `r` at distance `d`, with a 1 px linear edge.
    let cov = |d: f64, r: f64| (r + 0.5 - d).clamp(0.0, 1.0);
    for y in 0..h {
        for x in 0..w {
            let (cx, ring) = if x < 64 { (32.0, true) } else { (96.0, false) };
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - 32.0;
            let d = (dx * dx + dy * dy).sqrt();
            let a = if ring {
                cov(d, 24.0) - cov(d, 10.0)
            } else {
                cov(d, 26.0)
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
            let i = (y * w + x) * 4;
            rgba[i..i + 4].copy_from_slice(&[255, 255, 255, a8]);
        }
    }
    SlotAtlas {
        rgba,
        width: SLOT_ATLAS_W,
        height: SLOT_ATLAS_H,
        uv: SLOT_ATLAS_UV,
    }
}

// ── T-808 unit symbology (F-10) ──────────────────────────────────────────────────────────────────
//
// The map was dot soup: every slot drew `SLOT_GLYPH_RING` at yaw 0, so a rifleman, a medic and an AT
// gunner were the same pale circle, a marker was a circle, a comment was a circle, and heading was
// invisible. This module is the pure half of the cure — the engine (`map-engine-render`, wasm-only,
// therefore untestable natively) does nothing but pass data through these functions.
//
// THE VISUAL LANGUAGE (operator, 2026-08-10) — three SHAPE CLASSES that cannot be confused:
//   * UNIT  — a filled side-coloured CIRCLE WITH A FACING POINT (the point IS the heading), with the
//     role class knocked OUT of the disc so one tint paints body and symbol together.
//   * VEHICLE — a TOP-DOWN SILHOUETTE per kind ("a BTR looks like a BTR from above"): the hull's nose
//     is the facing, so no separate point.
//   * COMMENT — an OUTLINE speech bubble with a tail, in a neutral (non-selection) colour.
//   * MARKER — T-790's canonical set (`scene::MarkerGlyph`): centred, un-pointed, un-knocked-out
//     shapes. Untouched here; see the ticket's marker_delta report.
//
// ATLAS: these cells are APPENDED to whatever slot atlas the host uploads (T-790's widened marker
// atlas today) by [`extend_atlas_with_unit_glyphs`], so cells 0/1 stay byte-identical to
// `build_slot_atlas` by construction — the pin is a memcpy, not a re-derivation.

/// Screen size (CSS px) of an unselected unit glyph. Larger than [`SLOT_RING_PX`] because the role
/// symbol is knocked out of a 40 px-diameter body inside a 64 px cell: at 20 px the interior is
/// ~12 px across and a 4 px-stroke knockout lands sub-pixel. Selected still uses
/// [`SLOT_SELECTED_PX`], so the selection size step is unchanged.
pub const SLOT_UNIT_PX: f32 = 24.0;
/// Screen size (CSS px) of a vehicle silhouette — the hull is longer than it is wide, so it needs
/// more cell than a unit disc to stay readable.
pub const VEHICLE_SYMBOL_PX: f32 = 26.0;
/// Screen size (CSS px) of an unselected comment bubble.
pub const COMMENT_NOTE_PX: f32 = 22.0;

/// T-796 — comment tint: slate-300 `#cbd5e1`. Deliberately **not** [`SLOT_SELECTED_RGBA`]: every
/// comment rendered in the selection amber, so the map claimed a permanent selection that no click
/// had made and a genuinely selected comment was indistinguishable from an idle one.
pub const COMMENT_NOTE_RGBA: [u8; 4] = [203, 213, 225, 255];

/// Above this many metres per CSS pixel the symbology DEGRADES TO DOTS (plain
/// [`SLOT_GLYPH_DISC`]) — the stated zoom threshold the ticket requires in code.
///
/// Chosen from the glyph's own geometry, not taste: a [`SLOT_UNIT_PX`] glyph covers `24 · m_per_px`
/// metres of ground, so at 8 m/px it spans 192 m while the squad members it must separate stand
/// 10–20 m apart — under 3 px between centres. The role knockout is ~7/64 of the cell, i.e. under
/// 3 px of the 24 px glyph at ANY zoom, so past this point the interior is a smear and a clean disc
/// is the more honest mark. Both acceptance screenshot zooms (1.0 and 4.0 m/px) sit well inside.
pub const SYMBOLOGY_MAX_M_PER_PX: f32 = 8.0;

/// True when the camera is close enough to draw symbology rather than dots
/// (see [`SYMBOLOGY_MAX_M_PER_PX`]). Non-finite / non-positive scales degrade (fail safe).
#[must_use]
pub fn symbology_visible(m_per_px: f32) -> bool {
    m_per_px.is_finite() && m_per_px > 0.0 && m_per_px <= SYMBOLOGY_MAX_M_PER_PX
}

/// Role class of one slot — the drawable distinction a milsim author reads a map by.
///
/// This is the MINIMUM SET the ticket names (leader / medic / AT / MG / rifleman default) and the
/// discriminant is the symbology cell offset, so `class as u16` indexes the atlas directly.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitRoleClass {
    /// Plain filled disc — the DEFAULT for every unrecognised role or kit.
    Rifleman = 0,
    /// Chevron knocked out of the disc. Squad / team / platoon leaders, commanders, officers.
    Leader = 1,
    /// Plus cross knocked out of the disc. Medic / corpsman / CLS.
    Medic = 2,
    /// Solid down-triangle knocked out of the disc. AT gunner / launcher / assistant.
    AntiTank = 3,
    /// Twin horizontal bars knocked out of the disc. Automatic rifleman / MG / SAW gunner.
    MachineGun = 4,
}

/// Number of [`UnitRoleClass`] variants (also the count of unselected unit cells in the atlas).
pub const UNIT_ROLE_CLASS_COUNT: usize = 5;

/// Top-down vehicle silhouette class. Discriminant is the offset within the vehicle cell block.
///
/// The seeded roster is M1025 / M998 / M923A1 / M113, which is exactly the three classes the
/// operator named: wheeled light, truck, APC.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VehicleKind {
    /// Short 4-wheel hull with a chamfered nose — M1025 / M998 Humvee, jeeps, UAZ/BRDM-sized light
    /// wheeled. The DEFAULT for an unrecognised alias (the commonest thing on a mission map).
    WheeledLight = 0,
    /// Long hull with a separate cab block and six wheels — M923A1, Ural, cargo/fuel/supply trucks.
    Truck = 1,
    /// Trapezoid hull with continuous TRACK RAILS — M113, BTR/BMP-class carriers, IFVs, tanks.
    Apc = 2,
}

/// Number of [`VehicleKind`] variants (also the count of vehicle cells in the atlas).
pub const VEHICLE_KIND_COUNT: usize = 3;

// ── symbology cell layout (offsets from the appended block's base) ───────────────────────────────

/// Offset of the unselected unit block: cell `UNIT_CELL_BASE + (class as u16)`.
pub const UNIT_CELL_BASE: u16 = 0;
/// Offset of the SELECTED unit block — the same five role glyphs with the selection RING added
/// around them, so "selection ring + heading + role" is one instance and the O(delta) row patch in
/// `set_selection` keeps working (a second overlay instance per selected row would force a full
/// re-pack on every click).
pub const UNIT_SELECTED_CELL_BASE: u16 = 5;
/// Offset of the vehicle silhouette block: cell `VEHICLE_CELL_BASE + (kind as u16)`.
pub const VEHICLE_CELL_BASE: u16 = 10;
/// Offset of the comment speech-bubble cell.
pub const COMMENT_CELL: u16 = 13;
/// Offset of the SELECTED comment cell (bubble + selection ring).
pub const COMMENT_SELECTED_CELL: u16 = 14;
/// Total symbology cells [`extend_atlas_with_unit_glyphs`] appends.
pub const SYMBOLOGY_CELL_COUNT: usize = 15;

/// Cell edge in pixels — the slot/marker atlas convention (`build_slot_atlas`,
/// `scene::build_marker_slot_atlas`). An incoming atlas that is not a horizontal strip of these
/// cells is refused by [`extend_atlas_with_unit_glyphs`] rather than mangled.
pub const ATLAS_CELL_PX: u32 = 64;

/// Normalise an authored role / kit string: strip a `kit:` (or `veh:` / `vehicle:`) registry prefix,
/// trim, lowercase, and fold `-` / space / `/` to `_`.
///
/// Mirrors `scene::normalise_alias` (T-790) so the marker table and this one agree on what "the same
/// word" means, plus the prefix strip: the feeder may hand over `kit:us_medic` (the registry alias)
/// or `Medic` (the authored ORBAT role) and both must land on the same class.
#[must_use]
fn normalise_role(raw: &str) -> String {
    let t = raw
        .trim()
        .to_ascii_lowercase()
        .replace(['-', ' ', '/'], "_");
    for p in ["kit:", "veh:", "vehicle:", "preset:", "comp:"] {
        if let Some(rest) = t.strip_prefix(p) {
            return rest.to_string();
        }
    }
    t
}

/// **THE role → glyph table.** One place, unit-tested against every seeded kit; nothing else in the
/// tree may re-derive this mapping.
///
/// Accepts either vocabulary the feeder has — the registry kit alias (`kit:us_rifleman`) or the
/// authored ORBAT role string (`Squad Leader`, `AT Gunner`) — because `SlotSoa` carries `roles` and
/// the document carries `kit`, and which one is populated depends on whether the slot came from a
/// flatten or from hand placement. Matching is on `_`-separated TOKENS of the normalised string, so
/// `us_sl`, `SL`, `sl` and `Fire Team SL` all resolve, and a faction prefix can never shadow a role.
///
/// PRIORITY, when a string carries more than one cue, is the order the ticket names the classes:
/// leader, medic, AT, MG, rifleman. A "Medic Team Leader" therefore draws as a leader — the map is
/// read for command structure first, and the tie is resolved somewhere rather than by hash order.
///
/// Unknown / empty → [`UnitRoleClass::Rifleman`], the graceful downgrade that keeps an unforeseen
/// role on the map as a unit instead of dropping it.
#[must_use]
pub fn unit_role_class(role_or_kit: &str) -> UnitRoleClass {
    let n = normalise_role(role_or_kit);
    let has = |w: &str| n.split('_').any(|t| t == w);

    // Leader — seeded kits `*_sl` (squad leader) and `us_tl` (team leader).
    if has("sl")
        || has("tl")
        || has("pl")
        || has("co")
        || has("xo")
        || has("leader")
        || has("lead")
        || has("commander")
        || has("officer")
        || has("squadleader")
        || n.contains("squad_leader")
        || n.contains("team_leader")
        || n.contains("platoon_leader")
    {
        return UnitRoleClass::Leader;
    }
    // Medic — seeded kits `us_medic`, `fia_medic`.
    if has("medic")
        || has("medical")
        || has("corpsman")
        || has("cls")
        || has("doc")
        || has("aid")
        || has("casevac")
        || has("medevac")
    {
        return UnitRoleClass::Medic;
    }
    // Anti-tank. `at` as a whole token only — never a substring, or "Ambush AT Bridge" style prose
    // would be safe but `attack` / `station` would not be.
    if has("at")
        || has("atgm")
        || has("antitank")
        || has("rpg")
        || has("law")
        || has("panzerfaust")
        || has("launcher")
        || n.contains("anti_tank")
        || n.contains("at_gunner")
        || n.contains("at_rifleman")
        || n.contains("at_assistant")
    {
        return UnitRoleClass::AntiTank;
    }
    // Machine gun — seeded kits `us_ar`, `sov_ar` (automatic rifleman).
    if has("ar")
        || has("mg")
        || has("lmg")
        || has("hmg")
        || has("gpmg")
        || has("saw")
        || has("gunner")
        || has("machinegunner")
        || has("autorifleman")
        || n.contains("machine_gun")
        || n.contains("automatic_rifleman")
    {
        return UnitRoleClass::MachineGun;
    }
    UnitRoleClass::Rifleman
}

/// **THE vehicle alias → silhouette table.** Same contract as [`unit_role_class`]: one place,
/// unit-tested, unknown → [`VehicleKind::WheeledLight`].
///
/// Matches on the normalised alias as a SUBSTRING (not tokens) because a vehicle alias may be either
/// a registry key (`veh:us_m1025`) or a prefab path fragment
/// (`Prefabs/Vehicles/Wheeled/M998/M1025.et`), and the model designator is embedded in both.
#[must_use]
pub fn vehicle_kind_for_alias(alias: &str) -> VehicleKind {
    let n = normalise_role(alias);
    // Tracked first: an M113 path also contains "vehicles", and "apc" must beat "truck" in a name
    // like "apc_truck_variant".
    if n.contains("m113")
        || n.contains("apc")
        || n.contains("btr")
        || n.contains("bmp")
        || n.contains("bradley")
        || n.contains("tracked")
        || n.contains("tank")
        || n.contains("ifv")
    {
        return VehicleKind::Apc;
    }
    if n.contains("m923")
        || n.contains("truck")
        || n.contains("ural")
        || n.contains("cargo")
        || n.contains("supply")
        || n.contains("fuel")
        || n.contains("m35")
    {
        return VehicleKind::Truck;
    }
    VehicleKind::WheeledLight
}

/// Anti-aliased half-plane coverage: 1 well inside, 0 well outside, a one-pixel linear ramp across
/// `s = 0` (positive = inside). Same convention as `scene::edge_cov` (T-790).
#[must_use]
fn edge(s: f64) -> f64 {
    (s + 0.5).clamp(0.0, 1.0)
}

/// The facing POINT shared by every unit glyph: a triangle whose apex sits at the cell's NORTH edge.
///
/// North is `-dy` because the shader flips v (`shader.wgsl`: `1.0 - unit.y`), so cell row 0 is drawn
/// at `+Y` on a north-up map. At yaw 0 the point therefore aims at world north, and the shader's CCW
/// rotation by [`screen_yaw_for_heading_deg`] swings it to the document's compass heading.
#[must_use]
fn facing_point_cov(dx: f64, dy: f64) -> f64 {
    // apex at dy = -30, base at dy = -10 where the half-width reaches 11 (inside the r20 body, so
    // the spike reads as ~10 px of cell emerging past the disc edge at dy = -20).
    let t = ((dy + 30.0) / 20.0).clamp(0.0, 1.0);
    edge(dy + 30.0)
        .min(edge(-10.0 - dy))
        .min(edge(11.0f64.mul_add(t, -dx.abs())))
}

/// The role symbol KNOCKED OUT of the unit body. White-on-alpha means one tint paints the whole
/// glyph, so a symbol drawn *on* the disc would be invisible; a hole is the only way a filled
/// side-coloured circle can also carry a role.
#[must_use]
fn role_knockout_cov(class: u16, dx: f64, dy: f64) -> f64 {
    match class {
        // Rifleman — no knockout. Maximum ink: the plainest mark for the commonest role.
        0 => 0.0,
        // Leader — chevron "^", 45° arms, apex up. Held inside |dx| ≤ 13 so a solid rim survives all
        // the way round: arms that reached the r20 edge notched the disc's outline and the glyph
        // read as a broken circle rather than a circle carrying a chevron.
        1 => {
            let a = edge(3.5 - (dy + 2.0 + dx).abs());
            let b = edge(3.5 - (dy + 2.0 - dx).abs());
            a.max(b)
                .min(edge(dy + 10.0))
                .min(edge(9.0 - dy))
                .min(edge(13.0 - dx.abs()))
        }
        // Medic — plus cross.
        2 => {
            let v = edge(4.0 - dx.abs()).min(edge(12.0 - dy.abs()));
            let h = edge(4.0 - dy.abs()).min(edge(12.0 - dx.abs()));
            v.max(h)
        }
        // Anti-tank — solid triangle pointing DOWN (apex low), the armour-piercing wedge. Opposite
        // sense to the leader chevron so the two can never be read for one another.
        3 => {
            let t = ((12.0 - dy) / 22.0).clamp(0.0, 1.0);
            edge(12.0 - dy)
                .min(edge(dy + 10.0))
                .min(edge(11.0f64.mul_add(t, -dx.abs())))
        }
        // Machine gun — twin horizontal bars.
        4 => {
            let top = edge(3.0 - (dy + 6.0).abs());
            let bot = edge(3.0 - (dy - 6.0).abs());
            top.max(bot).min(edge(12.0 - dx.abs()))
        }
        _ => 0.0,
    }
}

/// The SELECTION RING: an annulus outside the r20 unit body / the comment bubble, inside the cell.
/// Drawn INTO the selected cell rather than as a second instance — see [`UNIT_SELECTED_CELL_BASE`].
#[must_use]
fn selection_ring_cov(d: f64) -> f64 {
    let cov = |r: f64| (r + 0.5 - d).clamp(0.0, 1.0);
    cov(30.0) - cov(25.0)
}

/// Top-down vehicle silhouettes, nose to the NORTH (`-dy`) exactly like the unit facing point.
#[must_use]
fn vehicle_silhouette_cov(kind: u16, dx: f64, dy: f64) -> f64 {
    let box_cov = |cx: f64, cy: f64, hx: f64, hy: f64| {
        edge(hx - (dx - cx).abs()).min(edge(hy - (dy - cy).abs()))
    };
    match kind {
        // WHEELED LIGHT (M1025 / M998) — short hull, chamfered nose, four wheels. The chamfer runs
        // a full 12 px and takes the half-width down to 3.5: a subtler nose left the hull looking
        // like a symmetric box, which would hide the heading the whole silhouette exists to show.
        0 => {
            let hw = if dy < -8.0 {
                5.5f64.mul_add(-((-8.0 - dy) / 12.0), 9.0)
            } else {
                9.0
            };
            let hull = edge(20.0 - dy.abs()).min(edge(hw - dx.abs()));
            hull.max(box_cov(-13.0, -12.0, 4.0, 6.0))
                .max(box_cov(13.0, -12.0, 4.0, 6.0))
                .max(box_cov(-13.0, 12.0, 4.0, 6.0))
                .max(box_cov(13.0, 12.0, 4.0, 6.0))
        }
        // TRUCK (M923A1) — cab block, a transverse GAP, a long bed, six wheels.
        1 => {
            let cab = edge(dy + 26.0)
                .min(edge(-14.0 - dy))
                .min(edge(10.0 - dx.abs()));
            let bed = edge(dy + 11.0)
                .min(edge(26.0 - dy))
                .min(edge(11.0 - dx.abs()));
            cab.max(bed)
                .max(box_cov(-14.0, -18.0, 3.5, 5.0))
                .max(box_cov(14.0, -18.0, 3.5, 5.0))
                .max(box_cov(-14.0, 10.0, 3.5, 5.0))
                .max(box_cov(14.0, 10.0, 3.5, 5.0))
                .max(box_cov(-14.0, 20.0, 3.5, 5.0))
                .max(box_cov(14.0, 20.0, 3.5, 5.0))
        }
        // APC (M113) — trapezoid hull with a sloped glacis and two CONTINUOUS track rails. The
        // unbroken rails (vs discrete wheel blobs) are what separates tracked from wheeled at a
        // glance, which is the whole point of a top-down silhouette.
        2 => {
            let hw = if dy < -10.0 {
                5.0f64.mul_add(-((-10.0 - dy) / 12.0), 11.0)
            } else {
                11.0
            };
            let hull = edge(dy + 22.0)
                .min(edge(22.0 - dy))
                .min(edge(hw - dx.abs()));
            let rail = edge(dy + 20.0)
                .min(edge(22.0 - dy))
                .min(edge(2.5 - (dx.abs() - 14.5).abs()));
            hull.max(rail)
        }
        _ => 0.0,
    }
}

/// T-796 — the comment glyph: an OUTLINE rounded-rect speech bubble with a solid tail.
///
/// Outline (not filled) and rectangular (not round) is what makes a comment unmistakable against a
/// filled unit disc and against T-790's marker shapes, whose only hollow members (`Ring`, `Target`)
/// are circular.
#[must_use]
fn comment_bubble_cov(dx: f64, dy: f64) -> f64 {
    // Rounded-rect coverage: signed distance to a box of half-extents (hx, hy) centred at (0, -6),
    // grown by the corner radius r.
    let rrect = |hx: f64, hy: f64, r: f64| {
        let qx = (dx.abs() - hx).max(0.0);
        let qy = ((dy + 6.0).abs() - hy).max(0.0);
        edge(r - qx.hypot(qy))
    };
    let outline = (rrect(16.0, 8.0, 6.0) - rrect(12.0, 4.0, 5.0)).clamp(0.0, 1.0);
    // Tail: a wedge hanging off the bubble's bottom-left, from the body edge (dy = 2) to dy = 16.
    let t = ((16.0 - dy) / 14.0).clamp(0.0, 1.0);
    let tail = edge(dy - 1.0)
        .min(edge(16.0 - dy))
        .min(edge(5.0f64.mul_add(t, -(dx + 9.0).abs())));
    outline.max(tail)
}

/// Straight-alpha coverage of symbology cell `offset` at cell-local pixel `(px, py)` in a
/// [`ATLAS_CELL_PX`] cell centred at (32, 32). White-on-alpha, matching the slot / marker atlases.
#[must_use]
fn symbology_cell_coverage(offset: u16, px: f64, py: f64) -> f64 {
    let dx = px + 0.5 - 32.0;
    let dy = py + 0.5 - 32.0;
    let d = dx.hypot(dy);
    let body = (20.5 - d).clamp(0.0, 1.0); // filled disc, r 20
    match offset {
        // 0..=4 unit role glyphs; 5..=9 the same with the selection ring.
        0..=9 => {
            let class = offset % 5;
            let unit = (body - role_knockout_cov(class, dx, dy))
                .clamp(0.0, 1.0)
                .max(facing_point_cov(dx, dy));
            if offset >= UNIT_SELECTED_CELL_BASE {
                unit.max(selection_ring_cov(d))
            } else {
                unit
            }
        }
        10..=12 => vehicle_silhouette_cov(offset - VEHICLE_CELL_BASE, dx, dy),
        COMMENT_CELL => comment_bubble_cov(dx, dy),
        COMMENT_SELECTED_CELL => comment_bubble_cov(dx, dy).max(selection_ring_cov(d)),
        _ => 0.0,
    }
}

/// A caller's slot atlas with the T-808 symbology cells appended — the output of
/// [`extend_atlas_with_unit_glyphs`], ready for `RenderEngine::ensure_slot_atlas`.
pub struct WidenedSlotAtlas {
    /// Straight-alpha RGBA8, white-on-alpha (the instance tint multiplies).
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Flat `[minU,minV,maxU,maxV]·N` table over ALL cells (base + symbology).
    pub uv: Vec<f32>,
    /// The caller's cell count — i.e. the glyph id the symbology block STARTS at. Every
    /// `*_CELL_BASE` offset in this module is relative to this, so T-790's atlas may grow without
    /// this file (or any caller) knowing the number.
    pub base_cells: u16,
}

/// Append the [`SYMBOLOGY_CELL_COUNT`] unit / vehicle / comment cells to a caller-built slot atlas.
///
/// `base_rgba` is a horizontal strip of [`ATLAS_CELL_PX`] cells — [`build_slot_atlas`] (2 cells) or
/// `scene::build_marker_slot_atlas` (11 cells) — and comes back **copied verbatim**, so every glyph
/// the marker / vehicle / comment / slot lanes already select is byte-identical afterwards. That is
/// why this takes the base as DATA rather than rebuilding it: `map-engine-core` cannot see
/// `map-engine-render`'s marker table, and a re-derivation is exactly how a pinned cell drifts.
///
/// Returns `None` when `base_rgba` is not a well-formed 64 px strip (the caller should then upload
/// its atlas unchanged and leave symbology off).
#[must_use]
pub fn extend_atlas_with_unit_glyphs(
    base_rgba: &[u8],
    base_w: u32,
    base_h: u32,
) -> Option<WidenedSlotAtlas> {
    let cell = ATLAS_CELL_PX as usize;
    if base_h != ATLAS_CELL_PX || base_w == 0 || !base_w.is_multiple_of(ATLAS_CELL_PX) {
        return None;
    }
    let bw = base_w as usize;
    if base_rgba.len() != bw * cell * 4 {
        return None;
    }
    let base_cells = bw / cell;
    let n = base_cells + SYMBOLOGY_CELL_COUNT;
    let w = n * cell;
    let mut rgba = vec![0u8; w * cell * 4];
    for y in 0..cell {
        // Verbatim row copy of the caller's strip — the byte-identity guarantee.
        let src = y * bw * 4;
        let dst = y * w * 4;
        rgba[dst..dst + bw * 4].copy_from_slice(&base_rgba[src..src + bw * 4]);
        for c in 0..SYMBOLOGY_CELL_COUNT {
            let cx0 = (base_cells + c) * cell;
            for x in 0..cell {
                #[allow(clippy::cast_precision_loss)]
                let a = symbology_cell_coverage(c as u16, x as f64, y as f64);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let a8 = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
                let i = (y * w + cx0 + x) * 4;
                rgba[i..i + 4].copy_from_slice(&[255, 255, 255, a8]);
            }
        }
    }
    let mut uv = Vec::with_capacity(n * 4);
    #[allow(clippy::cast_precision_loss)]
    for c in 0..n {
        uv.extend_from_slice(&[c as f32 / n as f32, 0.0, (c + 1) as f32 / n as f32, 1.0]);
    }
    #[allow(clippy::cast_possible_truncation)]
    Some(WidenedSlotAtlas {
        rgba,
        width: w as u32,
        height: cell as u32,
        uv,
        base_cells: base_cells as u16,
    })
}

/// Pack the slot lane as UNIT SYMBOLOGY: side colour + role glyph + real heading, selection on top.
///
/// Row `i` reads `xy[2i..2i+2]` (world metres), `selected[i]`, `side_tints[i]`, `roles[i]` (the
/// registry kit alias or the authored role — see [`unit_role_class`]) and `headings_deg[i]` (the
/// document's COMPASS heading; [`screen_yaw_for_heading_deg`] converts). Every parallel array is
/// tolerant of being short: missing selection is false, missing tint is BLUFOR, missing role is
/// rifleman, missing heading is 0 — a partially-wired feeder degrades per-field instead of panicking
/// or dropping rows.
///
/// `glyph_base` is the symbology block's atlas base from [`extend_atlas_with_unit_glyphs`].
/// `m_per_px` gates [`symbology_visible`]: zoomed out past [`SYMBOLOGY_MAX_M_PER_PX`] every row
/// falls back to the plain disc at yaw 0 — dots, deliberately, because the detail is sub-pixel there.
#[must_use]
pub fn pack_slot_symbology(
    xy: &[f32],
    selected: &[bool],
    side_tints: &[[u8; 4]],
    roles: &[String],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    let sel_tint = pack_rgba_u32(SLOT_SELECTED_RGBA);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let tint = if is_sel {
            sel_tint
        } else {
            pack_rgba_u32(side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA))
        };
        if !detailed {
            // Degraded: the pre-T-808 dot, minus the ambiguity — still side-tinted, still sized by
            // selection, but no glyph detail is claimed at a zoom that cannot show it.
            let size = if is_sel {
                SLOT_SELECTED_PX
            } else {
                SLOT_RING_PX
            };
            pack_icon_instance(&mut out, x, y, size, SLOT_GLYPH_DISC, tint);
            continue;
        }
        let class = roles
            .get(i)
            .map_or(UnitRoleClass::Rifleman, |r| unit_role_class(r)) as u16;
        let block = if is_sel {
            UNIT_SELECTED_CELL_BASE
        } else {
            UNIT_CELL_BASE
        };
        let size = if is_sel {
            SLOT_SELECTED_PX
        } else {
            SLOT_UNIT_PX
        };
        let yaw =
            screen_yaw_for_heading_deg(f64::from(headings_deg.get(i).copied().unwrap_or(0.0)));
        pack_icon_instance_yaw(&mut out, x, y, size, yaw, glyph_base + block + class, tint);
    }
    out
}

/// T-175 B4 × T-808 — the 12 B `[size, yaw, glyph, tint]` block at instance offset + 8 for ONE
/// symbology row, for the O(delta) selection patch path.
///
/// **Derived by packing that row and slicing it**, not re-implemented: `set_selection` patches rows
/// in place while the rest of the lane keeps whatever `pack_slot_symbology` wrote, so a hand-rolled
/// patch that disagreed by one byte would leave the GPU in a state no rematerialize could produce.
/// Making the patch a slice of the packer's own output turns that contract into a tautology — and it
/// is why the glyph and yaw now ride the patch, which the pre-T-808 `selected_row_patch` left zero
/// because slot rows had no glyph or heading to preserve.
#[must_use]
pub fn symbology_row_patch(
    selected: bool,
    role: &str,
    heading_deg: f32,
    side_rgba: [u8; 4],
    m_per_px: f32,
    glyph_base: u16,
) -> [u8; 12] {
    let row = pack_slot_symbology(
        &[0.0, 0.0],
        &[selected],
        &[side_rgba],
        std::slice::from_ref(&role.to_string()),
        &[heading_deg],
        m_per_px,
        glyph_base,
    );
    let mut p = [0u8; 12];
    p.copy_from_slice(&row[8..20]);
    p
}

/// T-808 — the drag overlay in symbology form, so a dragged unit does not morph into a bare ring
/// for the duration of the gesture. Same `(bytes, hidden_rows)` contract as [`pack_drag_overlay`].
#[must_use]
pub fn pack_drag_overlay_symbology(
    drag_ids: &[String],
    ids: &[String],
    xy: &[f32],
    roles: &[String],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> (Vec<u8>, Vec<usize>) {
    let mut id_to_row: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::with_capacity(ids.len());
    for (i, id) in ids.iter().enumerate() {
        id_to_row.insert(id.as_str(), i);
    }
    let mut out = Vec::with_capacity(drag_ids.len() * SLOT_ICON_STRIDE);
    let mut rows = Vec::with_capacity(drag_ids.len());
    for id in drag_ids {
        let Some(&row) = id_to_row.get(id.as_str()) else {
            continue;
        };
        let x = xy.get(row * 2).copied().unwrap_or(0.0);
        let y = xy.get(row * 2 + 1).copied().unwrap_or(0.0);
        // A dragged row is drawn selected: amber + the ringed role cell, the same treatment the
        // base lane gives the selection it was grabbed from.
        let role = roles.get(row).cloned().unwrap_or_default();
        let h = headings_deg.get(row).copied().unwrap_or(0.0);
        out.extend_from_slice(&pack_slot_symbology(
            &[x, y],
            &[true],
            &[],
            std::slice::from_ref(&role),
            &[h],
            m_per_px,
            glyph_base,
        ));
        rows.push(row);
    }
    (out, rows)
}

/// Pack the mission-vehicle lane as TOP-DOWN SILHOUETTES with real heading and side colour.
///
/// Same short-array tolerance and same [`SYMBOLOGY_MAX_M_PER_PX`] degrade as
/// [`pack_slot_symbology`]. Vehicles are not on the engine's selection bridge, so there is no
/// selected variant; the degraded fallback keeps the historic [`SLOT_SELECTED_RGBA`] disc of
/// `pack_vehicle_instances` only in tint-less callers, since a side tint is supplied here.
#[must_use]
pub fn pack_vehicle_symbology(
    xy: &[f32],
    aliases: &[String],
    side_tints: &[[u8; 4]],
    headings_deg: &[f32],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let tint = pack_rgba_u32(side_tints.get(i).copied().unwrap_or(SIDE_BLUFOR_RGBA));
        if !detailed {
            pack_icon_instance(&mut out, x, y, SLOT_RING_PX, SLOT_GLYPH_DISC, tint);
            continue;
        }
        let kind = aliases
            .get(i)
            .map_or(VehicleKind::WheeledLight, |a| vehicle_kind_for_alias(a))
            as u16;
        let yaw =
            screen_yaw_for_heading_deg(f64::from(headings_deg.get(i).copied().unwrap_or(0.0)));
        pack_icon_instance_yaw(
            &mut out,
            x,
            y,
            VEHICLE_SYMBOL_PX,
            yaw,
            glyph_base + VEHICLE_CELL_BASE + kind,
            tint,
        );
    }
    out
}

/// T-796 — pack the comment lane as neutral speech bubbles, with selection layering ON TOP.
///
/// Replaces the previous "selection-amber ring" rendering, which borrowed both the colour and the
/// glyph of a selected slot: a comment looked selected at all times and looked like a unit. Selected
/// comments take [`SLOT_SELECTED_RGBA`] + [`SLOT_SELECTED_PX`] + the ringed bubble cell, so the
/// selection treatment is additive rather than the comment's only appearance.
///
/// `selected` may be empty (the feeder has no comment ids yet) — then nothing is selected and every
/// comment draws in [`COMMENT_NOTE_RGBA`], which is still the fix for the amber.
#[must_use]
pub fn pack_comment_instances(
    xy: &[f32],
    selected: &[bool],
    m_per_px: f32,
    glyph_base: u16,
) -> Vec<u8> {
    let n = xy.len() / 2;
    let mut out = Vec::with_capacity(n * SLOT_ICON_STRIDE);
    let detailed = symbology_visible(m_per_px);
    for i in 0..n {
        let x = xy[i * 2];
        let y = xy[i * 2 + 1];
        let is_sel = selected.get(i).copied().unwrap_or(false);
        let tint = pack_rgba_u32(if is_sel {
            SLOT_SELECTED_RGBA
        } else {
            COMMENT_NOTE_RGBA
        });
        let size = if is_sel {
            SLOT_SELECTED_PX
        } else {
            COMMENT_NOTE_PX
        };
        let glyph = if detailed {
            glyph_base
                + if is_sel {
                    COMMENT_SELECTED_CELL
                } else {
                    COMMENT_CELL
                }
        } else {
            SLOT_GLYPH_DISC
        };
        pack_icon_instance(&mut out, x, y, size, glyph, tint);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn icon_stride_is_20() {
        assert_eq!(SLOT_ICON_STRIDE, 20);
        let one = pack_one_slot(1.5, -2.5, false);
        assert_eq!(one.len(), 20);
    }

    /// H8 — vehicle discs use yellow disc glyph (distinct from slot rings).
    #[test]
    fn pack_vehicle_instances_disc_yellow() {
        let xy = [6400.0_f32, 6370.0];
        let bytes = pack_vehicle_instances(&xy);
        assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
        let size = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size - SLOT_RING_PX).abs() < 1e-6);
        let glyph = u16::from_le_bytes(bytes[14..16].try_into().unwrap());
        assert_eq!(glyph, SLOT_GLYPH_DISC);
        let tint = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(tint, pack_rgba_u32(SLOT_SELECTED_RGBA));
        assert!(pack_vehicle_instances(&[]).is_empty());
    }

    #[test]
    fn pack_count_matches_xy() {
        let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
        let sel = [false, true, false];
        let bytes = pack_slot_instances(&xy, &sel, &[]);
        assert_eq!(bytes.len(), 3 * SLOT_ICON_STRIDE);
        // row 1 selected → size 28
        let size1 = f32::from_le_bytes(bytes[20 + 8..20 + 12].try_into().unwrap());
        assert!((size1 - SLOT_SELECTED_PX).abs() < 1e-6);
        let tint1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
        assert_eq!(tint1, pack_rgba_u32(SLOT_SELECTED_RGBA));
        // row 0 primary → size 20
        let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size0 - SLOT_RING_PX).abs() < 1e-6);
        assert_eq!(
            u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            pack_rgba_u32(SLOT_PRIMARY_RGBA)
        );
    }

    /// C1 — exact locked RGBA triples; pairwise distinct.
    #[test]
    fn side_tint_three_distinct() {
        assert_eq!(SIDE_BLUFOR_RGBA, [173, 198, 255, 255]);
        assert_eq!(SIDE_OPFOR_RGBA, [248, 113, 113, 255]);
        assert_eq!(SIDE_INDFOR_RGBA, [34, 197, 94, 255]);
        assert_eq!(SLOT_SELECTED_RGBA, [250, 204, 21, 255]);
        assert_eq!(SLOT_PRIMARY_RGBA, SIDE_BLUFOR_RGBA);
        assert_ne!(SIDE_BLUFOR_RGBA, SIDE_OPFOR_RGBA);
        assert_ne!(SIDE_BLUFOR_RGBA, SIDE_INDFOR_RGBA);
        assert_ne!(SIDE_OPFOR_RGBA, SIDE_INDFOR_RGBA);
    }

    /// C2 — selected always yellow regardless of side key.
    #[test]
    fn selected_overrides_side_tint() {
        let xy = [10.0_f32, 20.0];
        let bytes = pack_rings(&xy, &[true], &["OPFOR"]);
        let tint = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        assert_eq!(tint, pack_rgba_u32(SLOT_SELECTED_RGBA));
        let size = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size - SLOT_SELECTED_PX).abs() < 1e-6);
    }

    /// C3 — three sides → three distinct packed tint u32.
    #[test]
    fn pack_rings_side_tints() {
        let xy = [0.0_f32, 0.0, 1.0, 1.0, 2.0, 2.0];
        let bytes = pack_rings(&xy, &[false, false, false], &["BLUFOR", "OPFOR", "INDFOR"]);
        assert_eq!(bytes.len(), 3 * SLOT_ICON_STRIDE);
        let t0 = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let t1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
        let t2 = u32::from_le_bytes(bytes[40 + 16..40 + 20].try_into().unwrap());
        assert_eq!(t0, pack_rgba_u32(SIDE_BLUFOR_RGBA));
        assert_eq!(t1, pack_rgba_u32(SIDE_OPFOR_RGBA));
        assert_eq!(t2, pack_rgba_u32(SIDE_INDFOR_RGBA));
        assert_ne!(t0, t1);
        assert_ne!(t0, t2);
        assert_ne!(t1, t2);
    }

    /// C4 — missing / unknown side → BLUFOR.
    #[test]
    fn missing_side_defaults_blufor() {
        assert_eq!(side_rgba(""), SIDE_BLUFOR_RGBA);
        assert_eq!(side_rgba("UNKNOWN"), SIDE_BLUFOR_RGBA);
        let xy = [0.0_f32, 0.0, 1.0, 1.0];
        // short side_keys → second row pads BLUFOR
        let bytes = pack_rings(&xy, &[false, false], &["OPFOR"]);
        let t0 = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let t1 = u32::from_le_bytes(bytes[20 + 16..20 + 20].try_into().unwrap());
        assert_eq!(t0, pack_rgba_u32(SIDE_OPFOR_RGBA));
        assert_eq!(t1, pack_rgba_u32(SIDE_BLUFOR_RGBA));
        let bytes2 = pack_rings(&xy, &[false, false], &["", "UNKNOWN"]);
        assert_eq!(
            u32::from_le_bytes(bytes2[16..20].try_into().unwrap()),
            pack_rgba_u32(SIDE_BLUFOR_RGBA)
        );
        assert_eq!(
            u32::from_le_bytes(bytes2[20 + 16..20 + 20].try_into().unwrap()),
            pack_rgba_u32(SIDE_BLUFOR_RGBA)
        );
    }

    #[test]
    fn cluster_gate_truth_table() {
        assert!(!cluster_mode(0, -6.0));
        assert!(!cluster_mode(500, -6.0)); // not >
        assert!(!cluster_mode(501, -3.9));
        assert!(cluster_mode(501, -4.0));
        assert!(cluster_mode(10_000, -6.0));
        assert!(!cluster_mode(10_000, -2.0));
    }

    #[test]
    fn cluster_disc_size_formula() {
        assert!((cluster_disc_size_px(1) - 22.0).abs() < 1e-5);
        // log10(1000)=3 → 22+min(26,36)=22+26=48
        assert!((cluster_disc_size_px(1000) - 48.0).abs() < 1e-5);
    }

    #[test]
    fn drag_delta_math() {
        let (x, y) = drag_projected(100.0, 200.0, 3.5, -1.25);
        assert!((x - 103.5).abs() < 1e-12);
        assert!((y - 198.75).abs() < 1e-12);
    }

    #[test]
    fn px_to_m_at_default_zoom() {
        // zoom -2 → 2^2 = 4 m/px
        assert!((px_to_m_at_zoom(-2.0) - 4.0).abs() < 1e-6);
        assert!((px_to_m_at_zoom(0.0) - 1.0).abs() < 1e-6);
        assert!((px_to_m_at_zoom(3.0) - 0.125).abs() < 1e-6);
    }

    #[test]
    fn pack_cluster_instances_count() {
        let xs = [10.0, 20.0];
        let ys = [30.0, 40.0];
        let counts = [5u32, 100];
        let b = pack_cluster_instances(&xs, &ys, &counts);
        assert_eq!(b.len(), 2 * SLOT_ICON_STRIDE);
        assert_eq!(
            u16::from_le_bytes(b[14..16].try_into().unwrap()),
            SLOT_GLYPH_DISC
        );
    }

    #[test]
    fn classify_drag_transition_truth_table() {
        assert_eq!(
            classify_drag_transition(false, true, true, false),
            DragGpuPhase::Start
        );
        assert_eq!(
            classify_drag_transition(true, true, false, true),
            DragGpuPhase::Delta
        );
        assert_eq!(
            classify_drag_transition(true, false, true, true),
            DragGpuPhase::End
        );
        assert_eq!(
            classify_drag_transition(true, true, true, false),
            DragGpuPhase::Restart
        );
        assert_eq!(
            classify_drag_transition(true, true, false, false),
            DragGpuPhase::Idle
        );
        assert_eq!(
            classify_drag_transition(false, false, false, true),
            DragGpuPhase::Idle
        );
    }

    #[test]
    fn pack_selection_only_dense_k() {
        let xy = [0.0_f32, 0.0, 100.0, 200.0, 300.0, 400.0];
        let sel = [false, true, true];
        let bytes = pack_selection_only(&xy, &sel);
        assert_eq!(bytes.len(), 2 * SLOT_ICON_STRIDE);
        let size0 = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!((size0 - SLOT_SELECTED_PX).abs() < 1e-6);
        let x0 = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert!((x0 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn selected_mask_from_set() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let mut set = HashSet::new();
        set.insert("b".into());
        assert_eq!(selected_mask(&ids, &set), vec![false, true, false]);
    }

    #[test]
    fn pack_drag_overlay_rows() {
        let ids = vec!["a".into(), "b".into()];
        let xy = [1.0_f32, 2.0, 3.0, 4.0];
        let drag = vec!["b".into()];
        let (bytes, rows) = pack_drag_overlay(&drag, &ids, &xy);
        assert_eq!(rows, vec![1]);
        assert_eq!(bytes.len(), SLOT_ICON_STRIDE);
        let x = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert!((x - 3.0).abs() < 1e-6);
    }

    /// T-573 — a **mixed** drag (slots + vehicles) must offset exactly the dragged *vehicle* rows.
    ///
    /// This is the reported bug, stated as behaviour: the preview showed the slots moving and left
    /// the vehicles standing, so the overlay promised a move the drop would not perform — a tool
    /// reporting success over an input it never examined, at 60 fps. `move_entities_and_vehicles`
    /// (T-491, pinned by T-574) already commits both, so the preview was the only liar.
    ///
    /// The row set and the id order are chosen so the three ways a re-pack gets this wrong all
    /// fail: `drag_ids[0]` is a **slot**, so a "first dragged id only" collapse moves nothing; the
    /// dragged vehicles are rows 1 and 2, so an "offset every row" bug moves `v-parked`; and the
    /// two slot ids resolve to no row, so an implementation that defaults an unknown id to row 0
    /// moves `v-parked` too. `dx`/`dy` and every coordinate are exactly representable in f32 and
    /// pairwise distinct, so an x/y swap cannot hide behind a matching value.
    #[test]
    fn vehicle_drag_preview_offsets_only_the_dragged_vehicles_of_a_mixed_selection() {
        let points = vec![
            ("v-parked".to_string(), 10.0, 20.0),
            ("v-dragged-a".to_string(), 30.0, 40.0),
            ("v-dragged-b".to_string(), 50.0, 60.0),
        ];
        // Mixed AND out of row order, slot id first — the shape the host actually hands over.
        let drag: Vec<String> = ["slot-7", "v-dragged-b", "slot-9", "v-dragged-a"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();

        let lane = pack_vehicle_drag_preview(&drag, &points, 7.5, -3.25);

        assert_eq!(
            lane.len(),
            points.len() * 2,
            "every placed vehicle must stay in the dense lane — a dropped row VANISHES mid-drag"
        );
        assert_eq!(
            lane,
            vec![10.0_f32, 20.0, 37.5, 36.75, 57.5, 56.75],
            "v-parked stands still; both dragged vehicles move by exactly (+7.5, -3.25)"
        );
    }

    /// T-573 — a slots-only drag (and the empty-drag restore) must leave the vehicle lane on the
    /// authored positions. This is the "skip unknown ids" half stated on its own: every id in
    /// `drag_ids` names no vehicle, so nothing may move. Empty `drag_ids` is the same call the
    /// cancel/no-move paths make to put the lane back.
    #[test]
    fn vehicle_drag_preview_leaves_the_lane_authored_when_no_vehicle_is_dragged() {
        let points = vec![
            ("v0".to_string(), 10.0, 20.0),
            ("v1".to_string(), 30.0, 40.0),
        ];
        let slots_only: Vec<String> = vec!["slot-1".to_string(), "slot-2".to_string()];
        assert_eq!(
            pack_vehicle_drag_preview(&slots_only, &points, 7.5, -3.25),
            vec![10.0_f32, 20.0, 30.0, 40.0],
            "a slots-only drag must not smear the vehicle lane"
        );
        assert_eq!(
            pack_vehicle_drag_preview(&[], &points, 7.5, -3.25),
            vec![10.0_f32, 20.0, 30.0, 40.0],
            "empty drag = the identity re-pack the cancel path restores with"
        );
        assert!(
            pack_vehicle_drag_preview(&slots_only, &[], 1.0, 1.0).is_empty(),
            "no placed vehicles ⇒ empty lane (vehicles_bind drops the lane)"
        );
    }

    #[test]
    fn slot_atlas_shape_and_uv() {
        let a = build_slot_atlas();
        assert_eq!(a.rgba.len(), 128 * 64 * 4);
        assert_eq!((a.width, a.height), (128, 64));
        assert_eq!(a.uv, [0.0, 0.0, 0.5, 1.0, 0.5, 0.0, 1.0, 1.0]);
    }

    // ── T-808 unit symbology ─────────────────────────────────────────────────────────────────

    /// Every kit alias seeded in `apps/mod/tbd-framework/Data/registry.json` must map, and an
    /// unknown one must default — the ticket's acceptance stated as data.
    ///
    /// The list is the complete `"alias": "kit:*"` set of the shipped registry (12 rows). A kit that
    /// silently fell to the default would still LOOK fine on the map (rifleman is a plausible glyph
    /// for anything), which is exactly why the medic / leader / MG rows are asserted by CLASS rather
    /// than merely "does not panic".
    #[test]
    fn every_seeded_kit_maps_and_unknown_defaults() {
        let seeded: [(&str, UnitRoleClass); 12] = [
            ("kit:us_rifleman", UnitRoleClass::Rifleman),
            ("kit:us_sl", UnitRoleClass::Leader),
            ("kit:us_tl", UnitRoleClass::Leader),
            ("kit:us_ar", UnitRoleClass::MachineGun),
            ("kit:us_medic", UnitRoleClass::Medic),
            ("kit:fia_rifleman", UnitRoleClass::Rifleman),
            ("kit:fia_sl", UnitRoleClass::Leader),
            ("kit:fia_medic", UnitRoleClass::Medic),
            ("kit:sov_rifleman", UnitRoleClass::Rifleman),
            ("kit:sov_sl", UnitRoleClass::Leader),
            ("kit:sov_ar", UnitRoleClass::MachineGun),
            ("kit:civ_generic", UnitRoleClass::Rifleman),
        ];
        for (alias, want) in seeded {
            assert_eq!(unit_role_class(alias), want, "seeded kit {alias}");
            // The bare form (no `kit:` prefix) is what a flattened `slot.role` can carry.
            let bare = alias.strip_prefix("kit:").unwrap();
            assert_eq!(unit_role_class(bare), want, "bare {bare}");
        }
        // Unknown / empty / whitespace → the documented default.
        for unknown in [
            "",
            "   ",
            "kit:xx_unheard_of",
            "Sapper",
            "not-a-role",
            "kit:",
        ] {
            assert_eq!(
                unit_role_class(unknown),
                UnitRoleClass::Rifleman,
                "unknown {unknown:?} must default"
            );
        }
    }

    /// Authored ORBAT role strings — the OTHER vocabulary the one table must accept — plus the
    /// priority rule and the "`at` is a token, never a substring" guard.
    #[test]
    fn authored_role_strings_and_token_boundaries() {
        assert_eq!(unit_role_class("Squad Leader"), UnitRoleClass::Leader);
        assert_eq!(unit_role_class("Team Leader"), UnitRoleClass::Leader);
        assert_eq!(unit_role_class("Medic"), UnitRoleClass::Medic);
        assert_eq!(unit_role_class("AT Gunner"), UnitRoleClass::AntiTank);
        assert_eq!(
            unit_role_class("anti-tank rifleman"),
            UnitRoleClass::AntiTank
        );
        assert_eq!(unit_role_class("Machine Gunner"), UnitRoleClass::MachineGun);
        assert_eq!(
            unit_role_class("Automatic Rifleman"),
            UnitRoleClass::MachineGun
        );
        assert_eq!(unit_role_class("Rifleman"), UnitRoleClass::Rifleman);
        // Priority: leader beats medic beats AT beats MG, as documented.
        assert_eq!(unit_role_class("Medic Team Leader"), UnitRoleClass::Leader);
        assert_eq!(unit_role_class("AT Medic"), UnitRoleClass::Medic);
        assert_eq!(unit_role_class("AT Gunner MG"), UnitRoleClass::AntiTank);
        // `at` must not fire from inside another word — these three would all be AT if the table
        // matched substrings, and all three are common role text.
        assert_eq!(unit_role_class("Attack Rifleman"), UnitRoleClass::Rifleman);
        assert_eq!(unit_role_class("Station Guard"), UnitRoleClass::Rifleman);
        assert_eq!(unit_role_class("Combat Engineer"), UnitRoleClass::Rifleman);
        // ...and `ar` must not fire from inside "Marksman" / "Sharpshooter".
        assert_eq!(unit_role_class("Marksman"), UnitRoleClass::Rifleman);
        assert_eq!(unit_role_class("Sharpshooter"), UnitRoleClass::Rifleman);
    }

    /// The seeded vehicle roster (M1025 / M998 / M923A1 / M113) resolves to the three silhouette
    /// classes the operator named, from BOTH the registry-alias and the prefab-path form.
    #[test]
    fn seeded_vehicles_map_to_silhouette_kinds() {
        for a in [
            "veh:us_m1025",
            "M1025 Humvee",
            "{4A71F755A4513227}Prefabs/Vehicles/Wheeled/M998/M1025.et",
            "M998 Humvee",
        ] {
            assert_eq!(vehicle_kind_for_alias(a), VehicleKind::WheeledLight, "{a}");
        }
        for a in [
            "veh:us_m923a1",
            "M923A1",
            "Prefabs/Vehicles/Wheeled/M923A1/M923A1.et",
            "Ural cargo truck",
        ] {
            assert_eq!(vehicle_kind_for_alias(a), VehicleKind::Truck, "{a}");
        }
        for a in ["veh:us_m113", "M113A3", "BTR-70", "BMP-1", "tracked ifv"] {
            assert_eq!(vehicle_kind_for_alias(a), VehicleKind::Apc, "{a}");
        }
        // Unknown → the documented default.
        assert_eq!(vehicle_kind_for_alias(""), VehicleKind::WheeledLight);
        assert_eq!(
            vehicle_kind_for_alias("veh:unheard_of"),
            VehicleKind::WheeledLight
        );
    }

    /// T-832 — the lane must CARRY YAW. Two headings 90° apart may not produce the same bytes, and
    /// the sign must follow the document's compass convention (`headingDeg` clockwise from north)
    /// rather than the shader's CCW one.
    ///
    /// This is the defect stated as bytes: `pack_icon_instance` writes a literal `0_i16` at offset
    /// 12, so before this ticket heading 90 and heading 180 packed IDENTICALLY — which is exactly
    /// what T-832's pixel-identical crops measured.
    #[test]
    fn slot_lane_carries_yaw_and_the_sign_is_the_compass_convention() {
        let yaw_of = |bytes: &[u8]| i16::from_le_bytes(bytes[12..14].try_into().unwrap());
        let xy = [100.0_f32, 200.0];
        let roles = vec!["kit:us_rifleman".to_string()];
        let pack = |h: f32| pack_slot_symbology(&xy, &[false], &[], &roles, &[h], 1.0, 11);

        let h90 = pack(90.0);
        let h180 = pack(180.0);
        assert_ne!(
            h90, h180,
            "heading 90 and 180 must not pack identically — that IS the T-832 defect"
        );
        // Compass 90° (east) → screen CCW −90° → snorm −16384 (round of −0.5×32767 = −16383.5).
        assert_eq!(yaw_of(&h90), yaw_to_snorm16(-90.0));
        assert!(yaw_of(&h90) < 0, "east must rotate the point clockwise");
        assert_eq!(yaw_of(&h180), yaw_to_snorm16(-180.0));
        assert_eq!(yaw_of(&pack(0.0)), 0, "north is the un-rotated cell");
        // A missing heading column degrades to 0 rather than panicking.
        assert_eq!(
            yaw_of(&pack_slot_symbology(
                &xy,
                &[false],
                &[],
                &roles,
                &[],
                1.0,
                11
            )),
            0
        );
        // And the legacy packer is still the yaw-0 lane the old pins describe.
        assert_eq!(yaw_of(&pack_slot_instances(&xy, &[false], &[])), 0);
    }

    /// `screen_yaw_for_heading_deg` is the ONE place the sign flip lives; pin its truth table.
    #[test]
    fn screen_yaw_sign_and_degenerates() {
        assert!((screen_yaw_for_heading_deg(90.0) - -90.0).abs() < 1e-12);
        assert!((screen_yaw_for_heading_deg(270.0) - -270.0).abs() < 1e-12);
        assert!((screen_yaw_for_heading_deg(0.0)).abs() < 1e-12);
        assert!(
            !screen_yaw_for_heading_deg(0.0).is_sign_negative(),
            "0.0 must not become -0.0"
        );
        assert!((screen_yaw_for_heading_deg(f64::NAN)).abs() < 1e-12);
        assert!((screen_yaw_for_heading_deg(f64::INFINITY)).abs() < 1e-12);
        // snorm clamps rather than wrapping, so a >180° screen angle saturates.
        assert_eq!(yaw_to_snorm16(-270.0), -32767);
        assert_eq!(yaw_to_snorm16(180.0), 32767);
    }

    /// The yaw encoder copied into this file must agree with `world::glyph_math`'s, forever.
    /// Feature-gated because `world` is optional; `--all-features` (the gate) runs it.
    #[cfg(feature = "world")]
    #[test]
    fn yaw_encoders_agree() {
        for i in -400..=400 {
            let deg = f64::from(i) * 0.9375; // sweeps well past ±180 to cover the clamp
            assert_eq!(
                yaw_to_snorm16(deg),
                crate::world::yaw_to_snorm16(deg),
                "yaw encoders diverged at {deg}"
            );
        }
        for deg in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0] {
            assert_eq!(yaw_to_snorm16(deg), crate::world::yaw_to_snorm16(deg));
        }
        // The screen-angle convention is the same one the world lanes use.
        for deg in [0.0, 45.0, 90.0, 180.0, 359.5] {
            assert!(
                (screen_yaw_for_heading_deg(deg) - crate::world::deck_angle_for_rotation_deg(deg))
                    .abs()
                    < 1e-12,
                "screen-angle convention diverged at {deg}"
            );
        }
    }

    /// The widening must COPY the caller's atlas byte-for-byte — the T-790 pin (`cells 0/1 are
    /// byte-identical to `build_slot_atlas`) survives transitively, and the symbology block's base
    /// is the caller's cell count so T-790's atlas can grow without touching this file.
    #[test]
    fn extend_atlas_copies_the_base_verbatim_and_bases_after_it() {
        let base = build_slot_atlas();
        let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height)
            .expect("well-formed");
        let (rgba, w, h, uv, base_cells) = (&a.rgba, a.width, a.height, &a.uv, a.base_cells);
        assert_eq!(base_cells, 2, "build_slot_atlas is a 2-cell strip");
        let n = 2 + SYMBOLOGY_CELL_COUNT;
        #[allow(clippy::cast_possible_truncation)]
        {
            assert_eq!((w, h), ((n * 64) as u32, 64));
        }
        assert_eq!(rgba.len(), n * 64 * 64 * 4);
        assert_eq!(uv.len(), n * 4);
        // Row-by-row memcmp of the base region — not a spot probe.
        for y in 0..64usize {
            let src = y * (base.width as usize) * 4;
            let dst = y * (w as usize) * 4;
            let len = base.width as usize * 4;
            assert_eq!(
                &rgba[dst..dst + len],
                &base.rgba[src..src + len],
                "base atlas row {y} was not copied verbatim"
            );
        }
        // UV cell 0 still starts at u=0 and every cell is 1/n wide.
        #[allow(clippy::cast_precision_loss)]
        {
            assert!((uv[0]).abs() < 1e-9);
            assert!((uv[2] - 1.0 / n as f32).abs() < 1e-6);
        }
        // A 3-cell base moves the symbology block by exactly one cell — the runtime base at work.
        let wide = vec![0u8; 3 * 64 * 64 * 4];
        let b3 = extend_atlas_with_unit_glyphs(&wide, 3 * 64, 64).expect("3-cell");
        assert_eq!(b3.base_cells, 3);
    }

    /// Malformed strips are REFUSED, not mangled — the engine then uploads the caller's atlas
    /// untouched and leaves symbology off rather than pointing glyph ids at garbage.
    #[test]
    fn extend_atlas_refuses_a_strip_it_cannot_read() {
        assert!(extend_atlas_with_unit_glyphs(&[], 0, 64).is_none(), "empty");
        assert!(
            extend_atlas_with_unit_glyphs(&vec![0u8; 128 * 32 * 4], 128, 32).is_none(),
            "wrong cell height"
        );
        assert!(
            extend_atlas_with_unit_glyphs(&vec![0u8; 100 * 64 * 4], 100, 64).is_none(),
            "width not a whole number of cells"
        );
        assert!(
            extend_atlas_with_unit_glyphs(&[0u8; 10], 128, 64).is_none(),
            "rgba length disagrees with w·h·4"
        );
    }

    /// Render every symbology cell and prove the SHAPES are pairwise distinct — the acceptance's
    /// "medic vs rifleman vs leader pixel signatures pairwise distinct" and "marker vs unit vs
    /// comment shapes pairwise distinct", measured on the atlas the screenshots sample.
    ///
    /// Two independent signatures, because either alone is cheatable: total ink (a shape could match
    /// another's area by coincidence) AND the full alpha bitmap (which cannot).
    #[test]
    fn symbology_cells_are_pairwise_distinct_shapes() {
        // Base = T-790's 11-cell marker atlas, rebuilt here from its published contract: cells 0/1
        // are the slot ring/disc, so the marker-vs-unit half of the comparison uses REAL marker
        // pixels for those two and this crate cannot see the other nine (they live in
        // map-engine-render). `marker_delta` in the ticket report covers the remaining nine.
        let base = build_slot_atlas();
        let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
        let (rgba, base_cells) = (&a.rgba, a.base_cells);
        let w = a.width as usize;
        let cell_alpha = |c: usize| -> Vec<u8> {
            let mut v = Vec::with_capacity(64 * 64);
            for y in 0..64 {
                for x in 0..64 {
                    v.push(rgba[(y * w + c * 64 + x) * 4 + 3]);
                }
            }
            v
        };
        let named: Vec<(&str, usize)> = vec![
            ("marker-ring", 0),
            ("marker-disc", 1),
            (
                "unit-rifleman",
                base_cells as usize + UNIT_CELL_BASE as usize,
            ),
            (
                "unit-leader",
                base_cells as usize + UNIT_CELL_BASE as usize + 1,
            ),
            (
                "unit-medic",
                base_cells as usize + UNIT_CELL_BASE as usize + 2,
            ),
            ("unit-at", base_cells as usize + UNIT_CELL_BASE as usize + 3),
            ("unit-mg", base_cells as usize + UNIT_CELL_BASE as usize + 4),
            (
                "unit-rifleman-sel",
                base_cells as usize + UNIT_SELECTED_CELL_BASE as usize,
            ),
            (
                "veh-wheeled",
                base_cells as usize + VEHICLE_CELL_BASE as usize,
            ),
            (
                "veh-truck",
                base_cells as usize + VEHICLE_CELL_BASE as usize + 1,
            ),
            (
                "veh-apc",
                base_cells as usize + VEHICLE_CELL_BASE as usize + 2,
            ),
            ("comment", base_cells as usize + COMMENT_CELL as usize),
            (
                "comment-sel",
                base_cells as usize + COMMENT_SELECTED_CELL as usize,
            ),
        ];
        let shots: Vec<(&str, Vec<u8>, u32)> = named
            .iter()
            .map(|(n, c)| {
                let a = cell_alpha(*c);
                let ink: u32 = a.iter().map(|&p| u32::from(p)).sum();
                (*n, a, ink)
            })
            .collect();
        for (n, _, ink) in &shots {
            assert!(*ink > 0, "cell {n} is BLANK — a glyph that draws nothing");
        }
        for i in 0..shots.len() {
            for j in (i + 1)..shots.len() {
                assert_ne!(
                    shots[i].1, shots[j].1,
                    "{} and {} have identical alpha bitmaps",
                    shots[i].0, shots[j].0
                );
                assert_ne!(
                    shots[i].2, shots[j].2,
                    "{} and {} have identical total ink",
                    shots[i].0, shots[j].0
                );
            }
        }
    }

    /// The three role classes the acceptance names by name (medic / rifleman / leader) must differ
    /// in the CELL INTERIOR, not merely at the edges — the knockout is the whole mechanism, and an
    /// interior-blind check would pass on five identical discs with different rims.
    #[test]
    fn role_knockouts_differ_inside_the_disc() {
        let base = build_slot_atlas();
        let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
        let (rgba, bc) = (&a.rgba, a.base_cells);
        let w = a.width as usize;
        let interior = |c: usize| -> Vec<u8> {
            // r ≤ 14 around the cell centre: strictly inside the r20 body, and clear of the facing
            // point (which never reaches below dy = −10).
            let mut v = Vec::new();
            for y in 18..46 {
                for x in 18..46 {
                    let dx = x as f64 + 0.5 - 32.0;
                    let dy = y as f64 + 0.5 - 32.0;
                    if dx.hypot(dy) <= 14.0 {
                        v.push(rgba[(y * w + c * 64 + x) * 4 + 3]);
                    }
                }
            }
            v
        };
        let cell = |cls: UnitRoleClass| bc as usize + UNIT_CELL_BASE as usize + cls as usize;
        let rifleman = interior(cell(UnitRoleClass::Rifleman));
        let leader = interior(cell(UnitRoleClass::Leader));
        let medic = interior(cell(UnitRoleClass::Medic));
        let at = interior(cell(UnitRoleClass::AntiTank));
        let mg = interior(cell(UnitRoleClass::MachineGun));
        assert!(
            rifleman.iter().all(|&p| p == 255),
            "the default role must be a SOLID disc (no knockout)"
        );
        for (n, s) in [
            ("leader", &leader),
            ("medic", &medic),
            ("at", &at),
            ("mg", &mg),
        ] {
            assert!(s.contains(&0), "{n} must knock a real hole in the body");
        }
        let all = [
            ("rifleman", &rifleman),
            ("leader", &leader),
            ("medic", &medic),
            ("at", &at),
            ("mg", &mg),
        ];
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_ne!(
                    all[i].1, all[j].1,
                    "{} and {} share an interior",
                    all[i].0, all[j].0
                );
            }
        }
    }

    /// The facing point must actually be at the cell's NORTH edge, and the cell must be
    /// ASYMMETRIC north/south — a symmetric glyph is why T-832's heading was invisible even
    /// once yaw rode the lane.
    #[test]
    fn unit_cell_is_north_asymmetric_so_yaw_is_visible() {
        let base = build_slot_atlas();
        let a = extend_atlas_with_unit_glyphs(&base.rgba, base.width, base.height).expect("atlas");
        let (rgba, bc) = (&a.rgba, a.base_cells);
        let w = a.width as usize;
        let c = bc as usize + UNIT_CELL_BASE as usize; // rifleman: no knockout to confound this
        let a = |x: usize, y: usize| rgba[(y * w + c * 64 + x) * 4 + 3];
        // Column through the centre: ink well north of the r20 body, none the same distance south.
        assert!(a(32, 6) > 0, "the facing point must reach the north edge");
        assert_eq!(a(32, 57), 0, "nothing may hang off the south edge");
        // 180° apart on the same cell must differ — this is the crop pair T-832 measured.
        let north: Vec<u8> = (0..32)
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .map(|(x, y)| a(x, y))
            .collect();
        let south: Vec<u8> = (32..64)
            .rev()
            .flat_map(|y| (0..64).map(move |x| (x, y)))
            .map(|(x, y)| a(x, y))
            .collect();
        assert_ne!(
            north, south,
            "the cell is mirror-symmetric about its east-west axis, so a 180° yaw would be invisible"
        );
    }

    /// The stated zoom threshold, exercised at both acceptance zooms and across the boundary.
    #[test]
    fn symbology_degrades_to_dots_past_the_stated_m_per_px() {
        assert!(symbology_visible(1.0), "1.0 m/px is an acceptance zoom");
        assert!(symbology_visible(4.0), "4.0 m/px is an acceptance zoom");
        assert!(symbology_visible(SYMBOLOGY_MAX_M_PER_PX));
        assert!(!symbology_visible(SYMBOLOGY_MAX_M_PER_PX + 0.001));
        assert!(!symbology_visible(32.0));
        // Fail safe on nonsense.
        assert!(!symbology_visible(0.0));
        assert!(!symbology_visible(-1.0));
        assert!(!symbology_visible(f32::NAN));

        let xy = [10.0_f32, 20.0];
        let roles = vec!["kit:us_medic".to_string()];
        let glyph_of = |b: &[u8]| u16::from_le_bytes(b[14..16].try_into().unwrap());
        let size_of = |b: &[u8]| f32::from_le_bytes(b[8..12].try_into().unwrap());

        let near = pack_slot_symbology(&xy, &[false], &[], &roles, &[90.0], 4.0, 11);
        assert_eq!(glyph_of(&near), 11 + UnitRoleClass::Medic as u16);
        assert!((size_of(&near) - SLOT_UNIT_PX).abs() < 1e-6);

        let far = pack_slot_symbology(&xy, &[false], &[], &roles, &[90.0], 16.0, 11);
        assert_eq!(glyph_of(&far), SLOT_GLYPH_DISC, "degraded to a plain dot");
        assert!((size_of(&far) - SLOT_RING_PX).abs() < 1e-6);
        assert_eq!(
            i16::from_le_bytes(far[12..14].try_into().unwrap()),
            0,
            "a dot claims no heading"
        );
        // The degraded row keeps its SIDE colour — degradation drops detail, never information the
        // dot can still carry.
        assert_eq!(
            u32::from_le_bytes(far[16..20].try_into().unwrap()),
            pack_rgba_u32(SIDE_BLUFOR_RGBA)
        );
    }

    /// Selection stays a treatment ON TOP: amber, 28 px, and the RINGED variant of the same role
    /// cell — so a selected medic is still visibly a medic and still points where it points.
    #[test]
    fn selection_layers_over_the_role_glyph_and_keeps_heading() {
        let xy = [0.0_f32, 0.0, 50.0, 50.0];
        let roles = vec!["kit:us_medic".to_string(), "kit:us_medic".to_string()];
        let b = pack_slot_symbology(
            &xy,
            &[false, true],
            &[SIDE_OPFOR_RGBA, SIDE_OPFOR_RGBA],
            &roles,
            &[45.0, 45.0],
            1.0,
            11,
        );
        let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
        let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
        let size = |i: usize| f32::from_le_bytes(row(i)[8..12].try_into().unwrap());
        let tint = |i: usize| u32::from_le_bytes(row(i)[16..20].try_into().unwrap());
        let yaw = |i: usize| i16::from_le_bytes(row(i)[12..14].try_into().unwrap());

        assert_eq!(glyph(0), 11 + UNIT_CELL_BASE + UnitRoleClass::Medic as u16);
        assert_eq!(
            glyph(1),
            11 + UNIT_SELECTED_CELL_BASE + UnitRoleClass::Medic as u16,
            "the selected cell must be the SAME role, ringed — not a generic ring"
        );
        assert!((size(0) - SLOT_UNIT_PX).abs() < 1e-6);
        assert!((size(1) - SLOT_SELECTED_PX).abs() < 1e-6);
        assert_eq!(tint(0), pack_rgba_u32(SIDE_OPFOR_RGBA), "side colour kept");
        assert_eq!(tint(1), pack_rgba_u32(SLOT_SELECTED_RGBA));
        assert_eq!(yaw(0), yaw(1), "selection must not drop the heading");
        assert_eq!(yaw(1), yaw_to_snorm16(-45.0));
    }

    /// T-796 — a comment is a neutral bubble, never the selection amber; selection layers on top.
    #[test]
    fn comments_are_neutral_bubbles_with_selection_on_top() {
        let xy = [1.0_f32, 2.0, 3.0, 4.0];
        let b = pack_comment_instances(&xy, &[false, true], 1.0, 11);
        let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
        let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
        let tint = |i: usize| u32::from_le_bytes(row(i)[16..20].try_into().unwrap());

        assert_eq!(glyph(0), 11 + COMMENT_CELL);
        assert_eq!(tint(0), pack_rgba_u32(COMMENT_NOTE_RGBA));
        assert_ne!(
            tint(0),
            pack_rgba_u32(SLOT_SELECTED_RGBA),
            "an idle comment must NOT wear the selection colour (the T-796 defect)"
        );
        assert_ne!(
            glyph(0),
            SLOT_GLYPH_RING,
            "an idle comment must NOT wear the slot ring glyph (the T-796 defect)"
        );
        assert_eq!(glyph(1), 11 + COMMENT_SELECTED_CELL);
        assert_eq!(tint(1), pack_rgba_u32(SLOT_SELECTED_RGBA));
        // No selection feed at all (the shipped feeder today) still fixes the colour + shape.
        let none = pack_comment_instances(&xy, &[], 1.0, 11);
        assert_eq!(
            u32::from_le_bytes(none[16..20].try_into().unwrap()),
            pack_rgba_u32(COMMENT_NOTE_RGBA)
        );
        assert_eq!(none.len(), 2 * SLOT_ICON_STRIDE);
    }

    /// Vehicle silhouettes carry kind + heading + side colour, and degrade with everything else.
    #[test]
    fn vehicle_symbology_carries_kind_heading_and_side() {
        let xy = [0.0_f32, 0.0, 10.0, 10.0, 20.0, 20.0];
        let aliases = vec![
            "M1025".to_string(),
            "M923A1".to_string(),
            "M113A3".to_string(),
        ];
        let tints = [SIDE_BLUFOR_RGBA, SIDE_OPFOR_RGBA, SIDE_INDFOR_RGBA];
        let b = pack_vehicle_symbology(&xy, &aliases, &tints, &[0.0, 90.0, 270.0], 1.0, 11);
        assert_eq!(b.len(), 3 * SLOT_ICON_STRIDE);
        let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
        let glyph = |i: usize| u16::from_le_bytes(row(i)[14..16].try_into().unwrap());
        assert_eq!(
            glyph(0),
            11 + VEHICLE_CELL_BASE + VehicleKind::WheeledLight as u16
        );
        assert_eq!(glyph(1), 11 + VEHICLE_CELL_BASE + VehicleKind::Truck as u16);
        assert_eq!(glyph(2), 11 + VEHICLE_CELL_BASE + VehicleKind::Apc as u16);
        assert_ne!(glyph(0), glyph(1));
        assert_ne!(glyph(1), glyph(2));
        assert_eq!(
            i16::from_le_bytes(row(1)[12..14].try_into().unwrap()),
            yaw_to_snorm16(-90.0)
        );
        assert_eq!(
            u32::from_le_bytes(row(2)[16..20].try_into().unwrap()),
            pack_rgba_u32(SIDE_INDFOR_RGBA)
        );
        // Degraded + empty input.
        let far = pack_vehicle_symbology(&xy, &aliases, &tints, &[0.0, 90.0, 270.0], 99.0, 11);
        assert_eq!(
            u16::from_le_bytes(far[14..16].try_into().unwrap()),
            SLOT_GLYPH_DISC
        );
        assert!(pack_vehicle_symbology(&[], &[], &[], &[], 1.0, 11).is_empty());
    }

    /// Short / missing parallel columns degrade per-field rather than dropping rows or panicking —
    /// the partially-wired feeder the completion pass will build up incrementally.
    #[test]
    fn symbology_tolerates_short_parallel_columns() {
        let xy = [0.0_f32, 0.0, 1.0, 1.0, 2.0, 2.0];
        let b = pack_slot_symbology(&xy, &[true], &[SIDE_OPFOR_RGBA], &[], &[10.0], 1.0, 11);
        assert_eq!(b.len(), 3 * SLOT_ICON_STRIDE, "every row still packs");
        let row = |i: usize| &b[i * SLOT_ICON_STRIDE..(i + 1) * SLOT_ICON_STRIDE];
        // Row 0: selected + OPFOR-but-overridden-amber + rifleman (no roles) + heading 10.
        assert_eq!(
            u16::from_le_bytes(row(0)[14..16].try_into().unwrap()),
            11 + UNIT_SELECTED_CELL_BASE + UnitRoleClass::Rifleman as u16
        );
        // Rows 1/2: unselected, BLUFOR pad, rifleman, yaw 0.
        assert_eq!(
            u32::from_le_bytes(row(2)[16..20].try_into().unwrap()),
            pack_rgba_u32(SIDE_BLUFOR_RGBA)
        );
        assert_eq!(i16::from_le_bytes(row(2)[12..14].try_into().unwrap()), 0);
        assert!(pack_slot_symbology(&[], &[], &[], &[], &[], 1.0, 11).is_empty());
    }

    /// Every symbology glyph id must stay inside the shader's `min(glyph, 31u)` clamp
    /// (`scene::ATLAS_GLYPH_COUNT` = 32) on top of T-790's 11-cell marker base. A cell past 31 does
    /// not error — it silently samples cell 31, which is the worst kind of wrong.
    #[test]
    fn symbology_ids_fit_the_shader_glyph_clamp() {
        const ATLAS_GLYPH_COUNT: u16 = 32; // scene::ATLAS_GLYPH_COUNT (map-engine-render)
        const MARKER_GLYPH_COUNT: u16 = 11; // scene::MARKER_GLYPH_COUNT (T-790)
        #[allow(clippy::cast_possible_truncation)]
        let last = MARKER_GLYPH_COUNT + SYMBOLOGY_CELL_COUNT as u16 - 1;
        assert_eq!(last, 25);
        assert!(
            last < ATLAS_GLYPH_COUNT,
            "symbology overflows the 32-cell UV table"
        );
        // The block offsets are contiguous and non-overlapping.
        #[allow(clippy::cast_possible_truncation)]
        {
            assert_eq!(
                UNIT_SELECTED_CELL_BASE,
                UNIT_CELL_BASE + UNIT_ROLE_CLASS_COUNT as u16
            );
            assert_eq!(
                VEHICLE_CELL_BASE,
                UNIT_SELECTED_CELL_BASE + UNIT_ROLE_CLASS_COUNT as u16
            );
            assert_eq!(COMMENT_CELL, VEHICLE_CELL_BASE + VEHICLE_KIND_COUNT as u16);
            assert_eq!(COMMENT_SELECTED_CELL, COMMENT_CELL + 1);
            assert_eq!(SYMBOLOGY_CELL_COUNT as u16, COMMENT_SELECTED_CELL + 1);
        }
    }

    #[test]
    fn slot_atlas_ring_and_disc_probes() {
        let a = build_slot_atlas();
        let alpha = |x: usize, y: usize| a.rgba[(y * 128 + x) * 4 + 3];
        // Ring cell (center 32,32): hollow center, opaque band at r≈17, transparent outside r≈24.
        assert_eq!(alpha(32, 32), 0, "ring center must be hollow");
        assert_eq!(alpha(32 + 17, 32), 255, "ring band must be opaque");
        assert_eq!(alpha(32 + 30, 32), 0, "outside ring must be transparent");
        // Disc cell (center 96,32): opaque center and mid, transparent outside r≈26.
        assert_eq!(alpha(96, 32), 255, "disc center must be opaque");
        assert_eq!(alpha(96 + 20, 32), 255, "disc mid must be opaque");
        assert_eq!(alpha(96 + 30, 32), 0, "outside disc must be transparent");
        // White-on-alpha everywhere (tint multiplies).
        assert_eq!(&a.rgba[0..3], &[255, 255, 255]);
    }
}
