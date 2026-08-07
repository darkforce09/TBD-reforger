//! T-661 — the right dock (Factions / Vehicles / Zones / Markers palette) and the Eden side chips,
//! split from `eden_chrome.rs`.
//!
//! `palette_rows` is the drag-to-place tree the Factions/Vehicles/Objects tabs draw with; the Eden
//! side chips (`EDEN_SIDE_CHIPS` / [`EdenChip`]) drive `active_side` and the Objects place mode.
//! Not cfg-gated (the doc-driving `on:pointerdown` bodies are wasm-gated inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

use serde::{Deserialize, Serialize};

use crate::asset_catalog::{CatalogNode, CatalogPalette, CatalogState};
use crate::dto::RegistryItem;
use crate::eden_dock_left::collapse_chevron;
use crate::eden_layout::{DOCK_R, STUB_PX};
use crate::eden_tree::{chevron_or_spacer, guide_spans, PALETTE_LEAF};
use crate::eden_vehicles_panel::placed_vehicles_panel;
use crate::eden_zones::zones_panel;
use crate::ui::MaterialIcon;

/// T-076 (RIGHT-CREW-001) — the "place vehicle with crew" toggle rendered beside the Vehicles
/// search. A checkbox bound to `with_crew`: a change writes the [`crate::editor_ops`] placement
/// preference so the NEXT vehicle drop stamps the manned/unmanned intent (`crewed: false` when off)
/// onto its `vehiclesById` row. Eden's default is crewed, which is `with_crew`'s seed.
#[cfg(target_arch = "wasm32")]
fn crew_place_toggle(with_crew: RwSignal<bool>) -> impl IntoView {
    view! {
        <label class="mt-2 flex items-center gap-2 text-label-sm text-on-surface-variant">
            <input
                type="checkbox"
                class="size-3.5 shrink-0 accent-primary"
                aria-label="Place vehicle with crew"
                prop:checked=move || with_crew.get()
                on:change=move |ev| {
                    let on = event_target_checked(&ev);
                    with_crew.set(on);
                    crate::editor_ops::set_place_with_crew(on);
                }
            />
            <span>"Place with crew"</span>
        </label>
    }
}

/// Native shell: the placement preference lives in the wasm-only `editor_ops`, so there is nothing
/// to toggle — the toggle renders on the wasm build only. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
fn crew_place_toggle() -> impl IntoView {
    ().into_view()
}

/// T-215 — which palette a leaf belongs to. The tree machinery (guides, collapse, search) is
/// identical for both; only the glyph and which `editor_ops` arm the press calls differ, and those
/// are the two things that must not be shared — a Vehicles leaf that armed a character place would
/// silently write a `slots` row.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Character,
    Vehicle,
    /// T-254 — Objects chip → `entitiesById`.
    Object,
    /// T-650 (RIGHT-MODE-002) — the Compositions palette mode. Unlike the three above, a composition
    /// is not a `/registry` catalog leaf dragged onto the map: it is a saved multi-entity stamp
    /// listed from the doc, whose row press ARMS a place (the `compositions_panel` list, not
    /// `palette_rows`). The variant exists so the palette-mode vocabulary is complete and so a future
    /// unification of the two surfaces has a name to hang on; the leaf helpers below give it a glyph.
    Composition,
    /// T-079 (RIGHT-MODE-003) — the Triggers palette mode. Like [`Self::Composition`], a trigger is
    /// not a `/registry` catalog leaf: it is an authored AREA drawn with the shipped zone tool and
    /// listed from the doc (`triggers_panel`, not `palette_rows`). The variant completes the
    /// palette-mode vocabulary and gives the mode a glyph; the panel does the authoring.
    Trigger,
}

impl PaletteKind {
    const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
            Self::Composition => "dashboard_customize",
            Self::Trigger => "sensors",
        }
    }

    const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
            Self::Composition => "Click to arm, then click the map to place this composition",
            Self::Trigger => "Draw a trigger area on the map",
        }
    }
}

/// Render the palette recursively. A leaf (`payload.is_some()`) arms a place on `pointerdown` —
/// **pointer-drag, not HTML5 DnD**: the gates drive trusted `Input.dispatchMouseEvent`, which
/// synthesizes real pointer events into these handlers, where DnD would need `Input.setInterceptDrags`.
/// The chrome host stops `pointerdown` propagation, so this press cannot also open a map gesture; the
/// release is consumed by the container's `pointerup` (see `mission_editor`).
fn palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    // T-177 A1 — the parent row's guide-continuation vector (see `guide_spans`); `&[]` at the root.
    prefix: &[bool],
    // T-178 A4 — ancestor ids for guide click (`len == depth`).
    id_prefix: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
    // T-215 — Factions or Vehicles; picks the glyph and the `editor_ops` arm.
    kind: PaletteKind,
    // T-695 — the starred-asset collection, so every leaf carries its own star/unstar verb
    // (3DEN-CTX-001 / Eden F7). Threaded rather than global so the panel and the tree can never
    // disagree about what is starred.
    favourites: RwSignal<Favourites>,
) -> AnyView {
    let len = nodes.len();
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.label.clone();
            let aria = n.label.clone();
            // T-177 A1 — same continuation rule as the outliner's `flatten_visible`: roots draw no
            // column; every deeper row extends its parent's vector with its own `!is_last` bit.
            let anc: Vec<bool> = if depth == 0 {
                Vec::new()
            } else {
                let mut v = Vec::with_capacity(depth);
                v.extend_from_slice(prefix);
                v.push(i + 1 != len);
                v
            };
            let gids = id_prefix.to_vec();
            match n.payload.clone() {
                None => {
                    // Folder — collapsible (T-172 B6): chevron + open/closed icon; kids render
                    // only while open. The whole palette re-renders on a toggle (the DockRight
                    // closure tracks `collapsed`), so open state is read untracked here.
                    let open = !collapsed.with_untracked(|c| c.contains(&n.id));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, &n.id, collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let mut child_ids = gids.clone();
                    child_ids.push(n.id.clone());
                    let kids = if open {
                        palette_rows(
                            &n.children,
                            depth + 1,
                            &anc,
                            &child_ids,
                            collapsed,
                            kind,
                            favourites,
                        )
                    } else {
                        ().into_any()
                    };
                    let cid = n.id.clone();
                    view! {
                        <div
                            role="button"
                            tabindex="-1"
                            aria-label=aria
                            class="relative flex cursor-pointer items-center gap-1.5 px-1.5 py-1 text-label-sm text-outline transition-colors hover:text-on-surface"
                            on:click=move |_| {
                                collapsed
                                    .update(|c| {
                                        if !c.remove(&cid) {
                                            c.insert(cid.clone());
                                        }
                                    });
                            }
                        >
                            {guide_spans(&anc, &gids, collapsed)}
                            {toggle}
                            <MaterialIcon name=folder_icon class="block text-sm" />
                            <span class="truncate">{label}</span>
                        </div>
                        {kids}
                    }
                    .into_any()
                }
                // T-177 A2 — a placeable role: PALETTE_LEAF adds `cursor-grab`/`active:cursor-grabbing`
                // over ROW so hovering shows the drag affordance (folders keep `cursor-pointer`).
                //
                // T-695 — the leaf is now a ROW rather than a bare button: the place affordance plus
                // its star/unstar verb. A `<button>` cannot nest inside a `<button>`, so the wrapper
                // takes `group` (the T-666 hover idiom the star reads) and the place button keeps
                // PALETTE_LEAF verbatim, widened with `flex-1`. The leaf's id IS the asset id
                // (`resource_name`) the collection stores — see `payload.asset_id`.
                Some(payload) => {
                    let star =
                        favourite_star(favourites, payload.asset_id.clone(), payload.role.clone());
                    view! {
                    <div class="group relative flex items-center gap-1">
                    <button
                        type="button"
                        aria-label=aria
                        title=kind.leaf_title()
                        class=format!("{PALETTE_LEAF} min-w-0 flex-1")
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            match kind {
                                PaletteKind::Character => {
                                    crate::editor_ops::begin_place(payload.clone())
                                }
                                PaletteKind::Vehicle => {
                                    crate::editor_ops::begin_place_vehicle(payload.clone())
                                }
                                PaletteKind::Object => {
                                    crate::editor_ops::begin_place_object(payload.clone())
                                }
                                // T-650 — compositions are not catalog leaves; they arm from the
                                // `compositions_panel` list, not from a `palette_rows` payload. This
                                // arm only exists so the match is exhaustive.
                                PaletteKind::Composition => {}
                                // T-079 — triggers are not catalog leaves either; they are drawn from
                                // the `triggers_panel`, not armed from a `palette_rows` payload. Arm
                                // present only for exhaustiveness.
                                PaletteKind::Trigger => {}
                            }
                            // `editor_ops` is wasm-only, so the native view shell would see an
                            // unused capture (the `announcements.rs` `let _ = store;` idiom).
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {guide_spans(&anc, &gids, collapsed)}
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name=kind.leaf_icon() class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </button>
                    {star}
                    </div>
                    }
                    .into_any()
                }
            }
        })
        .collect::<Vec<_>>()
        .into_any()
}

/// Collect the folder ids whose `default_expanded` is false — the palette's initial collapsed
/// set (`buildCatalogTree` rule 3: only depth-0 faction folders start open). T-172 B6.
fn collapsed_seed(nodes: &[CatalogNode], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if n.payload.is_none() && !n.children.is_empty() && !n.default_expanded {
            out.insert(n.id.clone());
        }
        collapsed_seed(&n.children, out);
    }
}

// ── T-180.5 — Eden side chips (no F1–F6, no CIV) ─────────────────────────────────────────────────

/// Ordered chip labels the DockRight row iterates. Gate E1/E5 pin this exact list.
pub const EDEN_SIDE_CHIPS: &[&str] = &["BLUFOR", "OPFOR", "INDFOR", "Objects"];

/// Which Eden chip is selected (side place vs Objects world-entity place).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdenChip {
    Blufor,
    Opfor,
    Indfor,
    Objects,
}

impl EdenChip {
    /// Chip row label / `aria-label`.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Blufor => "BLUFOR",
            Self::Opfor => "OPFOR",
            Self::Indfor => "INDFOR",
            Self::Objects => "Objects",
        }
    }

    /// Tailwind fill class (Aegis tokens matching map SIDE_* / tactical-yellow).
    pub const fn fill_class(self) -> &'static str {
        match self {
            Self::Blufor => "bg-primary",
            Self::Opfor => "bg-error-alert",
            Self::Indfor => "bg-success",
            Self::Objects => "bg-tactical-yellow",
        }
    }

    /// Parse a chip label from [`EDEN_SIDE_CHIPS`].
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "BLUFOR" => Some(Self::Blufor),
            "OPFOR" => Some(Self::Opfor),
            "INDFOR" => Some(Self::Indfor),
            "Objects" => Some(Self::Objects),
            _ => None,
        }
    }
}

/// Apply a chip click to the shared place signals (same `active_side` OpsCtx / `place_at` read).
///
/// Side chips clear Objects mode and set the place side. Objects sets `objects_mode` only (leaves
/// `active_side` unchanged so flipping back restores the last side).
pub fn apply_eden_chip(
    chip: EdenChip,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
) {
    match chip {
        EdenChip::Objects => objects_mode.set(true),
        EdenChip::Blufor => {
            objects_mode.set(false);
            active_side.set(String::from("BLUFOR"));
        }
        EdenChip::Opfor => {
            objects_mode.set(false);
            active_side.set(String::from("OPFOR"));
        }
        EdenChip::Indfor => {
            objects_mode.set(false);
            active_side.set(String::from("INDFOR"));
        }
    }
}

/// Whether the chip row should show `chip` as selected given current side + objects mode.
pub fn eden_chip_selected(chip: EdenChip, active_side: &str, objects_mode: bool) -> bool {
    match chip {
        EdenChip::Objects => objects_mode,
        EdenChip::Blufor => !objects_mode && active_side == "BLUFOR",
        EdenChip::Opfor => !objects_mode && active_side == "OPFOR",
        EdenChip::Indfor => !objects_mode && active_side == "INDFOR",
    }
}

// ── T-646 (RIGHT-SUBMODE-001) — the Custom slot, visible only under Groups ────────────────────────
//
// Eden's chip row carries a sixth CUSTOM slot in ADDITION to the side chips, and it appears **only
// under the Groups sub-mode** — the mode where you place whole groups/squads. It is modelled here as
// its own pure predicate rather than a fifth `EdenChip` variant on purpose: `EdenChip` and
// `EDEN_SIDE_CHIPS` are pinned by the E1/E5 gate to the exact shipped 4-chip list (BLUFOR / OPFOR /
// INDFOR / Objects, no CIV, no F-keys), and widening that enum would both break those assertions and
// entangle the always-on side chips with a slot whose whole point is that it is conditional. Keeping
// Custom a standalone, submode-gated mechanic is what lets the visibility rule be tested in isolation
// (the "Custom-only-under-Groups" gate) without disturbing the shipped side row.

/// T-646 — which right-dock sub-mode the palette is showing. Eden cycles these with `Tab`; here they
/// map onto the dock's tabs. Only [`EdenSubmode::Groups`] (the character/squad-placing surface, the
/// Factions tab) reveals the Custom chip — Vehicles / Objects / Markers / Zones never do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdenSubmode {
    /// The Factions tab — placing characters that form groups/squads. Eden's "Groups" mode.
    Groups,
    /// The Vehicles tab.
    Vehicles,
    /// The Objects world-entity place (the Objects chip on the Factions tab).
    Objects,
    /// The Markers tab (T-069 stub).
    Markers,
    /// The Zones tab (T-582).
    Zones,
    /// T-650 — the Compositions tab (RIGHT-MODE-002).
    Compositions,
    /// T-079 — the Triggers tab (RIGHT-MODE-003).
    Triggers,
    /// T-695 — the Favourites tab (NEW-F5 / 3den E3): the starred-asset collection, not a palette
    /// over `/registry`. It is its own sub-mode for the same reason Compositions and Triggers are —
    /// so `from_tab` never reports a surface the operator is not looking at, and so the Groups-only
    /// Custom chip cannot leak onto it.
    Favourites,
}

impl EdenSubmode {
    /// Map a DockRight tab index (`0` Factions, `1` Vehicles, `2` Markers, `3` Zones, `4`
    /// Compositions, `5` Triggers, `6` Favourites) plus the Objects-chip flag to the sub-mode. The Objects chip lives
    /// on the Factions tab but is its own place surface, so it reports [`EdenSubmode::Objects`], not
    /// `Groups` — which is exactly why the Custom slot hides the moment the operator flips to Objects.
    #[must_use]
    pub fn from_tab(tab: usize, objects_mode: bool) -> Self {
        match tab {
            1 => Self::Vehicles,
            2 => Self::Markers,
            3 => Self::Zones,
            // T-650 — tab 4 is Compositions.
            4 => Self::Compositions,
            // T-079 — tab 5 is Triggers.
            5 => Self::Triggers,
            // T-695 — tab 6 is Favourites.
            6 => Self::Favourites,
            // tab 0 (Factions): Objects chip splits Groups vs Objects.
            _ if objects_mode => Self::Objects,
            _ => Self::Groups,
        }
    }
}

