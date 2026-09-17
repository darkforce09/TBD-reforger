//! Flattened outliner rows and visibility traversal.

use super::*;

/* ───────────────────────────── flattened rows for windowing ───────────────────────────── */

/// One flattened tree row (pre-order): the node's identity + its nesting depth. The windowed
/// renderer slices a `Vec<FlatRow>` and draws only the visible span (React `flattenOutliner`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatRow {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub depth: usize,
    /// True when the source node has children — drives the chevron + open/closed folder icon
    /// ( B6/B7). A collapsed container still renders its own row; its subtree does not.
    pub has_children: bool,
    /// YouTube-style guide continuation, one bool per guide column (`len == depth`):
    /// `ancestors[k]` = "the vertical line at column k continues below this row." For `k < depth-1`
    /// it's an ancestor spine (drawn iff that ancestor has a following sibling); `ancestors[depth-1]`
    /// is this row's own connector, whose bit = `!is_last` (draw the elbow's tail down to the next
    /// sibling, or trim it at the last child). Self-contained per row so the windowed slice needs no
    /// sibling lookup. Roots (depth 0) get `[]` → no guides. See `eden_chrome::guide_spans`.
    pub ancestors: Vec<bool>,
    /// owner id per guide column (`len == depth`); `guide_ids[k]` toggles on guide click.
    pub guide_ids: Vec<String>,
    /// copied from [`OutlinerNode::is_leader`] for the windowed SL badge.
    pub is_leader: bool,
    /// this Folder layer's OWN `hidden` flag (drives the eye-toggle glyph state).
    pub hidden: bool,
    /// this Folder layer's OWN `locked` flag (drives the lock-toggle glyph state).
    pub locked: bool,
    /// RESOLVED hidden (own or inherited): the windowed row renders dimmed when set.
    pub hidden_effective: bool,
    /// RESOLVED lock (own or inherited): the windowed row shows the inherited-lock adornment.
    pub locked_effective: bool,
    /// copied from [`OutlinerNode::tooltip`] for the windowed comment row's hover text.
    pub tooltip: String,
}

/// Flatten a tree to pre-order rows (parent before its children). Every node becomes exactly one
/// row — the window operates on this flat list, not the nested `OutlinerNode`s.
#[must_use]
pub fn flatten(nodes: &[OutlinerNode]) -> Vec<FlatRow> {
    flatten_visible(nodes, &std::collections::HashSet::new())
}

/// Flatten honoring a collapsed-id set ( B6): a collapsed node emits its own row but none
/// of its descendants. An empty set = the old fully-expanded `flatten`.
#[must_use]
pub fn flatten_visible(
    nodes: &[OutlinerNode],
    collapsed: &std::collections::HashSet<String>,
) -> Vec<FlatRow> {
    let mut out = Vec::new();
    // `prefix` = the parent row's `ancestors` vector (length == parent depth). A child's vector is
    // the parent's + its own `!is_last` bit ( A1); roots (depth 0) draw no guide column, so
    // their own bit is dropped (`ancestors == []`) and it never propagates as a spine — a depth-1
    // row's single column is its OWN elbow, not a root spine.
    // `id_prefix` = ancestor node ids for guide click ( A4); `len == depth`.
    fn walk(
        nodes: &[OutlinerNode],
        depth: usize,
        prefix: &[bool],
        id_prefix: &[String],
        collapsed: &std::collections::HashSet<String>,
        out: &mut Vec<FlatRow>,
    ) {
        let len = nodes.len();
        for (i, n) in nodes.iter().enumerate() {
            let is_last = i + 1 == len;
            let ancestors: Vec<bool> = if depth == 0 {
                Vec::new()
            } else {
                let mut v = Vec::with_capacity(depth);
                v.extend_from_slice(prefix);
                v.push(!is_last);
                v
            };
            let guide_ids = id_prefix.to_vec();
            out.push(FlatRow {
                id: n.id.clone(),
                label: n.label.clone(),
                kind: n.kind,
                depth,
                has_children: !n.children.is_empty(),
                ancestors: ancestors.clone(),
                guide_ids: guide_ids.clone(),
                is_leader: n.is_leader,
                hidden: n.hidden,
                locked: n.locked,
                hidden_effective: n.hidden_effective,
                locked_effective: n.locked_effective,
                tooltip: n.tooltip.clone(),
            });
            if !collapsed.contains(&n.id) {
                let mut child_ids = guide_ids;
                child_ids.push(n.id.clone());
                walk(
                    &n.children,
                    depth + 1,
                    &ancestors,
                    &child_ids,
                    collapsed,
                    out,
                );
            }
        }
    }
    walk(nodes, 0, &[], &[], collapsed, &mut out);
    out
}

