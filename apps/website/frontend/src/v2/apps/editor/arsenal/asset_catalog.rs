//! Asset catalog tree models and palette classification behavior.

#![allow(dead_code)]

use std::collections::HashSet;
use std::sync::OnceLock;
pub use website_map_engine::data::store::operations::assets::classname_tail;
pub use website_map_engine::data::store::operations::assets::derive_object_alias;

pub use website_map_engine::data::store::operations::assets::PlacePayload;

use crate::v2::core::api::dto::RegistryItem;

const MOD_SPAWN_REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../apps/mod/tbd-framework/Data/registry.json"
));

#[must_use]
fn mod_object_aliases() -> &'static HashSet<String> {
    static ALIASES: OnceLock<HashSet<String>> = OnceLock::new();
    ALIASES.get_or_init(|| {
        let v: serde_json::Value =
            serde_json::from_str(MOD_SPAWN_REGISTRY_JSON).expect("mod registry.json parses");
        let mut set = HashSet::new();
        if let Some(entries) = v.get("entries").and_then(|e| e.as_array()) {
            for e in entries {
                if let Some(alias) = e.get("alias").and_then(|a| a.as_str()) {
                    if alias.starts_with("prop:") || alias.starts_with("comp:") {
                        set.insert(alias.to_string());
                    }
                }
            }
        }
        set
    })
}

/// Reports whether an object alias exists in the mod spawn registry.
#[must_use]
pub fn object_alias_registered(resource_name: &str, display_name: &str) -> bool {
    mod_object_aliases().contains(&derive_object_alias(resource_name, display_name))
}

const EDEN_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

#[must_use]
fn path_has_side_segment(path: &str, side: &str) -> bool {
    path.split('/').any(|seg| seg == side)
}

#[must_use]
fn legacy_category_root_side(category: &str) -> Option<&'static str> {
    match category.split('/').next().unwrap_or("") {
        "NATO" => Some("BLUFOR"),
        "USSR" => Some("OPFOR"),
        "FIA" => Some("INDFOR"),
        _ => None,
    }
}

/// Checks whether a character row belongs to the selected side.
#[must_use]
pub fn character_matches_eden_side(item: &RegistryItem, side: &str) -> bool {
    if !EDEN_SIDES.contains(&side) {
        return false;
    }
    if path_has_side_segment(&item.category, side) {
        return true;
    }
    if path_has_side_segment(&item.resource_name, side) {
        return true;
    }
    legacy_category_root_side(&item.category) == Some(side)
}

/// One folder or placeable leaf in a catalog tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogNode {
    pub id: String,
    pub label: String,
    pub default_expanded: bool,
    pub children: Vec<CatalogNode>,
    pub payload: Option<PlacePayload>,
}

/// Loading state for the live asset catalog.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CatalogState {
    #[default]
    Loading,
    Failed,
    Ready(Vec<CatalogNode>),
}

/// Builds a side-filtered character tree from registry rows.
#[must_use]
pub fn build_catalog_tree(items: &[RegistryItem], side: &str) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items
        .iter()
        .filter(|i| i.kind == "character" && character_matches_eden_side(i, side))
    {
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();
        let folder_segs = &segs[..segs.len().saturating_sub(1)];

        let mut cur = &mut roots;
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
                        default_expanded: depth == 0, // top-level faction folders open (rule 3)
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

/// Checks whether any placeable row belongs to the selected side.
#[must_use]
pub fn asset_matches_eden_side(item: &RegistryItem, side: &str) -> bool {
    if !EDEN_SIDES.contains(&side) {
        return false;
    }
    path_has_side_segment(&item.category, side)
        || path_has_side_segment(&item.resource_name, side)
        || legacy_category_root_side(&item.category) == Some(side)
}

mod bounded_regex;
mod catalog_search_filter;
mod catalog_search_query;
mod faction_catalog_trees;
mod vehicle_and_object_catalog_trees;

pub use bounded_regex::{GlobPattern, Rx};
pub use catalog_search_filter::filter_catalog;
pub use catalog_search_query::{
    parse_search_query, search_empty_message, SearchField, SearchPattern, SearchQuery,
};
pub use faction_catalog_trees::{
    build_faction_catalog_tree, build_picker_catalog_tree, catalog_leaf_count,
};
use vehicle_and_object_catalog_trees::is_object_kind;
pub use vehicle_and_object_catalog_trees::{build_object_catalog_tree, build_vehicle_catalog_tree};

/// The palette that can place a catalog row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogPalette {
    Character,
    Vehicle,
    Object,
}

/// Finds a live registry row by its canonical resource name.
#[must_use]
pub fn find_catalog_item<'a>(
    items: &'a [RegistryItem],
    asset_id: &str,
) -> Option<&'a RegistryItem> {
    items.iter().find(|i| i.resource_name == asset_id)
}

/// Resolves the palette that can place a live registry row.
#[must_use]
pub fn placeable_palette(item: &RegistryItem) -> Option<CatalogPalette> {
    if item.kind == "character" {
        return Some(CatalogPalette::Character);
    }
    if item.kind == "vehicle" {
        return (item.r#abstract != Some(true)).then_some(CatalogPalette::Vehicle);
    }
    if is_object_kind(&item.kind)
        && item.r#abstract != Some(true)
        && object_alias_registered(&item.resource_name, &item.display_name)
    {
        return Some(CatalogPalette::Object);
    }
    None
}

#[cfg(test)]
use bounded_regex::RxParser;
#[cfg(test)]
use bounded_regex::RX_MAX_PATTERN;
#[cfg(test)]
use catalog_search_query::parse_search_pattern;

#[cfg(test)]
#[path = "tests/asset_catalog/catalog_tree_and_basic_filter.rs"]
mod catalog_tree_and_basic_filter_tests;
#[cfg(test)]
#[path = "tests/asset_catalog/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "tests/asset_catalog/object_catalog_and_favorites.rs"]
mod object_catalog_and_favorites_tests;
#[cfg(test)]
#[path = "tests/asset_catalog/search_operators_and_safety.rs"]
mod search_operators_and_safety_tests;
#[cfg(test)]
#[path = "tests/asset_catalog/side_vehicle_and_merged_catalog.rs"]
mod side_vehicle_and_merged_catalog_tests;
