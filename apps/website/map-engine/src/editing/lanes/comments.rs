//! Role: read the document's comment map once, and serve the glyph lane, its id column and its
//! pick from that single read.
//! Position: `editing::lanes` in the map engine.
//! Signals & state: none; every function is a pure map from the document's `commentsById` JSON to
//! lane arrays or a hit answer.
//! Invariants: the universe is `commentsById` in full, sorted by id, so the lane's instance order
//! cannot depend on the JSON map's own iteration order across undo, redo or a restore; a comment's
//! position is two horizontals `{x, z}` and carries no height; what the lane draws, what the id
//! column names and what a pick can return are one list.

/// Click tolerance for [`pick_comment`], in SCREEN pixels — the same radius the slot and vehicle
/// picks use (`MissionDocCore::PICK_RADIUS_PX`). A comment is drawn with the slot atlas's ring
/// glyph, so a tolerance of this lane's own invention would give the note a hit box a different
/// size from its picture. Restated here rather than referenced so the lane's tolerance reads
/// beside the pick that spends it; a source pin holds the restatement equal to its subject.
pub const COMMENT_PICK_PX: f64 = 4.0;

/// One comment glyph in WORLD metres: the unit of BOTH the render lane and the pick.
#[derive(Clone, Debug, PartialEq)]
pub struct CommentPoint {
    /// The `commentsById` key — what a pick puts into the selection.
    pub id: String,
    /// World easting (`position.x`), in metres.
    pub x: f64,
    /// World northing — the row's `position.z`, in metres. A comment's position is `{x, z}`: the
    /// marker vocabulary of TWO HORIZONTALS and no height. It is named `y` here because that is
    /// the axis it IS on the map plane; treating it as an elevation would file the note's northing
    /// as its altitude and draw it at the origin.
    pub y: f64,
}

/// **The document read the comment lane and the comment pick share** — `commentsById` parsed once
/// and sorted by id, so the lane's instance order is a property of the ids rather than of the JSON
/// map's iteration order.
#[must_use]
pub fn comment_points(comments_json: &str) -> Vec<CommentPoint> {
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

/// Flat interleaved `[x, z, …]` for the comments bind, **packed from [`comment_points`]**. The
/// lane is a projection of the pick's own list, so a comment cannot be drawn where it cannot be
/// clicked, nor clicked where nothing is drawn.
#[must_use]
pub fn comment_lane_xy(comments_json: &str) -> Vec<f32> {
    let pts = comment_points(comments_json);
    let mut xy = Vec::with_capacity(pts.len() * 2);
    #[allow(clippy::cast_possible_truncation)]
    for p in pts {
        xy.push(p.x as f32);
        xy.push(p.y as f32);
    }
    xy
}

/// **The lane's id column.** The sibling of [`comment_lane_xy`] and packed from the very same
/// [`comment_points`] list: row *i* of this array names the bubble drawn at rows `2i`/`2i+1` of
/// that one, so the pairing is a property of the shared read rather than of two feeders staying in
/// step. [`comment_drag_lane_xy`] maps that list one-for-one as well (it only OFFSETS the dragged
/// rows), so these ids stay aligned with the drag preview's lane too — which is what keeps a
/// note's selection ring on the note while it moves.
///
/// Without this column the renderer cannot answer "is bubble *i* selected?": the comments bind
/// marks every row unselected when its id cache is empty, so a selected note draws the neutral
/// bubble and the selection treatment is invisible.
#[must_use]
pub fn comment_lane_ids(comments_json: &str) -> Vec<String> {
    comment_points(comments_json)
        .into_iter()
        .map(|p| p.id)
        .collect()
}

/// The comment glyph under a world point, or `None`. `tol_m` is the click radius in world metres
/// (the caller converts [`COMMENT_PICK_PX`] through the frozen press camera, exactly as the
/// connection pick converts its own tolerance). NEAREST wins, so two notes within one click of
/// each other resolve deterministically instead of by listing order.
#[must_use]
pub fn pick_comment(points: &[CommentPoint], wx: f64, wy: f64, tol_m: f64) -> Option<String> {
    let mut best: Option<(f64, &str)> = None;
    for p in points {
        let d = (wx - p.x).hypot(wy - p.y);
        if d <= tol_m && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, p.id.as_str()));
        }
    }
    best.map(|(_, id)| id.to_string())
}

/// Which of the dragged ids are COMMENTS, paired with their AUTHORED position — membership asked
/// of the document's own comment map ([`comment_points`]) rather than of an id prefix, which is a
/// minting convention a hydrated mission need not follow. Order follows `points`, already
/// id-sorted, so a mixed drag's comment commit is deterministic.
///
/// Shared by the drag PREVIEW ([`comment_drag_lane_xy`], offset by the live delta) and the drag
/// COMMIT (base + delta into the move verb) so the glyph that follows the cursor and the position
/// finally stored are computed from ONE list.
#[must_use]
pub fn dragged_comment_points(points: &[CommentPoint], drag_ids: &[String]) -> Vec<CommentPoint> {
    points
        .iter()
        .filter(|p| drag_ids.iter().any(|d| d == &p.id))
        .cloned()
        .collect()
}

/// The comment lane re-packed for a live drag: EVERY comment the document holds (so the notes not
/// being dragged stay drawn where they are), with the dragged ones translated by the world delta
/// `(dx, dy)`. Feeds the comments bind mid-drag exactly as [`comment_lane_xy`] feeds it at rest —
/// a comment has its own lane, so its preview is this lane re-bound, and dropping the drag re-binds
/// [`comment_lane_xy`] (the authored positions) the same way a committed move does.
#[must_use]
pub fn comment_drag_lane_xy(
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