/// The folder a new entity is filed under when the tree has one focused, and the gestures that
/// move that focus. Wasm-only because they reach the live mission document; the tree vocabulary
/// above them is pure and native-testable, which is why the two halves share this module rather
/// than a wrapper somewhere else.
#[cfg(target_arch = "wasm32")]
mod active_folder {
    use super::{DEFAULT_LAYER_ID, DEFAULT_LAYER_NAME};
    use crate::v2::apps::editor::bridge::host_state::editor_context::{
        EditorContext, EDITOR_CONTEXT,
    };
    use leptos::prelude::{GetUntracked, Set};
    use website_map_engine::data::store::MissionDocCore;
    use website_map_engine::editing::hosted_commands as engine_ops;

    /// Focus a folder, or clear the focus. A focused folder is the drop target for the next place.
    pub fn set_active_layer(id: Option<String>) {
        EDITOR_CONTEXT.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                ctx.active_layer.set(id);
            }
        });
    }

    /// Resolve the folder a new entity is filed under against one context: the focused folder when
    /// one is set and still live, otherwise the default folder, minted under the LOCAL origin so
    /// the mint is part of the same undoable act as the place it serves. A focus pointing at a
    /// folder the document no longer holds is cleared as it is resolved.
    fn ensure_layer(ctx: &EditorContext, core: &MissionDocCore) -> String {
        let ensured = website_map_engine::data::store::operations::entity::ensure_layer(
            core,
            ctx.active_layer.get_untracked(),
            DEFAULT_LAYER_ID,
            DEFAULT_LAYER_NAME,
        );
        if ensured.active_layer_was_stale {
            ctx.active_layer.set(None);
        }
        ensured.layer_id
    }

    /// Resolve the folder a new entity is filed under without a context in hand. This is the form
    /// the engine's hosted commands take, which is why the folder id crosses the wall as an answer
    /// rather than the engine reaching for the tree's focus itself.
    pub fn ensure_active_layer(core: &MissionDocCore) -> String {
        EDITOR_CONTEXT
            .with(|c| c.borrow().as_ref().map(|ctx| ensure_layer(ctx, core)))
            .unwrap_or_else(|| DEFAULT_LAYER_ID.to_string())
    }

    /// Create a folder as a child of the focused folder (a root when none is focused), auto-named,
    /// with its inline rename armed, and focus it. Returns the new folder's id.
    pub fn create_layer() -> Option<String> {
        let active = EDITOR_CONTEXT.with(|c| {
            c.borrow()
                .as_ref()
                .and_then(|ctx| ctx.active_layer.get_untracked())
        });
        let created = engine_ops::create_layer(active)?;
        set_active_layer(Some(created.clone()));
        Some(created)
    }

    /// Delete a folder and its whole subtree, dropping the focus when it was the focused folder —
    /// a focus on a folder that no longer exists would file the next place into nothing.
    pub fn delete_layer(id: &str) -> bool {
        let did = engine_ops::delete_layer(id);
        if did {
            let focused = EDITOR_CONTEXT.with(|c| {
                c.borrow()
                    .as_ref()
                    .and_then(|ctx| ctx.active_layer.get_untracked())
            });
            if focused.as_deref() == Some(id) {
                set_active_layer(None);
            }
        }
        did
    }
}

#[cfg(target_arch = "wasm32")]
pub use active_folder::{create_layer, delete_layer, ensure_active_layer, set_active_layer};
