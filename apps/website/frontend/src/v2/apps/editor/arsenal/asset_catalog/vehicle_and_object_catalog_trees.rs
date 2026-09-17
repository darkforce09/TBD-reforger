//! Asset catalog vehicle and object catalog trees behavior.

use super::*;

const VEHICLE_OPEN_DEPTH: usize = 2;

/// Builds the vehicle tree and excludes abstract templates.
#[must_use]
pub fn build_vehicle_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items
        .iter()
        .filter(|i| i.kind == "vehicle" && i.r#abstract != Some(true))
    {
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();

        let mut cur = &mut roots;
        let mut prefix = String::new();
        for (depth, seg) in segs.iter().enumerate() {
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
                        default_expanded: depth < VEHICLE_OPEN_DEPTH,
                        children: Vec::new(),
                        payload: None,
                    });
                    cur.len() - 1
                }
            };
            cur = &mut cur[idx].children;
        }

        cur.push(CatalogNode {
            id: item.resource_name.clone(),
            label: item.display_name.clone(),
            default_expanded: false,
            children: Vec::new(),
            payload: Some(PlacePayload {
                asset_id: item.resource_name.clone(),
                role: item.display_name.clone(),
            }),
        });
    }

    roots
}

/// Reports whether a registry kind belongs to the object palette.
pub(super) fn is_object_kind(kind: &str) -> bool {
    matches!(kind, "crate" | "other")
}

const OBJECT_OPEN_DEPTH: usize = 2;

/// Builds placeable object leaves gated by the mod registry.
#[must_use]
pub fn build_object_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items.iter().filter(|i| {
        is_object_kind(&i.kind)
            && i.r#abstract != Some(true)
            && object_alias_registered(&i.resource_name, &i.display_name)
    }) {
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();

        let mut cur = &mut roots;
        let mut prefix = String::new();
        for (depth, seg) in segs.iter().enumerate() {
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
                        default_expanded: depth < OBJECT_OPEN_DEPTH,
                        children: Vec::new(),
                        payload: None,
                    });
                    cur.len() - 1
                }
            };
            cur = &mut cur[idx].children;
        }

        cur.push(CatalogNode {
            id: item.resource_name.clone(),
            label: item.display_name.clone(),
            default_expanded: false,
            children: Vec::new(),
            payload: Some(PlacePayload {
                asset_id: item.resource_name.clone(),
                role: item.display_name.clone(),
            }),
        });
    }

    roots
}
