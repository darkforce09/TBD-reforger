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
//! **T-255 — Eden side filter.** The live Workbench registry encodes side as a path segment in
//! `category` (and the matching `resource_name`): `…/Factions/BLUFOR/…`, `…/OPFOR/…`, `…/INDFOR/…`.
//! The committed 21-row golden still uses the older `NATO/…` category root with side only in
//! `resource_name`; both conventions are accepted. CIV / tutorial / untagged rows never match a
//! chip side, so a BLUFOR chip cannot surface a USSR character.
//!
//! Rows are consumed in array order — the API pre-sorts by `sort_order`, so faction/role order stays
//! stable without a sort here (the oracle's comment, and true of the golden).
//!
//! Pure + native-testable on purpose: no `web_sys`, no signals. The view layer is `eden_chrome`.
#![allow(dead_code)]

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::dto::RegistryItem;

/// Mod spawn registry (`apps/mod/tbd-framework/Data/registry.json`) — T-439 pins Objects
/// palette leaves to aliases this file actually resolves. Included at compile time so the
/// wasm palette cannot offer a synthesised `prop:`/`comp:` the mod would warn-skip.
const MOD_SPAWN_REGISTRY_JSON: &str =
    include_str!("../../../../apps/mod/tbd-framework/Data/registry.json");

/// `prop:` / `comp:` aliases present in the mod spawn registry (T-439).
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

/// True when a crate/other row's derived alias exists in the mod spawn registry (T-439).
#[must_use]
pub fn object_alias_registered(resource_name: &str, display_name: &str) -> bool {
    mod_object_aliases().contains(&derive_object_alias(resource_name, display_name))
}

/// Eden chip sides the Factions palette may filter on (T-180.5 — no CIV chip).
const EDEN_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// True when a slash-delimited path contains an exact segment equal to `side`.
#[must_use]
fn path_has_side_segment(path: &str, side: &str) -> bool {
    path.split('/').any(|seg| seg == side)
}

/// Legacy category-root aliases used by the committed golden / early registry seeds (T-068.2),
/// which file US Army under `NATO/…` instead of embedding `BLUFOR` in the category path.
#[must_use]
fn legacy_category_root_side(category: &str) -> Option<&'static str> {
    match category.split('/').next().unwrap_or("") {
        "NATO" => Some("BLUFOR"),
        "USSR" => Some("OPFOR"),
        "FIA" => Some("INDFOR"),
        _ => None,
    }
}

/// Whether a registry character belongs under the active Eden side chip (T-255).
///
/// Measured conventions (Workbench `registry-items.workbench.json` + golden fixture):
/// 1. `category` path segment equals the side (`…/BLUFOR/…`) — live export.
/// 2. `resource_name` path segment equals the side (`…/Factions/BLUFOR/…`) — golden + export.
/// 3. Legacy top-level category root `NATO` / `USSR` / `FIA` → BLUFOR / OPFOR / INDFOR.
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

