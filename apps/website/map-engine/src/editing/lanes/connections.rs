//! Role: reduce the document's connection rows to the map's edge lane, and answer clicks on it.
//! Position: `editing::lanes` in the map engine.
//! Signals & state: none; every function here is a pure map from document JSON plus endpoint
//! positions to lane vertices or a hit answer.
//! Invariants: the lane is rebuilt from the document on every read and caches nothing, so undo,
//! redo and a restore cannot leave it stale; an edge with an unresolvable endpoint and a self-link
//! are both skipped rather than drawn; the pick is point-to-segment and the nearest edge wins.

/// One connection edge reduced to what the map needs: its id and its two endpoints in world metres.
#[derive(Clone, Debug, PartialEq)]
pub struct ConnSegment {
    /// The connection id — the same id the Connections panel's delete verb takes, so the edge a
    /// pick returns and the edge Delete removes are one row.
    pub id: String,
    /// World easting of the `from` endpoint, in metres.
    pub ax: f64,
    /// World northing of the `from` endpoint, in metres.
    pub ay: f64,
    /// World easting of the `to` endpoint, in metres.
    pub bx: f64,
    /// World northing of the `to` endpoint, in metres.
    pub by: f64,
}

/// The unselected edge hairline: a pale blue at low alpha, so a dense graph reads as structure
/// rather than as a wall. Deliberately dimmer than the ORBAT squad hairlines — those carry
/// structural truth and an editor-only relation must not compete with them.
pub const CONN_LINE_RGBA: [f32; 4] = [173.0 / 255.0, 198.0 / 255.0, 1.0, 0.62];

/// The selected edge: opaque amber. A hue no other lane uses, because the only thing this colour
/// has to communicate is "Delete will remove THIS one".
pub const CONN_LINE_SELECTED_RGBA: [f32; 4] = [1.0, 0.78, 0.30, 1.0];

/// Click tolerance for [`pick_connection`], in SCREEN pixels — converted to world metres by the
/// caller through the frozen press camera, so the tolerance is constant on screen at every zoom.
/// Matches the slot pick's feel: a hairline is one pixel wide, and nobody can click a 1 px target.
pub const CONN_PICK_PX: f64 = 6.0;

/// Build the drawable edges from `rows_json` (`MissionDocCore::connection_rows_json`, the same
/// stable-ordered listing the Connections panel renders) and a map of entity id to world position.
///
/// An edge whose endpoint has no position is **skipped**, not drawn to the origin: a dangling edge
/// is a validation finding and the panel is where a finding is reported, whereas a line to (0, 0)
/// would be a second, wordless report that is also wrong about where the entity is.
///
/// Self-links are skipped too — the add verb refuses them, a hydrated one is flagged, and a
/// zero-length segment is not a clickable artifact in any case.
#[must_use]
pub fn connection_segments(
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

/// Pack the edges into the flat `[x, y, r, g, b, a]` LineList the connections bind takes: six
/// floats per vertex, two vertices per segment, in `segs` order.
///
/// `selected` tints exactly one edge. It is matched by id against the same ids
/// [`pick_connection`] returns and the delete verb consumes, so the highlighted line and the line
/// Delete removes are the same line by construction rather than by convention.
#[must_use]
pub fn connection_lane_verts(segs: &[ConnSegment], selected: Option<&str>) -> Vec<f32> {
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

/// The edge under a world point, or `None`. `tol_m` is the click radius in world metres (the
/// caller converts [`CONN_PICK_PX`] through the press camera). Nearest edge wins, so overlapping
/// edges resolve deterministically instead of by listing order.
///
/// Distance is point-to-SEGMENT, not point-to-line: the infinite-line form would let a click far
/// off the end of a short edge, but on its extension, select it — a hit on nothing.
#[must_use]
pub fn pick_connection(segs: &[ConnSegment], wx: f64, wy: f64, tol_m: f64) -> Option<String> {
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
