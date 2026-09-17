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
pub use website_map_engine::data::store::operations::assets::classname_tail;
pub use website_map_engine::data::store::operations::assets::derive_object_alias;

pub use website_map_engine::data::store::operations::assets::PlacePayload;

use crate::v2::core::api::dto::RegistryItem;

/// Mod spawn registry (`apps/mod/tbd-framework/Data/registry.json`) — T-439 pins Objects
/// palette leaves to aliases this file actually resolves. Included at compile time so the
/// wasm palette cannot offer a synthesised `prop:`/`comp:` the mod would warn-skip.
const MOD_SPAWN_REGISTRY_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../apps/mod/tbd-framework/Data/registry.json"
));

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

/// Whether a **non-character** registry row (vehicle / object) belongs under the active Eden side.
///
/// T-809 (F-22) — the merged Factions tree files vehicles beside characters under one faction root,
/// so a vehicle needs the same side test a character gets from [`character_matches_eden_side`]. The
/// live seed roots its vehicle rows the faction way (`NATO/US_Army/Vehicles`, T-800), not the
/// addon way the standalone Vehicles tab assumed (`ArmaReforger/Vehicles/…`), so the same two
/// conventions apply: an explicit `…/BLUFOR/…` segment in either the category or the resource_name,
/// or the legacy `NATO`/`USSR`/`FIA` category root. A row that carries neither (an addon-rooted
/// `ArmaReforger/…` vehicle with no faction) matches NO side and so never lands in a faction tree —
/// which is the honest answer: nothing in the row says which faction it is.
#[must_use]
pub fn asset_matches_eden_side(item: &RegistryItem, side: &str) -> bool {
    if !EDEN_SIDES.contains(&side) {
        return false;
    }
    path_has_side_segment(&item.category, side)
        || path_has_side_segment(&item.resource_name, side)
        || legacy_category_root_side(&item.category) == Some(side)
}

/// How deep the merged Factions tree opens on first paint (T-809). Only the depth-0 faction folder
/// opens — same rule and same reason as [`build_catalog_tree`]: depth 0 is the FACTION, the axis an
/// author picks first, and everything under it (roles, a Vehicles sub-tree, objects) stays folded
/// until the author drills in. This deliberately does NOT inherit the standalone Vehicles tab's
/// `VEHICLE_OPEN_DEPTH` of 2: that value existed to skip an addon root the merged tree does not have.
const FACTION_OPEN_DEPTH: usize = 1;

/// T-809 (F-22) — the **merged Factions tree**: ONE tree per faction, characters and vehicles (and
/// objects when catalogued) filed together under their shared faction root, so a vehicle leaf is
/// reachable inside its faction instead of stranded on a separate tab (the "know TBD's filing system
/// before you can find anything" defect the UX review named).
///
/// This is the Eden F1-Objects shape: under `NATO` sit the roles AND a Vehicles sub-tree, not three
/// parallel tabs. It composes the three per-kind rules already proven above rather than inventing a
/// fourth:
///
/// * **characters** keep [`build_catalog_tree`]'s rule 2 — the last category segment is the role and
///   the leaf is the `display_name` (`NATO/US_Army/Rifleman` → `NATO` > `US_Army` > "US Rifleman");
/// * **vehicles** keep [`build_vehicle_catalog_tree`]'s whole-path rule and its `abstract` exclusion
///   (`NATO/US_Army/Vehicles/…` → the family folder survives, `*_base.et` templates are dropped);
/// * **objects** keep [`build_object_catalog_tree`]'s whole-path rule, `abstract` exclusion, and the
///   T-439 spawn-registry gate.
///
/// All three are **side-filtered** (characters via [`character_matches_eden_side`], the rest via
/// [`asset_matches_eden_side`]) so the tree the chips draw is one faction's material only. Folder ids
/// are the accumulated path prefix (rule 3) exactly as the per-kind builders mint them, so a folder
/// two kinds both file into (e.g. `NATO/US_Army`) is the SAME node — that shared-id merge is what
/// puts characters and vehicles under one `US_Army`. Rows are consumed in array order (the API
/// pre-sorts by `sort_order`), so within a folder characters precede vehicles precede objects only
/// where `sort_order` already puts them so; no re-sort here.
///
/// A leaf carries no per-kind tag: the view resolves which place-arm a leaf fires from the live
/// registry row ([`placeable_palette`]) at press time, the same resolution the Favourites collection
/// uses — so this builder stays pure, native-testable, and free of the view's `PaletteKind`.
#[must_use]
pub fn build_faction_catalog_tree(items: &[RegistryItem], side: &str) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    // File one leaf under the folder path `folder_segs`, minting shared-id folders as needed.
    // `open_depth` decides which folders start expanded (rule 3, generalised to the merged tree).
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
            // Rule 2 — drop the role segment; `display_name` is the leaf.
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
            // Whole category path is the folder chain (a vehicle's last segment is its family).
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

