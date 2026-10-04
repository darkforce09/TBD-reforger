//! Folder slot membership: which slots a folder click selects.
//!
//! Folder selection reads authored entity ids, including slots hidden from materialized map rows.
//! Direct selection uses one folder; descendant selection walks its child folders recursively.

use super::LayerRow;
use mission_document::ids::LayerId;

/// A folder's DIRECT slot children — the ids listed in its own `entityIds`, in doc order.
/// Unknown `id` → empty. Reads the unfiltered doc source (see the module note above).
#[must_use]
pub fn layer_direct_slot_children(layers: &[LayerRow], id: &LayerId) -> Vec<String> {
    layers
        .iter()
        .find(|l| l.id == *id)
        .map(|l| l.entity_ids.clone())
        .unwrap_or_default()
}

/// Every slot in a folder's subtree — its own `entityIds` plus, recursively, those of every
/// descendant folder (`parentId` chain). Order is this folder's slots first, then each child
/// folder's subtree in `layers` order; a cycle-guard (`seen`) mirrors `build_outliner`'s
/// belt-and-braces against a malformed `parentId`. Reads the unfiltered doc source.
#[must_use]
pub fn layer_descendant_slots(layers: &[LayerRow], id: &LayerId) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    fn walk<'a>(
        layers: &'a [LayerRow],
        id: &LayerId,
        seen: &mut std::collections::HashSet<&'a LayerId>,
        out: &mut Vec<String>,
    ) {
        let Some(layer) = layers.iter().find(|l| l.id == *id) else {
            return;
        };
        if !seen.insert(&layer.id) {
            return;
        }
        out.extend(layer.entity_ids.iter().cloned());
        for child in layers
            .iter()
            .filter(|l| l.parent_id.as_ref() == Some(&layer.id))
        {
            walk(layers, &child.id, seen, out);
        }
    }
    walk(layers, id, &mut seen, &mut out);
    out
}