/// T-646 (RIGHT-SUBMODE-001) — the Custom chip's `aria-label` / row text. The sixth slot; a fixed
/// label so the gate can pin it without a render.
pub const EDEN_CUSTOM_CHIP: &str = "Custom";

/// T-646 (RIGHT-SUBMODE-001) — whether the Custom slot is shown in the chip row.
///
/// The whole rule in one predicate: **Custom appears only under Groups.** Every other sub-mode hides
/// it, so an author on the Vehicles or Objects surface never sees a group-only affordance.
#[must_use]
pub fn custom_chip_visible(submode: EdenSubmode) -> bool {
    matches!(submode, EdenSubmode::Groups)
}

// ── T-695 — Favourites: a starred-asset collection across the catalogue ──────────────────────────
// (NEW-F5 + 3den E3 + 3DEN-CTX-001; Eden F7.)
//
// This is NOT T-646's search, and the distinction is the stated reason the two are separate
// tickets: search FILTERS the live tree and holds nothing between keystrokes, where this is a
// persistent COLLECTION with its own two explicit verbs — star (add) and unstar (remove) — spanning
// all three catalogue palettes (Factions / Vehicles / Objects). Nothing here touches
// `filter_catalog` or the search boxes.
//
// Pure SPA: one localStorage key, no API call, no backend, no migration endpoint.
//
// **KEY NAMESPACE + VERSION.** The established frontend convention (grepped, not invented) is a
// `tbd-<area>-<thing>` key holding a JSON blob with an integer `version` field:
// `world_layer_prefs::EDITOR_PREFS_KEY` = `tbd-mc-editor-prefs` with `EDITOR_PREFS_VERSION`,
// `auth::AUTH_PERSIST_KEY` = `tbd-auth`, `editor_session` = `tbd-editor-session`. This follows it
// exactly — [`FAVOURITES_KEY`] + [`FAVOURITES_VERSION`], defaults-on-parse-failure as the floor, and
// one [`migrate_favourites`] chokepoint so a future shape change has an obvious home. (The T-691
// store seam — a field on `world_layer_prefs::EditorPrefs` — would have been the tidier home, but
// that file is not this slice's to touch; see the slice report.)
//
// **STALE FAVOURITES — decided: KEEP AND MARK.** A starred id can leave the live catalogue (a
// modpack switched off, a prefab renamed, a row that stopped being placeable). Such an entry is
// NOT pruned from storage and does NOT render as a normal row: it renders disabled, labelled with
// the display name remembered at star time, saying it is not in the current catalogue — with the
// remove verb still live so the operator can clear it deliberately. That is neither a broken row
// (it cannot arm a place, and it says why) nor a silent vanishing (switch the modpack back on and
// the entry resolves live again, because nothing was thrown away behind the operator's back).

/// T-695 — the one localStorage key the favourites collection persists under. Namespaced
/// `tbd-mc-editor-…` like the sibling editor-local store; see the section header.
const FAVOURITES_KEY: &str = "tbd-mc-editor-favourites";
/// T-695 — the persisted blob's schema version. Bump when a field's shape changes in a way a raw
/// serde load of an older blob cannot absorb (adding a `#[serde(default)]` field does NOT need a
/// bump); [`migrate_favourites`] then owns the upgrade.
const FAVOURITES_VERSION: u32 = 1;
/// T-695 — how many entries the collection keeps. A cap exists because localStorage is a shared,
/// small, synchronously-parsed budget and nothing else bounds an add loop; 250 is far past any
/// plausible working set (the live registry offers a few hundred placeable rows in total).
const FAVOURITES_MAX: usize = 250;

/// T-695 — one starred asset.
///
/// `asset_id` is the full Enfusion `resource_name` — the SAME string a catalogue leaf uses as its
/// `CatalogNode::id` and hands the map as `PlacePayload::asset_id`. Storing the registry row's uuid
/// instead would break the moment a modpack is re-ingested with fresh row ids.
///
/// `label` is the display name **remembered at star time**. It is not the source of truth while the
/// asset is live (the catalogue's current `display_name` wins, so a renamed prefab shows its new
/// name); it exists so a STALE entry can still name itself instead of showing a raw prefab path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavouriteAsset {
    pub asset_id: String,
    #[serde(default)]
    pub label: String,
}

/// T-695 — the persisted favourites blob: a version plus the starred entries, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Favourites {
    /// Schema version of the persisted blob (see [`FAVOURITES_VERSION`]).
    #[serde(default)]
    pub version: u32,
    /// The starred entries in display order — most recently starred first.
    #[serde(default)]
    pub items: Vec<FavouriteAsset>,
}

impl Default for Favourites {
    fn default() -> Self {
        Self {
            version: FAVOURITES_VERSION,
            items: Vec::new(),
        }
    }
}

impl Favourites {
    /// Parse a persisted blob, falling back to empty on any serde failure and normalising through
    /// [`migrate_favourites`]. Pure — no localStorage — so the whole storage contract is testable
    /// on the native build.
    #[must_use]
    fn from_json(raw: &str) -> Self {
        migrate_favourites(serde_json::from_str::<Self>(raw).unwrap_or_default())
    }

    /// Serialize for persistence (empty string only if serde itself fails, which the round-trip
    /// test precludes for this shape).
    #[must_use]
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Is this asset id starred?
    #[must_use]
    pub fn contains(&self, asset_id: &str) -> bool {
        self.items.iter().any(|f| f.asset_id == asset_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The ADD verb. Newest first, so the row an operator just starred is the one they see. A
    /// duplicate add is a no-op (the collection is a set keyed by `asset_id`), and an empty id is
    /// refused rather than stored as an entry nothing can ever resolve.
    pub fn add(&mut self, asset_id: &str, label: &str) {
        if asset_id.is_empty() || self.contains(asset_id) {
            return;
        }
        self.items.insert(
            0,
            FavouriteAsset {
                asset_id: asset_id.to_string(),
                label: label.to_string(),
            },
        );
        self.items.truncate(FAVOURITES_MAX);
    }

    /// The REMOVE verb. Idempotent — unstarring something that is not starred is a no-op.
    pub fn remove(&mut self, asset_id: &str) {
        self.items.retain(|f| f.asset_id != asset_id);
    }

    /// The star/unstar toggle behind the leaf's context action. Returns the NEW state: `true` when
    /// the asset is now starred, `false` when it was just removed.
    pub fn toggle(&mut self, asset_id: &str, label: &str) -> bool {
        if self.contains(asset_id) {
            self.remove(asset_id);
            false
        } else {
            self.add(asset_id, label);
            self.contains(asset_id)
        }
    }
}

/// T-695 — bring a freshly-loaded blob up to the current version and normalise it. Idempotent.
///
/// Beyond the version stamp this is the integrity floor for a blob any other tab (or a person with
/// devtools) may have written: entries with an empty id are dropped, duplicates collapse to their
/// first occurrence, and the list is capped. Without it a duplicated id would render two rows whose
/// unstar buttons both target the same entry.
fn migrate_favourites(mut fav: Favourites) -> Favourites {
    if fav.version < FAVOURITES_VERSION {
        // No field-shape migrations exist yet (v0 → v1 is field-compatible via serde defaults);
        // future versions add their transforms here, gated on the incoming `version`.
        fav.version = FAVOURITES_VERSION;
    }
    let mut seen = std::collections::HashSet::new();
    fav.items
        .retain(|f| !f.asset_id.is_empty() && seen.insert(f.asset_id.clone()));
    fav.items.truncate(FAVOURITES_MAX);
    fav
}

#[cfg(target_arch = "wasm32")]
fn favourites_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// T-695 — load the favourites collection. Off wasm (the native test build) this is always empty,
/// exactly like `world_layer_prefs::load_store`.
#[must_use]
pub fn load_favourites() -> Favourites {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = favourites_storage() {
            if let Ok(Some(raw)) = s.get_item(FAVOURITES_KEY) {
                return Favourites::from_json(&raw);
            }
        }
    }
    Favourites::default()
}

/// T-695 — persist the favourites collection (no-op off wasm). The version is stamped current on
/// write so a load never sees a stale version this build wrote itself.
pub fn save_favourites(fav: &Favourites) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(s) = favourites_storage() {
            let mut out = fav.clone();
            out.version = FAVOURITES_VERSION;
            let _ = s.set_item(FAVOURITES_KEY, &out.to_json());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = fav;
}

/// T-695 — how one favourite resolved against the live catalogue. The whole stale-degradation rule
/// is this two-variant enum: a favourite is either live (and therefore placeable, through a named
/// palette) or stale (and therefore rendered disabled, named, and removable) — there is no third
/// state in which it is quietly dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FavouriteRow {
    /// The id is in the live catalogue and still placeable. `label` is the catalogue's CURRENT
    /// display name, not the remembered one.
    Live {
        asset_id: String,
        label: String,
        palette: CatalogPalette,
    },
    /// The id is gone from the live catalogue, or the row is no longer placeable by any palette.
    /// Kept, not pruned; `label` is the name remembered at star time (or the raw id if the blob
    /// carried none), so the row is never blank.
    Stale { asset_id: String, label: String },
}

impl FavouriteRow {
    /// The asset id either variant carries — what the unstar verb targets.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        match self {
            Self::Live { asset_id, .. } | Self::Stale { asset_id, .. } => asset_id,
        }
    }

    /// The name the row renders.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Live { label, .. } | Self::Stale { label, .. } => label,
        }
    }

    /// Whether this row can arm a place.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        matches!(self, Self::Live { .. })
    }
}

/// T-695 — resolve the persisted collection against the live catalogue rows, preserving order and
/// **count**: every stored favourite yields exactly one row. That invariant is the "degrade
/// honestly" requirement in one sentence — a favourite the catalogue no longer offers becomes a
/// [`FavouriteRow::Stale`], never a missing row.
///
/// Pure over `(&Favourites, &[RegistryItem])`, so the rule is unit-testable without a DOM.
#[must_use]
pub fn resolve_favourites(fav: &Favourites, items: &[RegistryItem]) -> Vec<FavouriteRow> {
    fav.items
        .iter()
        .map(|f| {
            let live = crate::asset_catalog::find_catalog_item(items, &f.asset_id)
                .and_then(|it| crate::asset_catalog::placeable_palette(it).map(|p| (it, p)));
            match live {
                Some((item, palette)) => FavouriteRow::Live {
                    asset_id: f.asset_id.clone(),
                    label: item.display_name.clone(),
                    palette,
                },
                None => FavouriteRow::Stale {
                    asset_id: f.asset_id.clone(),
                    label: if f.label.trim().is_empty() {
                        f.asset_id.clone()
                    } else {
                        f.label.clone()
                    },
                },
            }
        })
        .collect()
}

/// T-695 — the star/unstar context action behind a palette leaf. ONE place writes the collection:
/// flip the signal, then persist — so a starred asset is on disk before the next render, and a
/// reload cannot lose the verb the operator just used.
fn toggle_favourite(favourites: RwSignal<Favourites>, asset_id: &str, label: &str) {
    favourites.update(|f| {
        f.toggle(asset_id, label);
    });
    save_favourites(&favourites.get_untracked());
}

/// T-695 — the leaf's star toggle: the add/remove verb reachable from the asset itself
/// (3DEN-CTX-001, Eden F7). It sits on the palette ROW rather than in the right-click context menu
/// because `context_menu.rs` is another slice's file this wave — the gap is reported, not reached
/// across (see the slice report's `found_not_fixed`).
///
/// Rendered as a hover action in the T-666 idiom (`group-hover:opacity-100`), except that a STARRED
/// leaf keeps its glyph visible at all times — the collection has to be legible from the tree
/// without hunting row by row with the pointer.
fn favourite_star(favourites: RwSignal<Favourites>, asset_id: String, label: String) -> AnyView {
    let starred_id = asset_id.clone();
    // A Memo, not a bare closure: the glyph, the label, the pressed state and the class all read it,
    // and a closure capturing the owned id is not `Copy` (it could be moved into one of them only).
    let starred = Memo::new(move |_| favourites.with(|f| f.contains(&starred_id)));
    view! {
        <button
            type="button"
            aria-label=move || {
                if starred.get() { "Remove from favourites" } else { "Add to favourites" }
            }
            aria-pressed=move || starred.get()
            title=move || {
                if starred.get() { "Unstar this asset" } else { "Star this asset" }
            }
            class=move || {
                if starred.get() {
                    "shrink-0 rounded-md p-1 text-primary opacity-100 transition-opacity hover:bg-white/10"
                } else {
                    "shrink-0 rounded-md p-1 text-on-surface-variant opacity-0 transition-opacity hover:bg-white/10 group-hover:opacity-100 focus:opacity-100"
                }
            }
            on:click=move |_| {
                toggle_favourite(favourites, &asset_id, &label);
            }
        >
            <span class="material-symbols-outlined block text-sm">
                {move || if starred.get() { "star" } else { "star_border" }}
            </span>
        </button>
    }
    .into_any()
}

/// T-695 — arm a place from a FAVOURITES row.
///
/// The three arms are spelled out here rather than shared with `palette_rows` on purpose: the T-215
/// gate pins the leaf's own call expression by source inspection, and folding both call sites into
/// one helper would satisfy that needle from this function instead — a check passing over an input
/// it never examined, which is exactly the defect class this programme is about. Note the argument
/// is moved, not cloned, so the two call sites stay textually distinct.
#[cfg(target_arch = "wasm32")]
fn arm_favourite_place(palette: CatalogPalette, payload: crate::asset_catalog::PlacePayload) {
    match palette {
        CatalogPalette::Character => crate::editor_ops::begin_place(payload),
        CatalogPalette::Vehicle => crate::editor_ops::begin_place_vehicle(payload),
        CatalogPalette::Object => crate::editor_ops::begin_place_object(payload),
    }
}

impl PaletteKind {
    /// T-695 — the catalogue-side palette a favourite resolved to, in the dock's own vocabulary
    /// (for the row glyph and title).
    const fn from_catalog(palette: CatalogPalette) -> Self {
        match palette {
            CatalogPalette::Character => Self::Character,
            CatalogPalette::Vehicle => Self::Vehicle,
            CatalogPalette::Object => Self::Object,
        }
    }
}

