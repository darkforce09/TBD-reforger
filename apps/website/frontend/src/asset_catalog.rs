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

/// The CLASSNAME TAIL of an Enfusion `resource_name`: the last path segment with its extension
/// dropped — `{26A9756790131354}Prefabs/…/Character_US_Rifleman.et` → `Character_US_Rifleman`.
///
/// **THE WAVE-105 MINOR-2 DECISION.** T-646 matched `class:` against full `resource_name` PREFIXES
/// only. Reforger resource names are GUID-headed, so a bare classname — the thing an author actually
/// knows and types — could never prefix-match, and `class:Character_US_Rifleman` SILENTLY EMPTIED
/// THE TREE. That is the defect: a query the author reasonably expects to work returning nothing,
/// with no way to tell a miss from a broken operator.
///
/// The decision: **`class:` matches the full `resource_name` prefix OR the classname-tail prefix.**
/// Both, not either/or, so
/// * `class:{26A9756790131354}Prefabs` — T-646's GUID-path prefix — still works, and
/// * `class:Character_US_Ri` — a bare classname — now works.
///
/// Tail matching stays a PREFIX, not a substring: `class:Rifleman` against
/// `Character_US_Rifleman` is still a miss, because a substring `class:` would collapse into the
/// label search it exists to be different from. A mid-classname token has its own spelling in this
/// grammar now — `class:*Rifleman` (glob) or `class:/rifleman/` (regex) — which is exactly why the
/// tail rule can afford to stay strict.
#[must_use]
pub fn classname_tail(id: &str) -> &str {
    let seg = id.rsplit('/').next().unwrap_or(id);
    match seg.rfind('.') {
        // `i > 0` keeps a dotfile-shaped segment whole rather than yielding "".
        Some(i) if i > 0 => &seg[..i],
        _ => seg,
    }
}

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
    /// A literal, already ASCII-lowercased.
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
                _ => toks.push(GlobTok::Ch(c.to_ascii_lowercase())),
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
const RX_BUDGET: u32 = 200_000;

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

    fn node(&self, n: &RxNode, pos: usize, k: RxCont) -> bool {
        if !self.step() {
            return false;
        }
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
        let mut p = RxParser { src: &src, pos: 0 };
        let alt = p.alt()?;
        // Trailing input means the parse stopped at an unmatched `)`.
        (p.pos == src.len()).then_some(Self { alt })
    }

    /// Unanchored search, case-insensitive.
    fn is_match(&self, hay: &str) -> bool {
        let h: Vec<char> = hay.to_lowercase().chars().collect();
        let ctx = RxCtx {
            hay: &h,
            budget: std::cell::Cell::new(RX_BUDGET),
        };
        // `..=len` so `/x$/`-shaped patterns can match at the very end, and `//`-empty cannot get
        // here (it is `Pending`).
        (0..=h.len()).any(|start| ctx.alt(&self.alt, start, &|_| true))
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
    ///
    /// T-084 reshaped [`SearchQuery`] from a flat enum into `{field, pattern}` (the operator and the
    /// pattern are now independent axes), so these assertions read through the new pair. Every
    /// BEHAVIOUR T-646 pinned is asserted unchanged.
    #[test]
    fn parse_search_query_recognises_class_operator() {
        assert_eq!(
            parse_search_query("class:B_Soldier"),
            SearchQuery {
                field: SearchField::ClassName,
                pattern: SearchPattern::Plain("b_soldier".to_string()),
            },
            "class: → lowercased operand"
        );
        assert_eq!(
            parse_search_query("  CLASS: B_Soldier "),
            SearchQuery {
                field: SearchField::ClassName,
                pattern: SearchPattern::Plain("b_soldier".to_string()),
            },
            "operator is case-insensitive; leading/trailing space trimmed"
        );
        assert_eq!(
            parse_search_query("class:"),
            SearchQuery {
                field: SearchField::ClassName,
                pattern: SearchPattern::Pending,
            },
            "empty operand is recognised (and filters to nothing)"
        );
        // Not the operator: a bare word, or `class:` appearing past the start.
        assert_eq!(
            parse_search_query("rifleman"),
            SearchQuery {
                field: SearchField::Label,
                pattern: SearchPattern::Plain("rifleman".to_string()),
            }
        );
        assert_eq!(
            parse_search_query("classy"),
            SearchQuery {
                field: SearchField::Label,
                pattern: SearchPattern::Plain("classy".to_string()),
            },
            "`class` without the colon is a label"
        );
        assert_eq!(
            parse_search_query("first class:"),
            SearchQuery {
                field: SearchField::Label,
                pattern: SearchPattern::Plain("first class:".to_string()),
            },
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

    // ── T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar ───────────────────────────────────────

    /// **THE DECISION THIS TICKET EXISTS TO MAKE** (wave-105 MINOR-2), pinned against a REAL
    /// GUID-headed id from the committed catalogue —
    /// `{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et`.
    ///
    /// T-646's `class:` was a prefix over the WHOLE `resource_name`, and every Reforger resource
    /// name starts with a GUID the author has never seen. So `class:<bare classname>` — the only
    /// spelling an author actually knows — matched nothing and the tree silently emptied. This test
    /// fires exactly there: it asserts the bare classname now HITS, and asserts (right beside it)
    /// that T-646's GUID-path prefix still hits, because the fix is an OR, not a replacement.
    #[test]
    fn class_tail_matches_a_bare_classname_on_a_real_guid_headed_id() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        let rifleman_id =
            "{26A9756790131354}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Rifleman.et";
        // Guard: this really is the shipped id, GUID head and `.et` tail included.
        assert_eq!(tree[0].children[0].children[0].id, rifleman_id);
        assert_eq!(
            classname_tail(rifleman_id),
            "Character_US_Rifleman",
            "the tail is the last path segment minus the extension"
        );

        // THE DEFECT, FIXED — a bare classname. Under T-646 this returned an EMPTY TREE.
        let bare = filter_catalog(&tree, "class:Character_US_Rifleman");
        assert_eq!(bare.len(), 1, "a bare classname must not empty the tree");
        assert_eq!(bare[0].children[0].children.len(), 1);
        assert_eq!(bare[0].children[0].children[0].id, rifleman_id);

        // A PARTIAL bare classname, as typed keystroke by keystroke.
        let partial = filter_catalog(&tree, "class:character_us_ri");
        assert_eq!(
            partial[0].children[0].children[0].id, rifleman_id,
            "tail matching is a prefix, and case-insensitive"
        );

        // T-646 UNREGRESSED — the full `resource_name` prefix still selects the same leaf.
        let guid = filter_catalog(&tree, "class:{26A9756790131354}Prefabs");
        assert_eq!(guid[0].children[0].children[0].id, rifleman_id);

        // The tail rule is a PREFIX, deliberately: a mid-classname token is still a miss…
        assert!(
            filter_catalog(&tree, "class:Rifleman").is_empty(),
            "tail matching is prefix-only, not substring"
        );
        // …and the grammar gives that query its own spelling instead.
        let globbed = filter_catalog(&tree, "class:*Rifleman");
        assert_eq!(
            globbed[0].children[0].children[0].id, rifleman_id,
            "a mid-classname token is reachable as a glob"
        );
    }

    /// RIGHT-SEARCH-003 — `mod:` filters by the ADDON, which in this catalogue is the root of the
    /// category path and therefore the tree's depth-0 folder (`ArmaReforger/Vehicles/Wheeled/…`).
    /// A hit keeps the root's whole subtree; a miss empties the tree. Both spellings of the operator
    /// (`mod:` and the parity table's `mod `) are the same operator.
    #[test]
    fn mod_operator_filters_by_the_addon_root() {
        let tree = build_vehicle_catalog_tree(&vehicle_items());
        assert_eq!(
            tree[0].label, "ArmaReforger",
            "guard: the root is the addon"
        );
        let all_leaves = tree[0].children[0].children.len();

        let hit = filter_catalog(&tree, "mod:ArmaReforger");
        assert_eq!(hit, tree, "an addon hit keeps the addon's whole subtree");

        assert_eq!(
            filter_catalog(&tree, "mod:arma"),
            tree,
            "`mod:` is a PREFIX and case-insensitive"
        );
        assert_eq!(
            filter_catalog(&tree, "mod ArmaReforger"),
            tree,
            "the space-separated spelling is the same operator"
        );
        assert!(
            filter_catalog(&tree, "mod:TBD_Framework").is_empty(),
            "an addon that is not in the tree empties it"
        );
        // `mod:` is NOT a label search: `Wheeled` is a real folder, one level down, and must not
        // survive a mod query — otherwise the operator would just be a slower label match.
        assert!(
            filter_catalog(&tree, "mod:Wheeled").is_empty(),
            "mod: matches the addon root only, not any folder"
        );
        // Sanity: the label search over the same token still finds it.
        assert!(
            !filter_catalog(&tree, "Wheeled").is_empty(),
            "guard: `Wheeled` is a real folder the label search finds"
        );
        assert!(all_leaves > 0, "guard: the fixture tree has leaves");
    }

    /// RIGHT-SEARCH-004 — `*` (any run) and `?` (exactly one) matched WHOLE-STRING, over whichever
    /// field the operator picked. Whole-string is the point: `US*` is starts-with, `*Medic` is
    /// ends-with, `*ri*` is contains — three behaviours a bare token cannot express.
    #[test]
    fn glob_patterns_are_whole_string_over_the_selected_field() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        let leaves = |t: &[CatalogNode]| -> Vec<String> {
            t.first().map_or_else(Vec::new, |root| {
                root.children[0]
                    .children
                    .iter()
                    .map(|n| n.label.clone())
                    .collect()
            })
        };

        assert_eq!(
            leaves(&filter_catalog(&tree, "*Medic")),
            ["US Medic"],
            "ends-with"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, "?S Medic")),
            ["US Medic"],
            "? is exactly one character"
        );
        assert!(
            filter_catalog(&tree, "?S Medicc").is_empty(),
            "the glob is whole-string: a trailing character it cannot consume is a miss"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, "*Anti-Tank")),
            ["US Light Anti-Tank"],
            "a glob spans spaces and punctuation"
        );

        // Crossed with `class:` — the same pattern engine, a different field. The tail arm makes
        // `*_MG` work without spelling out the GUID.
        let mg = filter_catalog(&tree, "class:*_MG");
        assert_eq!(leaves(&mg), ["US Machine Gunner"], "glob over the tail");
        let et = filter_catalog(&tree, "class:*Character_US_Medic.et");
        assert_eq!(
            leaves(&et),
            ["US Medic"],
            "glob over the full resource_name"
        );

        // Crossed with `mod:`.
        let vehicles = build_vehicle_catalog_tree(&vehicle_items());
        assert_eq!(
            filter_catalog(&vehicles, "mod:Arma*"),
            vehicles,
            "glob over the addon root"
        );
    }

    /// RIGHT-SEARCH-005 — `/…/` is a regex over the selected field, unanchored (so `^`/`$` mean
    /// something), case-insensitive, with alternation, classes and quantifiers. `{` is a LITERAL in
    /// this subset — deliberately, because it opens every Reforger GUID.
    #[test]
    fn regex_patterns_search_the_selected_field() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        let leaves = |t: &[CatalogNode]| -> Vec<String> {
            t.first().map_or_else(Vec::new, |root| {
                root.children[0]
                    .children
                    .iter()
                    .map(|n| n.label.clone())
                    .collect()
            })
        };

        assert_eq!(
            leaves(&filter_catalog(&tree, "/^us (medic|engineer)$/")),
            ["US Medic", "US Engineer"],
            "alternation + anchors over the label"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, "/anti-tank/")),
            ["US Light Anti-Tank"],
            "an un-anchored regex is a search, not a full match"
        );
        assert!(
            filter_catalog(&tree, "/^medic$/").is_empty(),
            "^ anchors: no label IS 'medic'"
        );

        // Over the classname, where a regex earns its keep: `{` and `}` are literals, escaped or not.
        assert_eq!(
            leaves(&filter_catalog(&tree, r"class:/^\{26a9756790131354\}/")),
            ["US Rifleman"],
            "an escaped GUID head"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, "class:/^{26A9756790131354}/")),
            ["US Rifleman"],
            "and an unescaped one — an open brace is a literal in this subset"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, r"class:/character_us_(mg|ar)\.et$/")),
            ["US Automatic Rifleman", "US Machine Gunner"],
            "alternation over the classname, in the tree's own order"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, r"class:/us_[a-z]{2}\.et$/")),
            Vec::<String>::new(),
            "a brace count is NOT a repetition in this subset — it is a literal, so this misses"
        );
        assert_eq!(
            leaves(&filter_catalog(&tree, r"class:/us_[a-l]+\.et$/")),
            ["US Grenadier"],
            "a character range + `+`: only `…_US_GL.et` is all a-l after the underscore"
        );
        // `\d` over the real GUID heads: 7 of the 8 shipped BLUFOR ids open with a digit, the
        // Medic's `{C9E4FEAF…}` does not — a discriminator only real data provides.
        let digit_guid = filter_catalog(&tree, r"class:/^.\d/");
        assert_eq!(digit_guid[0].children[0].children.len(), 7);
        assert!(
            !leaves(&digit_guid).contains(&"US Medic".to_string()),
            "the one GUID that starts with a letter is excluded"
        );

        // Crossed with `mod:`.
        let vehicles = build_vehicle_catalog_tree(&vehicle_items());
        assert_eq!(
            filter_catalog(&vehicles, "mod:/^arma(reforger|3)$/"),
            vehicles,
            "regex over the addon root"
        );
    }

    /// A `/…/` body this engine cannot read is reported, not silently answered. Falling back to a
    /// literal search for the regex text is the failure mode this ticket was opened on: an empty
    /// tree that reads like "nothing matches" when the truth is "your pattern has a typo".
    #[test]
    fn a_broken_regex_says_so_instead_of_emptying_silently() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        for broken in ["/us(/", "/[a-/", "/*us/", "/us)/"] {
            assert_eq!(
                parse_search_query(broken).pattern,
                SearchPattern::Invalid,
                "{broken} must not parse"
            );
            assert!(filter_catalog(&tree, broken).is_empty());
            assert_eq!(
                search_empty_message(broken, "assets"),
                "That /…/ pattern could not be read — check the brackets and parentheses.",
                "a broken pattern reads as a syntax problem, not as a miss"
            );
        }
        // A lone slash is NOT a regex — an author searching a path fragment gets a literal search.
        assert_eq!(
            parse_search_query("/").pattern,
            SearchPattern::Plain("/".to_string())
        );
    }

    /// Every operator's mid-type state says what to type next. T-646 shipped this for `class:`; the
    /// grammar generalises it, because a half-typed `mod:` or `//` is exactly as much "not a miss".
    #[test]
    fn every_operator_has_a_mid_type_empty_state() {
        let tree = build_catalog_tree(&golden_items(), "BLUFOR");
        for (q, msg) in [
            ("class:", "Type a class name after class:"),
            ("mod:", "Type a mod name after mod:"),
            ("  MOD:  ", "Type a mod name after mod:"),
            ("//", "Type a pattern between the slashes."),
        ] {
            assert!(
                filter_catalog(&tree, q).is_empty(),
                "{q} is mid-type and must not show the whole tree"
            );
            assert_eq!(search_empty_message(q, "assets"), msg);
        }
        // The genuinely empty query is still identity, not a mid-type state.
        assert_eq!(filter_catalog(&tree, "   "), tree);
    }

    /// The regex engine runs inside the wasm render loop on EVERY keystroke, so a catastrophic
    /// backtracker must terminate rather than hang the tab. `(a+)+$` over a long non-matching
    /// subject is the textbook exponential case; the step budget caps it.
    #[test]
    fn a_catastrophic_regex_terminates_on_the_step_budget() {
        let long = "a".repeat(40);
        let items = vec![character_row(
            &format!("{{Z}}Prefabs/{long}b.et"),
            &long,
            "NATO/US_Army/Long",
        )];
        let tree = build_catalog_tree(&items, "BLUFOR");
        // The answer itself is not the assertion — RETURNING is. Under an uncapped backtracker this
        // line never comes back.
        let _ = filter_catalog(&tree, "/^(a+)+$/");
        let _ = filter_catalog(&tree, "class:/(a|aa)+c/");
        // And a pattern the budget can afford still answers correctly.
        assert_eq!(filter_catalog(&tree, "/^a+$/").len(), 1);
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
            assert_eq!(
                parse_search_query(q).field,
                SearchField::Label,
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

    /// T-695 — the favourites resolution helpers, and the one claim their doc comments make: that
    /// `placeable_palette` MIRRORS the three tree builders. The pin compares the two directly —
    /// every leaf the builders offer must be placeable, and every row they reject (`abstract`
    /// vehicles, unregistered object aliases) must not be. A laxer rule here would let a favourite
    /// arm a place the palette itself refuses to offer.
    #[test]
    fn favourite_resolution_mirrors_the_palette_builders() {
        let items = object_items();

        // Lookup is by `resource_name` — the id a leaf and a `PlacePayload` both carry.
        let known = "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd.et";
        assert_eq!(
            find_catalog_item(&items, known).map(|i| i.display_name.as_str()),
            Some("AmmoBox 50cal 100rnd")
        );
        assert!(
            find_catalog_item(&items, "{NOPE}Prefabs/Gone.et").is_none(),
            "an id that left the catalogue must resolve to None, not to a neighbour"
        );

        // Objects: the registered crate is placeable; the abstract one and the unregistered one
        // are not — exactly the rows `build_object_catalog_tree` drops.
        assert_eq!(
            find_catalog_item(&items, known).and_then(placeable_palette),
            Some(CatalogPalette::Object)
        );
        for rejected in [
            "{7007B975BEC018D9}Prefabs/Props/Military/AmmoBoxes/AmmoBox_50cal_100rnd_base.et",
            "{DEADBEEFDEADBEEF}Prefabs/Props/Military/Unregistered.et",
        ] {
            assert_eq!(
                find_catalog_item(&items, rejected).and_then(placeable_palette),
                None,
                "the Objects palette drops {rejected}, so a favourite must read it stale"
            );
        }

        // Vehicles: the abstract `*_base.et` template is rejected, the two live variants are not.
        let vehicles = vehicle_items();
        assert_eq!(
            find_catalog_item(
                &vehicles,
                "{B}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469_PKM.et"
            )
            .and_then(placeable_palette),
            Some(CatalogPalette::Vehicle)
        );
        for item in &vehicles {
            if item.kind == "vehicle" && item.r#abstract == Some(true) {
                assert_eq!(
                    placeable_palette(item),
                    None,
                    "an abstract vehicle is not placeable: {}",
                    item.resource_name
                );
            }
        }

        // Characters: every leaf the Factions tree offers for a side must resolve placeable, and
        // the SIDE filter must NOT be applied here — a favourite spans the whole catalogue, so a
        // BLUFOR role is live even while another chip is up.
        let chars = golden_items();
        fn leaf_ids(nodes: &[CatalogNode], out: &mut Vec<String>) {
            for n in nodes {
                if n.payload.is_some() {
                    out.push(n.id.clone());
                }
                leaf_ids(&n.children, out);
            }
        }
        let mut ids = Vec::new();
        leaf_ids(&build_catalog_tree(&chars, "BLUFOR"), &mut ids);
        assert!(!ids.is_empty(), "the golden must offer BLUFOR leaves");
        for id in &ids {
            assert_eq!(
                find_catalog_item(&chars, id).and_then(placeable_palette),
                Some(CatalogPalette::Character),
                "a Factions leaf must resolve placeable: {id}"
            );
        }
        // Same rows, OPFOR chip up: still live, because the chip is a view filter, not the
        // catalogue.
        assert!(build_catalog_tree(&chars, "OPFOR").is_empty());
        for id in &ids {
            assert!(
                find_catalog_item(&chars, id)
                    .and_then(placeable_palette)
                    .is_some(),
                "the Eden side chip must not make a favourite stale: {id}"
            );
        }

        // `gear_*` rows belong to the Arsenal, not the map — no palette places them.
        for item in chars.iter().filter(|i| i.kind.starts_with("gear")) {
            assert_eq!(placeable_palette(item), None);
        }
    }
}