/// T-810 (F-23) — the **Attributes TYPE picker** tree: [`build_faction_catalog_tree`] with the Eden
/// SIDE filter removed, so a single tree spans every faction.
///
/// The dock's Factions tree is side-filtered because the chips pick one faction to browse. The
/// Attributes modal is editing an EXISTING slot whose faction the operator is not choosing here —
/// they are re-typing what a placed entity spawns as, and constraining that to whichever chip
/// happens to be up would hide the very leaf they mean (an OPFOR slot edited while the BLUFOR chip
/// is active). It also lets this builder stay a pure function of the registry alone: the modal is
/// handed `registry_items` but NOT `active_side` (that signal lives on `mission_editor.rs`, past
/// this file's boundary), so a side-aware picker here would need a parameter the call site cannot
/// pass. The result is `resource_name`-keyed leaves exactly like the dock tree, so a picked leaf
/// writes the SAME canonical `assetId` a drop would and the `ASSET-RESOLVES` validator clears.
///
/// ADDITIVE per the T-809 boundary: this composes the three per-kind rules `build_faction_catalog_tree`
/// already owns (via the shared filing walk) and does not touch it. The one difference is the side
/// gate; everything else — the `abstract` exclusions, the object spawn-registry gate, the
/// shared-id folder merge — is identical, so the picker and the dock offer the same leaves modulo side.
#[must_use]
pub fn build_picker_catalog_tree(items: &[RegistryItem]) -> Vec<CatalogNode> {
    let mut roots: Vec<CatalogNode> = Vec::new();

    // The same folder-minting walk `build_faction_catalog_tree` uses (duplicated as a nested fn rather
    // than shared to keep that function's minimal T-809 surface untouched — this is the only extra
    // caller and it needs the identical shape). `open_depth` mirrors `FACTION_OPEN_DEPTH`.
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
            // Rule 2 — drop the role segment; `display_name` is the leaf.
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

/// T-810 — count the placeable LEAVES in a (possibly filtered) tree. A leaf is `payload.is_some()`
/// (module docs), so this is "how many things could actually be picked" — the number the TYPE
/// picker's empty-state branch turns on. Zero leaves on a NON-empty registry is a real "nothing to
/// offer" (a modpack with only abstract rows, say); zero on an empty registry is the dev-without-seed
/// case. Either way the picker must show the cause+retry surface, never a bare empty list (F-23 / the
/// T-800 lesson), and this is the predicate that decides it — pure and native-testable.
#[must_use]
pub fn catalog_leaf_count(nodes: &[CatalogNode]) -> usize {
    nodes
        .iter()
        .map(|n| usize::from(n.payload.is_some()) + catalog_leaf_count(&n.children))
        .sum()
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
            // the vehicle place path reads `asset_id` only (`armed_placement::place_at`).
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

// ── T-084 (RIGHT-SEARCH-002/003/004/005) — the asset-browser search GRAMMAR ──────────────────────
//
// One query string, two independent halves:
//
//   [operator] [pattern]
//    class:     Character_US_Ri      → FIELD = classname,  PATTERN = plain
//    mod:       ArmaReforger         → FIELD = mod root,   PATTERN = plain
//    (none)     *rifle*              → FIELD = label,      PATTERN = glob
//    class:     /us_(mg|ar)\.et$/    → FIELD = classname,  PATTERN = regex
//
// The two halves are parsed separately and then crossed, which is why four parity ids
// (`RIGHT-SEARCH-002` `class:`, `003` `mod:`, `004` glob, `005` regex) cost one grammar rather than
// four filters: a pattern is matched against whichever field the operator selected, so every
// operator gains every pattern for free and a new operator is one table row.
//
// T-646 shipped the `class:` half of this (operator recognition + classname matching) and this
// rewrite subsumes it: every behaviour T-646 pinned still holds, with ONE deliberate change — see
// [`classname_tail`] for the wave-105 MINOR-2 decision on bare classnames.

/// Which FIELD of a palette leaf an operator selects.
///
/// The three are genuinely different data, not three spellings of one string:
/// * `Label` is the author-facing `display_name` ("US Rifleman") — the historical T-055 search.
/// * `ClassName` is the Enfusion `resource_name`
///   (`{26A9756790131354}Prefabs/…/Character_US_Rifleman.et`) — what a drop actually carries.
/// * `Mod` is the addon the row came from, which in this catalogue is the ROOT of the category path
///   (`ArmaReforger/Vehicles/Wheeled/UAZ469`) and therefore the tree's depth-0 folder. See
///   `VEHICLE_OPEN_DEPTH`'s note: the vehicle and object trees are addon-rooted by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchField {
    /// Default: the leaf/folder `label`, case-insensitive SUBSTRING (T-055, unchanged).
    Label,
    /// `class:` — the leaf `id` (`resource_name`) or its [`classname_tail`], PREFIX.
    ClassName,
    /// `mod:` / `mod ` — the depth-0 (addon) folder, PREFIX.
    Mod,
}

/// The PATTERN half of a query — how the operand is matched, independent of which field it is
/// matched against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchPattern {
    /// The raw query was empty/whitespace: no filter at all, the tree is returned unchanged.
    All,
    /// An operator (or a `/`) was typed with NO operand yet. Matches nothing, and the dock shows
    /// guidance rather than "no match" — a half-typed query is a mid-type state, not a failed
    /// search. (T-646's `class:` empty-operand rule, generalised to every operator.)
    Pending,
    /// A literal operand, already lowercased. Substring for `Label`, prefix for the others.
    Plain(String),
    /// `RIGHT-SEARCH-004` — `*` (any run) / `?` (exactly one) wildcards, matched WHOLE-STRING.
    Glob(GlobPattern),
    /// `RIGHT-SEARCH-005` — `/…/` regex, matched as an unanchored SEARCH (use `^`/`$` to anchor).
    Regex(Rx),
    /// A `/…/` body this engine cannot parse. Matches nothing and says so — silently falling back
    /// to a literal search for the regex text would hide the typo behind an empty tree.
    Invalid,
}