/// T-695 — one favourites row: a live entry that arms a place plus its unstar verb, or a stale
/// entry rendered disabled and named, whose unstar verb still works.
fn favourite_row_view(row: FavouriteRow, favourites: RwSignal<Favourites>) -> AnyView {
    let unstar_id = row.asset_id().to_string();
    let unstar = view! {
        <button
            type="button"
            aria-label="Remove from favourites"
            title="Remove from favourites"
            class="shrink-0 rounded-md p-1 text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
            on:click=move |_| {
                favourites
                    .update(|f| {
                        f.remove(&unstar_id);
                    });
                save_favourites(&favourites.get_untracked());
            }
        >
            <span class="material-symbols-outlined block text-sm">"star"</span>
        </button>
    };
    match row {
        FavouriteRow::Live {
            asset_id,
            label,
            palette,
        } => {
            let kind = PaletteKind::from_catalog(palette);
            let payload = crate::asset_catalog::PlacePayload {
                asset_id,
                role: label.clone(),
            };
            let aria = label.clone();
            view! {
                <li class="group relative flex items-center gap-1">
                    <button
                        type="button"
                        aria-label=aria
                        title=kind.leaf_title()
                        class=format!("{PALETTE_LEAF} flex-1")
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            arm_favourite_place(palette, payload.clone());
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        <MaterialIcon name=kind.leaf_icon() class="block text-sm" />
                        <span class="truncate">{label}</span>
                    </button>
                    {unstar}
                </li>
            }
            .into_any()
        }
        // The stale row: no place affordance at all (a disabled button, not a grabbable leaf), the
        // remembered name so it is identifiable, and a plain-language reason. The unstar verb is
        // deliberately still live — removing it is the operator's call, not the reload's.
        FavouriteRow::Stale { label, .. } => {
            let aria = format!("{label} — not in the current catalogue");
            view! {
            <li class="group relative flex items-center gap-1">
                <button
                    type="button"
                    disabled=true
                    aria-label=aria
                    title="This asset is not in the catalogue the editor loaded. Its modpack may be off, or the prefab was renamed."
                    class="relative flex flex-1 cursor-not-allowed items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-outline opacity-70"
                >
                    <MaterialIcon name="warning" class="block text-sm" />
                    <span class="flex min-w-0 flex-col">
                        <span class="truncate line-through">{label}</span>
                        <span class="truncate text-[10px] text-outline">
                            "Not in the current catalogue"
                        </span>
                    </span>
                </button>
                {unstar}
            </li>
            }
            .into_any()
        }
    }
}

/// T-695 — the Favourites tab: the starred collection over the WHOLE catalogue, resolved live.
///
/// Three states, and the middle one matters: while the registry fetch is still in flight there is
/// nothing to resolve against, so the panel says so instead of declaring every favourite stale.
fn favourites_panel(
    favourites: RwSignal<Favourites>,
    registry_items: RwSignal<Option<Vec<RegistryItem>>>,
) -> AnyView {
    view! {
        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Favourites"</h3>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Starred assets from every palette. Star one with the ★ on its palette row."
        </p>
        <div class="mt-2">
            {move || {
                if favourites.with(Favourites::is_empty) {
                    return view! {
                        <p class="text-label-sm text-outline">
                            "No favourites yet — hover an asset in Factions, Vehicles or Objects and press its star."
                        </p>
                    }
                        .into_any();
                }
                let Some(items) = registry_items.get() else {
                    let n = favourites.with(Favourites::len);
                    return view! {
                        <p class="text-label-sm text-outline">
                            {format!("Resolving {n} favourite(s) against the catalogue…")}
                        </p>
                    }
                        .into_any();
                };
                let rows = favourites.with(|f| resolve_favourites(f, &items));
                view! {
                    <ul class="flex flex-col gap-0.5">
                        {rows
                            .into_iter()
                            .map(|r| favourite_row_view(r, favourites))
                            .collect_view()}
                    </ul>
                }
                    .into_any()
            }}
        </div>
    }
    .into_any()
}

