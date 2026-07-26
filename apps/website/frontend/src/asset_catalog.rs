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

/// How deep the vehicle tree opens on first paint. The character tree's rule 3 opens depth 0
/// because depth 0 there is the FACTION — the axis an author picks first. The vehicle catalog is
/// addon-rooted (`ArmaReforger/Vehicles/Wheeled/UAZ469`), so depth 0 is the addon and depth 1 is the
/// literal word "Vehicles"; opening only depth 0 would show one folder containing one folder. Two
/// levels lands the author on the axis that actually discriminates — Wheeled / Tracked / Helicopters.
const VEHICLE_OPEN_DEPTH: usize = 2;

/// T-215 — the **Vehicles** palette tree, off the same flat `/registry` fetch the Factions tab uses.
///
/// Two deliberate differences from [`build_catalog_tree`], both because a vehicle is not a role:
///
/// 1. **The folders are the WHOLE category path**, not the path minus its last segment. Rule 2 drops
///    the last segment for characters because there it names the role and the leaf already is the
///    role (`NATO/US_Army/Rifleman` → leaf "US Rifleman"). For a vehicle the last segment names the
///    **family** (`.../Wheeled/UAZ469`) while the leaf is a specific variant ("UAZ469 PKM"), so
///    dropping it would flatten every variant of every family into one folder and lose the only
///    grouping the author has.
/// 2. **`abstract` rows are excluded.** They are `*_base.et` templates that exist to be inherited
///    from, not spawned — 40 of the 218 live vehicle rows. `faction_manager::kind_options` already
///    filters them out of its vehicle picker for the same reason; placing one would author a
///    resource the game cannot instantiate, and nothing downstream would report it.
///
/// `variant_of` is deliberately **not** filtered (unlike `kind_options`): zero live vehicle rows
/// carry it today, and if any ever do, a factory variant of a vehicle is a thing an author wants to
/// place, where a factory variant of a *weapon* is an Arsenal-picker duplicate.
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
            // `role` carries the display label so the leaf is self-describing in a log or a test;
            // the vehicle place path reads `asset_id` only (`editor_ops::place_at`).
            payload: Some(PlacePayload {
                asset_id: item.resource_name.clone(),
                role: item.display_name.clone(),
            }),
        });
    }

    roots
}

/// Registry kinds that place into schema `entities[]` (not characters, not T-215 vehicles).
fn is_object_kind(kind: &str) -> bool {
    matches!(kind, "crate" | "other")
}

/// How deep the Objects tree opens on first paint — same rationale as [`VEHICLE_OPEN_DEPTH`]:
/// addon-rooted categories need two levels before the author reaches a discriminating folder.
const OBJECT_OPEN_DEPTH: usize = 2;