/// A parsed query: which field, matched how.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchQuery {
    pub field: SearchField,
    pub pattern: SearchPattern,
}

/// The operator table. Adding an operator is one row here plus one arm in [`filter_catalog`].
///
/// `mod ` (space) is accepted alongside `mod:` because the parity sweep names the Eden token with a
/// trailing space; the colon form is the one the placeholder advertises, and both are LEADING tokens
/// so a label search for "mod" or "classy" is untouched. Order matters only in that no token is a
/// prefix of another.
const OPERATORS: &[(&str, SearchField)] = &[
    ("class:", SearchField::ClassName),
    ("mod:", SearchField::Mod),
    ("mod ", SearchField::Mod),
];

/// Parse a raw search box string into [`SearchQuery`].
///
/// Operator recognition is LEADING-TOKEN only and case-insensitive: `CLASS: B_Soldier` is the
/// operator, `classy` and `first class:` are plain label queries (Eden's rule). The pattern half is
/// then read off the operand: `/…/` ⇒ regex, otherwise any `*`/`?` ⇒ glob, otherwise a literal.
#[must_use]
pub fn parse_search_query(query: &str) -> SearchQuery {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return SearchQuery {
            field: SearchField::Label,
            pattern: SearchPattern::All,
        };
    }
    let (field, body) = OPERATORS
        .iter()
        .find_map(|(tok, field)| strip_prefix_ci(trimmed, tok).map(|rest| (*field, rest)))
        .unwrap_or((SearchField::Label, trimmed));
    SearchQuery {
        field,
        pattern: parse_search_pattern(body.trim()),
    }
}

/// The pattern half of the grammar. `body` is already operator-stripped and trimmed.
fn parse_search_pattern(body: &str) -> SearchPattern {
    if body.is_empty() {
        return SearchPattern::Pending;
    }
    if let Some(inner) = regex_body(body) {
        if inner.is_empty() {
            // `//` — the slashes are typed, the pattern is not. Mid-type, like a bare `class:`.
            return SearchPattern::Pending;
        }
        return Rx::parse(inner).map_or(SearchPattern::Invalid, SearchPattern::Regex);
    }
    if body.contains('*') || body.contains('?') {
        return SearchPattern::Glob(GlobPattern::parse(body));
    }
    SearchPattern::Plain(body.to_lowercase())
}

/// The inside of a `/…/` literal, or `None` when `body` is not one. A lone `/` is NOT a regex (it is
/// a path fragment an author may well be searching for), so two delimiters are required.
fn regex_body(body: &str) -> Option<&str> {
    let b = body.as_bytes();
    (b.len() >= 2 && b[0] == b'/' && b[b.len() - 1] == b'/').then(|| &body[1..body.len() - 1])
}

impl SearchPattern {
    /// Does this pattern match `hay`? `prefix` picks the [`SearchPattern::Plain`] rule — PREFIX for
    /// the `class:`/`mod:` fields, SUBSTRING for the historical label search. Glob and regex ignore
    /// it: a glob is whole-string by definition and a regex carries its own anchors.
    fn hits(&self, hay: &str, prefix: bool) -> bool {
        match self {
            SearchPattern::Plain(q) => {
                let lower = hay.to_lowercase();
                if prefix {
                    lower.starts_with(q.as_str())
                } else {
                    lower.contains(q.as_str())
                }
            }
            SearchPattern::Glob(g) => g.matches(hay),
            SearchPattern::Regex(r) => r.is_match(hay),
            SearchPattern::All | SearchPattern::Pending | SearchPattern::Invalid => false,
        }
    }
}

/// The empty-state line the dock shows when `filter_catalog(query)` came back empty.
///
/// `noun` is the tab's word (`"assets"` / `"objects"` / `"vehicles"`). A half-typed operator or a
/// broken regex is NOT a failed search, and saying "No assets match." for either is the lie this
/// function exists to prevent — the author would read a syntax mistake as "the catalogue has
/// nothing", which is precisely the silent-empty-tree failure this ticket was opened on.
#[must_use]
pub fn search_empty_message(query: &str, noun: &str) -> String {
    let q = parse_search_query(query);
    match (q.field, &q.pattern) {
        (SearchField::ClassName, SearchPattern::Pending) => {
            "Type a class name after class:".to_string()
        }
        (SearchField::Mod, SearchPattern::Pending) => "Type a mod name after mod:".to_string(),
        // Label + Pending is `//` — the slashes without a pattern between them.
        (_, SearchPattern::Pending) => "Type a pattern between the slashes.".to_string(),
        (_, SearchPattern::Invalid) => {
            "That /…/ pattern could not be read — check the brackets and parentheses.".to_string()
        }
        _ => format!("No {noun} match."),
    }
}

// ── RIGHT-SEARCH-004 — globs ─────────────────────────────────────────────────────────────────────

/// One token of a compiled glob.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlobTok {
    /// `*` — any run of characters, including none.
    Star,
    /// `?` — exactly one character.
    AnyOne,
    /// A literal, already Unicode-lowercased (same fold as [`GlobPattern::matches`]'s haystack).
    Ch(char),
}

/// A compiled `*`/`?` glob, matched WHOLE-STRING and case-insensitively.
///
/// Whole-string is the choice that makes the operator worth having: `US*` is "starts with", `*US*`
/// is "contains", `*.et` is "ends with". A substring-by-default glob would make `*` decorative,
/// since a bare token already substring-matches on the label field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobPattern {
    toks: Vec<GlobTok>,
}

