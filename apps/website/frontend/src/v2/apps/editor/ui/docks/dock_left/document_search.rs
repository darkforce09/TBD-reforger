//! Document search for the left editor dock.

use super::*;

///
/// `eden_settings::owner_is_routable`. [`crate::v2::apps::editor::ui::inspector::validation_panel::subject_id_routes`] is the
/// REGISTERED route probe — an `Rc` of the same resolution `route_select_by_subject_id` runs,
/// narrowed by `mission_editor::route_availability` — so the affordance and the click cannot answer
/// differently. Asking it per ROW rather than per KIND is the whole of the fix.
///
/// This replaced `DocKind::is_selectable`, a hardcoded `Slot | Vehicle` list. That list was true of
/// `Entity` (placed object) arm, so zone and object hits were painted INERT over a click that would
/// have worked, while the row's title asserted a router limit that no longer existed. **Do not
/// re-derive this from a kind list, and do not fall back to `mission_editor::route_target` when no
/// probe is registered** — no probe means no router to click into, and `false` is the honest answer.
#[must_use]
pub fn hit_is_routable(hit: &DocHit) -> bool {
    crate::v2::apps::editor::ui::inspector::validation_panel::subject_id_routes(&hit.entity.id)
}

/// (and its `aria-description`) so the answer is available exactly where the click would have been.
///
/// stopped being true when the router grew its zone and entity arms — an inert row was explaining
/// itself with a false statement about the code. It now says what [`hit_is_routable`] actually
/// found: nothing to route to for this row, right now.
#[must_use]
pub fn unselectable_reason(kind: DocKind) -> String {
    format!(
        "Found, but not selectable from here: the editor's click-to-select router resolves no \
         selection for this {} right now, so a click would do nothing. Open it from its own panel.",
        kind.noun()
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// One ranked document search result and its matching entity.
pub struct DocHit {
    pub entity: DocEntity,
    /// The `text` field name that matched, or `"faction"` when only the faction/folder matched
    pub field: &'static str,
}

/// the results header); this only bounds the DOM. A 240 px column cannot show 2,000 rows usefully and
/// mounting them would cost more than the search does.
pub const MAX_DOC_HITS: usize = 200;

/// glyph is 1 em square.
pub(super) const HIT_ICON_PX: f64 = 14.0;
/// Horizontal gap between a hit icon and label.
pub(super) const HIT_GAP_PX: f64 = 4.0;
/// Horizontal padding reserved by a search result row.
pub(super) const HIT_ROW_PAD_PX: f64 = 8.0;
/// is 15 px; overlay scrollbars take 0. Budget for the classic one — the pin must not pass only on
/// the machine whose scrollbars happen to be free.
pub(super) const LIST_SCROLLBAR_PX: f64 = 15.0;
/// this the row degrades into an ellipsis with a badge beside it, which is furniture: it would name
/// nothing the author could recognise, and a search result that cannot be read is not a result.
pub(super) const HIT_MIN_LABEL_PX: f64 = 80.0;

///
/// `text` is the string being searched, `class_name` the row's Enfusion class (what `class:`
/// matches), `group` its faction (what `mod:` matches, and what a plain query matches as a
/// containing folder). See the section header for why this is a `filter_catalog` call over a
/// two-node projection rather than a matcher of its own.
#[must_use]
pub fn query_hits(query: &str, text: &str, class_name: &str, group: &str) -> bool {
    let leaf = crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode {
        id: class_name.to_string(),
        label: text.to_string(),
        default_expanded: false,
        children: Vec::new(),
        payload: Some(
            crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload {
                asset_id: class_name.to_string(),
                role: String::new(),
            },
        ),
    };
    let root = crate::v2::apps::editor::arsenal::asset_catalog::CatalogNode {
        id: String::new(),
        label: group.to_string(),
        default_expanded: false,
        children: vec![leaf],
        payload: None,
    };
    !crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(
        std::slice::from_ref(&root),
        query,
    )
    .is_empty()
}

/// attribute), carrying the FIRST text attribute that matched, in `rows` order — or `"faction"`
///
/// A blank query returns NO hits rather than every row: an untouched filter box is not a request to
/// list the mission, and answering it with 137 rows would bury the tree under the box that opened
/// them. Half-typed (`class:`) and unreadable (`/[/`) queries return none too — `filter_catalog`
/// already draws that line, and [`crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message`] is what says which of
/// the three empty answers this is.
#[must_use]
pub fn search_document(rows: &[DocEntity], query: &str) -> Vec<DocHit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    rows.iter()
        .filter_map(|e| {
            if let Some((field, _)) = e
                .text
                .iter()
                .find(|(_, v)| query_hits(query, v, &e.class_name, ""))
            {
                return Some(DocHit {
                    entity: e.clone(),
                    field,
                });
            }
            if query_hits(query, "", &e.class_name, &e.faction) {
                return Some(DocHit {
                    entity: e.clone(),
                    field: "faction",
                });
            }
            None
        })
        .collect()
}
#[derive(Clone, Debug, PartialEq, Eq)]
/// A grouped selection attribute and its available values.
pub struct SelectionFacet {
    /// `"Type"` or `"Faction"` — which axis of the ticket's "by type or faction" this is.
    pub axis: &'static str,
    /// The chip's label (`"vehicle"`, `"BLUFOR"`).
    pub label: String,
    /// The ids the selection becomes. Always a PROPER, non-empty subset of the input (see
    /// [`selection_facets`]).
    pub ids: Vec<String>,
}

/// then by faction, types in [`DocKind`] order and factions alphabetical.
///
/// A facet is emitted **only when it is a proper subset**. A chip that would keep everything selected
/// looks like it acts and does not. So a homogeneous selection (six BLUFOR slots) yields NO chips and
/// the dock says so, rather than offering "slot (6)" and "BLUFOR (6)" as no-ops.
///
/// Rows with an empty `faction` are grouped under one explicit "no faction" chip rather than being
/// dropped: "the ones that belong to nobody" is a real thing to narrow to, and silently omitting them
/// would make the chip counts fail to sum to the selection.
#[must_use]
pub fn selection_facets(rows: &[DocEntity]) -> Vec<SelectionFacet> {
    let total = rows.len();
    if total < 2 {
        return Vec::new();
    }
    let mut out: Vec<SelectionFacet> = Vec::new();
    let mut kinds: Vec<DocKind> = rows.iter().map(|e| e.kind).collect();
    kinds.sort_unstable();
    kinds.dedup();
    for k in kinds {
        let ids: Vec<String> = rows
            .iter()
            .filter(|e| e.kind == k)
            .map(|e| e.id.clone())
            .collect();
        if ids.len() < total {
            out.push(SelectionFacet {
                axis: "Type",
                label: k.noun().to_string(),
                ids,
            });
        }
    }
    let mut factions: Vec<&str> = rows.iter().map(|e| e.faction.as_str()).collect();
    factions.sort_unstable();
    factions.dedup();
    for f in factions {
        let ids: Vec<String> = rows
            .iter()
            .filter(|e| e.faction == f)
            .map(|e| e.id.clone())
            .collect();
        if ids.len() < total {
            out.push(SelectionFacet {
                axis: "Faction",
                label: if f.is_empty() {
                    "no faction".to_string()
                } else {
                    f.to_string()
                },
                ids,
            });
        }
    }
    out
}

/// A no-op (and `false`) off wasm and before the editor mounts, like every other `editor_ops` reach
/// in this file.
#[allow(unused_variables)]
pub fn apply_selection(ids: Vec<String>) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        entity_selection::set_selection_ids(ids) > 0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

#[must_use]
/// Read searchable entities from the active document.
pub fn document_rows() -> Vec<DocEntity> {
    #[cfg(target_arch = "wasm32")]
    {
        website_map_engine::editing::hosted_commands::document_entities()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}

/// same rows the search is. Empty off wasm / before the editor mounts.
#[must_use]
pub fn selection_rows() -> Vec<DocEntity> {
    #[cfg(target_arch = "wasm32")]
    {
        engine_ops::selection_entities()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Vec::new()
    }
}