/// T-254 — Objects palette: non-character, non-vehicle registry rows that belong on
/// `entities[]` (`crate` / placeable `other`). Whole category path kept as folders (like
/// vehicles). `abstract` rows excluded.
#[must_use]
pub fn build_object_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items
        .iter()
        .filter(|i| is_object_kind(&i.kind) && i.r#abstract != Some(true))
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

/// Derive a schema `#/$defs/alias` for a placed object from its ResourceName + display name.
///
/// Prefer a known mod-registry reverse hit (`comp:checkpoint_small`); otherwise synthesise
/// `prop:<slug>` / `comp:<slug>` from the display name (Composition path → `comp:`).
#[must_use]
pub fn derive_object_alias(resource_name: &str, display_name: &str) -> String {
    const KNOWN: &[(&str, &str)] = &[(
        "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et",
        "comp:checkpoint_small",
    )];
    for (guid, alias) in KNOWN {
        if resource_name == *guid {
            return (*alias).to_string();
        }
    }
    let prefix = if resource_name.contains("Composition") || resource_name.contains("Compositions")
    {
        "comp"
    } else {
        "prop"
    };
    let slug = object_alias_slug(display_name);
    format!("{prefix}:{slug}")
}

fn object_alias_slug(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_repl = false;
    for c in raw.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            prev_repl = false;
        } else if !prev_repl {
            out.push('_');
            prev_repl = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "object".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Asset-search filter (T-172 B9 — the T-055 React behavior): case-insensitive label substring.
/// A folder survives on a self-match (keeping its whole subtree) or on any descendant match
/// (keeping only the matching children). Empty/whitespace query returns the tree unchanged.
#[must_use]
pub fn filter_catalog(nodes: &[CatalogNode], query: &str) -> Vec<CatalogNode> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return nodes.to_vec();
    }
    fn keep(node: &CatalogNode, q: &str) -> Option<CatalogNode> {
        if node.label.to_lowercase().contains(q) {
            return Some(node.clone()); // self-match → full subtree
        }
        let children: Vec<CatalogNode> = node.children.iter().filter_map(|c| keep(c, q)).collect();
        if children.is_empty() {
            return None;
        }
        let mut out = node.clone();
        out.children = children;
        Some(out)
    }
    nodes.iter().filter_map(|n| keep(n, &q)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::RegistryResponse;

    /// The same committed golden the R-api gate pins (`dto::r_api`), so this test and the live
    /// palette read byte-identical data.
    const GOLDEN: &str = include_str!("../tests/fixtures/api/GET__registry.json");

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

    /// T-172 B9 — search filter: descendant match prunes siblings, folder self-match keeps the
    /// whole subtree, empty query is identity, no match → empty.
    #[test]
    fn filter_catalog_rules() {
        let tree = build_catalog_tree(&golden_items());
        assert_eq!(filter_catalog(&tree, "  "), tree, "empty query = identity");

        let rifle = filter_catalog(&tree, "rifleman");
        assert_eq!(rifle.len(), 1, "NATO kept via descendant");
        let leaves = &rifle[0].children[0].children;
        assert!(!leaves.is_empty() && leaves.len() < 8, "siblings pruned");
        assert!(leaves
            .iter()
            .all(|c| c.label.to_lowercase().contains("rifleman")));

        let nato = filter_catalog(&tree, "nato");
        assert_eq!(nato, tree, "folder self-match keeps the full subtree");

        assert!(filter_catalog(&tree, "zzz-none").is_empty());
    }

    // ── T-215 — the Vehicles palette ────────────────────────────────────────────────────────────

    /// A registry row shaped like the live `/registry` vehicle rows (the golden fixture is the
    /// 21-row character/gear capture, so it holds none).
    fn vehicle_row(resource: &str, name: &str, category: &str, is_abstract: bool) -> RegistryItem {
        serde_json::from_value(serde_json::json!({
            "id": resource,
            "modpack_id": "mp",
            "resource_name": resource,
            "display_name": name,
            "category": category,
            "kind": "vehicle",
            "abstract": is_abstract,
            "sort_order": 0,
            "created_at": "2026-07-26T00:00:00Z",
            "updated_at": "2026-07-26T00:00:00Z",
        }))
        .expect("vehicle row deserializes")
    }

    fn vehicle_items() -> Vec<RegistryItem> {
        let mut v = golden_items(); // 8 characters + 13 gear — none may reach this tree
        v.push(vehicle_row(
            "{A}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et",
            "UAZ469",
            "ArmaReforger/Vehicles/Wheeled/UAZ469",
            false,
        ));
        v.push(vehicle_row(
            "{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et",
            "UAZ469 PKM",
            "ArmaReforger/Vehicles/Wheeled/UAZ469",
            false,
        ));
        v.push(vehicle_row(
            "{C}Prefabs/Vehicles/Helicopters/Mi8MT/Mi8MT_base.et",
            "Mi8MT base",
            "ArmaReforger/Vehicles/Helicopters/Mi8MT",
            true, // abstract — a template, not placeable
        ));
        v
    }

    /// The whole category path becomes folders (not path-minus-last), so two variants of one family
    /// stay under that family instead of collapsing into its parent.
    #[test]
    fn vehicle_tree_keeps_the_family_folder() {
        let tree = build_vehicle_catalog_tree(&vehicle_items());

        assert_eq!(tree.len(), 1, "one addon root");
        assert_eq!(tree[0].id, "ArmaReforger");
        assert!(tree[0].default_expanded, "addon root opens");

        let vehicles = &tree[0].children[0];
        assert_eq!(vehicles.id, "ArmaReforger/Vehicles");
        assert!(vehicles.default_expanded, "depth 1 opens too");

        let wheeled = vehicles
            .children
            .iter()
            .find(|n| n.label == "Wheeled")
            .expect("Wheeled folder");
        assert!(
            !wheeled.default_expanded,
            "depth 2 is where the author chooses"
        );

        let uaz = &wheeled.children[0];
        assert_eq!(
            uaz.id, "ArmaReforger/Vehicles/Wheeled/UAZ469",
            "the family segment survives as a folder"
        );
        assert_eq!(
            uaz.children
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>(),
            vec!["UAZ469", "UAZ469 PKM"],
            "both variants sit under their family"
        );
        assert_eq!(
            uaz.children[1].payload,
            Some(PlacePayload {
                asset_id: "{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et".to_string(),
                role: "UAZ469 PKM".to_string(),
            }),
            "the drop carries the real ResourceName"
        );
    }

    /// `abstract` rows are templates the engine cannot spawn, and character/gear rows belong to the
    /// other tab. Neither may reach a placeable leaf.
    #[test]
    fn vehicle_tree_excludes_abstract_and_non_vehicle_rows() {
        let tree = build_vehicle_catalog_tree(&vehicle_items());

        fn leaves(nodes: &[CatalogNode], out: &mut Vec<String>) {
            for n in nodes {
                if n.payload.is_some() {
                    out.push(n.label.clone());
                }
                leaves(&n.children, out);
            }
        }
        let mut found = Vec::new();
        leaves(&tree, &mut found);
        found.sort();

        assert_eq!(
            found,
            vec!["UAZ469".to_string(), "UAZ469 PKM".to_string()],
            "the abstract Mi8MT base and every character/gear row are excluded"
        );
        assert!(
            !tree.iter().any(|n| n.label == "NATO"),
            "the Factions tree must not leak into the Vehicles tab"
        );
    }

    fn object_row(
        resource: &str,
        name: &str,
        category: &str,
        kind: &str,
        is_abstract: bool,
    ) -> RegistryItem {
        serde_json::from_value(serde_json::json!({
            "id": resource,
            "modpack_id": "mp",
            "resource_name": resource,
            "display_name": name,
            "category": category,
            "kind": kind,
            "abstract": is_abstract,
            "sort_order": 0,
            "created_at": "2026-07-26T00:00:00Z",
            "updated_at": "2026-07-26T00:00:00Z",
        }))
        .expect("object row deserializes")
    }

    fn object_items() -> Vec<RegistryItem> {
        let mut v = golden_items();
        v.push(object_row(
            "{FA}Prefabs/Props/Military/AmmoBox.et",
            "Ammo Crate 5.56",
            "ArmaReforger/Props/Military",
            "crate",
            false,
        ));
        v.push(object_row(
            "{FB}Prefabs/Props/Military/AmmoBox_base.et",
            "Ammo Crate base",
            "ArmaReforger/Props/Military",
            "crate",
            true,
        ));
        v.push(object_row(
            "{FC}Prefabs/Items/Demining/MineFlag.et",
            "Mine Flag",
            "ArmaReforger/Items/Demining",
            "other",
            false,
        ));
        v
    }

    #[test]
    fn object_tree_keeps_crates_and_excludes_abstract_and_characters() {
        let tree = build_object_catalog_tree(&object_items());
        fn leaves(nodes: &[CatalogNode], out: &mut Vec<String>) {
            for n in nodes {
                if n.payload.is_some() {
                    out.push(n.label.clone());
                }
                leaves(&n.children, out);
            }
        }
        let mut found = Vec::new();
        leaves(&tree, &mut found);
        found.sort();
        assert_eq!(
            found,
            vec!["Ammo Crate 5.56".to_string(), "Mine Flag".to_string()],
            "abstract crates and character/gear rows must not reach Objects leaves"
        );
        assert!(
            !tree.iter().any(|n| n.label == "NATO"),
            "Factions tree must not leak into Objects"
        );
    }

    #[test]
    fn derive_object_alias_slugs_display_name_and_hits_known_comp() {
        assert_eq!(
            derive_object_alias("{FA}Prefabs/Props/X.et", "Ammo Crate 5.56"),
            "prop:ammo_crate_5_56"
        );
        assert_eq!(
            derive_object_alias(
                "{E1D01D77D7F47EF3}PrefabsEditable/Auto/Compositions/Misc/SubCompositions/E_Sandbag_Barricade_US_04.et",
                "Sandbag Barricade"
            ),
            "comp:checkpoint_small"
        );
        assert_eq!(
            derive_object_alias(
                "{X}PrefabsEditable/Auto/Compositions/Misc/Foo.et",
                "Checkpoint Small"
            ),
            "comp:checkpoint_small"
        );
    }
}