impl GlobPattern {
    /// Compile — infallible: every character is either a wildcard or a literal, so there is no such
    /// thing as a malformed glob (unlike a regex, which is why only `/…/` has an `Invalid` arm).
    /// Consecutive `*` collapse so `***` cannot multiply the backtracking below.
    fn parse(pattern: &str) -> Self {
        let mut toks: Vec<GlobTok> = Vec::new();
        for c in pattern.chars() {
            match c {
                '*' => {
                    if toks.last() != Some(&GlobTok::Star) {
                        toks.push(GlobTok::Star);
                    }
                }
                '?' => toks.push(GlobTok::AnyOne),
                // Same fold as `matches` (`hay.to_lowercase()`). `to_ascii_lowercase` left non-ASCII
                // uppercase literals untouched (É stays É) while the haystack folded them (É → é),
                // so `CAFÉ*` missed `café_x` — wave-117 MINOR-1 / T-765. Iterate `to_lowercase()`'s
                // chars so a multi-char fold (if the stdlib ever emits one) stays token-aligned
                // with the haystack; Rust today maps ß → ß (not ss), so that case is one Ch.
                _ => {
                    for lc in c.to_lowercase() {
                        toks.push(GlobTok::Ch(lc));
                    }
                }
            }
        }
        Self { toks }
    }

    /// Whole-string match. The classic single-star-backtrack walk: linear in practice and, unlike a
    /// recursive matcher, it cannot blow the wasm stack on `*a*a*a*a*…`. Every dock keystroke runs
    /// this over every node of the tree.
    fn matches(&self, hay: &str) -> bool {
        let h: Vec<char> = hay.to_lowercase().chars().collect();
        let p = &self.toks;
        let (mut i, mut j) = (0usize, 0usize);
        // Where to resume if the current `*` guess turns out to be too short.
        let mut star: Option<usize> = None;
        let mut mark = 0usize;
        while i < h.len() {
            match p.get(j) {
                Some(GlobTok::Ch(c)) if *c == h[i] => {
                    i += 1;
                    j += 1;
                }
                Some(GlobTok::AnyOne) => {
                    i += 1;
                    j += 1;
                }
                Some(GlobTok::Star) => {
                    star = Some(j);
                    mark = i;
                    j += 1;
                }
                _ => match star {
                    Some(s) => {
                        j = s + 1;
                        mark += 1;
                        i = mark;
                    }
                    None => return false,
                },
            }
        }
        // Trailing `*`s may still consume nothing.
        while p.get(j) == Some(&GlobTok::Star) {
            j += 1;
        }
        j == p.len()
    }
}

// ── RIGHT-SEARCH-005 — the `/…/` regex subset ────────────────────────────────────────────────────
//
// Hand-rolled on purpose. The crate has no `regex` dependency and `Cargo.toml` is not this ticket's
// to edit; more to the point, `regex` is ~1.5 MB of generated DFA code in a wasm bundle whose whole
// job here is to filter a few hundred palette rows. This engine is a backtracker over a tiny AST,
// which is the right trade at this size — and it is a pure function, so it is testable without a
// browser.
//
// SUPPORTED: literals, `.`, `[abc]` / `[a-z]` / `[^…]`, `\d \D \w \W \s \S`, escapes, `(…)` groups,
// `|` alternation, greedy `*` `+` `?`, and the `^` / `$` anchors.
// NOT SUPPORTED (deliberate, and they parse as LITERALS rather than erroring): `{n,m}` counts —
// which is a feature here, because `{` is the first character of every Reforger GUID and
// `/^\{26A9/` must mean what it looks like it means. Backreferences and lazy `*?` are absent;
// a pattern using them is read greedily.

/// How many matcher steps one `is_match` may spend before giving up. A backtracker is exponential in
/// the worst case and this runs on every keystroke inside the wasm render loop, so the budget is a
/// correctness property, not a nicety: exceeding it returns "no match" instead of hanging the tab.
/// 200k steps is ~100x the cost of the worst realistic catalogue pattern.
///
/// **THIS BOUNDS WORK, NOT STACK.** See [`RX_MAX_DEPTH`] — the budget alone was the wave-117 defect.
const RX_BUDGET: u32 = 200_000;

/// How deep the matcher may recurse before it refuses. **This is a separate property from
/// [`RX_BUDGET`] and the wave-117 adversarial verifier proved the difference by building a rig:** it
/// lifted this engine verbatim onto a 1 MiB thread (the conventional wasm32 stack size) and found
/// that the step budget bounds STEPS, NOT STACK DEPTH. [`RxCtx::node`] burns one native frame per
/// step through the boxed continuations, so a deep-but-cheap input exhausts the stack long before it
/// exhausts 200k steps, and the result is not "no match" — it is a wasm trap that kills the Leptos
/// runtime and takes unsaved placements with it. Four shapes were measured to ABORT THE PROCESS:
/// `(((…)))` nesting, `^^^…`, `.?.?…`, and `(x+x+)+y` over a plain `x…` haystack — that last one
/// with a NINE-CHARACTER pattern, which is why capping pattern length cannot substitute for this.
///
/// **Why 400.** Re-measuring the abort floors on a 1 MiB thread in an unoptimised native build (the
/// most frame-hungry configuration available, ~3x hungrier than the verifier's) put the shallowest
/// at ~800 levels — the `(((…)))` shape, which spends one `node` frame per nesting level and is
/// therefore the worst stack cost per unit of depth, ~1.3 KB. 400 levels is ~525 KB, half of a
/// 1 MiB stack, with the other half left for whatever called in. Against the verifier's own
/// (cheaper-framed) numbers it is ~6x under. And it is far beyond any real query: the longest
/// `resource_name` in the shipped catalogue is 95 characters, and depth for an honest pattern grows
/// with the haystack it consumes, so a catalogue-sized subject bottoms out around 100 — 4x under
/// this bound.
///
/// Refusal takes the SAME path as an exhausted budget (`false`, i.e. no match), so the failure mode
/// is an ordinary refusal rather than a trap.
const RX_MAX_DEPTH: u32 = 400;

