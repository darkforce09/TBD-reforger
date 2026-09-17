//! Asset catalog faction catalog trees behavior.

use super::*;

const FACTION_OPEN_DEPTH: usize = 1;

/// Builds one side-filtered tree combining placeable kinds.
#[must_use]
pub fn build_faction_catalog_tree(items: &[RegistryItem], side: &str) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    fn file_leaf(
        roots: &mut Vec<CatalogNode>,
        folder_segs: &[&str],
        leaf_id: &str,
        leaf_label: &str,
    ) {
        let mut cur = roots;
        let mut prefix = String::new();
        for (depth, seg) in folder_segs.iter().enumerate() {
            if prefix.is_empty() {
                prefix.push_str(seg);
            } else {
                prefix.push('/');
                prefix.push_str(seg);
            }
            let idx = match cur.iter().position(|n| n.id == prefix) {
                Some(i) => i,
                None => {
                    cur.push(CatalogNode {
                        id: prefix.clone(),
                        label: (*seg).to_string(),
                        default_expanded: depth < FACTION_OPEN_DEPTH,
                        children: Vec::new(),
                        payload: None,
                    });
                    cur.len() - 1
                }
            };
            cur = &mut cur[idx].children;
        }
        cur.push(CatalogNode {
            id: leaf_id.to_string(),
            label: leaf_label.to_string(),
            default_expanded: false,
            children: Vec::new(),
            payload: Some(PlacePayload {
                asset_id: leaf_id.to_string(),
                role: leaf_label.to_string(),
            }),
        });
    }

    for item in items {
        let matches = if item.kind == "character" {
            character_matches_eden_side(item, side)
        } else {
            asset_matches_eden_side(item, side)
        };
        if !matches {
            continue;
        }
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();
        if item.kind == "character" {
            let folder_segs = &segs[..segs.len().saturating_sub(1)];
            file_leaf(
                &mut roots,
                folder_segs,
                &item.resource_name,
                &item.display_name,
            );
        } else if item.kind == "vehicle" {
            if item.r#abstract == Some(true) {
                continue; // *_base.et templates the engine cannot spawn (build_vehicle_catalog_tree rule)
            }
            file_leaf(&mut roots, &segs, &item.resource_name, &item.display_name);
        } else if is_object_kind(&item.kind)
            && item.r#abstract != Some(true)
            && object_alias_registered(&item.resource_name, &item.display_name)
        {
            file_leaf(&mut roots, &segs, &item.resource_name, &item.display_name);
        }
    }

    roots
}

/// Builds a catalog picker tree spanning every side.
#[must_use]
pub fn build_picker_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    fn file_leaf(
        roots: &mut Vec<CatalogNode>,
        folder_segs: &[&str],
        leaf_id: &str,
        leaf_label: &str,
    ) {
        let mut cur = roots;
        let mut prefix = String::new();
        for (depth, seg) in folder_segs.iter().enumerate() {
            if prefix.is_empty() {
                prefix.push_str(seg);
            } else {
                prefix.push('/');
                prefix.push_str(seg);
            }
            let idx = match cur.iter().position(|n| n.id == prefix) {
                Some(i) => i,
                None => {
                    cur.push(CatalogNode {
                        id: prefix.clone(),
                        label: (*seg).to_string(),
                        default_expanded: depth < FACTION_OPEN_DEPTH,
                        children: Vec::new(),
                        payload: None,
                    });
                    cur.len() - 1
                }
            };
            cur = &mut cur[idx].children;
        }
        cur.push(CatalogNode {
            id: leaf_id.to_string(),
            label: leaf_label.to_string(),
            default_expanded: false,
            children: Vec::new(),
            payload: Some(PlacePayload {
                asset_id: leaf_id.to_string(),
                role: leaf_label.to_string(),
            }),
        });
    }

    for item in items {
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();
        if item.kind == "character" {
            let folder_segs = &segs[..segs.len().saturating_sub(1)];
            file_leaf(
                &mut roots,
                folder_segs,
                &item.resource_name,
                &item.display_name,
            );
        } else if item.kind == "vehicle" {
            if item.r#abstract == Some(true) {
                continue;
            }
            file_leaf(&mut roots, &segs, &item.resource_name, &item.display_name);
        } else if is_object_kind(&item.kind)
            && item.r#abstract != Some(true)
            && object_alias_registered(&item.resource_name, &item.display_name)
        {
            file_leaf(&mut roots, &segs, &item.resource_name, &item.display_name);
        }
    }

    roots
}

/// Counts placeable leaves beneath catalog folders.
#[must_use]
pub fn catalog_leaf_count(nodes: &[CatalogNode]) -> usize {
    nodes
        .iter()
        .map(|n| usize::from(n.payload.is_some()) + catalog_leaf_count(&n.children))
        .sum()
}