/// Right dock — the **Factions** palette (spec O2), off the live `GET /api/v1/registry`. Leaves drag
/// onto the map to place their slot. `fm_open` toggles the T-167 Faction Manager dialog.
///
/// T-180.5 — Eden side chips above search drive `active_side` / Objects stub.
///
/// T-215 — the **Vehicles** tab is a real palette off the same `/registry` fetch (`vehicle_catalog`,
/// built by `asset_catalog::build_vehicle_catalog_tree`), not the T-070 placeholder it was. Its
/// leaves arm `editor_ops::begin_place_vehicle`, so a release on the canvas writes a `vehiclesById`
/// row at that world point.
///
/// T-638 — `collapsed` collapses this dock to the [`STUB_PX`]-square stub in its outer top-RIGHT
/// corner; the `R` key and the tab-strip chevron both flip it (see [`collapse_chevron`]).
#[component]
pub fn DockRight(
    catalog: RwSignal<CatalogState>,
    /// T-215 — the `kind == "vehicle"` half of the same registry fetch.
    vehicle_catalog: RwSignal<CatalogState>,
    /// T-215 — the raw registry rows, for the placed-vehicle cargo picker's labels and options.
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    /// T-215 — the doc-change tick the placed-vehicle list re-reads on.
    doc_tick: RwSignal<u64>,
    fm_open: RwSignal<bool>,
    active_side: RwSignal<String>,
    objects_mode: RwSignal<bool>,
    /// T-638 — collapse latch (owned by `mission_editor`; `R`/chevron toggle it, the accessor + reflow
    /// observe it).
    collapsed: RwSignal<bool>,
) -> impl IntoView {
    // Palette collapse state (T-172 B6), seeded from `default_expanded` whenever the catalog
    // turns Ready or the Eden side chip rebuilds the tree (T-255). User toggles stick until the
    // next side-driven rebuild (NATO folders are meaningless under OPFOR).
    let palette_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    Effect::new(move |_| {
        let _ = active_side.get(); // T-255 — re-seed when chips flip the filtered tree
        if let CatalogState::Ready(nodes) = catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            palette_collapsed.set(set);
        }
    });
    // T-172 B9 — screen-05 palette chrome: FACTIONS / VEHICLES / MARKERS tabs + Asset Browser
    // search. Vehicles/Markers placement stays T-070/T-069 — React's tabs were stubs too, so the
    // panels say exactly that. Search filters the catalog (T-055 behavior) and force-expands
    // matches (an empty collapse set while a query is live).
    let tab = RwSignal::new(0usize);
    let search = RwSignal::new(String::new());
    let no_collapse = RwSignal::new(std::collections::HashSet::<String>::new());
    // T-215 — the Vehicles tab keeps its OWN collapse set and search box. Sharing either with the
    // Factions tab would mean a query typed against 178 vehicles silently filtering the roles the
    // author switches back to, and a folder id collision between two trees built from different
    // path vocabularies.
    let vehicle_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let vehicle_seeded = StoredValue::new(false);
    Effect::new(move |_| {
        if vehicle_seeded.get_value() {
            return;
        }
        if let CatalogState::Ready(nodes) = vehicle_catalog.get() {
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            vehicle_collapsed.set(set);
            vehicle_seeded.set_value(true);
        }
    });
    let vehicle_search = RwSignal::new(String::new());
    // T-076 (RIGHT-CREW-001) — the "place vehicle with crew" toggle, seeded from the editor-ops
    // preference so it reflects the live place mode (Eden default: crewed). Flipping it writes the
    // preference back; the next vehicle placement stamps the manned/unmanned intent on the row.
    // wasm-only: the preference lives in `editor_ops`, a wasm32-only module.
    #[cfg(target_arch = "wasm32")]
    let place_with_crew = RwSignal::new(crate::editor_ops::place_with_crew());
    // T-254 — Objects chip palette (entities[]): own collapse + search, built from registry_items.
    let object_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let object_search = RwSignal::new(String::new());
    // T-215 — which placed vehicles have their cargo editor open.
    let vehicle_expanded = RwSignal::new(std::collections::HashSet::<String>::new());
    // T-582 — the selected zone (Attributes target). Its own selection, NOT `select_tool`'s: that
    // one runs over the slot SoA and drives SEL/highlight, so putting a zone id in it would show
    // `SEL 1` with nothing highlighted anywhere — the same reason `place_at` keeps vehicle and
    // entity ids out of it.
    let zone_selected = RwSignal::new(None::<String>);
    // T-650 — the composition id currently in inline-edit (rename/recategorize), or `None`. Its own
    // signal, like `zone_selected`: a composition is neither a slot nor a zone, so it does not touch
    // `select_tool`'s selection or the zone selection.
    let comp_editing = RwSignal::new(None::<String>);
    // T-079 — the selected trigger (Attributes target + the owner-link line's subject). Its own
    // selection, exactly like `zone_selected` and for the same reason: a trigger is neither a slot
    // nor a zone, so putting its id in `select_tool`'s selection would show `SEL 1` with nothing
    // highlighted. The owner-link line renders while this is `Some`.
    let trigger_selected = RwSignal::new(None::<String>);
    // T-695 (NEW-F5 / 3den E3) — the starred-asset collection, seeded from localStorage on mount so
    // it survives a catalogue reload, and written back on every star/unstar. It is dock-local
    // because it is a per-user editor preference, not mission state: nothing in the document, in
    // `editor_ops` or on the wire knows or should know what an author has starred.
    let favourites = RwSignal::new(load_favourites());
    let tab_btn = move |i: usize, label: &'static str| {
        view! {
            <button
                type="button"
                class=move || {
                    if tab.get() == i {
                        "border-b-2 border-primary px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface"
                    } else {
                        "border-b-2 border-transparent px-1.5 pb-1 text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant transition-colors hover:text-on-surface"
                    }
                }
                on:click=move |_| tab.set(i)
            >
                {label}
            </button>
        }
    };
    let full = move || {
        view! {
            <aside class=DOCK_R>
                // T-638 — the tab strip carries the collapse chevron at its outer (top-RIGHT) end, after
                // "Manage"; » while expanded, flips to « collapsed.
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-1">
                        {tab_btn(0, "Factions")}
                        {tab_btn(1, "Vehicles")}
                        // T-582 — Zones sits before the Markers stub: it is a live surface and that
                        // one is still a promise (T-069).
                        {tab_btn(3, "Zones")}
                        // T-650 — Compositions is a live surface too, so it also precedes Markers.
                        {tab_btn(4, "Compositions")}
                        // T-079 — Triggers is a live surface (draw area + owner link), so it precedes
                        // the Markers stub as well.
                        {tab_btn(5, "Triggers")}
                        // T-695 — Favourites is a live surface (the starred collection over the
                        // whole catalogue), so it precedes the Markers stub too.
                        {tab_btn(6, "Favourites")}
                        {tab_btn(2, "Markers")}
                    </div>
                    <div class="flex items-center gap-1">
                        <button
                            type="button"
                            aria-label="Manage factions"
                            on:click=move |_| fm_open.set(true)
                            class="rounded-md px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-primary transition-colors hover:bg-primary/15"
                        >
                            "Manage"
                        </button>
                        {collapse_chevron(collapsed, false)}
                    </div>
                </div>
                {move || match tab.get() {
                    0 => view! {
                        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Asset Browser"</h3>
                        <p class="mt-0.5 text-label-sm normal-case text-outline">
                            "Drag a role onto the map to place its slot."
                        </p>
                        // T-180.5 — Eden side chips above search (E-L4). No F1–F6 row, no CIV.
                        <div
                            class="mt-2 flex items-center gap-1.5"
                            role="group"
                            aria-label="Eden side"
                        >
                            {EDEN_SIDE_CHIPS
                                .iter()
                                .filter_map(|label| EdenChip::from_label(label))
                                .map(|chip| {
                                    let fill = chip.fill_class();
                                    view! {
                                        <button
                                            type="button"
                                            aria-label=chip.label()
                                            aria-pressed=move || {
                                                eden_chip_selected(
                                                    chip,
                                                    &active_side.get(),
                                                    objects_mode.get(),
                                                )
                                            }
                                            class=move || {
                                                let selected = eden_chip_selected(
                                                    chip,
                                                    &active_side.get(),
                                                    objects_mode.get(),
                                                );
                                                if selected {
                                                    format!(
                                                        "{fill} h-5 w-8 shrink-0 rounded-sm ring-2 ring-offset-1 ring-offset-surface-container-lowest ring-white/90 opacity-100"
                                                    )
                                                } else {
                                                    format!(
                                                        "{fill} h-5 w-8 shrink-0 rounded-sm opacity-45 transition-opacity hover:opacity-75"
                                                    )
                                                }
                                            }
                                            on:click=move |_| {
                                                // T-255 — writes `active_side`; mission_editor's
                                                // Effect rebuilds `catalog` via build_catalog_tree(_, side).
                                                apply_eden_chip(chip, active_side, objects_mode)
                                            }
                                        />
                                    }
                                })
                                .collect_view()}
                            // T-646 (RIGHT-SUBMODE-001) — the sixth Custom slot, shown only under the
                            // Groups sub-mode (Factions tab, side place — never Objects). Renders as a
                            // labelled outline chip: unlike the side swatches it carries no fill token,
                            // and it is inert here (its persistent custom-collection verbs are T-078's
                            // separate ticket), so it declares itself disabled rather than feigning a
                            // place. `Show` keeps it out of the DOM entirely when Objects is active.
                            <Show when=move || custom_chip_visible(
                                EdenSubmode::from_tab(0, objects_mode.get()),
                            )>
                                <button
                                    type="button"
                                    disabled=true
                                    aria-label=EDEN_CUSTOM_CHIP
                                    title="Custom groups arrive in T-078"
                                    class="flex h-5 shrink-0 items-center rounded-sm border border-outline-variant/60 px-1.5 text-[10px] font-semibold uppercase tracking-wide text-outline opacity-45"
                                >
                                    {EDEN_CUSTOM_CHIP}
                                </button>
                            </Show>
                        </div>
                        <input
                            type="search"
                            aria-label=move || {
                                if objects_mode.get() {
                                    "Search objects"
                                } else {
                                    "Search assets"
                                }
                            }
                            // T-646 — hint the `class:` operator (RIGHT-SEARCH-002). The broader
                            // `mod `/glob/regex grammar is T-084 and lands its own copy here.
                            placeholder=move || {
                                if objects_mode.get() {
                                    "Search objects or class:…"
                                } else {
                                    "Search assets or class:…"
                                }
                            }
                            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                            on:input=move |ev| {
                                let v = event_target_value(&ev);
                                if objects_mode.get_untracked() {
                                    object_search.set(v);
                                } else {
                                    search.set(v);
                                }
                            }
                        />
                        <div class="mt-2">
                            {move || {
                                if objects_mode.get() {
                                    let items = registry_items.get().unwrap_or_default();
                                    let nodes =
                                        crate::asset_catalog::build_object_catalog_tree(&items);
                                    if nodes.is_empty() {
                                        return view! {
                                            <p class="text-label-sm text-outline">
                                                "No placeable objects in the registry."
                                            </p>
                                        }
                                        .into_any();
                                    }
                                    let q = object_search.get();
                                    if q.trim().is_empty() {
                                        object_collapsed.track();
                                        return palette_rows(
                                            &nodes,
                                            0,
                                            &[],
                                            &[],
                                            object_collapsed,
                                            PaletteKind::Object,
                                            favourites,
                                        );
                                    }
                                    let filtered = crate::asset_catalog::filter_catalog(&nodes, &q);
                                    if filtered.is_empty() {
                                        // T-646 — a `class:` with an empty operand says so; a genuine
                                        // miss reads "No objects match."
                                        let msg = crate::asset_catalog::search_empty_message(&q, "objects");
                                        return view! {
                                            <p class="text-label-sm text-outline">{msg}</p>
                                        }
                                        .into_any();
                                    }
                                    return palette_rows(
                                        &filtered,
                                        0,
                                        &[],
                                        &[],
                                        no_collapse,
                                        PaletteKind::Object,
                                        favourites,
                                    );
                                }
                                match catalog.get() {
                                    CatalogState::Loading => {
                                        view! {
                                            <p class="text-label-sm text-outline">"Loading assets…"</p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Failed => {
                                        view! {
                                            <p class="text-label-sm text-outline">
                                                "Could not load the catalog."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Ready(nodes) if nodes.is_empty() => {
                                        view! {
                                            <p class="text-label-sm text-outline">"No placeable assets."</p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Ready(nodes) => {
                                        let q = search.get();
                                        if q.trim().is_empty() {
                                            // Track the collapse set so a chevron toggle re-renders the
                                            // tree (palette_rows reads it untracked).
                                            palette_collapsed.track();
                                            palette_rows(
                                                &nodes,
                                                0,
                                                &[],
                                                &[],
                                                palette_collapsed,
                                                PaletteKind::Character,
                                                favourites,
                                            )
                                        } else {
                                            let filtered =
                                                crate::asset_catalog::filter_catalog(&nodes, &q);
                                            if filtered.is_empty() {
                                                // T-646 — `class:` empty operand says so (see
                                                // `search_empty_message`); a real miss reads "No assets match."
                                                let msg = crate::asset_catalog::search_empty_message(
                                                    &q, "assets",
                                                );
                                                view! {
                                                    <p class="text-label-sm text-outline">{msg}</p>
                                                }
                                                    .into_any()
                                            } else {
                                                palette_rows(
                                                    &filtered,
                                                    0,
                                                    &[],
                                                    &[],
                                                    no_collapse,
                                                    PaletteKind::Character,
                                                    favourites,
                                                )
                                            }
                                        }
                                    }
                                }
                            }}
                        </div>
                    }
                        .into_any(),
                    // T-215 — Vehicles: the same tree machinery over the `kind == "vehicle"` rows.
                    // A leaf drop writes a `vehiclesById` row at the world point, owned by whichever
                    // Eden side the Factions tab's chips have selected (`active_side`) — the chips are
                    // not repeated here because there is one active side per editor, not per tab.
                    1 => view! {
                        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Vehicles"</h3>
                        <p class="mt-0.5 text-label-sm normal-case text-outline">
                            "Drag a vehicle onto the map to place it."
                        </p>
                        <input
                            type="search"
                            aria-label="Search vehicles"
                            placeholder="Search vehicles or class:…"
                            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                            on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                        />
                        // T-076 (RIGHT-CREW-001) — the manned/unmanned placement toggle, beside the
                        // Vehicles search. Checked ⇒ a placed vehicle is authored with crew (the Eden
                        // default); unchecked ⇒ `crewed: false` is stamped on the row. Native builds omit
                        // it (the `editor_ops` preference is wasm-only).
                        {crew_place_toggle(
                            #[cfg(target_arch = "wasm32")]
                            place_with_crew,
                        )}
                        <div class="mt-2">
                            {move || {
                                if objects_mode.get() {
                                    return view! {
                                        <p class="text-label-sm text-outline">
                                            "Objects place from the Factions tab while the Objects chip is selected."
                                        </p>
                                    }
                                        .into_any();
                                }
                                match vehicle_catalog.get() {
                                    CatalogState::Loading => {
                                        view! {
                                            <p class="text-label-sm text-outline">"Loading vehicles…"</p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Failed => {
                                        view! {
                                            <p class="text-label-sm text-outline">
                                                "Could not load the catalog."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Ready(nodes) if nodes.is_empty() => {
                                        view! {
                                            <p class="text-label-sm text-outline">
                                                "No placeable vehicles."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    CatalogState::Ready(nodes) => {
                                        let q = vehicle_search.get();
                                        if q.trim().is_empty() {
                                            vehicle_collapsed.track();
                                            palette_rows(
                                                &nodes,
                                                0,
                                                &[],
                                                &[],
                                                vehicle_collapsed,
                                                PaletteKind::Vehicle,
                                                favourites,
                                            )
                                        } else {
                                            let filtered =
                                                crate::asset_catalog::filter_catalog(&nodes, &q);
                                            if filtered.is_empty() {
                                                // T-646 — `class:` empty operand says so; else "No vehicles match."
                                                let msg = crate::asset_catalog::search_empty_message(
                                                    &q, "vehicles",
                                                );
                                                view! {
                                                    <p class="text-label-sm text-outline">{msg}</p>
                                                }
                                                    .into_any()
                                            } else {
                                                palette_rows(
                                                    &filtered,
                                                    0,
                                                    &[],
                                                    &[],
                                                    no_collapse,
                                                    PaletteKind::Vehicle,
                                                    favourites,
                                                )
                                            }
                                        }
                                    }
                                }
                            }}
                        </div>
                        {move || placed_vehicles_panel(doc_tick, registry_items, vehicle_expanded)}
                    }
                        .into_any(),
                    // T-582 — the zone draw tool. T-211 shipped the document layer and eleven
                    // mutators; this is the first thing that calls them.
                    3 => zones_panel(doc_tick, zone_selected),
                    // T-650 — the Compositions palette: save the current selection, list saved
                    // compositions grouped by category, arm a row to place, inline-edit rows.
                    4 => compositions_panel(doc_tick, comp_editing),
                    // T-079 — the Triggers palette (RIGHT-MODE-003): draw a trigger area (second
                    // consumer of the zone tool), list authored triggers, and edit the selected
                    // one's name / activation / owner link / rules. The owner-link line renders while
                    // a trigger is selected.
                    5 => triggers_panel(doc_tick, trigger_selected),
                    // T-695 — the Favourites collection (NEW-F5 / 3den E3): starred assets from
                    // every palette, resolved against the live registry rows.
                    6 => favourites_panel(favourites, registry_items),
                    _ => view! {
                        <p class="mt-3 text-label-sm normal-case text-outline">
                            "Marker placement lands in T-069."
                        </p>
                    }
                        .into_any(),
                }}
            </aside>
        }
    };
    // T-638 — collapsed: render ONLY the 24×24 stub (the expand chevron) at the outer top-RIGHT
    // corner, overlaying the map. Its wrapper in `mission_editor` shrinks to STUB_PX so the freed
    // area is click-through to the map. `justify-end` docks the stub to the corner.
    let stub = move || {
        view! {
            <div
                class="pointer-events-auto flex items-start justify-end bg-surface-container-lowest/55 backdrop-blur-xl"
                style=format!("width:{STUB_PX}px;height:{STUB_PX}px")
            >
                {collapse_chevron(collapsed, false)}
            </div>
        }
    };
    // T-638 — swap the whole dock for the corner stub while collapsed.
    move || {
        if collapsed.get() {
            stub().into_any()
        } else {
            full().into_any()
        }
    }
}

// ── T-650 — the Compositions palette (RIGHT-MODE-002) ────────────────────────────────────────────
//
// A saved composition is a reusable multi-entity stamp captured from the current selection. This
// panel is one function (native-stubbed with the same signature, like `zones_panel` /
// `placed_vehicles_panel`) with three jobs:
//   • SAVE (COMP-SAVE-001): a "Save composition…" header affordance, shown only when a selection
//     exists, that opens a small INLINE title/category form (not a new dialog file) and writes the
//     row from the current selection.
//   • LIST + PLACE (COMP-PLACE-001): the saved compositions grouped by category, each row showing
//     its title, an author line and an entity count; a row press ARMS the place (the T-647 armed
//     state — the canvas release stamps it as one undo step via `place_composition`).
//   • EDIT (COMP-EDIT-001 + the three ATTR-FIELD-COMP-* metadata fields): inline rename /
//     recategorize / delete (the T-666 hover-actions + inline-input idiom).

/// T-650 — the Compositions panel. `editing` holds the composition id currently in inline-edit
/// (rename/recategorize), or `None`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn compositions_panel(
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
) -> AnyView {
    use crate::eden_tree::{ROW, ROW_ACTIVE};
    use crate::editor_ops as ops;

    // The inline save form's open state + field buffers. Opening seeds no defaults; a blank title
    // reads "Untitled" on save so the row is always addressable.
    let save_open = RwSignal::new(false);
    let save_title = RwSignal::new(String::new());
    let save_category = RwSignal::new(String::new());
    let input_class = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60";

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Compositions"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    ops::composition_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Reusable multi-entity stamps. Select entities and Save; click a saved row to arm, then click the map to place."
        </p>

        // ── Save affordance (shown only when a selection exists) ──────────────────────────────
        {move || {
            let _ = doc_tick.get();
            if ops::selection_len() == 0 {
                // No selection → the affordance is not offered; make that explicit rather than
                // showing a button that no-ops.
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Select one or more placed entities to save a composition."
                    </p>
                }
                    .into_any();
            }
            if save_open.get() {
                // The inline form (title + category), not a new dialog file.
                view! {
                    <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                        <label class="block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                            "Title"
                        </label>
                        <input
                            type="text"
                            aria-label="Composition title"
                            placeholder="Fireteam + Technical"
                            class=input_class
                            prop:value=move || save_title.get()
                            on:input=move |ev| save_title.set(event_target_value(&ev))
                        />
                        <label class="mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                            "Category"
                        </label>
                        <input
                            type="text"
                            aria-label="Composition category"
                            placeholder="Infantry"
                            class=input_class
                            prop:value=move || save_category.get()
                            on:input=move |ev| save_category.set(event_target_value(&ev))
                        />
                        <div class="mt-2 flex gap-1.5">
                            <button
                                type="button"
                                class="flex-1 rounded-md bg-primary/25 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-primary/40"
                                on:click=move |_| {
                                    let title = save_title.get_untracked();
                                    let title = if title.trim().is_empty() {
                                        "Untitled".to_string()
                                    } else {
                                        title
                                    };
                                    let category = save_category.get_untracked();
                                    let category = if category.trim().is_empty() {
                                        "Uncategorized".to_string()
                                    } else {
                                        category
                                    };
                                    // Author = the current user's display string (as-authored) —
                                    // read off the AuthStore context; "You" when unauthenticated.
                                    let author = use_context::<crate::auth::AuthStore>()
                                        .and_then(|s| s.user.get_untracked().map(|u| u.username))
                                        .filter(|u| !u.is_empty())
                                        .unwrap_or_else(|| "You".to_string());
                                    let _ = ops::save_composition(title, category, author);
                                    save_open.set(false);
                                    save_title.set(String::new());
                                    save_category.set(String::new());
                                    doc_tick.update(|n| *n = n.wrapping_add(1));
                                }
                            >
                                "Save"
                            </button>
                            <button
                                type="button"
                                class="rounded-md px-2 py-1.5 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                                on:click=move |_| save_open.set(false)
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <button
                        type="button"
                        class="mt-3 w-full rounded-md border border-primary/40 px-2 py-1.5 text-label-sm text-primary transition-colors hover:bg-primary/15"
                        on:click=move |_| save_open.set(true)
                    >
                        {move || format!("Save composition… ({} selected)", ops::selection_len())}
                    </button>
                }
                    .into_any()
            }
        }}

        // ── The saved compositions, grouped by category ───────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let rows = ops::composition_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "No saved compositions yet."
                    </p>
                }
                    .into_any();
            }
            // `composition_rows` is sorted by (category, title), so a run of equal categories is
            // contiguous — group by walking and emitting a heading when the category changes.
            let mut groups: Vec<(String, Vec<crate::editor_ops::CompositionRow>)> = Vec::new();
            for r in rows {
                match groups.last_mut() {
                    Some((cat, list)) if *cat == r.category => list.push(r),
                    _ => groups.push((r.category.clone(), vec![r])),
                }
            }
            view! {
                <div class="mt-3 flex flex-col gap-2" role="list" aria-label="Saved compositions">
                    {groups
                        .into_iter()
                        .map(|(category, list)| {
                            let heading = if category.is_empty() {
                                "Uncategorized".to_string()
                            } else {
                                category
                            };
                            view! {
                                <div>
                                    <h4 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                                        {heading}
                                    </h4>
                                    <ul class="mt-1 flex flex-col gap-0.5" role="list">
                                        {list
                                            .into_iter()
                                            .map(|c| composition_row_view(c, doc_tick, editing, ROW, ROW_ACTIVE))
                                            .collect_view()}
                                    </ul>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }}
    }
    .into_any()
}

/// T-650 — one saved-composition row: press to ARM the place, hover actions to inline-edit / delete.
/// When `editing == this id`, the row swaps to inline title + category inputs (the T-666 idiom).
#[cfg(target_arch = "wasm32")]
fn composition_row_view(
    c: crate::editor_ops::CompositionRow,
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
    row: &'static str,
    row_active: &'static str,
) -> AnyView {
    use crate::editor_ops as ops;

    // `row_active` is part of the shared row vocabulary; a composition row does not carry a
    // persistent "selected" state (its selection IS the transient arm), so only `row` is used.
    let _ = row_active;
    let id = c.id.clone();
    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let is_editing = {
        let id = id.clone();
        move || editing.get().as_deref() == Some(id.as_str())
    };

    // The inline-edit buffers, seeded from the current row when the pencil opens. All three
    // ATTR-FIELD-COMP-* metadata fields (title/author/category) are editable here.
    let edit_title = RwSignal::new(c.title.clone());
    let edit_category = RwSignal::new(c.category.clone());
    let edit_author = RwSignal::new(c.author.clone());
    let input_class = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60";

    let title = c.title.clone();
    let title = if title.trim().is_empty() {
        "Untitled".to_string()
    } else {
        title
    };
    let author = c.author.clone();
    let count = c.entity_count;

    // Handlers, each cloning the id they need (Leptos closures are `move`).
    let arm_id = id.clone();
    let edit_open_id = id.clone();
    let (title_id, cat_id, del_id, save_id) = (id.clone(), id.clone(), id.clone(), id.clone());

    view! {
        <li>
            {move || {
                if is_editing() {
                    let (title_id, cat_id, save_id, edit_title, edit_category, edit_author) = (
                        title_id.clone(),
                        cat_id.clone(),
                        save_id.clone(),
                        edit_title,
                        edit_category,
                        edit_author,
                    );
                    view! {
                        <div class="rounded-md border border-primary/40 bg-primary/10 p-2">
                            <input
                                type="text"
                                aria-label="Composition title"
                                class=input_class
                                prop:value=move || edit_title.get()
                                on:input=move |ev| edit_title.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                aria-label="Composition category"
                                class=format!("{input_class} mt-1")
                                prop:value=move || edit_category.get()
                                on:input=move |ev| edit_category.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                aria-label="Composition author"
                                class=format!("{input_class} mt-1")
                                prop:value=move || edit_author.get()
                                on:input=move |ev| edit_author.set(event_target_value(&ev))
                            />
                            <div class="mt-1.5 flex gap-1.5">
                                <button
                                    type="button"
                                    class="flex-1 rounded-md bg-primary/25 px-2 py-1 text-label-sm text-on-surface transition-colors hover:bg-primary/40"
                                    on:click=move |_| {
                                        let t = edit_title.get_untracked();
                                        let t = if t.trim().is_empty() { "Untitled".to_string() } else { t };
                                        ops::rename_composition(save_id.clone(), t);
                                        ops::recategorize_composition(
                                            save_id.clone(),
                                            edit_category.get_untracked(),
                                        );
                                        ops::set_composition_author(
                                            save_id.clone(),
                                            edit_author.get_untracked(),
                                        );
                                        editing.set(None);
                                        bump();
                                    }
                                >
                                    "Save"
                                </button>
                                <button
                                    type="button"
                                    class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                                    on:click=move |_| editing.set(None)
                                >
                                    "Cancel"
                                </button>
                            </div>
                        </div>
                    }
                        .into_any()
                } else {
                    let (arm_id, edit_open_id, del_id) =
                        (arm_id.clone(), edit_open_id.clone(), del_id.clone());
                    let title = title.clone();
                    let author = author.clone();
                    view! {
                        <div class="group relative flex items-center gap-1">
                            <button
                                type="button"
                                title="Click to arm, then click the map to place"
                                class=format!("{row} flex-1")
                                on:pointerdown=move |_| {
                                    ops::begin_place_composition(arm_id.clone());
                                }
                            >
                                <MaterialIcon name="dashboard_customize" class="block text-sm" />
                                <span class="flex min-w-0 flex-col">
                                    <span class="truncate">{title}</span>
                                    <span class="truncate text-[10px] text-outline">
                                        {format!("by {author} · {count} item{}", if count == 1 { "" } else { "s" })}
                                    </span>
                                </span>
                            </button>
                            // Hover actions (T-666): edit + delete.
                            <button
                                type="button"
                                aria-label="Edit composition"
                                title="Rename / recategorize"
                                class="shrink-0 rounded-md p-1 text-on-surface-variant opacity-0 transition-opacity hover:bg-white/10 group-hover:opacity-100"
                                on:click=move |_| {
                                    editing.set(Some(edit_open_id.clone()));
                                }
                            >
                                <MaterialIcon name="edit" class="block text-sm" />
                            </button>
                            <button
                                type="button"
                                aria-label="Delete composition"
                                title="Delete"
                                class="shrink-0 rounded-md p-1 text-error opacity-0 transition-opacity hover:bg-error/15 group-hover:opacity-100"
                                on:click=move |_| {
                                    ops::delete_composition(del_id.clone());
                                    bump();
                                }
                            >
                                <MaterialIcon name="delete" class="block text-sm" />
                            </button>
                        </div>
                    }
                        .into_any()
                }
            }}
        </li>
    }
    .into_any()
}

/// Native shell: no document, so no compositions. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn compositions_panel(
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
) -> AnyView {
    let _ = (doc_tick, editing);
    ().into_any()
}

// ── T-079 — the Triggers palette (RIGHT-MODE-003 + CONN-TRG-OWNER-001) ────────────────────────────
//
// The Triggers tab authors trigger AREAS as a SECOND CONSUMER of the shipped zone draw tool: the
// draw controls call `editor_ops::begin_zone_draw(&activation, shape, DrawTarget::Trigger)` and the
// reshape buttons `begin_zone_reshape(&id, shape, DrawTarget::Trigger)` — the SAME calls the Zones
// panel makes with `DrawTarget::Zone`, so the whole geometry state machine is shared, not forked
// (the ticket's constraint). The panel adds only the trigger-specific surface:
//   • the ACTIVATION picker (presence/radio/timer — the T-676-runtime placeholder, stored not run),
//   • the OWNER picker (CONN-TRG-OWNER-001 — a `<select>` over placed slots/vehicles that writes
//     `ownerId`; this is the DATA EDGE, not the T-672 drag-connect gesture, whose context-menu row
//     stays disabled),
//   • the RULES controls, reusing `eden_zones::zone_rule_fields()` (the schema vocabulary) but
//     writing through `set_trigger_rule`,
//   • and the owner-link LINE overlay ([`TriggerOwnerLine`]) drawn while a trigger is selected.

/// T-079 — the Triggers panel. `selected` holds the selected trigger id (Attributes target + the
/// owner-link line's subject), or `None`. One function with a native stub, exactly like
/// [`zones_panel`] / [`compositions_panel`].
#[cfg(target_arch = "wasm32")]
pub(crate) fn triggers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::eden_tree::{ROW, ROW_ACTIVE};
    use crate::eden_zones::{humanize_token, DrawTarget, ZoneShape};
    use crate::editor_ops as ops;

    // The activation the NEXT draw will carry, seeded to the first of the three (presence).
    let draw_activation = RwSignal::new(
        ops::TRIGGER_ACTIVATIONS
            .first()
            .copied()
            .unwrap_or("presence")
            .to_string(),
    );

    let arm = move |shape: ZoneShape| {
        let activation = draw_activation.get_untracked();
        // SECOND CONSUMER: identical call to the Zones panel's `arm`, targeting triggers.
        ops::begin_zone_draw(&activation, shape, DrawTarget::Trigger);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Triggers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    ops::trigger_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Trigger areas. Pick an activation, then draw the area exactly like a zone — Circle: click centre then rim. Polygon: click each vertex, then Close. Select a trigger to set its owner."
        </p>

        // ── Draw controls (activation + the shared Circle/Polygon arm) ───────────────────────
        <label class="mt-3 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
            "Activation"
        </label>
        <select
            aria-label="Trigger activation to draw"
            class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
            on:change=move |ev| draw_activation.set(event_target_value(&ev))
        >
            {ops::TRIGGER_ACTIVATIONS
                .iter()
                .map(|a| {
                    let a = (*a).to_string();
                    let label = humanize_token(&a);
                    view! {
                        <option value=a.clone() selected=move || draw_activation.get() == a>
                            {label}
                        </option>
                    }
                })
                .collect_view()}
        </select>
        <div class="mt-2 flex gap-1.5">
            <button
                type="button"
                class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                on:click=move |_| arm(ZoneShape::Circle)
            >
                "Circle"
            </button>
            <button
                type="button"
                class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                on:click=move |_| arm(ZoneShape::Polygon)
            >
                "Polygon"
            </button>
        </div>

        // ── Live draw state (shared draft; shown only for a TRIGGER draw) ─────────────────────
        {move || {
            let _ = doc_tick.get();
            let Some(d) = ops::zone_draft() else {
                return ().into_any();
            };
            // The draft is shared with the Zones tool; only render the trigger-flavoured hint when
            // THIS draw is targeting triggers (so the Zones panel's own hint is the one shown for a
            // zone draw, and vice versa).
            if d.collection != DrawTarget::Trigger {
                return ().into_any();
            }
            let is_poly = d.shape == ZoneShape::Polygon;
            let n = d.verts.len();
            let hint = if is_poly {
                match n {
                    0 => "Click the first vertex.".to_string(),
                    1 | 2 => format!("{n} of 3 vertices — a ring needs at least three."),
                    _ => format!("{n} vertices. Close to commit."),
                }
            } else if d.centre.is_some() {
                "Centre set. Click the rim.".to_string()
            } else {
                "Click the centre.".to_string()
            };
            let can_close = is_poly && crate::eden_zones::polygon_is_committable(&d.verts);
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {
                            let shape = if is_poly { "polygon" } else { "circle" };
                            d.target.as_ref().map_or_else(
                                || format!("Drawing a {} trigger {shape}", humanize_token(&d.kind)),
                                |id| format!("Reshaping {id} as a {shape} — name, owner and rules are kept"),
                            )
                        }
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">{hint}</p>
                    <div class="mt-1.5 flex gap-1.5">
                        {is_poly
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        disabled=!can_close
                                        class="rounded-md bg-primary/25 px-2 py-1 text-label-sm text-on-surface transition-colors hover:bg-primary/40 disabled:opacity-30 disabled:hover:bg-primary/25"
                                        on:click=move |_| {
                                            ops::close_zone_polygon();
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        "Close ring"
                                    </button>
                                    <button
                                        type="button"
                                        disabled=n == 0
                                        class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30"
                                        on:click=move |_| {
                                            ops::zone_draw_pop_vertex();
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        "Undo vertex"
                                    </button>
                                }
                            })}
                        <button
                            type="button"
                            class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                ops::cancel_zone_draw();
                                doc_tick.update(|n| *n = n.wrapping_add(1));
                            }
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            }
                .into_any()
        }}

        // ── Authored triggers ────────────────────────────────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let rows = ops::trigger_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "No triggers yet."
                    </p>
                }
                    .into_any();
            }
            view! {
                <ul class="mt-3 flex flex-col gap-0.5" role="list" aria-label="Authored triggers">
                    {rows
                        .into_iter()
                        .map(|t| {
                            let id = t.id.clone();
                            let sel_id = t.id.clone();
                            let sel_id2 = t.id.clone();
                            let title = t
                                .name
                                .clone()
                                .filter(|l| !l.is_empty())
                                .unwrap_or_else(|| format!("Trigger {}", t.id));
                            let summary = t.shape_summary();
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        aria-pressed=move || selected.get().as_deref() == Some(sel_id.as_str())
                                        class=move || {
                                            if selected.get().as_deref() == Some(sel_id2.as_str()) {
                                                ROW_ACTIVE
                                            } else {
                                                ROW
                                            }
                                        }
                                        on:click=move |_| selected.set(Some(id.clone()))
                                    >
                                        <MaterialIcon
                                            name=if t.circle.is_some() {
                                                "radio_button_unchecked"
                                            } else {
                                                "pentagon"
                                            }
                                            class="block text-sm"
                                        />
                                        <span class="truncate">{title}</span>
                                        <span class="ml-auto shrink-0 font-mono text-code-md text-outline">
                                            {summary}
                                        </span>
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            }
                .into_any()
        }}

        // ── Attributes for the selected trigger ──────────────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let Some(id) = selected.get() else {
                return ().into_any();
            };
            let Some(t) = ops::trigger_rows().into_iter().find(|r| r.id == id) else {
                // Deleted underneath us (undo, or a reload that dropped it).
                return ().into_any();
            };
            trigger_attributes(t, doc_tick, selected).into_any()
        }}

        // T-079 (CONN-TRG-OWNER-001) — the owner-link line. Rendered here (inside the panel, which is
        // the only place `selected` is live) via a Portal so the SVG escapes the dock's clipping /
        // backdrop-filter box and spans the viewport — the ruler-overlay idiom, mounted from an owned
        // module (this slice does not own `mission_editor` / `ruler_tool`, so it cannot add a mount
        // there). Draws nothing when no trigger is selected or the owner is dangling.
        <TriggerOwnerLine selected doc_tick />
    }
    .into_any()
}