/// The longest `/…/` body [`Rx::parse`] will compile, in chars.
///
/// [`RX_MAX_DEPTH`] guards the MATCHER; this guards the PARSER, which is a separate recursive
/// descent ([`RxParser::alt`] → `seq` → `atom` → `alt`) that runs to completion BEFORE any matching
/// and recurses once per nested `(`. No matcher-side bound can reach it.
///
/// **Why 512 — and the arithmetic an earlier version of this comment got wrong, twice.** It claimed
/// 512 chars caps parser nesting at "~256 levels", "~3x" under the abort floor. Both numbers were
/// wrong, because both assumed two characters per level, i.e. a BALANCED `()` pair. **Nothing
/// requires a pattern to balance.** An unbalanced `(` opens a level that no `)` ever closes, so the
/// worst case is ONE PARSER LEVEL PER CHARACTER: `(`x512 is accepted by this cap (512 is not *over*
/// 512) and drives `alt` → `seq` → `atom` → `alt` **512** levels deep before the missing `)` fails
/// it. Re-measured against THIS source by bisecting `(`xN through [`RxParser::alt`] on a 1 MiB
/// thread — the vector is now pinned in
/// `deep_regex_input_refuses_instead_of_trapping_the_wasm_stack`:
///
/// | build | deepest clean | aborts at | ~cost/level | 512 levels | margin |
/// |---|---|---|---|---|---|
/// | native debug — most frame-hungry available | 790 | 795 | ~1.3 KB | ~681 KB, 65% of 1 MiB | **1.54x** |
/// | native release — proxy for the shipped wasm | 2500 | 3000 | ~0.4 KB | ~215 KB, 21% of 1 MiB | **4.9x** |
///
/// So the honest margin is **1.54x**, not 3x — and it is 1.54x against a deliberately pessimistic
/// proxy. 512 is kept because it is CLEAN IN EVERY CONFIGURATION MEASURED (there is no build in
/// which a pattern this cap admits traps), because the configuration that actually ships is release
/// wasm, where it spends a fifth of the stack, and because it is still 5x the longest
/// `resource_name` in the catalogue (95 chars) and so cannot narrow an honest query.
///
/// **What the corrected arithmetic does change:** at 512 levels the PARSER, not the matcher, is
/// this engine's worst-case stack consumer — ~681 KB against [`RX_MAX_DEPTH`]'s ~525 KB. The two do
/// not add (the parser runs to completion and returns before the matcher starts, so the peak is the
/// larger of the two) but the peak is now this one, and in an unoptimised build it is two thirds of
/// a 1 MiB stack, tighter than the half-the-stack standard [`RX_MAX_DEPTH`] was sized to.
/// **Do not size a third bound off these ratios — re-measure.** The debug:release frame cost here
/// is ~3.2x (790 vs 2500); that is a property of this build, not a constant.
///
/// This is a belt to the depth cap's braces, NOT a replacement for it: `(x+x+)+y` overflows on a
/// long HAYSTACK with a nine-character pattern, which no input-length rule can see.
const RX_MAX_PATTERN: usize = 512;

/// One item inside a `[…]` class.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ClassItem {
    Ch(char),
    Range(char, char),
    Digit,
    NotDigit,
    Word,
    NotWord,
    Space,
    NotSpace,
}

/// One node of the regex AST.
#[derive(Clone, Debug, PartialEq, Eq)]
enum RxNode {
    Ch(char),
    Any,
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    Group(RxAlt),
    Repeat {
        node: Box<RxNode>,
        min: usize,
        max: Option<usize>,
    },
    Start,
    End,
}

/// A `|`-separated list of sequences — the top level of a pattern and of every group.
#[derive(Clone, Debug, PartialEq, Eq)]
struct RxAlt(Vec<Vec<RxNode>>);

/// A compiled `/…/` pattern. Matched as an unanchored SEARCH — `^`/`$` anchor it explicitly, which
/// is what an author who typed slashes expects, and what makes `/rifleman/` a usable
/// "contains" over a GUID-headed classname.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rx {
    alt: RxAlt,
}

/// Recursive-descent parser over the pattern's chars.
struct RxParser<'a> {
    src: &'a [char],
    pos: usize,
}

