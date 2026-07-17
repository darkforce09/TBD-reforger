//! T-159.22 — flat registry rows → the right dock's Factions palette tree.
//!
//! A **verbatim port** of React's `buildCatalogTree` (T-068.3,
//! `features/mission-creator/registry/buildCatalogTree.ts`, deleted at `c4ccb9c3` when T-152 swapped
//! the React palette onto the T-153 Faction Library). That builder — not the Faction Library — is
//! what spec O2 names, and it is the one that matches the committed `GET__registry.json` golden.
//!
//! The oracle's four load-bearing rules, ported exactly:
//!
//! 1. **Only `kind == "character"` rows are placed.** `gear_*` rows feed the Arsenal loadout
//!    dropdowns (T-068.4), not the map palette.
//! 2. **The folders are the category path MINUS its last segment**, because the leaf is the row's
//!    `display_name`. So `"NATO/US_Army/Rifleman"` → `NATO` > `US_Army` > leaf `"US Rifleman"` —
//!    there is deliberately **no** `Rifleman` folder.
//! 3. **A folder's id is its accumulated path prefix** (`"NATO"`, `"NATO/US_Army"`) so ids are
//!    stable, and only depth-0 folders open by default.
//! 4. **A leaf's id is the full Enfusion `resource_name`** "so a drop carries the real classname".
//!
//! Rows are consumed in array order — the API pre-sorts by `sort_order`, so faction/role order stays
//! stable without a sort here (the oracle's comment, and true of the golden).
//!
//! Pure + native-testable on purpose: no `web_sys`, no signals. The view layer is `eden_chrome`.
#![allow(dead_code)]

use crate::dto::RegistryItem;

/// What a palette leaf hands the map when it is dropped: the doc fields a placed slot needs.
/// `asset_id` is the full `resource_name` (T-068.3: "DnD `assetId` = full `resource_name`").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacePayload {
    pub asset_id: String,
    pub role: String,
}

/// One palette node. A **leaf is `payload.is_some()`** (folders never carry one), which also makes
/// "placeable" and "is a leaf" the same predicate — the oracle's `payloadById.get(node.id)` miss is
/// what made a React vehicle leaf non-draggable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogNode {
    pub id: String,
    pub label: String,
    pub default_expanded: bool,
    pub children: Vec<CatalogNode>,
    pub payload: Option<PlacePayload>,
}

/// The right dock's fetch state — the `AssetBrowser.tsx:86-136` loading / error / empty / tree
/// branches, as a signal value the native view shell can hold too (it simply never leaves
/// `Loading`, since `api_get` is wasm-only).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CatalogState {
    #[default]
    Loading,
    Failed,
    Ready(Vec<CatalogNode>),
}

/// Build the palette tree from the flat registry rows. See the module docs for the ported rules.
#[must_use]
pub fn build_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items.iter().filter(|i| i.kind == "character") {
        let segs: Vec<&str> = item.category.split('/').filter(|s| !s.is_empty()).collect();
        // Drop the role segment — `display_name` is the leaf (rule 2). `saturating_sub` keeps a
        // single-segment (or empty) category from panicking; it simply files the leaf at the root.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::RegistryResponse;

    /// The same committed golden the R-api gate pins (`dto::r_api`), so this test and the live
    /// palette read byte-identical data.
    const GOLDEN: &str =
        include_str!("../../../.ai/artifacts/t159_gates/fixtures/api/GET__registry.json");

    fn golden_items() -> Vec<RegistryItem> {
        serde_json::from_str::<RegistryResponse>(GOLDEN)
            .expect("golden deserializes")
            .data
    }

    /// The exact tree the fixture must yield: NATO (expanded) > US_Army > the 8 character leaves in
    /// `sort_order` order. Pins every ported rule at once.
    #[test]
    fn golden_yields_nato_us_army_and_eight_leaves() {
        let tree = build_catalog_tree(&golden_items());

        assert_eq!(tree.len(), 1, "one root faction folder");
        let nato = &tree[0];
        assert_eq!(nato.id, "NATO");
        assert_eq!(nato.label, "NATO");
        assert!(nato.default_expanded, "depth-0 folders open by default");
        assert!(nato.payload.is_none(), "folders are not placeable");

        assert_eq!(nato.children.len(), 1, "one sub-folder, no Rifleman folder");
        let army = &nato.children[0];
        assert_eq!(
            army.id, "NATO/US_Army",
            "folder id is the accumulated prefix"
        );
        assert_eq!(army.label, "US_Army");
        assert!(!army.default_expanded, "only depth 0 opens by default");

        let labels: Vec<&str> = army.children.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "US Rifleman",
                "US Grenadier",
                "US Medic",
                "US Automatic Rifleman",
                "US Machine Gunner",
                "US Platoon Leader",
                "US Light Anti-Tank",
                "US Engineer",
            ],
            "leaves are display_name, in the API's sort_order array order"
        );
    }

    /// Rule 4 + the payload contract: a leaf's id AND its drop `asset_id` are the full Enfusion
    /// ResourceName, and its `role` is the display name.
    #[test]
    fn leaf_id_and_payload_carry_the_resource_name() {
        let tree = build_catalog_tree(&golden_items());
        let rifleman = &tree[0].children[0].children[0];
        let expected =
            "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";
        assert_eq!(rifleman.id, expected);
        assert_eq!(
            rifleman.payload,
            Some(PlacePayload {
                asset_id: expected.to_string(),
                role: "US Rifleman".to_string(),
            })
        );
        assert!(rifleman.children.is_empty());
    }

    /// Rule 1: the golden's 13 `gear_*` rows must not reach the map palette. Proven by count, so the
    /// test fails if the filter is dropped (21 rows would yield extra folders/leaves).
    #[test]
    fn gear_rows_are_excluded() {
        let items = golden_items();
        assert_eq!(items.len(), 21, "golden row count");
        let characters = items.iter().filter(|i| i.kind == "character").count();
        assert_eq!(characters, 8);

        let tree = build_catalog_tree(&items);
        let leaves = tree[0].children[0].children.len();
        assert_eq!(leaves, 8, "only character rows are placed");
        // The gear categories (NATO/Uniform, NATO/Vest, …) would have added sibling folders.
        assert_eq!(tree[0].children.len(), 1, "no gear folders under NATO");
    }
}