/// T-079 — the Attributes panel for one trigger: name, activation, the OWNER picker
/// (CONN-TRG-OWNER-001), reshape, schema-driven rules, delete. The [`crate::eden_zones`]
/// `zone_attributes` twin, with the owner picker + activation in place of zone label/faction/type.
#[cfg(target_arch = "wasm32")]
fn trigger_attributes(
    t: crate::editor_ops::TriggerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::eden_zones::{humanize_token, DrawTarget, ZoneShape};
    use crate::editor_ops as ops;

    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let tid = t.id.clone();
    let input_class = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60";
    let field_label =
        "mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant";

    let (id_name, id_activation, id_owner, id_delete) =
        (tid.clone(), tid.clone(), tid.clone(), tid.clone());
    let rules = t.rules.clone();
    // The owner picker's options are read ONCE per render of this panel (doc_tick above re-renders
    // it). Includes the current owner even if it is now dangling, so the select can show it.
    let owner_opts = ops::placed_owner_options();
    let current_owner = t.owner_id.clone();
    let current_owner_dangling = current_owner
        .as_ref()
        .is_some_and(|o| !owner_opts.iter().any(|opt| &opt.id == o));

    view! {
        <div class="mt-3 border-t border-white/10 pt-2">
            <h4 class="text-label-md font-semibold text-on-surface">
                {format!("Attributes — {}", t.id)}
            </h4>

            // `name` — optional; Clear removes the key, empty box sends None (mirrors zone label).
            <label class=field_label>"Name"</label>
            <input
                type="text"
                aria-label="Trigger name"
                placeholder="(unnamed)"
                class=input_class
                prop:value=t.name.clone().unwrap_or_default()
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    let next = (!v.trim().is_empty()).then_some(v);
                    ops::set_trigger_name(&id_name, next);
                    bump();
                }
            />

            <label class=field_label>"Activation"</label>
            <select
                aria-label="Trigger activation"
                class=input_class
                on:change=move |ev| {
                    ops::set_trigger_activation(&id_activation, &event_target_value(&ev));
                    bump();
                }
            >
                {
                    let current = t.activation.clone();
                    ops::TRIGGER_ACTIVATIONS
                        .iter()
                        .map(|a| {
                            let a = (*a).to_string();
                            let is = a == current;
                            let label = humanize_token(&a);
                            view! { <option value=a selected=is>{label}</option> }
                        })
                        .collect_view()
                }
            </select>

            // ── Owner picker (CONN-TRG-OWNER-001) — the DATA EDGE, not the drag-connect gesture ──
            <label class=field_label>"Owner"</label>
            <select
                aria-label="Trigger owner"
                class=input_class
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    // The empty option is "unowned" → clear; any other value is a placed entity id.
                    let next = (!v.is_empty()).then_some(v);
                    ops::set_trigger_owner(&id_owner, next);
                    bump();
                }
            >
                <option value="" selected=current_owner.is_none()>
                    "(unowned)"
                </option>
                // A dangling current owner (its entity was deleted) is still shown, marked, so the
                // select reflects the stored edge rather than silently snapping to "(unowned)".
                {current_owner_dangling
                    .then(|| {
                        let o = current_owner.clone().unwrap_or_default();
                        view! {
                            <option value=o.clone() selected=true>
                                {format!("{o} (deleted)")}
                            </option>
                        }
                    })}
                {
                    let current_owner = current_owner.clone();
                    owner_opts
                        .into_iter()
                        .map(|opt| {
                            let is = current_owner.as_deref() == Some(opt.id.as_str());
                            view! { <option value=opt.id selected=is>{opt.label}</option> }
                        })
                        .collect_view()
                }
            </select>

            // Reshape — SECOND CONSUMER of the zone tool's reshape (whole-`shape` replacement, so
            // name / activation / owner / rules survive).
            <label class=field_label>"Shape"</label>
            <div class="flex gap-1.5">
                {
                    let (a, b) = (tid.clone(), tid.clone());
                    view! {
                        <button
                            type="button"
                            title="Redraw this trigger as a circle — click the centre, then the rim"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                ops::begin_zone_reshape(&a, ZoneShape::Circle, DrawTarget::Trigger);
                                bump();
                            }
                        >
                            "Redraw circle"
                        </button>
                        <button
                            type="button"
                            title="Redraw this trigger as a polygon — click each vertex, then Close"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                ops::begin_zone_reshape(&b, ZoneShape::Polygon, DrawTarget::Trigger);
                                bump();
                            }
                        >
                            "Redraw polygon"
                        </button>
                    }
                }
            </div>

            <h4 class="mt-3 text-label-md font-semibold text-on-surface">"Rules"</h4>
            <p class="mt-0.5 text-label-sm normal-case text-outline">
                "Reuses the mission schema's zoneRules vocabulary — the same controls the Zones panel draws. Blank means the key is not authored and the mod's default applies."
            </p>
            {crate::eden_zones::zone_rule_fields()
                .into_iter()
                .map(|f| trigger_rule_control(tid.clone(), f, rules.clone(), doc_tick))
                .collect_view()}

            <button
                type="button"
                class="mt-3 w-full rounded-md border border-error/40 px-2 py-1.5 text-label-sm text-error transition-colors hover:bg-error/15"
                on:click=move |_| {
                    ops::delete_trigger(&id_delete);
                    selected.set(None);
                    bump();
                }
            >
                "Delete trigger"
            </button>
        </div>
    }
    .into_any()
}