impl RxParser<'_> {
    fn peek(&self) -> Option<char> {
        self.src.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    /// `alt := seq ('|' seq)*`
    fn alt(&mut self) -> Option<RxAlt> {
        let mut branches = vec![self.seq()?];
        while self.peek() == Some('|') {
            self.pos += 1;
            branches.push(self.seq()?);
        }
        Some(RxAlt(branches))
    }

    /// `seq := (atom quantifier?)*` — stops at `|`, `)` or end of input.
    fn seq(&mut self) -> Option<Vec<RxNode>> {
        let mut out = Vec::new();
        loop {
            match self.peek() {
                None | Some('|') | Some(')') => break,
                _ => {}
            }
            let atom = self.atom()?;
            out.push(self.quantified(atom));
        }
        Some(out)
    }

    /// Wrap `node` in whatever greedy quantifiers follow it. Stacking (`a*?`) is accepted and read
    /// as a repeat of a repeat rather than rejected — this engine has no lazy quantifiers to confuse
    /// it with.
    fn quantified(&mut self, node: RxNode) -> RxNode {
        let mut node = node;
        loop {
            let (min, max) = match self.peek() {
                Some('*') => (0, None),
                Some('+') => (1, None),
                Some('?') => (0, Some(1)),
                _ => return node,
            };
            self.pos += 1;
            node = RxNode::Repeat {
                node: Box::new(node),
                min,
                max,
            };
        }
    }

    fn atom(&mut self) -> Option<RxNode> {
        match self.next()? {
            '(' => {
                let alt = self.alt()?;
                (self.next() == Some(')')).then_some(RxNode::Group(alt))
            }
            '[' => self.class(),
            '.' => Some(RxNode::Any),
            '^' => Some(RxNode::Start),
            '$' => Some(RxNode::End),
            '\\' => self.escape(),
            // A quantifier with nothing to repeat is the one shape that must NOT be read as a
            // literal: `/*foo/` is a glob typed into regex slashes, and answering it with a literal
            // `*` would be a silent wrong answer.
            '*' | '+' | '?' => None,
            c => Some(RxNode::Ch(c.to_ascii_lowercase())),
        }
    }

    /// `\d` and friends outside a class; anything else escapes to itself (so `\.`, `\/`, `\\`).
    fn escape(&mut self) -> Option<RxNode> {
        let c = self.next()?;
        Some(match class_shorthand(c) {
            Some(item) => RxNode::Class {
                negated: false,
                items: vec![item],
            },
            None => RxNode::Ch(c.to_ascii_lowercase()),
        })
    }

    /// `[…]` — the opening bracket is already consumed. `]` first is a literal `]` (POSIX rule).
    fn class(&mut self) -> Option<RxNode> {
        let negated = self.peek() == Some('^');
        if negated {
            self.pos += 1;
        }
        let mut items = Vec::new();
        loop {
            let c = self.next()?; // unterminated class ⇒ None ⇒ Invalid
            if c == ']' && !items.is_empty() {
                return Some(RxNode::Class { negated, items });
            }
            let lo = if c == '\\' {
                let e = self.next()?;
                if let Some(item) = class_shorthand(e) {
                    items.push(item);
                    continue;
                }
                e
            } else {
                c
            };
            // `a-z`, but a trailing `-` before `]` is a literal dash.
            if self.peek() == Some('-') && self.src.get(self.pos + 1).is_some_and(|n| *n != ']') {
                self.pos += 1;
                let hi = self.next()?;
                items.push(ClassItem::Range(
                    lo.to_ascii_lowercase(),
                    hi.to_ascii_lowercase(),
                ));
            } else {
                items.push(ClassItem::Ch(lo));
            }
        }
    }
}

/// `\d \D \w \W \s \S` → a class item; anything else is not a shorthand.
fn class_shorthand(c: char) -> Option<ClassItem> {
    Some(match c {
        'd' => ClassItem::Digit,
        'D' => ClassItem::NotDigit,
        'w' => ClassItem::Word,
        'W' => ClassItem::NotWord,
        's' => ClassItem::Space,
        'S' => ClassItem::NotSpace,
        _ => return None,
    })
}

/// ASCII-case-insensitive char equality (the subject is already lowercased, so this only has to
/// forgive a pattern literal the parser could not fold, e.g. a non-ASCII one).
fn eq_ci(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

fn class_hit(items: &[ClassItem], h: char) -> bool {
    items.iter().any(|it| match it {
        ClassItem::Ch(c) => eq_ci(*c, h),
        // The subject is lowercased before matching, so `[A-Z]` would otherwise never fire; test the
        // uppercase form too, which makes classes case-insensitive like the rest of the grammar.
        ClassItem::Range(a, b) => {
            (*a..=*b).contains(&h) || (*a..=*b).contains(&h.to_ascii_uppercase())
        }
        ClassItem::Digit => h.is_ascii_digit(),
        ClassItem::NotDigit => !h.is_ascii_digit(),
        ClassItem::Word => h.is_alphanumeric() || h == '_',
        ClassItem::NotWord => !(h.is_alphanumeric() || h == '_'),
        ClassItem::Space => h.is_whitespace(),
        ClassItem::NotSpace => !h.is_whitespace(),
    })
}

/// The matcher's continuation: "given that this node matched up to `pos`, can the REST match?".
/// Continuation passing is what lets `(ab|a)b` backtrack across a group boundary — the group cannot
/// know how much of the string the rest of the pattern will need.
type RxCont<'a> = &'a dyn Fn(usize) -> bool;

struct RxCtx<'h> {
    hay: &'h [char],
    budget: std::cell::Cell<u32>,
    /// Live recursion depth — how many [`RxCtx::node`] frames are currently on the stack. Kept
    /// alongside `budget` rather than folded into it because they measure different things: a
    /// pattern can be cheap in steps and fatal in depth (`^^^…`), or expensive in steps and flat
    /// (`(a|aa)+c` over a short subject).
    depth: std::cell::Cell<u32>,
    /// Latched when [`RX_MAX_DEPTH`] actually refused a branch. Nothing in production reads it —
    /// it exists so a test can tell "returned false because it hit the depth cap" apart from
    /// "returned false because the pattern honestly did not match", which is the difference between
    /// proving the cap and proving nothing.
    depth_capped: std::cell::Cell<bool>,
}