/// Build the palette tree from the flat registry rows for one Eden side. See the module docs.
///
/// `side` is the active chip (`"BLUFOR"` / `"OPFOR"` / `"INDFOR"`). Rows that do not match are
/// dropped before folders are built, so a BLUFOR tree never contains a USSR leaf.
#[must_use]
pub fn build_catalog_tree(items: &[RegistryItem], side: &str) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    for item in items
        .iter()
        .filter(|i| i.kind == "character" && character_matches_eden_side(i, side))
    {
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
///
/// T-439 — only leaves whose [`derive_object_alias`] exists in mod `Data/registry.json`
/// (`prop:` / `comp:`). Unregistered kinds are dropped from the palette so SpawnMissionEntities
/// never warn-skips a leaf the author was offered.
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

/// Derive a schema `#/$defs/alias` for a placed object from its ResourceName + display name.
///
/// Prefer a known mod-registry reverse hit (`comp:checkpoint_small`); otherwise synthesise
/// `prop:<slug>` / `comp:<slug>` from the display name (Composition path → `comp:`).
///
/// T-439 — synthesis alone is not enough for spawn: [`build_object_catalog_tree`] only offers
/// leaves whose alias is present in mod `Data/registry.json`, and
/// `scripts/mod/verify-t439-objects-registry-aliases.sh` pins every workbench Objects-eligible
/// kind to a matching registry row (guid == resource_name).
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

/// T-646 (RIGHT-SEARCH-002) — the search operator a query opens with, recognised in front of the
/// default label-substring match. Eden's full grammar (`mod `, `*`/`?` globs, `/…/` regex) is
/// T-084's rewrite; this ticket adds **only** `class:` and must stay additive so the two compose —
/// see [`filter_catalog`].
///
/// `class:B_Soldier` matches by CLASSNAME (a leaf's `id` = its Enfusion `resource_name`), prefix,
/// case-insensitive. `class:` with an empty operand is a deliberate no-match (the dock's empty state
/// says so) — an author mid-typing `class:` should see nothing, not the whole tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchQuery<'a> {
    /// No recognised operator: the historical case-insensitive **label** substring (T-055).
    Label(&'a str),
    /// `class:<operand>` — case-insensitive **classname** (`id`) prefix. `operand` is already
    /// lowercased and trimmed; empty ⇒ match nothing.
    ClassPrefix(String),
}

/// The `class:` operator token. A trailing space is allowed (`class: B_Soldier`) — the operand is
/// trimmed — but the token itself is exact so `classy` / `subclass:` never trip it.
const CLASS_PREFIX: &str = "class:";

/// T-646 (RIGHT-SEARCH-002) — recognise the search operator in front of a raw query.
///
/// Kept as its own function (not inlined into [`filter_catalog`]) so T-084 extends the grammar in
/// one place and the recogniser is unit-testable on its own. Only `class:` is recognised here; every
/// other query — including one that merely *contains* `class:` past the start, e.g.
/// `"first class:"` — is a plain [`SearchQuery::Label`], matching Eden, where the operator is a
/// leading token.
#[must_use]
pub fn parse_search_query(query: &str) -> SearchQuery<'_> {
    let trimmed = query.trim_start();
    if let Some(rest) = strip_prefix_ci(trimmed, CLASS_PREFIX) {
        return SearchQuery::ClassPrefix(rest.trim().to_lowercase());
    }
    // Historical path: trim + lowercase happens in `filter_catalog` (unchanged), so hand back the
    // raw slice untouched here.
    SearchQuery::Label(query)
}

/// T-646 (RIGHT-SEARCH-002) — the empty-state line the dock shows when a `filter_catalog(query)`
/// came back empty, so the `class:` empty-operand case "says so" instead of reading like a genuine
/// no-match. `noun` is the tab's word (`"assets"` / `"objects"` / `"vehicles"`). A `class:` with an
/// empty operand is a mid-type state, not a failed search — the message tells the author to keep
/// typing the classname rather than implying nothing matched.
#[must_use]
pub fn search_empty_message(query: &str, noun: &str) -> String {
    if matches!(parse_search_query(query), SearchQuery::ClassPrefix(ref op) if op.is_empty()) {
        "Type a class name after class:".to_string()
    } else {
        format!("No {noun} match.")
    }
}