/// T-079 — ONE `$defs/zoneRules` property as a control for a TRIGGER, writing through
/// `set_trigger_rule`. Reuses `eden_zones`'s vocabulary machinery ([`ZoneRuleField`] /
/// [`ZoneRuleKind`], read from the schema by `zone_rule_fields`) — the load-bearing "no second
/// vocabulary" reuse — and mirrors `eden_zones::zone_rule_control`'s rendering, differing only in the
/// mutator it calls. Clearing a control removes the key (the mod's default returns), exactly as the
/// zone control does.
#[cfg(target_arch = "wasm32")]
fn trigger_rule_control(
    trigger_id: String,
    f: crate::eden_zones::ZoneRuleField,
    rules: serde_json::Value,
    doc_tick: RwSignal<u64>,
) -> AnyView {
    use crate::eden_zones::{humanize_key, humanize_token, ZoneRuleKind};
    use crate::editor_ops as ops;

    let current = rules.get(&f.key).cloned();
    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let label = humanize_key(&f.key);
    let doc = f.doc.clone();
    let key = f.key.clone();
    let row = "mt-2";
    let ctl = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60";

    let body = match f.kind {
        ZoneRuleKind::Bool { default } => {
            let checked = current.as_ref().and_then(serde_json::Value::as_bool);
            let k = key.clone();
            view! {
                <label class="mt-2 flex items-center gap-2 text-label-sm text-on-surface">
                    <input
                        type="checkbox"
                        aria-label=label.clone()
                        prop:checked=checked.unwrap_or(default)
                        prop:indeterminate=checked.is_none()
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            ops::set_trigger_rule(&trigger_id, &k, Some(serde_json::Value::Bool(on)));
                            bump();
                        }
                    />
                    <span>{label.clone()}</span>
                    <span class="ml-auto font-mono text-code-md text-outline">
                        {format!("default {default}")}
                    </span>
                </label>
            }
            .into_any()
        }
        ZoneRuleKind::Choice { options, default } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <select
                        aria-label=label.clone()
                        class=ctl
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.is_empty()).then(|| serde_json::Value::String(v));
                            ops::set_trigger_rule(&trigger_id, &k, next);
                            bump();
                        }
                    >
                        <option value="" selected=cur.is_none()>
                            {default
                                .as_ref()
                                .map_or_else(
                                    || "(not authored)".to_string(),
                                    |d| format!("(not authored — default {d})"),
                                )}
                        </option>
                        {options
                            .into_iter()
                            .map(|o| {
                                let is = cur.as_deref() == Some(o.as_str());
                                let l = humanize_token(&o);
                                view! { <option value=o selected=is>{l}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Number {
            default,
            minimum,
            exclusive_minimum,
            maximum,
            integer,
        } => {
            let cur = current.as_ref().and_then(serde_json::Value::as_f64);
            let k = key.clone();
            let step = if integer { 1.0 } else { 0.1 };
            let min_attr = minimum.or_else(|| exclusive_minimum.map(|m| m + step));
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="number"
                        aria-label=label.clone()
                        class=ctl
                        step=step
                        min=min_attr.map(|m| m.to_string())
                        max=maximum.map(|m| m.to_string())
                        placeholder=default
                            .map_or_else(
                                || "(not authored)".to_string(),
                                |d| format!("(not authored — default {d})"),
                            )
                        prop:value=cur.map(|v| v.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let raw = event_target_value(&ev);
                            let next = if raw.trim().is_empty() {
                                None
                            } else {
                                raw.trim()
                                    .parse::<f64>()
                                    .ok()
                                    .and_then(serde_json::Number::from_f64)
                                    .map(serde_json::Value::Number)
                            };
                            if next.is_some() || raw.trim().is_empty() {
                                ops::set_trigger_rule(&trigger_id, &k, next);
                                bump();
                            }
                        }
                    />
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Text { default, pattern } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="text"
                        aria-label=label.clone()
                        class=ctl
                        pattern=pattern
                        placeholder=default.unwrap_or_else(|| "(not authored)".to_string())
                        prop:value=cur
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.trim().is_empty())
                                .then(|| serde_json::Value::String(v.trim().to_string()));
                            ops::set_trigger_rule(&trigger_id, &k, next);
                            bump();
                        }
                    />
                </div>
            }
            .into_any()
        }
    };
    view! {
        <div title=doc>{body}</div>
    }
    .into_any()
}

/// T-079 (CONN-TRG-OWNER-001) — the owner-link line overlay: a thin line from the selected trigger's
/// centre to its owner entity, drawn while the trigger is selected. Uses the ruler-overlay idiom
/// exactly — a `pointer-events-none` SVG that reads the live camera off `world_assets::camera_snapshot`
/// and re-projects off the `cursor` (pan) + `doc_tick` (any edit) heartbeats — but is rendered
/// through a [`leptos::portal::Portal`] to `document.body` so the SVG escapes the right dock's
/// `overflow`/`backdrop-filter` clipping box and spans the viewport. (This slice owns neither
/// `mission_editor` nor `ruler_tool`, so it cannot add a shared overlay mount there; the Portal keeps
/// the whole line self-contained in an owned file.) The projection math is the pure, native-tested
/// [`crate::eden_zones::project_owner_line`]. Nothing renders when no trigger is selected or the
/// owner is dangling (`owner_line_world` returns `None`).
#[cfg(target_arch = "wasm32")]
#[component]
fn TriggerOwnerLine(selected: RwSignal<Option<String>>, doc_tick: RwSignal<u64>) -> impl IntoView {
    use crate::editor_ops as ops;
    use leptos::portal::Portal;

    // Pan/zoom heartbeat. `mission_editor` threads its `cursor`/`debug_hud` heartbeats into the ruler
    // overlay, but this component is mounted from the dock and receives neither (wiring them would be
    // a `mission_editor` edit — not this slice's to make). So the line re-projects off a SELF-CONTAINED
    // rAF that only ticks while a trigger is selected (the sole moment the line is drawn), and stops
    // itself on unmount (leaving the Triggers tab). It early-returns every frame `selected` is `None`,
    // so an open-but-idle Triggers tab costs one no-op closure per frame and no reprojection.
    let tick = RwSignal::new(0u64);
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let disposed = Arc::new(AtomicBool::new(false));
        // The self-referential rAF-closure cell — the same shape `mission_editor::start_raf` uses.
        #[allow(clippy::type_complexity)]
        let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
        let g = f.clone();
        {
            let disposed = disposed.clone();
            *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
                if disposed.load(Ordering::Relaxed) {
                    f.borrow_mut().take(); // drop the loop closure — no further frames
                    return;
                }
                // Only pay for a reprojection while a trigger is selected; the projection closure
                // subscribes to `tick`, so bumping it re-runs the projection against the live camera.
                if selected.get_untracked().is_some() {
                    tick.update(|n| *n = n.wrapping_add(1));
                }
                let cb_ref = f.borrow();
                if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                    let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
                }
            }) as Box<dyn FnMut()>));
        }
        let cb_ref = g.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
        on_cleanup(move || disposed.store(true, Ordering::Relaxed));
    }

    let projected = move || -> Option<crate::eden_zones::ProjectedOwnerLine> {
        // Subscribe to selection, doc edits (owner assign / geometry / delete) and the pan heartbeat
        // (`tick`, bumped per rAF while selected). The camera is read live off the snapshot.
        let _ = doc_tick.get();
        let _ = tick.get();
        let sel = selected.get();
        let (world_a, world_b) = ops::owner_line_world(sel.as_deref())?;
        let (tx, ty, zoom) = crate::world_assets::camera_snapshot()?;
        let win = web_sys::window()?;
        let vw = win.inner_width().ok().and_then(|v| v.as_f64())?;
        let vh = win.inner_height().ok().and_then(|v| v.as_f64())?;
        if vw <= 0.0 || vh <= 0.0 {
            return None;
        }
        // Full-bleed canvas → the camera viewport IS the whole window, built exactly as the ruler
        // overlay does (`select_tool::frozen_camera`).
        let cam = crate::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
        let project = move |x: f64, y: f64| {
            let p = cam.project([x, y, 0.0]);
            (p[0], p[1])
        };
        Some(crate::eden_zones::project_owner_line(
            world_a, world_b, project,
        ))
    };

    view! {
        <Portal>
            <svg
                data-trigger-owner-line
                class="pointer-events-none fixed inset-0 z-10"
                width="100%"
                height="100%"
            >
                {move || {
                    projected().map(|l| {
                        view! {
                            <line
                                x1=format!("{:.1}", l.x1)
                                y1=format!("{:.1}", l.y1)
                                x2=format!("{:.1}", l.x2)
                                y2=format!("{:.1}", l.y2)
                                class="stroke-primary/80"
                                stroke-width="1.5"
                                stroke-dasharray="5 3"
                            />
                        }
                    })
                }}
            </svg>
        </Portal>
    }
}