impl RxCtx<'_> {
    /// Spend one unit of [`RX_BUDGET`]; `false` once it is gone, which unwinds every branch.
    fn step(&self) -> bool {
        let b = self.budget.get();
        if b == 0 {
            return false;
        }
        self.budget.set(b - 1);
        true
    }

    fn alt(&self, a: &RxAlt, pos: usize, k: RxCont) -> bool {
        self.step() && a.0.iter().any(|s| self.seq(s, pos, k))
    }

    fn seq(&self, s: &[RxNode], pos: usize, k: RxCont) -> bool {
        match s.split_first() {
            None => k(pos),
            Some((n, rest)) => self.node(n, pos, &|p| self.seq(rest, p, k)),
        }
    }

    /// Every recursive cycle in this matcher passes through here — `alt`→`seq`→`node`,
    /// `node`→`alt` (a group), `node`→`repeat`→`node`, and `node`→`k`→`seq`→`node` (a continuation)
    /// all re-enter `node` — so `node` is the one cut point where a depth bound catches all of
    /// them. That is why the counter lives here and not in [`RxCtx::step`], which is called on
    /// paths that do not recurse and would over-count.
    fn node(&self, n: &RxNode, pos: usize, k: RxCont) -> bool {
        if !self.step() {
            return false;
        }
        let d = self.depth.get();
        if d >= RX_MAX_DEPTH {
            self.depth_capped.set(true);
            // The same `false` an exhausted budget returns: refuse this branch, let the caller
            // backtrack, and end at "no match" — never at a trap.
            return false;
        }
        self.depth.set(d + 1);
        let hit = self.node_inner(n, pos, k);
        // `d`, not `d - 1`: restoring the value we entered with is correct even if a branch below
        // unwound early.
        self.depth.set(d);
        hit
    }

    /// The body of [`RxCtx::node`], split out only so the depth counter has a single unmissable
    /// restore point instead of one before every `return`.
    fn node_inner(&self, n: &RxNode, pos: usize, k: RxCont) -> bool {
        match n {
            RxNode::Start => pos == 0 && k(pos),
            RxNode::End => pos == self.hay.len() && k(pos),
            RxNode::Ch(c) => self.hay.get(pos).is_some_and(|h| eq_ci(*h, *c)) && k(pos + 1),
            RxNode::Any => pos < self.hay.len() && k(pos + 1),
            RxNode::Class { negated, items } => {
                self.hay
                    .get(pos)
                    .is_some_and(|h| class_hit(items, *h) != *negated)
                    && k(pos + 1)
            }
            RxNode::Group(a) => self.alt(a, pos, k),
            RxNode::Repeat { node, min, max } => self.repeat(node, *min, *max, pos, 0, k),
        }
    }

    /// Greedy repeat: try one more iteration before handing control to the continuation.
    fn repeat(
        &self,
        node: &RxNode,
        min: usize,
        max: Option<usize>,
        pos: usize,
        count: usize,
        k: RxCont,
    ) -> bool {
        if !self.step() {
            return false;
        }
        if max.is_none_or(|m| count < m)
            && self.node(node, pos, &|p| {
                if p == pos {
                    // A zero-width iteration (`(a?)*`): counting it again forever is the classic
                    // hang, so credit it once and move on.
                    count + 1 >= min && k(p)
                } else {
                    self.repeat(node, min, max, p, count + 1, k)
                }
            })
        {
            return true;
        }
        count >= min && k(pos)
    }
}

impl Rx {
    /// Compile, or `None` for a body this subset cannot read (unbalanced `(`/`[`, a dangling
    /// quantifier). `None` becomes [`SearchPattern::Invalid`], which the dock reports.
    fn parse(pattern: &str) -> Option<Self> {
        let src: Vec<char> = pattern.chars().collect();
        // Length first, BEFORE the recursive descent below — [`RX_MAX_PATTERN`] exists precisely
        // because `RxParser::alt` recurses once per nested `(` and would overflow the wasm stack
        // while building the AST, i.e. before the matcher's depth cap could ever be consulted.
        if src.len() > RX_MAX_PATTERN {
            return None;
        }
        let mut p = RxParser { src: &src, pos: 0 };
        let alt = p.alt()?;
        // Trailing input means the parse stopped at an unmatched `)`.
        (p.pos == src.len()).then_some(Self { alt })
    }

    /// Unanchored search, case-insensitive.
    fn is_match(&self, hay: &str) -> bool {
        self.search(hay).0
    }

