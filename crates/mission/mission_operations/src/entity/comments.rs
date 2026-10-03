//! Role: comments.
//! Position: the `entity::comments` module of `mission_operations`; hosted commands drive it.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use mission_document::ids::CommentId;

use super::CommentRow;
use super::MissionDocCore;
use super::layer_rows;

/// Domain representation of comment detail.
#[derive(Clone, Debug, PartialEq)]
pub struct CommentDetail {
    /// Id.
    pub id: CommentId,

    /// Title.
    pub title: String,

    /// Tooltip.
    pub tooltip: String,

    /// X.
    pub x: f64,

    /// Z.
    pub z: f64,
}

/// Comment rows using the supplied domain data.
pub fn comment_rows(core: &MissionDocCore) -> Vec<CommentRow> {
    comment_details(core)
        .into_iter()
        .map(|d| CommentRow {
            id: d.id,
            title: d.title,
            tooltip: d.tooltip,
        })
        .collect()
}

/// Comment details using the supplied domain data.
#[must_use]
pub fn comment_details(core: &MissionDocCore) -> Vec<CommentDetail> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.comments_json()) else {
        return Vec::new();
    };
    let Some(obj) = map.as_object() else {
        return Vec::new();
    };
    let mut rows: Vec<CommentDetail> = obj
        .iter()
        .map(|(id, v)| {
            let pos = |k: &str| {
                v.get("position")
                    .and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
            CommentDetail {
                id: id.clone().into(),
                title: s("title"),
                tooltip: s("tooltip"),
                x: pos("x"),
                z: pos("z"),
            }
        })
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    rows
}

/// Mint an unused `cmt-{n}` id, proven unique against the live comments map.
pub fn mint_comment_id(core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.comments_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    let mut n = existing.len() + 1;
    loop {
        let id = format!("cmt-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n += 1;
    }
}

/// Apply place_comment to explicit document state.
pub fn place_comment(
    core: &MissionDocCore,
    x: f64,
    z: f64,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let id = mint_comment_id(core);
    let layer_id = ensure_layer(core);
    core.add_comment(id.as_str(), "Comment", "", x, z);
    core.move_comment_to_layer(id.as_str(), layer_id.as_str());
    Some(id)
}

/// Apply duplicate_comment to explicit document state.
pub fn duplicate_comment(
    core: &MissionDocCore,
    id: impl Into<CommentId>,
    offset: f64,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<String> {
    let id: CommentId = id.into();
    let id = id.as_str();
    let new_id = mint_comment_id(core);
    if !core.duplicate_comment(id, new_id.as_str(), offset, -offset) {
        return None;
    }

    let layer_id = layer_rows(core)
        .into_iter()
        .find(|l| l.entity_ids.iter().any(|e| e == id))
        .map_or_else(|| ensure_layer(core), |l| l.id.to_string());
    core.move_comment_to_layer(new_id.as_str(), layer_id.as_str());
    Some(new_id)
}