/// Case-insensitive `strip_prefix`: `Some(remainder)` when `s` begins with `prefix` ignoring ASCII
/// case, else `None`. `class:` is ASCII, so `eq_ignore_ascii_case` on the head is exact and avoids
/// allocating a lowercased copy of the whole query just to test the operator.
fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    // is_char_boundary is load-bearing, not defensive: `s[..prefix.len()]` is a BYTE slice, and a
    // multibyte char straddling that offset ("beauté" splits é at byte 6) panics — in wasm that
    // aborts the whole Leptos runtime, and every dock search routes every keystroke through here.
    // Wave-105 verifier BLOCKER-1; the boundary test above the fix reproduces it.
    if s.len() >= prefix.len()
        && s.is_char_boundary(prefix.len())
        && s[..prefix.len()].eq_ignore_ascii_case(prefix)
    {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

/// Asset-search filter (T-172 B9 — the T-055 React behavior): case-insensitive label substring.
/// A folder survives on a self-match (keeping its whole subtree) or on any descendant match
/// (keeping only the matching children). Empty/whitespace query returns the tree unchanged.
///
/// T-646 (RIGHT-SEARCH-002) — a `class:<operand>` query switches to CLASSNAME (`id`) prefix
/// matching: a leaf is kept when its `id` starts with the operand (case-insensitive), and a folder
/// is kept (with only its matching descendants) when any leaf under it does. The recogniser
/// ([`parse_search_query`]) runs FIRST; every non-`class:` query falls through to the byte-identical
/// label `keep` closure below, so this stays additive — T-084 owns the wider grammar rewrite and
/// composes on top of the same recogniser without touching this core matcher.
#[must_use]
pub fn filter_catalog(nodes: &[CatalogNode], query: &str) -> Vec<CatalogNode> {
    match parse_search_query(query) {
        SearchQuery::ClassPrefix(operand) => {
            // Empty operand (`class:` with nothing after) matches nothing — the dock shows its
            // "No assets match" empty state, not the whole tree.
            if operand.is_empty() {
                return Vec::new();
            }
            fn keep(node: &CatalogNode, operand: &str) -> Option<CatalogNode> {
                if node.payload.is_some() {
                    // Leaf: kept iff its classname (id) prefix-matches.
                    return node
                        .id
                        .to_lowercase()
                        .starts_with(operand)
                        .then(|| node.clone());
                }
                // Folder: unlike the label path there is no self-match — a folder has no classname —
                // so it survives only on a descendant leaf, keeping just the matching children.
                let children: Vec<CatalogNode> = node
                    .children
                    .iter()
                    .filter_map(|c| keep(c, operand))
                    .collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, &operand)).collect()
        }
        SearchQuery::Label(raw) => {
            let q = raw.trim().to_lowercase();
            if q.is_empty() {
                return nodes.to_vec();
            }
            fn keep(node: &CatalogNode, q: &str) -> Option<CatalogNode> {
                if node.label.to_lowercase().contains(q) {
                    return Some(node.clone()); // self-match → full subtree
                }
                let children: Vec<CatalogNode> =
                    node.children.iter().filter_map(|c| keep(c, q)).collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, &q)).collect()
        }
    }
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
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");

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
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
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

        let tree = build_catalog_tree(&items, "BLUFOR");
        let leaves = tree[0].children[0].children.len();
        assert_eq!(leaves, 8, "only character rows are placed");
        // The gear categories (NATO/Uniform, NATO/Vest, …) would have added sibling folders.
        assert_eq!(tree[0].children.len(), 1, "no gear folders under NATO");
    }

    /// T-172 B9 — search filter: descendant match prunes siblings, folder self-match keeps the
    /// whole subtree, empty query is identity, no match → empty.
    #[test]
    fn filter_catalog_rules() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
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

    // ── T-646 (RIGHT-SEARCH-002) — the `class:` recogniser ────────────────────────────────────────

    /// The recogniser reads a leading `class:` and hands back a lowercased/trimmed operand; anything
    /// else is a plain label query. The operator is a LEADING token only — a query that merely
    /// contains it later stays a label match (Eden's grammar), and `class` without the colon is not
    /// the operator.
    #[test]
    fn parse_search_query_recognises_class_operator() {
        assert_eq!(
            parse_search_query("class:B_Soldier"),
            SearchQuery::ClassPrefix("b_soldier".to_string()),
            "class: → lowercased operand"
        );
        assert_eq!(
            parse_search_query("  CLASS: B_Soldier "),
            SearchQuery::ClassPrefix("b_soldier".to_string()),
            "operator is case-insensitive; leading/trailing space trimmed"
        );
        assert_eq!(
            parse_search_query("class:"),
            SearchQuery::ClassPrefix(String::new()),
            "empty operand is recognised (and filters to nothing)"
        );
        // Not the operator: a bare word, or `class:` appearing past the start.
        assert_eq!(
            parse_search_query("rifleman"),
            SearchQuery::Label("rifleman")
        );
        assert_eq!(
            parse_search_query("classy"),
            SearchQuery::Label("classy"),
            "`class` without the colon is a label"
        );
        assert_eq!(
            parse_search_query("first class:"),
            SearchQuery::Label("first class:"),
            "the operator is a leading token, not a substring"
        );
    }

    /// `class:<prefix>` matches a LEAF by its classname (`id` = `resource_name`), prefix,
    /// case-insensitively — HIT (one leaf), MISS (empty tree), CASE (lower/upper agree), and
    /// EMPTY-OPERAND (`class:` alone ⇒ nothing). The un-prefixed label path is unchanged — proven by
    /// re-running a label query and getting the historical result.
    #[test]
    fn filter_catalog_class_prefix() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        let rifleman_id =
            "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";

        // HIT — a GUID prefix that only the Rifleman leaf carries. The NATO / US_Army folders survive
        // by descent; every non-matching sibling leaf is pruned.
        let hit = filter_catalog(&tree, "class:{26A9756790131354}");
        assert_eq!(hit.len(), 1, "NATO kept via the one matching descendant");
        let leaves = &hit[0].children[0].children;
        assert_eq!(leaves.len(), 1, "only the prefix-matching leaf survives");
        assert_eq!(leaves[0].id, rifleman_id);
        assert!(
            leaves[0].payload.is_some(),
            "the survivor is the placeable leaf"
        );

        // A broader classname prefix every US_Army leaf shares → all 8 back (folders by descent).
        let all = filter_catalog(&tree, "class:{");
        assert_eq!(
            all[0].children[0].children.len(),
            8,
            "all classnames share the GUID-brace start"
        );

        // CASE — the operand is matched case-insensitively against the id.
        let lower = filter_catalog(&tree, "class:{26a9756790131354}prefabs");
        assert_eq!(
            lower.len(),
            1,
            "lowercased operand matches the mixed-case id"
        );
        assert_eq!(lower[0].children[0].children[0].id, rifleman_id);

        // MISS — a prefix no classname starts with.
        assert!(
            filter_catalog(&tree, "class:{ZZZZ}").is_empty(),
            "a non-matching class prefix yields the empty tree"
        );
        // MISS — the operand is a PREFIX, not a substring: `Rifleman` sits mid-id, so it must NOT hit.
        assert!(
            filter_catalog(&tree, "class:Rifleman").is_empty(),
            "class: is prefix-only — a mid-classname token does not match"
        );

        // EMPTY-OPERAND — `class:` with nothing after matches nothing (the dock's empty state).
        assert!(
            filter_catalog(&tree, "class:").is_empty(),
            "class: with an empty operand matches nothing"
        );
        assert!(
            filter_catalog(&tree, "class:   ").is_empty(),
            "class: with whitespace-only operand also matches nothing"
        );

        // ADDITIVE PROOF — the label path is untouched: a plain query still self-matches the folder
        // and returns the historical full subtree (the `filter_catalog_rules` contract).
        assert_eq!(
            filter_catalog(&tree, "nato"),
            tree,
            "an un-prefixed query is still the T-055 label substring match"
        );
    }

    /// T-646 — the empty state distinguishes a mid-type `class:` (no operand yet) from a genuine
    /// miss, so the dock "says so" rather than implying nothing matched.
    #[test]
    fn class_empty_operand_has_its_own_empty_message() {
        assert_eq!(
            search_empty_message("class:", "assets"),
            "Type a class name after class:",
            "empty operand → guidance, not a miss"
        );
        assert_eq!(
            search_empty_message("class:   ", "vehicles"),
            "Type a class name after class:",
            "whitespace-only operand is still empty"
        );
        // A real miss (non-empty operand, or a label query) reads the plain noun message.
        assert_eq!(
            search_empty_message("class:zzz", "objects"),
            "No objects match."
        );
        assert_eq!(
            search_empty_message("rifleman", "assets"),
            "No assets match."
        );
    }

    /// Wave-105 verifier BLOCKER-1: `strip_prefix_ci` byte-sliced `s[..6]` with no char-boundary
    /// check, so any query whose 6th byte split a multibyte char — "beauté", "a日本" — panicked on
    /// the keystroke and aborted the wasm runtime. Every dock search routes every keystroke here.
    /// All prior tests were ASCII, which is why nothing covered it.
    #[test]
    fn multibyte_queries_do_not_panic_the_recogniser() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        // 6th byte mid-'é' (é is 2 bytes at byte 5): the exact boundary-split shape.
        for q in [
            "beauté",
            "abcdeß",
            "a日本",
            "clasé",
            "über-search",
            "日本語のクエリ",
        ] {
            // Must not panic; multibyte text is never the `class:` operator, so it label-matches.
            let _ = filter_catalog(&tree, q);
            assert!(
                matches!(parse_search_query(q), SearchQuery::Label(_)),
                "{q} must stay Label"
            );
        }
        // A multibyte OPERAND after a well-formed `class:` head must also survive.
        let _ = filter_catalog(&tree, "class:beauté");
    }

    /// The single load-bearing assertion, wired to FIRE once: `class:` selects a leaf by its
    /// classname where a plain label query over the SAME token cannot. If the recogniser were
    /// dropped (the query fell through to the label path) the classname-only token would find nothing
    /// and this would fail — so a GREEN here means the `class:` arm actually ran.
    #[test]
    fn class_prefix_fires_where_label_cannot() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        // `Character_US_Rifleman` is in every leaf's classname (`id`) but in NO label
        // (labels are "US Rifleman", "US Grenadier", …), so it is the perfect discriminator.
        let token = "Character_US_Rifleman";

        // Label path over the token: the historical matcher finds nothing (it is not in any label).
        assert!(
            filter_catalog(&tree, token).is_empty(),
            "guard: the classname token is absent from every label"
        );
        // class: path over the same token: the recogniser routes to id-prefix matching. It is a
        // prefix of the id only after the `{GUID}Prefabs/…/` head, so match on the full leading id.
        let classq =
            "class:{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman";
        let hit = filter_catalog(&tree, classq);
        assert_eq!(
            hit.len(),
            1,
            "class: fired: the classname-prefix leaf was selected"
        );
        assert_eq!(
            hit[0].children[0].children[0].label, "US Rifleman",
            "and it is the right leaf"
        );
    }

    /// CHIP + SEARCH COMPOSITION — the active chip filters the tree (via `build_catalog_tree`, which
    /// side-filters through `character_matches_eden_side`) BEFORE `class:`/label search runs on the
    /// result. An OPFOR chip + a BLUFOR-classname `class:` query is empty (the BLUFOR leaves were
    /// already dropped by the chip), while the same query on the BLUFOR tree hits — proving the two
    /// filters compose in that order.
    #[test]
    fn chip_side_then_class_search_compose() {
        let mut items = golden_items();
        items.push(character_row(
            "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
            "USSR Rifleman",
            "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
        ));

        // Chip = OPFOR → the tree holds only USSR. A BLUFOR-classname class: query finds nothing.
        let opfor = build_catalog_tree(&items, "OPFOR");
        assert!(
            filter_catalog(&opfor, "class:{26A9756790131354}").is_empty(),
            "chip filtered BLUFOR out before search; the BLUFOR class prefix cannot match"
        );
        // …but the OPFOR classname does match on the OPFOR tree.
        let opfor_hit = filter_catalog(&opfor, "class:{DCB41B3746FDD1BE}");
        assert_eq!(
            opfor_hit.len(),
            1,
            "the OPFOR class prefix matches the USSR leaf"
        );

        // Chip = BLUFOR → the same BLUFOR class query now hits (chip kept the NATO leaves).
        let blufor = build_catalog_tree(&items, "BLUFOR");
        assert_eq!(
            filter_catalog(&blufor, "class:{26A9756790131354}").len(),
            1,
            "on the BLUFOR tree the BLUFOR class prefix hits — search runs on the chip-filtered tree"
        );
        // Composition also holds for a plain label query on the chip-filtered tree.
        assert!(
            filter_catalog(&opfor, "US Rifleman").is_empty(),
            "a label query for a BLUFOR role is empty under the OPFOR chip"
        );
    }

    /// CHIP PREDICATE PER SIDE — `character_matches_eden_side` (the predicate RIGHT-SUBMODE-001 rides,
    /// and what `build_catalog_tree` filters through) admits a row for exactly its own side. This
    /// pins the predicate the chip filtering depends on, independently of the tree builder.
    #[test]
    fn chip_side_predicate_per_side() {
        let us = character_row(
            "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et",
            "US Rifleman",
            "ArmaReforger/Characters/Factions/BLUFOR/US_Army/Rifleman",
        );
        let ussr = character_row(
            "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
            "USSR Rifleman",
            "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
        );
        let fia = character_row(
            "{84B40583F4D1B7A3}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_Rifleman.et",
            "FIA Rifleman",
            "ArmaReforger/Characters/Factions/INDFOR/FIA/Rifleman",
        );
        // Each row matches its own side and no other.
        assert!(character_matches_eden_side(&us, "BLUFOR"));
        assert!(!character_matches_eden_side(&us, "OPFOR"));
        assert!(!character_matches_eden_side(&us, "INDFOR"));
        assert!(character_matches_eden_side(&ussr, "OPFOR"));
        assert!(!character_matches_eden_side(&ussr, "BLUFOR"));
        assert!(character_matches_eden_side(&fia, "INDFOR"));
        assert!(!character_matches_eden_side(&fia, "BLUFOR"));
        // Unknown / empty side never matches (the chip row admits only the three sides).
        assert!(!character_matches_eden_side(&us, "CIV"));
        assert!(!character_matches_eden_side(&us, ""));
    }

    fn character_row(resource: &str, name: &str, category: &str) -> RegistryItem {
        serde_json::from_value(serde_json::json!({
            "id": resource,
            "modpack_id": "mp",
            "resource_name": resource,
            "display_name": name,
            "category": category,
            "kind": "character",
            "sort_order": 0,
            "created_at": "2026-07-26T00:00:00Z",
            "updated_at": "2026-07-26T00:00:00Z",
        }))
        .expect("character row deserializes")
    }

    fn leaf_labels(nodes: &[CatalogNode]) -> Vec<String> {
        let mut out = Vec::new();
        fn walk(nodes: &[CatalogNode], out: &mut Vec<String>) {
            for n in nodes {
                if n.payload.is_some() {
                    out.push(n.label.clone());
                }
                walk(&n.children, out);
            }
        }
        walk(nodes, &mut out);
        out
    }

    /// T-255 Class-R — mixed BLUFOR+OPFOR rows: each chip sees only its side. Perturbation RED:
    /// dropping the side filter (kind-only) would put USSR under BLUFOR and NATO under OPFOR.
    #[test]
    fn side_filter_excludes_cross_side_characters() {
        let mut items = golden_items();
        items.push(character_row(
            "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
            "USSR Rifleman",
            "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
        ));
        items.push(character_row(
            "{84B40583F4D1B7A3}Prefabs/Characters/Factions/INDFOR/FIA/Character_FIA_Rifleman.et",
            "FIA Rifleman",
            "ArmaReforger/Characters/Factions/INDFOR/FIA/Rifleman",
        ));

        let blufor = leaf_labels(&build_catalog_tree(&items, "BLUFOR"));
        assert!(
            blufor.iter().any(|l| l == "US Rifleman"),
            "BLUFOR must keep US Army leaves"
        );
        assert!(
            !blufor
                .iter()
                .any(|l| l.contains("USSR") || l.contains("FIA")),
            "BLUFOR chip must not accept USSR/FIA — got {blufor:?}"
        );

        let opfor = leaf_labels(&build_catalog_tree(&items, "OPFOR"));
        assert_eq!(opfor, vec!["USSR Rifleman".to_string()]);
        assert!(
            !opfor.iter().any(|l| l.starts_with("US ")),
            "OPFOR chip must not accept NATO leaves — got {opfor:?}"
        );

        let indfor = leaf_labels(&build_catalog_tree(&items, "INDFOR"));
        assert_eq!(indfor, vec!["FIA Rifleman".to_string()]);

        // Empty / unknown side → empty tree (never dump the whole registry).
        assert!(build_catalog_tree(&items, "").is_empty());
        assert!(build_catalog_tree(&items, "CIV").is_empty());
    }

    /// T-255 — legacy golden root `NATO/…` matches BLUFOR even when the category omits the side
    /// segment (resource_name still carries `/Factions/BLUFOR/`).
    #[test]
    fn legacy_nato_root_matches_blufor_chip() {
        let items = golden_items();
        assert!(
            !build_catalog_tree(&items, "BLUFOR").is_empty(),
            "golden NATO characters must match BLUFOR"
        );
        assert!(
            build_catalog_tree(&items, "OPFOR").is_empty(),
            "golden has no OPFOR characters"
        );
        let ussr = character_row(
            "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
            "USSR Rifleman",
            "USSR/USSR_Army/Rifleman",
        );
        assert!(
            character_matches_eden_side(&ussr, "OPFOR"),
            "legacy USSR/ root must map to OPFOR"
        );
        assert!(!character_matches_eden_side(&ussr, "BLUFOR"));
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
        // Registered in mod Data/registry.json (T-439) — must reach Objects leaves.
        v.push(object_row(
            "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd.et",
            "AmmoBox 50cal 100rnd",
            "ArmaReforger/Props/Military/AmmoBoxes",
            "crate",
            false,
        ));
        // Abstract — excluded regardless of registry.
        v.push(object_row(
            "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd_base.et",
            "AmmoBox 50cal base",
            "ArmaReforger/Props/Military/AmmoBoxes",
            "crate",
            true,
        ));
        // Registered composition-path crate.
        v.push(object_row(
            "{3568138FF7A659A1}Prefabs/Compositions/Misc/CustomEntities/InteractionPoints/AmmoBoxArsenal_Equipment_US_Apparel.et",
            "AmmoBoxArsenal Equipment US Apparel",
            "ArmaReforger/Compositions/Misc/CustomEntities/InteractionPoints",
            "crate",
            false,
        ));
        // Synthesises prop:unregistered_test_crate — NOT in mod registry → dropped (T-439).
        v.push(object_row(
            "{DEADBEEFDEADBEEF}Prefabs/Props/Military/Unregistered.et",
            "Unregistered Test Crate",
            "ArmaReforger/Props/Military",
            "crate",
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
            vec![
                "AmmoBox 50cal 100rnd".to_string(),
                "AmmoBoxArsenal Equipment US Apparel".to_string(),
            ],
            "abstract crates, unregistered aliases, and character/gear rows must not reach Objects leaves"
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

    /// T-439 Class-R: mod spawn registry must expose the Objects alias set the palette filters on.
    #[test]
    fn t439_mod_registry_exposes_prop_and_comp_aliases() {
        let aliases = mod_object_aliases();
        let prop = aliases.iter().filter(|a| a.starts_with("prop:")).count();
        let comp = aliases.iter().filter(|a| a.starts_with("comp:")).count();
        assert!(
            prop >= 289,
            "expected ≥289 prop: rows in mod registry, got {prop}"
        );
        assert!(
            comp >= 45,
            "expected ≥45 comp: rows in mod registry (incl. checkpoint_small), got {comp}"
        );
        assert!(
            aliases.contains("comp:checkpoint_small"),
            "POC comp:checkpoint_small must remain registered"
        );
        assert!(
            object_alias_registered(
                "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd.et",
                "AmmoBox 50cal 100rnd"
            ),
            "workbench crate must resolve to a registered prop: alias"
        );
        assert!(
            !object_alias_registered(
                "{DEADBEEFDEADBEEF}Prefabs/Props/Military/Unregistered.et",
                "Unregistered Test Crate"
            ),
            "unregistered synthesised alias must not pass the palette gate"
        );
    }
}