    /// `(matched, depth_capped)`. The second field is the evidence half: it is `true` only if
    /// [`RX_MAX_DEPTH`] actually turned a branch away, so a test can prove its input reached the cap
    /// rather than failing shallowly for some unrelated reason. [`Rx::is_match`] discards it.
    fn search(&self, hay: &str) -> (bool, bool) {
        let h: Vec<char> = hay.to_lowercase().chars().collect();
        let ctx = RxCtx {
            hay: &h,
            budget: std::cell::Cell::new(RX_BUDGET),
            depth: std::cell::Cell::new(0),
            depth_capped: std::cell::Cell::new(false),
        };
        // `..=len` so `/x$/`-shaped patterns can match at the very end, and `//`-empty cannot get
        // here (it is `Pending`).
        let hit = (0..=h.len()).any(|start| ctx.alt(&self.alt, start, &|_| true));
        (hit, ctx.depth_capped.get())
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
/// T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar ([`parse_search_query`]) runs FIRST and picks
/// both the field and the matcher; the three fields keep three different TREE rules, because the
/// three fields live at three different depths:
///
/// * `Label` — unchanged from T-055: folder self-match keeps the whole subtree, otherwise a folder
///   survives on descendants with only the matching children.
/// * `ClassName` — LEAF-ONLY (a folder has no classname), folders survive on descendants. T-646's
///   rule, now matching the [`classname_tail`] as well as the full `resource_name`.
/// * `Mod` — DEPTH-0 ONLY. The addon is the root of the category path, so "this mod" is exactly
///   "this root folder", and a hit keeps the root's whole subtree. Recursing would be wrong, not
///   merely slower: `mod:ArmaReforger` must not prune a vanilla vehicle out of a vanilla addon
///   because the leaf itself is spelled differently.
#[must_use]
pub fn filter_catalog(nodes: &[CatalogNode], query: &str) -> Vec<CatalogNode> {
    let q = parse_search_query(query);
    // `All` is the empty query (identity); `Pending`/`Invalid` are half-typed or broken and match
    // nothing — the dock reads `search_empty_message` to say which.
    match q.pattern {
        SearchPattern::All => return nodes.to_vec(),
        SearchPattern::Pending | SearchPattern::Invalid => return Vec::new(),
        _ => {}
    }
    let p = &q.pattern;
    match q.field {
        SearchField::Label => {
            fn keep(node: &CatalogNode, p: &SearchPattern) -> Option<CatalogNode> {
                if p.hits(&node.label, false) {
                    return Some(node.clone()); // self-match → full subtree
                }
                let children: Vec<CatalogNode> =
                    node.children.iter().filter_map(|c| keep(c, p)).collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, p)).collect()
        }
        SearchField::ClassName => {
            fn keep(node: &CatalogNode, p: &SearchPattern) -> Option<CatalogNode> {
                if node.payload.is_some() {
                    // Leaf: the full `resource_name` OR its classname tail (see `classname_tail` —
                    // the tail arm is what makes a bare classname reachable at all).
                    let id = &node.id;
                    return (p.hits(id, true) || p.hits(classname_tail(id), true))
                        .then(|| node.clone());
                }
                let children: Vec<CatalogNode> =
                    node.children.iter().filter_map(|c| keep(c, p)).collect();
                if children.is_empty() {
                    return None;
                }
                let mut out = node.clone();
                out.children = children;
                Some(out)
            }
            nodes.iter().filter_map(|n| keep(n, p)).collect()
        }
        SearchField::Mod => nodes
            .iter()
            .filter(|n| p.hits(&n.label, true))
            .cloned()
            .collect(),
    }
}

// ── T-695 (NEW-F5 / 3den E3) — resolving a starred asset id back to the live catalogue ───────────
//
// The right dock's Favourites collection persists ASSET IDS, not catalog rows: an id is the full
// Enfusion `resource_name`, which is exactly what a leaf's `CatalogNode::id` and
// `PlacePayload::asset_id` already carry (module rule 4). Turning one back into something the dock
// can render needs two facts the three tree builders above already encode but never expose — does
// the row still EXIST, and is it still PLACEABLE. Both live here, beside the filters they mirror,
// rather than being re-derived in the view.
//
// Deliberately ADDITIVE: the builders and the search grammar are untouched (T-084 rewrites the
// grammar in this file three waves out, and a restructure now would collide with it).

/// T-695 — which of the three placeable palettes a live registry row belongs to. The dock's
/// `PaletteKind` is the view-side vocabulary (it also has non-catalog arms for compositions and
/// triggers); this is the CATALOG-side subset, so the resolution can stay pure and native-testable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogPalette {
    Character,
    Vehicle,
    Object,
}

/// T-695 — the live catalogue row for an asset id (`resource_name`), or `None` when the id is not in
/// the registry at all: a modpack switched off, a prefab renamed between sessions, or a hand-edited
/// persisted blob. `None` is the honest answer the dock renders its "not in the current catalogue"
/// row from — it is never a reason to silently drop the operator's starred entry.
#[must_use]
pub fn find_catalog_item<'a>(
    items: &'a [RegistryItem],
    asset_id: &str,
) -> Option<&'a RegistryItem> {
    items.iter().find(|i| i.resource_name == asset_id)
}

/// T-695 — which palette a live row PLACES through, or `None` when the row exists but no palette
/// offers it (so starring it can no longer arm a place). The three arms mirror the builders above
/// exactly, and the mirroring is the point — a favourite that resolved "placeable" through a laxer
/// rule than the tree used would arm a place the palette itself refuses to offer:
///
/// * `character` — [`build_catalog_tree`] applies no `abstract` filter, so neither does this.
/// * `vehicle` — [`build_vehicle_catalog_tree`] drops `abstract` (`*_base.et`) rows.
/// * object kinds — [`build_object_catalog_tree`] drops `abstract` rows AND rows whose alias is not
///   in the mod spawn registry (T-439), because the mod would warn-skip them.
///
/// The Eden **side** filter ([`character_matches_eden_side`]) is deliberately NOT applied: it is a
/// per-chip VIEW filter over one tab, and a favourites collection spans the whole catalogue. A
/// BLUFOR role starred while the OPFOR chip is up is live, not stale.
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