/// Native shell: no document, so no triggers. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn triggers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_eden_chip, custom_chip_visible, eden_chip_selected, EdenChip, EdenSubmode,
        EDEN_CUSTOM_CHIP, EDEN_SIDE_CHIPS,
    };
    use leptos::prelude::*;

    /// T-646 (RIGHT-SUBMODE-001) — the Custom slot appears **only under Groups**. The predicate is
    /// true for the Groups sub-mode and false for every other, and the tab→sub-mode map places the
    /// Objects chip (Factions tab + objects_mode) into Objects — so flipping to Objects hides Custom
    /// even though it is the same tab. Perturbation RED: were `custom_chip_visible` to admit any
    /// non-Groups sub-mode, one of the `!` assertions below fails.
    #[test]
    fn custom_chip_only_under_groups() {
        assert!(
            custom_chip_visible(EdenSubmode::Groups),
            "Groups shows Custom"
        );
        assert!(!custom_chip_visible(EdenSubmode::Vehicles));
        assert!(!custom_chip_visible(EdenSubmode::Objects));
        assert!(!custom_chip_visible(EdenSubmode::Markers));
        assert!(!custom_chip_visible(EdenSubmode::Zones));
        // T-650 — the Compositions tab is not a Groups surface, so it hides Custom too.
        assert!(!custom_chip_visible(EdenSubmode::Compositions));
        // T-079 — nor is the Triggers tab a Groups surface.
        assert!(!custom_chip_visible(EdenSubmode::Triggers));
        // T-695 — nor is the Favourites tab (a persistent collection, not a place surface).
        assert!(!custom_chip_visible(EdenSubmode::Favourites));
        assert_eq!(EdenSubmode::from_tab(6, false), EdenSubmode::Favourites);
        assert_eq!(
            EdenSubmode::from_tab(6, true),
            EdenSubmode::Favourites,
            "the Objects chip splits the Factions tab alone — tab index wins"
        );

        // Tab → sub-mode: Factions (tab 0) is Groups unless the Objects chip is on.
        assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
        assert_eq!(EdenSubmode::from_tab(0, true), EdenSubmode::Objects);
        assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
        assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
        assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
        // T-650 — tab 4 is the Compositions surface.
        assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
        // T-079 — tab 5 is the Triggers surface.
        assert_eq!(EdenSubmode::from_tab(5, false), EdenSubmode::Triggers);

        // The end-to-end visibility rule the render uses: Custom on the Factions tab iff not Objects,
        // and never on any other tab.
        assert!(
            custom_chip_visible(EdenSubmode::from_tab(0, false)),
            "Factions+side → Custom shown"
        );
        assert!(
            !custom_chip_visible(EdenSubmode::from_tab(0, true)),
            "Factions+Objects → Custom hidden"
        );
        for tab in [1usize, 2, 3, 4, 5, 6] {
            assert!(
                !custom_chip_visible(EdenSubmode::from_tab(tab, false)),
                "Custom hidden on tab {tab}"
            );
        }

        // The Custom slot is a SIXTH slot, distinct from the pinned 4-chip side row — it must not be
        // one of the side labels (that would fold it into the always-on row).
        assert_eq!(EDEN_CUSTOM_CHIP, "Custom");
        assert!(
            !EDEN_SIDE_CHIPS.iter().any(|c| *c == EDEN_CUSTOM_CHIP),
            "Custom is not a side chip"
        );
    }

    /// The tab was a one-line promise that placement would arrive in T-070, and the only vehicle
    /// path was the ORBAT Manager's derived position. Both halves of the replacement are pinned:
    /// the placeholder is gone, and a Vehicles leaf arms the **vehicle** place, not the character
    /// one.
    ///
    /// Source inspection, following `orbat_manager`'s precedent, because the thing under test is a
    /// Leptos view whose handlers are `#[cfg(target_arch = "wasm32")]` — a native test cannot mount
    /// it or fire the `pointerdown`. What it can do is fail loudly if the wiring is unpicked.
    ///
    /// **Every needle is assembled at run time, and must stay that way.** This test searches the
    /// file it is written in, so a needle spelled out contiguously — in an assertion, or in prose
    /// *about* an assertion — puts itself into the haystack: absence checks can then never pass and
    /// presence checks can never fail. That happened three times while writing this, and the third
    /// was caught only by perturbation: a bare-symbol `contains` for the vehicle arm stayed GREEN
    /// after the leaf was rewired to the character path, because the test's own literal satisfied
    /// it. This program's signature defect in miniature — a check reporting success over an input
    /// it never examined. The needle is therefore the whole call **expression**, which the file's
    /// prose (which names the bare function) never contains.
    #[test]
    fn vehicles_tab_places_instead_of_promising() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        let stub = |what: &str, ticket: &str| format!("{what} placement {} {ticket}.", "lands in");
        let arm = |f: &str| format!("editor_ops::{f}{}", "(payload.clone())");

        assert!(
            !SRC.contains(&stub("Vehicle", "T-070")),
            "the Vehicles tab placeholder must be gone"
        );
        assert!(
            SRC.contains(&arm("begin_place_vehicle")),
            "a Vehicles leaf must arm the vehicle place path"
        );
        // The Markers tab is deliberately still a stub (T-069) — if this ever stops being true the
        // assertion above stops proving that THIS tab is the one that got wired.
        assert!(
            SRC.contains(&stub("Marker", "T-069")),
            "the Markers stub is out of scope and must be untouched"
        );

        let ops = include_str!("editor_ops.rs");
        assert!(
            ops.contains("pub fn begin_place_vehicle"),
            "editor_ops must expose the vehicle arm"
        );
        assert!(
            ops.contains("core.add_vehicle("),
            "the vehicle place must reach the core mutator"
        );
    }

    /// E1 + E5 — exact chip list; no CIV; no F-key labels in the chip row source of truth.
    #[test]
    fn eden_side_chips_labels_no_civ() {
        assert_eq!(EDEN_SIDE_CHIPS, &["BLUFOR", "OPFOR", "INDFOR", "Objects"]);
        assert_eq!(EDEN_SIDE_CHIPS.len(), 4);
        assert!(!EDEN_SIDE_CHIPS.iter().any(|c| *c == "CIV"));
        for label in EDEN_SIDE_CHIPS {
            assert!(
                !label.starts_with('F') || label == &"Objects",
                "F1–F6 mode row banned: {label}"
            );
            // F1…F6 are two-char labels like "F1" — none of our chips match.
            assert!(!matches!(*label, "F1" | "F2" | "F3" | "F4" | "F5" | "F6"));
        }
    }

    /// E2 — OPFOR chip writes the same side string `place_at` / OpsCtx read.
    #[test]
    fn apply_eden_chip_opfor_sets_active_side() {
        let active_side = RwSignal::new(String::from("BLUFOR"));
        let objects_mode = RwSignal::new(true);
        apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
        assert_eq!(active_side.get_untracked(), "OPFOR");
        assert!(!objects_mode.get_untracked());
        assert!(eden_chip_selected(
            EdenChip::Opfor,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
    }

    /// E3 — Objects chip flips objects_mode without clobbering side; coming-soon stub is gone.
    #[test]
    fn objects_chip_enables_mode_without_clobbering_side() {
        // T-254 — stub constant name must not remain (split so this assert's own source cannot
        // false-fail the contains check).
        let src = include_str!("eden_dock_right.rs");
        let stub_const = ["OBJECTS_", "COMING_", "SOON"].concat();
        assert!(
            !src.contains(&stub_const),
            "Objects stub constant must be removed"
        );
        assert!(
            src.contains("begin_place_object") || src.contains("PaletteKind::Object"),
            "Objects palette must arm object places"
        );
        let active_side = RwSignal::new(String::from("OPFOR"));
        let objects_mode = RwSignal::new(false);
        apply_eden_chip(EdenChip::Objects, active_side, objects_mode);
        assert!(objects_mode.get_untracked());
        assert_eq!(
            active_side.get_untracked(),
            "OPFOR",
            "Objects must leave last side intact"
        );
        assert!(eden_chip_selected(
            EdenChip::Objects,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
        assert!(!eden_chip_selected(
            EdenChip::Opfor,
            &active_side.get_untracked(),
            objects_mode.get_untracked()
        ));
    }

    /// T-255 — chip write + `build_catalog_tree(_, side)` is the dock rebuild contract: OPFOR
    /// after a BLUFOR default must drop NATO leaves and keep only the USSR perturbation row.
    #[test]
    fn eden_chip_side_rebuilds_filtered_catalog() {
        use crate::asset_catalog::build_catalog_tree;
        use crate::dto::RegistryResponse;

        let golden: RegistryResponse =
            serde_json::from_str(include_str!("../tests/fixtures/api/GET__registry.json"))
                .expect("golden");
        let mut items = golden.data;
        items.push(
            serde_json::from_value(serde_json::json!({
                "id": "ussr",
                "modpack_id": "mp",
                "resource_name": "{DCB41B3746FDD1BE}Prefabs/Characters/Factions/OPFOR/USSR_Army/Character_USSR_Rifleman.et",
                "display_name": "USSR Rifleman",
                "category": "ArmaReforger/Characters/Factions/OPFOR/USSR_Army/Rifleman",
                "kind": "character",
                "sort_order": 99,
                "created_at": "2026-07-26T00:00:00Z",
                "updated_at": "2026-07-26T00:00:00Z",
            }))
            .expect("ussr row"),
        );

        let active_side = RwSignal::new(String::from("BLUFOR"));
        let objects_mode = RwSignal::new(false);
        let blufor_tree = build_catalog_tree(&items, &active_side.get_untracked());
        assert!(
            blufor_tree.iter().any(|n| n.id == "NATO"),
            "default BLUFOR chip keeps NATO"
        );

        apply_eden_chip(EdenChip::Opfor, active_side, objects_mode);
        let opfor_tree = build_catalog_tree(&items, &active_side.get_untracked());
        assert_eq!(active_side.get_untracked(), "OPFOR");
        assert_eq!(opfor_tree.len(), 1);
        assert!(
            !opfor_tree.iter().any(|n| n.id == "NATO"),
            "OPFOR chip must rebuild without NATO — got {:?}",
            opfor_tree.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
        );
        fn has_leaf(nodes: &[crate::asset_catalog::CatalogNode], label: &str) -> bool {
            nodes
                .iter()
                .any(|n| (n.payload.is_some() && n.label == label) || has_leaf(&n.children, label))
        }
        assert!(
            has_leaf(&opfor_tree, "USSR Rifleman"),
            "OPFOR rebuild must keep USSR Rifleman"
        );
        assert!(!has_leaf(&opfor_tree, "US Rifleman"));
    }

    /// T-650 (RIGHT-MODE-002) — the Compositions palette mode + tab exist and map to their own
    /// sub-mode. The pure pins: the tab-index → sub-mode function reports Compositions for tab 4, and
    /// tab 4 is NOT one of the pre-existing surfaces (so it did not silently reuse another tab's
    /// slot).
    #[test]
    fn compositions_tab_maps_to_its_own_submode() {
        assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);
        // Objects mode on the Compositions tab does not turn it into Objects (that split is the
        // Factions tab's alone) — tab index wins.
        assert_eq!(EdenSubmode::from_tab(4, true), EdenSubmode::Compositions);
        // The four pre-existing surfaces keep their tabs.
        assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
        assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
        assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
        assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
    }

    /// T-650 — the Compositions palette is a LIVE surface (not a T-069-style stub) wired to the
    /// editor-ops save/arm/edit seam. Source inspection, following `vehicles_tab_places_instead_of
    /// _promising`, because `compositions_panel` is a wasm-only view a native test cannot mount.
    ///
    /// **Every needle is assembled at run time** (the file's own hard-won rule): this test's source
    /// is part of the haystack it searches, so a contiguous literal would make an absence check
    /// unfailable and a presence check unpassable. Each needle is therefore split and re-joined.
    #[test]
    fn compositions_tab_is_wired_not_stubbed() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        // The panel aliases `use crate::editor_ops as ops`, so the calls read `ops::<fn>(`.
        let call = |f: &str| format!("ops::{f}(");

        // The tab strip renders a Compositions tab at index 4.
        assert!(
            SRC.contains(&format!("tab_btn(4, {:?})", "Compositions")),
            "a Compositions tab must be in the tab strip"
        );
        // The panel dispatch routes tab 4 to the compositions panel.
        assert!(
            SRC.contains(&format!("4 => {}(", "compositions_panel")),
            "tab 4 must dispatch to the compositions panel"
        );
        // SAVE (COMP-SAVE-001): the panel reaches the capture seam.
        assert!(
            SRC.contains(&call("save_composition")),
            "the Save affordance must call save_composition"
        );
        // PLACE (COMP-PLACE-001): a row press ARMS via the T-647 arm seam.
        assert!(
            SRC.contains(&call("begin_place_composition")),
            "a composition row must arm the place"
        );
        // EDIT (COMP-EDIT-001) + the three ATTR-FIELD-COMP-* metadata fields, inline.
        for f in [
            "rename_composition",
            "recategorize_composition",
            "set_composition_author",
            "delete_composition",
        ] {
            assert!(
                SRC.contains(&call(f)),
                "the inline edit must call {f} (COMP-EDIT-001 / metadata fields)"
            );
        }

        // The editor-ops seam actually exposes those functions AND the place reaches the core
        // one-undo-step mutator (the claim the store round-trip test rests on).
        let ops = include_str!("editor_ops.rs");
        assert!(
            ops.contains("pub fn save_composition")
                && ops.contains("pub fn begin_place_composition"),
            "editor_ops must expose the composition save + arm"
        );
        assert!(
            ops.contains("core.place_composition("),
            "the composition place must reach the core mutator"
        );
    }

    /// T-650 — the composition arm rides the SAME T-647 armed-state machine as the object place: its
    /// arm is a `Pending::Composition`, and `has_pending()` (which gates the map's place branch and
    /// ghost) is true while one is armed. Source-inspection pin that the arm goes through `arm(…)`
    /// like `begin_place_object`, not a bespoke path.
    #[test]
    fn composition_arm_rides_the_shared_pending_machine() {
        let ops = include_str!("editor_ops.rs");
        // The arm variant exists and the public arm fn routes through the shared `arm(…)`.
        assert!(
            ops.contains("Composition(String)"),
            "the Pending enum must carry a Composition arm"
        );
        let arm_call = format!("arm(Pending::{}(", "Composition");
        assert!(
            ops.contains(&arm_call),
            "begin_place_composition must route through the shared arm() like begin_place_object"
        );
        // place_at_impl handles the Composition arm (the one-shot consume + place).
        assert!(
            ops.contains(&format!("Pending::{}(comp_id)", "Composition")),
            "place_at_impl must consume the Composition arm on a canvas release"
        );
    }

    /// T-079 (RIGHT-MODE-003) — the Triggers palette mode + tab exist and map to their own sub-mode.
    /// The pure pins: tab-index → sub-mode reports Triggers for tab 5, and tab 5 is NOT one of the
    /// pre-existing surfaces (so it did not silently reuse another tab's slot). Also that a
    /// `PaletteKind::Trigger` variant was added (the palette-mode vocabulary the ticket asks to grow).
    #[test]
    fn triggers_tab_maps_to_its_own_submode() {
        assert_eq!(EdenSubmode::from_tab(5, false), EdenSubmode::Triggers);
        // Objects mode on the Triggers tab does not turn it into Objects (that split is the Factions
        // tab's alone) — tab index wins.
        assert_eq!(EdenSubmode::from_tab(5, true), EdenSubmode::Triggers);
        // The pre-existing surfaces keep their tabs.
        assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
        assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
        assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
        assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
        assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);

        // The PaletteKind vocabulary grew a Trigger variant with its own glyph + title (the mode
        // exists in the enum, not just the tab). Source-inspected because `PaletteKind` is a private
        // enum a native test cannot name without pulling the wasm-gated module graph. The needle is
        // assembled so this test's own text is not the thing that satisfies the check.
        let src = include_str!("eden_dock_right.rs");
        let variant = ["PaletteKind", "::", "Trigger"].concat();
        assert!(
            src.contains(&variant),
            "the palette-mode vocabulary must carry a Trigger variant"
        );
    }

    /// T-079 (RIGHT-MODE-003) — the Triggers palette is a LIVE surface wired to the editor-ops
    /// trigger seam, not a T-069-style stub. Source inspection (the `compositions_tab_is_wired_not
    /// _stubbed` precedent): the panel is a wasm-only view a native test cannot mount.
    ///
    /// **Every needle is assembled at run time** (this file searches itself, so a contiguous literal
    /// would make an absence check unfailable and a presence check unpassable — the hard-won rule at
    /// the top of this file). Each needle is split and re-joined.
    #[test]
    fn triggers_tab_is_wired_not_stubbed() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        let call = |f: &str| format!("ops::{f}(");

        // The tab strip renders a Triggers tab at index 5.
        assert!(
            SRC.contains(&format!("tab_btn(5, {:?})", "Triggers")),
            "a Triggers tab must be in the tab strip"
        );
        // The panel dispatch routes tab 5 to the triggers panel.
        assert!(
            SRC.contains(&format!("5 => {}(", "triggers_panel")),
            "tab 5 must dispatch to the triggers panel"
        );
        // The panel reaches the trigger edit seam: name / activation / owner / rule / delete.
        for f in [
            "set_trigger_name",
            "set_trigger_activation",
            "set_trigger_owner",
            "set_trigger_rule",
            "delete_trigger",
        ] {
            assert!(
                SRC.contains(&call(f)),
                "the Triggers panel must call {f} (the trigger edit surface)"
            );
        }
        // The OWNER picker (CONN-TRG-OWNER-001) reads the placed-entity list AND the line reads its
        // resolved endpoints.
        assert!(
            SRC.contains(&call("placed_owner_options")),
            "the Owner picker must list placed entities via placed_owner_options"
        );
        assert!(
            SRC.contains(&call("owner_line_world")),
            "the owner-link line must resolve its endpoints via owner_line_world"
        );

        // The editor-ops seam actually exposes those functions AND the geometry reaches the core
        // trigger mutators (the claim the store round-trip rests on).
        let ops = include_str!("editor_ops.rs");
        for f in [
            "pub fn set_trigger_owner",
            "pub fn trigger_rows",
            "pub fn placed_owner_options",
            "pub fn owner_line_world",
        ] {
            assert!(ops.contains(f), "editor_ops must expose `{f}`");
        }
        assert!(
            ops.contains("core.add_circle_trigger(") || ops.contains("core.add_polygon_trigger("),
            "a trigger draw must reach the core trigger mutator"
        );
    }

    /// T-079 — the trigger AREA is a SECOND CONSUMER of the SHIPPED zone draw tool: the Triggers
    /// panel arms the SAME `begin_zone_draw` / `begin_zone_reshape` calls the Zones panel does, only
    /// with `DrawTarget::Trigger`. This proves BOTH halves of the ticket's "parameterize, do not
    /// fork" constraint:
    ///   • the zone tool is UNTOUCHED FOR ZONES — the Zones panel still arms with `DrawTarget::Zone`;
    ///   • no forked trigger draw state machine was invented — there is no `begin_trigger_draw` /
    ///     `advance_trigger_draw` / `close_trigger_polygon`; the trigger path routes through the
    ///     `zone_draw`/`zone_polygon` functions with the target flag.
    #[test]
    fn trigger_draw_is_second_consumer_of_the_zone_tool() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        let zones_src = include_str!("eden_zones.rs");
        let ops = include_str!("editor_ops.rs");

        // Assemble the target tokens so this test's own source cannot satisfy the checks by accident.
        let trigger_target = ["Draw", "Target", "::", "Trigger"].concat();
        let zone_target = ["Draw", "Target", "::", "Zone"].concat();
        let begin_draw = ["begin_", "zone_draw"].concat();
        let begin_reshape = ["begin_", "zone_reshape"].concat();

        // The Triggers panel arms the SHARED draw tool, targeting triggers.
        assert!(
            SRC.contains(&begin_draw) && SRC.contains(&trigger_target),
            "the Triggers panel must arm the shared zone-draw tool with the Trigger target"
        );
        assert!(
            SRC.contains(&begin_reshape),
            "trigger reshape must route through the shared zone-reshape, not a forked one"
        );
        // The Zones panel is UNTOUCHED for zones — it still targets the Zone collection.
        assert!(
            zones_src.contains(&begin_draw) && zones_src.contains(&zone_target),
            "the Zones panel must still arm the zone-draw tool with the Zone target (untouched)"
        );

        // No forked trigger draw state machine exists anywhere: the geometry accumulation is the ONE
        // shared `advance_zone_draw` / `close_zone_polygon`. A `begin_trigger_draw` /
        // `advance_trigger_draw` / `close_trigger_polygon` would be exactly the fork the ticket bans.
        for forked in [
            ["begin_", "trigger_draw"].concat(),
            ["advance_", "trigger_draw"].concat(),
            ["close_", "trigger_polygon"].concat(),
        ] {
            assert!(
                !ops.contains(&forked),
                "found a FORKED trigger draw fn `{forked}` — the draw flow must be parameterized by \
                 DrawTarget, not forked (the second-consumer constraint)"
            );
        }
        // The single per-collection branch really is on the target: the commit calls the trigger
        // mutators under a `DrawTarget::Trigger` match arm.
        assert!(
            ops.contains(&trigger_target) && ops.contains("core.add_circle_trigger("),
            "the commit's Trigger branch must call the core trigger mutator"
        );
    }

    /// T-079 (CONN-TRG-OWNER-001) — the owner-link LINE renders through the selection-overlay idiom
    /// (a `pointer-events-none` SVG projected by the pure, native-tested `project_owner_line`), keyed
    /// off the SELECTED trigger, and it TOLERATES a dangling owner by drawing nothing. Source pins;
    /// the projection math + dangling tolerance are proven behaviourally by `project_owner_line`'s
    /// native test (below) and the store's `owner_edge_assigns_clears_and_tolerates_dangling`.
    #[test]
    fn owner_line_uses_the_selection_overlay_idiom() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        // Every SRC needle assembled at run time — this test's own source is part of the haystack, so
        // a contiguous literal would make a presence check unpassable-by-code (satisfied by the test
        // itself). The overlay is the ruler idiom: a non-interactive SVG projected by the pure helper.
        let non_interactive = ["pointer-events-", "none"].concat();
        let project_fn = ["project_", "owner_line"].concat();
        let resolve_fn = ["owner_", "line_world"].concat();
        assert!(
            SRC.contains(&non_interactive) && SRC.contains(&project_fn),
            "the owner line must be a pointer-events-none SVG drawn via the pure projection helper"
        );
        // Its endpoints come from the resolver that returns None (→ no line) when the owner dangles.
        assert!(
            SRC.contains(&resolve_fn),
            "the line's endpoints must come from the resolver (None on a dangling owner)"
        );
        // T-727 keying trap: the trigger LIST must not use a `<For>` keyed on the (repeatable) name.
        // Like the Zones list, it is a full `.map(...).collect_view()` re-render off `doc_tick`, so
        // there is no `<For>` node to mis-key — row identity is the trigger id, never its name.
        let list_label = ["Authored ", "triggers"].concat();
        assert!(
            SRC.contains(&list_label),
            "the trigger list must render (the authored-triggers list)"
        );
        let name_key = ["key=", "|t| t.name"].concat();
        assert!(
            !SRC.contains(&name_key),
            "the trigger list must not be <For>-keyed on the repeatable name (T-727)"
        );
    }

    /// T-079 (CONN-TRG-OWNER-001) — the owner-link line's projection is PURE and native-tested: two
    /// world endpoints through a projector give the screen `<line>` endpoints. Perturb / restore: a
    /// projector that scales + offsets must move BOTH endpoints through it (a bug that projected only
    /// one end, or dropped the offset, fails here). The dangling-owner "draw nothing" path is proven
    /// in the store test; this proves the geometry the overlay draws when there IS a line.
    #[test]
    fn project_owner_line_maps_both_endpoints() {
        use crate::eden_zones::project_owner_line;
        // Trigger centre (10,20) → owner (110,220), through a scale-2 + offset projector.
        let l = project_owner_line((10.0, 20.0), (110.0, 220.0), |x, y| {
            (x * 2.0 + 5.0, y * 2.0 + 7.0)
        });
        assert!(
            (l.x1 - 25.0).abs() < 1e-9 && (l.y1 - 47.0).abs() < 1e-9,
            "endpoint A not projected"
        );
        assert!(
            (l.x2 - 225.0).abs() < 1e-9 && (l.y2 - 447.0).abs() < 1e-9,
            "endpoint B not projected"
        );
        // Identity projector → world coords pass through unchanged (the two ends are distinct).
        let id = project_owner_line((1.0, 2.0), (3.0, 4.0), |x, y| (x, y));
        assert_eq!((id.x1, id.y1, id.x2, id.y2), (1.0, 2.0, 3.0, 4.0));
    }

    /// T-079 — `DrawTarget` is the second-consumer parameter, and its two variants are distinct
    /// (so a zone draw and a trigger draw can never collapse into one). A tiny pin, but it is the
    /// hinge the whole "one shared draw tool" design turns on.
    #[test]
    fn draw_target_variants_are_distinct() {
        use crate::eden_zones::DrawTarget;
        assert_ne!(DrawTarget::Zone, DrawTarget::Trigger);
        assert_eq!(DrawTarget::Trigger.noun(), "trigger");
        assert_eq!(DrawTarget::Zone.noun(), "zone");
    }

    // ── T-695 — Favourites ───────────────────────────────────────────────────────────────────────

    /// T-695 — the storage contract: a NAMESPACED key and a VERSIONED blob, following the
    /// convention the frontend already uses rather than inventing one.
    ///
    /// The version is load-bearing in both directions: a fresh blob carries it on the wire (so the
    /// first shape change has something to branch on), and a blob written before the field existed
    /// (`version` absent ⇒ serde default 0) is stamped forward on load instead of being discarded.
    /// Perturbation RED: drop the stamp in `migrate_favourites` and the v0 assertion fails; widen
    /// the key to an un-namespaced string and the prefix assertion fails.
    #[test]
    fn favourites_key_is_namespaced_and_versioned() {
        use super::{Favourites, FAVOURITES_KEY, FAVOURITES_VERSION};

        assert!(
            FAVOURITES_KEY.starts_with("tbd-"),
            "the key must carry the frontend's `tbd-` namespace, got {FAVOURITES_KEY:?}"
        );
        assert!(
            FAVOURITES_KEY.contains("favourite"),
            "the key must say what it holds, got {FAVOURITES_KEY:?}"
        );
        // It must not collide with the sibling editor-local store or the auth blob.
        assert_ne!(FAVOURITES_KEY, "tbd-mc-editor-prefs");
        assert_ne!(FAVOURITES_KEY, "tbd-auth");
        assert!(FAVOURITES_VERSION >= 1, "an unversioned blob is banned");

        // A fresh blob serialises its version.
        let mut fav = Favourites::default();
        fav.add("{AAA}Prefabs/X.et", "X");
        let raw = fav.to_json();
        assert!(
            raw.contains(&format!("\"version\":{FAVOURITES_VERSION}")),
            "the persisted blob must carry its version, got {raw}"
        );

        // A pre-version blob (the shape a hand-written or older writer would leave) loads and is
        // stamped forward rather than thrown away.
        let v0 = r#"{"items":[{"asset_id":"{AAA}Prefabs/X.et","label":"X"}]}"#;
        let loaded = Favourites::from_json(v0);
        assert_eq!(loaded.version, FAVOURITES_VERSION, "v0 blob must migrate");
        assert!(
            loaded.contains("{AAA}Prefabs/X.et"),
            "v0 entry must survive"
        );

        // Outright garbage falls back to empty rather than panicking (the defaults floor).
        assert!(Favourites::from_json("not json at all").is_empty());
    }

    /// T-695 — the two verbs and the reload. `add`/`remove` are explicit and independent of any
    /// search or filter; a round-trip through the persisted string is what "survives a catalogue
    /// reload" means for a pure-SPA store, since a reload re-reads exactly that string.
    #[test]
    fn favourites_add_remove_and_survive_a_reload() {
        use super::Favourites;

        let a = "{AAA}Prefabs/Characters/Rifleman.et";
        let b = "{BBB}Prefabs/Vehicles/UAZ.et";

        let mut fav = Favourites::default();
        assert!(fav.is_empty());
        assert!(fav.toggle(a, "US Rifleman"), "first toggle stars");
        assert!(fav.toggle(b, "UAZ469"), "second toggle stars");
        assert_eq!(fav.len(), 2);
        // Newest first — the row just starred is the one the panel shows at the top.
        assert_eq!(fav.items[0].asset_id, b);

        // The reload: persist, then load exactly what was persisted.
        let reloaded = Favourites::from_json(&fav.to_json());
        assert_eq!(reloaded, fav, "a reload must reproduce the collection");
        assert!(reloaded.contains(a) && reloaded.contains(b));

        // Remove is the second verb, and it is idempotent.
        let mut fav = reloaded;
        assert!(!fav.toggle(a, "US Rifleman"), "second toggle unstars");
        assert!(!fav.contains(a));
        fav.remove(a);
        assert_eq!(fav.len(), 1, "removing an absent id is a no-op");
        // A duplicate add cannot grow the collection.
        fav.add(b, "UAZ469");
        assert_eq!(fav.len(), 1);
    }

    /// T-695 — the integrity floor over a blob another tab (or devtools) may have written: empty
    /// ids are dropped, duplicates collapse to the first occurrence, and the list is capped. A
    /// duplicated id would otherwise render two rows whose unstar buttons target one entry.
    #[test]
    fn favourites_blob_is_deduped_and_capped() {
        use super::{Favourites, FAVOURITES_MAX};

        let raw = r#"{"version":1,"items":[
            {"asset_id":"a","label":"A"},
            {"asset_id":"","label":"blank"},
            {"asset_id":"a","label":"A again"},
            {"asset_id":"b","label":"B"}
        ]}"#;
        let fav = Favourites::from_json(raw);
        assert_eq!(fav.len(), 2, "empty id dropped, duplicate collapsed");
        assert_eq!(fav.items[0].label, "A", "the FIRST occurrence is kept");
        assert!(fav.contains("b"));

        let mut big = Favourites::default();
        for i in 0..(FAVOURITES_MAX + 25) {
            big.add(&format!("asset-{i}"), "x");
        }
        assert_eq!(big.len(), FAVOURITES_MAX, "the collection is capped");
    }

    /// T-695 — **the stale-favourite rule**, and the acceptance boundary's sharpest edge: a starred
    /// id that has left the live catalogue must neither render as a normal (broken) row nor vanish.
    ///
    /// The chosen behaviour is KEEP AND MARK, and this pins all three halves of it:
    ///   * the resolved row COUNT equals the stored count (nothing silently disappears),
    ///   * the missing id resolves to `Stale` carrying the name remembered at star time — never a
    ///     `Live` row that would offer a place the catalogue cannot honour,
    ///   * a live id resolves to `Live` with the catalogue's CURRENT display name and its palette.
    ///
    /// It also pins the two non-obvious sub-cases: a row that is present but no longer PLACEABLE
    /// (an `abstract` vehicle) is stale too, and a stale entry whose remembered label is blank
    /// falls back to the id rather than rendering a nameless row.
    ///
    /// Perturbation RED: make `resolve_favourites` drop unresolvable entries (`filter_map`) and the
    /// count assertion fails; make it emit `Live` regardless and the `Stale` match fails.
    #[test]
    fn stale_favourite_is_kept_and_marked_not_dropped() {
        use super::{resolve_favourites, FavouriteAsset, FavouriteRow, Favourites};
        use crate::asset_catalog::CatalogPalette;
        use crate::dto::RegistryResponse;

        let golden: RegistryResponse =
            serde_json::from_str(include_str!("../tests/fixtures/api/GET__registry.json"))
                .expect("golden");
        let mut items = golden.data;
        let live = items
            .iter()
            .find(|i| i.kind == "character")
            .expect("golden has a character row")
            .clone();

        // An `abstract` vehicle: in the registry, but no palette offers it (T-215 filters it out),
        // so a favourite pointing at it is stale even though the row exists.
        let mut abstract_vehicle = live.clone();
        abstract_vehicle.id = "abs".into();
        abstract_vehicle.kind = "vehicle".into();
        abstract_vehicle.resource_name = "{ABS}Prefabs/Vehicles/Vehicle_base.et".into();
        abstract_vehicle.display_name = "Vehicle Base".into();
        abstract_vehicle.r#abstract = Some(true);
        items.push(abstract_vehicle.clone());

        let gone = "{GONE}Prefabs/Characters/FromAnUninstalledModpack.et";
        let fav = Favourites {
            version: 1,
            items: vec![
                FavouriteAsset {
                    asset_id: live.resource_name.clone(),
                    // Deliberately STALE remembered label — the live row must win for a live entry.
                    label: "an old name".into(),
                },
                FavouriteAsset {
                    asset_id: gone.into(),
                    label: "Remembered Rifleman".into(),
                },
                FavouriteAsset {
                    asset_id: abstract_vehicle.resource_name.clone(),
                    label: "Vehicle Base".into(),
                },
                FavouriteAsset {
                    asset_id: "{NOLABEL}Prefabs/Nothing.et".into(),
                    label: String::new(),
                },
            ],
        };

        let rows = resolve_favourites(&fav, &items);
        assert_eq!(
            rows.len(),
            fav.len(),
            "every stored favourite must yield exactly one row — nothing may vanish"
        );

        match &rows[0] {
            FavouriteRow::Live {
                asset_id,
                label,
                palette,
            } => {
                assert_eq!(asset_id, &live.resource_name);
                assert_eq!(
                    label, &live.display_name,
                    "a live row shows the catalogue's CURRENT name, not the remembered one"
                );
                assert_eq!(*palette, CatalogPalette::Character);
            }
            other => panic!("a live catalogue row must resolve Live, got {other:?}"),
        }

        match &rows[1] {
            FavouriteRow::Stale { asset_id, label } => {
                assert_eq!(
                    asset_id, gone,
                    "the stale row keeps its id for the unstar verb"
                );
                assert_eq!(
                    label, "Remembered Rifleman",
                    "a stale row names itself from the label remembered at star time"
                );
            }
            other => panic!("a missing id must resolve Stale, got {other:?}"),
        }

        assert!(
            !rows[2].is_live(),
            "a present-but-unplaceable row is stale too — it cannot arm a place"
        );
        assert_eq!(
            rows[3].label(),
            "{NOLABEL}Prefabs/Nothing.et",
            "a stale row with no remembered label falls back to its id, never blank"
        );

        // And the collection itself is untouched by resolution: resolving does NOT prune.
        assert_eq!(
            fav.len(),
            4,
            "resolution must not mutate the stored collection"
        );
    }

    /// T-695 — the surface is WIRED, not promised: a Favourites tab exists at its own index, the
    /// panel is dispatched from it, the star verb hangs off every palette leaf, and the collection
    /// is read from and written to the namespaced localStorage key.
    ///
    /// Source inspection, following `vehicles_tab_places_instead_of_promising`, because the panel
    /// is a Leptos view whose place handler is `#[cfg(target_arch = "wasm32")]` — a native test
    /// cannot mount it. **Every needle is assembled at run time**, the file's hard-won rule: this
    /// test's own source is part of the haystack it searches, so a contiguous literal would make a
    /// presence check unfailable.
    #[test]
    fn favourites_tab_is_wired_not_stubbed() {
        const SRC: &str = include_str!("eden_dock_right.rs");

        assert!(
            SRC.contains(&format!("tab_btn(6, {:?})", "Favourites")),
            "a Favourites tab must be in the tab strip"
        );
        assert!(
            SRC.contains(&format!("6 => {}(", "favourites_panel")),
            "tab 6 must dispatch the favourites panel"
        );
        // The star/unstar verb reaches every palette leaf (Factions, Vehicles and Objects all go
        // through `palette_rows`).
        assert!(
            SRC.contains(&format!("{}(favourites,", "favourite_star")),
            "a palette leaf must carry the star verb"
        );
        // Persistence is real: the key is both read and written.
        assert!(
            SRC.contains(&format!("{}(FAVOURITES_KEY", ".get_item")),
            "the collection must be loaded from localStorage"
        );
        assert!(
            SRC.contains(&format!("{}(FAVOURITES_KEY", ".set_item")),
            "the collection must be persisted to localStorage"
        );
        // T-646's search is a separate mechanism and must be undisturbed — the palettes still
        // filter through `filter_catalog`, and favourites is not a filter over the tree.
        assert!(
            SRC.contains(&format!("filter_catalog{}", "(&nodes, &q)")),
            "T-646's search must still filter the catalogue tree"
        );
        // The Markers tab is deliberately still a stub (T-069); if that stopped being true the
        // "Favourites got its own tab" assertions above would stop proving anything about indices.
        assert!(SRC.contains(&format!("Marker placement {} T-069.", "lands in")));
    }
}
