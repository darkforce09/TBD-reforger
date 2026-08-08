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

// ── T-084 (RIGHT-SEARCH-002/003/004/005) — the search grammar's copy ─────────────────────────────
//
// A grammar nobody can discover is a grammar nobody uses, and the placeholder alone cannot teach
// four operators. So the copy is split: the PLACEHOLDER names the operators (it is what an author
// reads before typing anything), and the HINT under the box shows one worked example of each,
// spelled against real catalogue shapes rather than `foo`/`bar` — `class:Character_US` is a bare
// classname of the kind the tail rule was added for, and `mod:ArmaReforger` is a real addon root.
//
// Both live here as one const each so the three search boxes (Factions, Objects, Vehicles) cannot
// drift apart, and so the copy is assertable without mounting a view.

/// The `placeholder=` tail shared by all three asset-browser search boxes.
pub const SEARCH_PLACEHOLDER_GRAMMAR: &str = " — class: mod: * /re/";

/// The worked-example line under every asset-browser search box.
pub const SEARCH_GRAMMAR_HINT: &str =
    "class:Character_US · mod:ArmaReforger · *Rifleman · /^us (mg|ar)$/";

/// The hint row rendered under each `type="search"` box.
fn search_grammar_hint() -> impl IntoView {
    view! {
        <p
            class="mt-1 text-[10px] leading-tight text-outline"
            title="class: matches the Enfusion classname (the bare name works — the GUID head is optional). \
                   mod: matches the addon. * and ? are wildcards over the whole name. /…/ is a regex."
        >
            {SEARCH_GRAMMAR_HINT}
        </p>
    }
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
/// it never examined, which is exactly the defect class this programme is about. The argument is
/// **moved, not cloned**, so the two call sites stay textually distinct — pinned by
/// `favourites_place_arm_stays_clone_free` (T-751): a future tidy that cloned the payload here
/// would let the favourites path satisfy T-215's palette needle while the palette itself regressed.
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

/// T-695 / T-750 — the Favourites tab: the starred collection over the WHOLE catalogue, resolved live.
///
/// Four states, and the middle two matter: while the registry fetch is still in flight there is
/// nothing to resolve against, so the panel says so instead of declaring every favourite stale
/// (T-695). When the fetch has *failed* (`registry_failed`), that is a terminal state with Retry —
/// not another turn of "Resolving…" (T-750 / wave-114 MINOR-2).
fn favourites_panel(
    favourites: RwSignal<Favourites>,
    registry_items: RwSignal<Option<Vec<RegistryItem>>>,
    registry_failed: RwSignal<bool>,
    registry_fetch_gen: RwSignal<u64>,
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
                    if registry_failed.get() {
                        return view! {
                            <div
                                class="flex flex-col gap-2"
                                data-testid="favourites-registry-error"
                            >
                                <p class="text-label-sm text-error">
                                    "Could not load the catalogue — favourites cannot be resolved."
                                </p>
                                <button
                                    type="button"
                                    data-testid="favourites-registry-retry"
                                    class="self-start rounded border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface transition hover:bg-surface-container-high"
                                    on:click=move |_| {
                                        registry_fetch_gen.update(|n| *n = n.wrapping_add(1));
                                    }
                                >
                                    "Retry"
                                </button>
                            </div>
                        }
                            .into_any();
                    }
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

// ── T-637 — the tab strip, as a WIDTH BUDGET ─────────────────────────────────────────────────────
//
// The strip's cells and its gaps are consts rather than inline class strings so
// `t637_the_tab_strip_fits_the_dock` can ADD THEM UP against `eden_layout::DOCK_PX` and fail if the
// next tab would push the trailing cell off the panel. That is the T-632 defect this ticket absorbed:
// the seventh tab clipped at the window edge, and nothing in the codebase could tell.

/// T-637 — the tab strip's own row: the tab group, then the Manage verb + collapse chevron.
const TAB_STRIP: &str = "flex shrink-0 items-center justify-between gap-1";
/// T-637 — the gap between cells inside each group.
const TAB_GROUP: &str = "flex items-center gap-0.5";
/// T-637 — a tab cell, selected. `size-5` (20 px) is the cell budget; the strip's arithmetic is
/// written from it.
const TAB_CELL_ON: &str = "flex size-5 shrink-0 items-center justify-center rounded border-b-2 border-primary text-primary";
/// T-637 — a tab cell at rest.
const TAB_CELL_OFF: &str = "flex size-5 shrink-0 items-center justify-center rounded border-b-2 border-transparent text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
/// T-637 — the Manage verb's cell: the same box as a tab, in the primary tint (it is the strip's one
/// verb, not an eighth tab).
const TAB_CELL_VERB: &str =
    "flex size-5 shrink-0 items-center justify-center rounded text-primary transition-colors hover:bg-primary/15";
/// T-637 — how many TAB cells the strip renders (Factions · Vehicles · Zones · Compositions ·
/// Triggers · Favourites · Markers). Stated so the budget test costs a compile error to get wrong,
/// and pinned against the actual `tab_btn` call count in the view.
const TAB_COUNT: usize = 7;

/// T-637 — the tab strip's glyph for tab `i`.
///
/// **THE TAB STRIP RAN OFF THE VIEWPORT, AND THE WORDS WERE WHY.** The dock carries SEVEN tabs
/// (Factions · Vehicles · Zones · Compositions · Triggers · Favourites · Markers) plus a Manage verb
/// and the collapse chevron — nine cells. As uppercase `text-label-sm` words that is roughly 470 px
/// of content, which did not fit the old 320 px dock (the trailing tab clipped at the window edge)
/// and comes nowhere near the equalised 240. Eden solves exactly this by labelling its cells with
/// glyphs and nothing else; nine 20 px cells is 180 px, which fits with room to spare.
///
/// **The label does not disappear — it moves.** Every tab keeps its word as both `title` (the hover
/// tooltip) and `aria-label` (the accessible name), so the strip is still readable by pointer and by
/// screen reader, and every existing `[aria-label]`-driven gate selector still resolves. What is
/// gone is only the rendered text.
///
/// An unknown index is a programming error, not a state, so it falls back to a neutral glyph rather
/// than panicking inside a view.
#[must_use]
fn tab_icon(i: usize) -> &'static str {
    match i {
        0 => "groups",         // Factions — the ORBAT roles palette
        1 => "directions_car", // Vehicles
        2 => "push_pin",       // Markers
        3 => "crop_free",      // Zones — a drawn area
        4 => "dashboard",      // Compositions — prefab clusters
        5 => "bolt",           // Triggers — activation
        6 => "star",           // Favourites — the starred collection
        _ => "help",
    }
}

/* ══════════ T-754 — the Zones panel's selection, reachable from OUTSIDE this component ══════════
 *
 * A zone's selection is deliberately NOT `select_tool`'s: it is `zone_selected`, an `RwSignal` local
 * to [`DockRight`] (see its declaration for why — a zone id in the slot selection reads `SEL 1` with
 * nothing highlighted). That locality is what made T-655's click-to-select router return `false` for
 * every zone: the router lives in `mission_editor.rs`, holding the `!Send` doc/selection/engine
 * handles, and had no way to reach a signal declared inside this component's body.
 *
 * So the panel EXPOSES its selection the same way the validation panel exposes its router — a
 * thread_local hook registered at mount, read by a free function. This is a SEAM, not a second
 * selection path: the hook only sets the signal this panel already owns (and raises the Zones tab so
 * the selection is visible), and the ONE router is still `route_select_by_subject_id`.
 */

/// The Zones tab's index in the tab strip. Named because THREE places must agree — the tab button,
/// the panel that renders under it, and [`register_select_zone`]'s "show me the selection I just
/// made". A literal in the third place is a silent way for a routed click to select a zone on a tab
/// the author is not looking at.
pub(crate) const ZONES_TAB: usize = 3;

/// The registered zone-selection hook: takes a `zonesById` id and makes it the Zones panel's
/// selection. Set once at [`DockRight`] mount; `None` on the host / pre-mount.
type ZoneSelectHook = std::rc::Rc<dyn Fn(&str)>;

thread_local! {
    /// The Zones panel's selection hook. Peer of `validation_panel::SELECT_BY_ID` and
    /// `PAYLOAD_SOURCE`, and thread_local for the same reason: the signal is `!Send` panel state
    /// that no caller can hold.
    static SELECT_ZONE: std::cell::RefCell<Option<ZoneSelectHook>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the zone-selection hook (called once at [`DockRight`] mount).
///
/// Prefer [`install_select_zone`] from inside a component: a bare register with no matching
/// unregister is the wave-129 F2 defect (see that function's docs).
pub(crate) fn register_select_zone(f: ZoneSelectHook) {
    SELECT_ZONE.with(|c| *c.borrow_mut() = Some(f));
}

/// Unregister the zone-selection hook at [`DockRight`] unmount — but ONLY if `f` is still the LIVE
/// registration.
///
/// The `Rc::ptr_eq` guard is the whole point, not a formality. Mount and unmount are not guaranteed
/// to interleave the way the writing order suggests: a remount can install its NEWER hook BEFORE the
/// OLD component's cleanup runs. An unconditional clear would then delete the live panel's hook and
/// leave the routed zone click dead again — the exact failure this cleanup exists to prevent.
///
/// Returns whether this call is the one that cleared it; a superseded (losing) cleanup returns
/// `false` and leaves the newer hook alone. The `Rc` is taken OUT of the cell and dropped after the
/// borrow ends, for the same re-entrancy reason [`route_select_zone`] clones out before calling.
pub(crate) fn unregister_select_zone(f: &ZoneSelectHook) -> bool {
    let taken = SELECT_ZONE.with(|c| {
        let mut slot = c.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|live| std::rc::Rc::ptr_eq(live, f))
        {
            slot.take()
        } else {
            None
        }
    });
    taken.is_some()
}

/// Install the zone-selection hook for the CURRENT reactive owner: register it now, and unregister
/// it when that owner is cleaned up (i.e. at unmount).
///
/// **Wave-129 F2.** T-754 registered and never unregistered. Backspace hide-chrome unmounts
/// [`DockRight`] (the chrome toggle in `mission_editor` has no modal guard — the aggregated-settings
/// dialog deliberately SURVIVES that hide), and the stale closure stayed callable: every `set` in it
/// then landed on DISPOSED signals, which `reactive_graph` 0.2.14 makes a silent no-op, while
/// [`route_select_zone`] still returned `true`. The router therefore reported a click that
/// "succeeded" and selected nothing — T-754's dead click, resurrected by lifecycle. Unregistering is
/// what makes that `false` an honest report instead of `true` over a no-op.
///
/// The hook is parked in a `StoredValue` (LOCAL storage — an `Rc<dyn Fn>` is `!Send`) because
/// `on_cleanup` is `Send + Sync`-bound and so cannot carry the `Rc` itself. An owner runs its
/// cleanup functions BEFORE it removes its arena nodes, so the value is still readable there; and
/// holding that clone keeps the allocation alive, which is what makes the `Rc::ptr_eq` identity
/// check meaningful rather than an address a later hook could be re-allocated onto.
pub(crate) fn install_select_zone(f: ZoneSelectHook) {
    let mine = StoredValue::new_local(std::rc::Rc::clone(&f));
    register_select_zone(f);
    on_cleanup(move || {
        let _ = mine.try_with_value(unregister_select_zone);
    });
}

/// Select `zone_id` in the Zones panel. Returns whether the panel was there to select it — `false`
/// on the host / pre-mount, which the router reports as "this click selected nothing" rather than
/// pretending. The `Rc` is cloned OUT before the call so the hook (which sets signals, and so can
/// re-enter the view) never runs under this cell's borrow.
#[must_use]
pub(crate) fn route_select_zone(zone_id: &str) -> bool {
    let hook = SELECT_ZONE.with(|c| c.borrow().clone());
    match hook {
        Some(f) => {
            f(zone_id);
            true
        }
        None => false,
    }
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
    /// T-750 — terminal `/registry` failure (distinct from `registry_items == None` = still loading).
    registry_failed: RwSignal<bool>,
    /// T-750 — bump to re-kick the cold `/registry` fetch (Favourites Retry).
    registry_fetch_gen: RwSignal<u64>,
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
    // T-637 — see [`tab_icon`]: the tab strip is GLYPHS now, not words.
    //
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
    // T-754 — publish that selection so T-655's ONE click-to-select router can drive it. A routed
    // zone click makes the zone the panel's selection AND raises the Zones tab (and un-collapses the
    // dock, T-638): a selection the author cannot see is the same dead click in a different costume.
    // `RwSignal` is `Copy`, so the hook holds the signals themselves, not a borrow of this body.
    //
    // wave-129 F2 — `install_*`, not `register_*`: the registration is unregistered on THIS owner's
    // cleanup. Backspace hide-chrome unmounts this dock, and a hook left behind keeps answering
    // `true` while writing to disposed signals — a click that reports success and selects nothing.
    install_select_zone(std::rc::Rc::new(move |id: &str| {
        zone_selected.set(Some(id.to_string()));
        tab.set(ZONES_TAB);
        collapsed.set(false);
    }));
    // T-650 — the composition id currently in inline-edit (rename/recategorize), or `None`. Its own
    // signal, like `zone_selected`: a composition is neither a slot nor a zone, so it does not touch
    // `select_tool`'s selection or the zone selection.
    let comp_editing = RwSignal::new(None::<String>);
    // T-079 — the selected trigger (Attributes target + the owner-link line's subject). Its own
    // selection, exactly like `zone_selected` and for the same reason: a trigger is neither a slot
    // nor a zone, so putting its id in `select_tool`'s selection would show `SEL 1` with nothing
    // highlighted. The owner-link line renders while this is `Some`.
    let trigger_selected = RwSignal::new(None::<String>);
    // T-069 (RIGHT-MODE-006) — the selected marker's doc id (Attributes target), or `None`. Its own
    // signal for the same reason `zone_selected` / `trigger_selected` are: a marker is not a slot,
    // so putting its id in `select_tool`'s selection would show `SEL 1` with nothing highlighted.
    // The id alone is enough of a handle even though the store addresses a marker by
    // `(factionId, id)` — `editor_ops::mint_marker_id` mints ids unique across every faction
    // precisely so this one-string selection stays unambiguous, and the row carries its own faction.
    let marker_selected = RwSignal::new(None::<String>);
    // T-695 (NEW-F5 / 3den E3) — the starred-asset collection, seeded from localStorage on mount so
    // it survives a catalogue reload, and written back on every star/unstar. It is dock-local
    // because it is a per-user editor preference, not mission state: nothing in the document, in
    // `editor_ops` or on the wire knows or should know what an author has starred.
    let favourites = RwSignal::new(load_favourites());
    let tab_btn = move |i: usize, label: &'static str| {
        let icon = tab_icon(i);
        view! {
            <button
                type="button"
                role="tab"
                title=label
                aria-label=label
                aria-selected=move || (tab.get() == i).to_string()
                class=move || if tab.get() == i { TAB_CELL_ON } else { TAB_CELL_OFF }
                on:click=move |_| tab.set(i)
            >
                <MaterialIcon name=icon class="block text-sm leading-none" />
            </button>
        }
    };
    let full = move || {
        view! {
            <aside class=DOCK_R>
                // T-638 — the tab strip carries the collapse chevron at its outer (top-RIGHT) end, after
                // "Manage"; » while expanded, flips to « collapsed.
                <div class=TAB_STRIP>
                    <div class=TAB_GROUP role="tablist">
                        {tab_btn(0, "Factions")}
                        {tab_btn(1, "Vehicles")}
                        // T-582 — Zones sits before the Markers stub: it is a live surface and that
                        // one is still a promise (T-069).
                        {tab_btn(ZONES_TAB, "Zones")}
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
                    <div class=TAB_GROUP>
                        // T-637 — "Manage" was a WORD in a strip that had already run out of room.
                        // It keeps its primary tint (it is the strip's one verb, not an eighth tab)
                        // and its name in the tooltip + `aria-label`.
                        <button
                            type="button"
                            title="Manage factions"
                            aria-label="Manage factions"
                            on:click=move |_| fm_open.set(true)
                            class=TAB_CELL_VERB
                        >
                            <MaterialIcon name="tune" class="block text-sm leading-none" />
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
                            // T-084 — the placeholder names every operator the grammar accepts
                            // (T-646 shipped the `class:`-only version of this line); the hint row
                            // below shows one worked example of each.
                            placeholder=move || {
                                if objects_mode.get() {
                                    format!("Search objects{SEARCH_PLACEHOLDER_GRAMMAR}")
                                } else {
                                    format!("Search assets{SEARCH_PLACEHOLDER_GRAMMAR}")
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
                        {search_grammar_hint()}
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
                            // T-084 — same grammar, same copy, on all three palettes.
                            placeholder=format!("Search vehicles{SEARCH_PLACEHOLDER_GRAMMAR}")
                            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                            on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                        />
                        {search_grammar_hint()}
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
                    // T-754 — the constant, not a literal: a routed zone click raises this same
                    // index, and the two must not be able to drift apart.
                    ZONES_TAB => zones_panel(doc_tick, zone_selected),
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
                    6 => favourites_panel(
                        favourites,
                        registry_items,
                        registry_failed,
                        registry_fetch_gen,
                    ),
                    // T-069 (RIGHT-MODE-006) — the Markers palette, replacing the one-line stub
                    // that had stood here since the dock was written. `EdenSubmode::Markers` and
                    // `from_tab(2)` were already in place; only the BODY was missing.
                    2 => markers_panel(doc_tick, marker_selected),
                    // Unreachable through the tab strip (every button above names its own arm);
                    // present because the match is over `usize`. Renders nothing rather than a
                    // placeholder — a "coming soon" line for a tab that cannot be selected is how
                    // the marker stub outlived the surface it was describing.
                    _ => ().into_any(),
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

// ── T-069 — the Markers palette (RIGHT-MODE-006) ─────────────────────────────────────────────────
//
// Tab 2 carried a one-line "lands in this ticket" stub from the day the dock was written;
// `EdenSubmode::Markers` and `from_tab(2)` were already there, so only the BODY was missing. This is
// that body. (The stub's exact wording is deliberately not quoted anywhere in this file —
// `favourites_tab_is_wired_not_stubbed` asserts its ABSENCE, and a quotation in a comment would make
// that search find its own description.)
//
// **The vocabulary is READ from the schema, never typed here.** `$defs/marker.icon` is a CLOSED enum
// of 64 aliases — the `TBD_MarkerIcons.EnsureAliases` register keys, the words a mission author may
// use. Before that enum existed a typo validated clean and then DEGRADED at runtime (`Resolve()`
// returned the fallback DOT and logged once), so the marker still drew, but not as authored. A
// hand-copied `const MARKER_ICONS: [&str; 64]` in this file would be exactly the second source of
// truth that reopens that failure the first time the schema moves, so the list is parsed out of the
// embedded `mission.schema.json` and `the_icon_list_is_the_schemas_own` re-reads the schema
// independently and compares alias for alias.
//
// This is the same `include_str!` of the same bytes that `eden_zones` and `eden_settings` already
// make (their headers carry the full argument). Three embeds, ONE vocabulary.
//
// SCOPE: the four schema-carried fields. `$defs/marker` also declares `size` / `rotationDeg` /
// `shape` / `area`, each stamped "T-673, lands after T-069" in its own schema description — marker
// STYLE and Eden's second Area-marker model. This panel authors none of them.

/// `mission.schema.json`, embedded — the ONE source of the marker icon vocabulary.
const MISSION_SCHEMA_JSON: &str =
    include_str!("../../../../packages/tbd-schema/schema/mission.schema.json");

/// The closed `$defs/marker.icon` alias list, in schema order, parsed once.
///
/// Schema order is kept rather than sorted alphabetically: the enum opens with the paired base
/// glyphs (`dot` / `dot2`, `objective_marker` / `objective_marker2`, …) and then runs through the
/// semantic aliases, which is a more useful browse order than the alphabet, and it is the order a
/// reader comparing this list against the schema will see.
///
/// An empty list is the honest answer if the schema ever stops declaring the enum — every writer
/// gates on [`marker_icon_is_authorable`], so the surface would refuse to author rather than fall
/// back to a guess.
#[must_use]
pub fn marker_icons() -> &'static [String] {
    static ICONS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    ICONS.get_or_init(|| {
        let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA_JSON) else {
            return Vec::new();
        };
        schema
            .get("$defs")
            .and_then(|d| d.get("marker"))
            .and_then(|m| m.get("properties"))
            .and_then(|p| p.get("icon"))
            .and_then(|i| i.get("enum"))
            .and_then(serde_json::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Is `icon` one of the closed `$defs/marker.icon` aliases?
///
/// Every marker write in [`crate::editor_ops`] passes through this. It is exact and
/// case-SENSITIVE: the enum is lower-case and `additionalProperties`-style validators do not
/// case-fold, so accepting `"Objective"` here would author a value the schema rejects at save time,
/// far from the control that produced it.
#[must_use]
pub fn marker_icon_is_authorable(icon: &str) -> bool {
    marker_icons().iter().any(|a| a == icon)
}

/// The alias a fresh place uses when the author has not picked one — the schema enum's first entry
/// (`dot`), not a literal. Empty only if the schema stopped declaring the enum.
#[must_use]
pub fn default_marker_icon() -> &'static str {
    marker_icons().first().map_or("", String::as_str)
}

/// The icon rows a search box shows: a case-insensitive SUBSTRING match over the closed list, with
/// an empty/whitespace query meaning "all of them".
///
/// Substring rather than prefix because the aliases are compound (`point_of_interest`,
/// `rally_point`, `observation_post`) and an author looking for a rally point types "rally" or
/// "point" with equal likelihood. The match also folds `_` to a space so typing "rally point"
/// finds `rally_point` — the alias is a token, but nobody reads it as one.
#[must_use]
pub fn filter_marker_icons(query: &str) -> Vec<&'static str> {
    let q = query.trim().to_ascii_lowercase();
    marker_icons()
        .iter()
        .map(String::as_str)
        .filter(|a| q.is_empty() || a.contains(&q) || a.replace('_', " ").contains(&q))
        .collect()
}

/// T-069 (RIGHT-MODE-006) — the Markers panel: the icon list that arms a place, the authored-marker
/// list, and the three-field Attributes block for the selected one.
///
/// One function with a native stub, exactly like [`zones_panel`] / [`triggers_panel`] /
/// [`compositions_panel`] — the doc reads and writes go through `editor_ops`, which is wasm-only.
#[cfg(target_arch = "wasm32")]
pub(crate) fn markers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::eden_tree::{ROW, ROW_ACTIVE};
    use crate::eden_zones::humanize_token;
    use crate::editor_ops as ops;

    let icon_search = RwSignal::new(String::new());

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Markers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    ops::marker_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Map markers for the active side's briefing. Pick an icon, then click the map to drop it. \
             Select a marker to caption it or nudge its position."
        </p>

        // ── RIGHT-MODE-006 "Marker icons in list" — the closed schema vocabulary ──────────────
        <input
            type="search"
            aria-label="Search marker icons"
            placeholder="Search icons"
            class="mt-3 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
            prop:value=move || icon_search.get()
            on:input=move |ev| icon_search.set(event_target_value(&ev))
        />
        {move || {
            let rows = filter_marker_icons(&icon_search.get());
            if rows.is_empty() {
                return view! {
                    <p class="mt-2 text-label-sm normal-case text-outline">"No icon matches."</p>
                }
                    .into_any();
            }
            view! {
                <ul
                    class="mt-1.5 flex max-h-48 flex-col gap-0.5 overflow-y-auto"
                    role="list"
                    aria-label="Marker icons"
                >
                    {rows
                        .into_iter()
                        .map(|alias| {
                            let armed = alias.to_string();
                            let label = humanize_token(alias);
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class=PALETTE_LEAF
                                        title="Click to arm, then click the map to place this marker"
                                        // `pointerdown`, not `click`: the palette arm/release
                                        // contract — the chrome host stops propagation here and the
                                        // map container's `pointerup` commits the drop.
                                        on:pointerdown=move |_| {
                                            ops::begin_place_marker(armed.clone());
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        <MaterialIcon name="place" class="block text-sm" />
                                        <span class="truncate">{label}</span>
                                        <span class="ml-auto shrink-0 font-mono text-code-md text-outline">
                                            {alias}
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

        // ── Armed state (T-723 KeepArmed: Esc/RMB cancel; off-canvas LMB keeps the arm) ───────
        {move || {
            let _ = doc_tick.get();
            let Some(icon) = ops::armed_marker_icon() else {
                return ().into_any();
            };
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {format!("Placing a {} marker", humanize_token(&icon))}
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Click the map to drop it on the active side's briefing."
                    </p>
                </div>
            }
                .into_any()
        }}

        // ── Authored markers ─────────────────────────────────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let rows = ops::marker_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">"No markers yet."</p>
                }
                    .into_any();
            }
            view! {
                <ul class="mt-3 flex flex-col gap-0.5" role="list" aria-label="Authored markers">
                    {rows
                        .into_iter()
                        .map(|m| {
                            let id = m.id.clone();
                            let sel_id = m.id.clone();
                            let sel_id2 = m.id.clone();
                            // The caption, or the alias when uncaptioned — never an invented
                            // placeholder, because an empty label is a real authored state.
                            let title = if m.label.is_empty() {
                                format!("{} ({})", humanize_token(&m.icon), m.side())
                            } else {
                                format!("{} ({})", m.label, m.side())
                            };
                            let pos = m.position_summary();
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
                                        <MaterialIcon name="place" class="block text-sm" />
                                        <span class="truncate">{title}</span>
                                        <span class="ml-auto shrink-0 font-mono text-code-md text-outline">
                                            {pos}
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

        // ── Attributes for the selected marker ───────────────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let Some(id) = selected.get() else {
                return ().into_any();
            };
            let Some(m) = ops::marker_rows().into_iter().find(|r| r.id == id) else {
                // Deleted underneath us (undo, or a reload that dropped it).
                return ().into_any();
            };
            marker_attributes(m, doc_tick, selected).into_any()
        }}
    }
    .into_any()
}

/// T-069 — the Attributes block for one marker: the three schema-carried editable fields
/// (ATTR-FIELD-MRK-TYPE / -MRK-TEXT / -MRK-POSITION) and delete.
///
/// Deliberately short of Eden's marker attributes: Size / Rotation / Shape / Brush / Colour / Alpha
/// are a `$defs/marker` WIDENING and belong to T-673, which ships after this. The three fields here
/// are the ones the closed `{x, z, icon, label}` shape can carry today.
#[cfg(target_arch = "wasm32")]
fn marker_attributes(
    m: crate::editor_ops::MarkerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::eden_zones::humanize_token;
    use crate::editor_ops as ops;

    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let faction = m.faction_id.clone();
    let mid = m.id.clone();

    let (f_icon, i_icon) = (faction.clone(), mid.clone());
    let (f_label, i_label) = (faction.clone(), mid.clone());
    let (f_x, i_x) = (faction.clone(), mid.clone());
    let (f_z, i_z) = (faction.clone(), mid.clone());
    let (f_del, i_del) = (faction.clone(), mid.clone());

    let current_icon = m.icon.clone();
    let label_value = m.label.clone();
    let (x_value, z_value) = (m.x, m.z);

    view! {
        <div class="mt-3 rounded-md border border-outline-variant/40 p-2">
            <h4 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {format!("Marker {} — {}", m.id, m.side())}
            </h4>

            // ATTR-FIELD-MRK-TYPE — the closed enum, as a picker. No free-text box exists for this
            // field anywhere in the panel: a typo used to validate clean and then degrade to DOT.
            <label class="mt-2 block text-label-sm text-on-surface-variant">"Type"</label>
            <select
                aria-label="Marker type"
                class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                on:change=move |ev| {
                    let next = event_target_value(&ev);
                    if ops::set_marker_icon(&f_icon, &i_icon, &next) {
                        bump();
                    }
                }
            >
                {marker_icons()
                    .iter()
                    .map(|alias| {
                        let a = alias.clone();
                        let is_current = *alias == current_icon;
                        let label = humanize_token(alias);
                        view! { <option value=a selected=is_current>{label}</option> }
                    })
                    .collect_view()}
            </select>

            // ATTR-FIELD-MRK-TEXT — stored VERBATIM. The mod caps the label at render time and the
            // emitter applies that cap when it compiles; capping here would destroy the authored
            // value in the one place the author could still see and fix it.
            <label class="mt-2 block text-label-sm text-on-surface-variant">"Text"</label>
            <input
                type="text"
                aria-label="Marker text"
                placeholder="Caption shown on the map"
                class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                prop:value=label_value
                on:change=move |ev| {
                    let next = event_target_value(&ev);
                    if ops::set_marker_label(&f_label, &i_label, &next) {
                        bump();
                    }
                }
            />

            // ATTR-FIELD-MRK-POSITION — `$defs/marker` is `{x, z}`: a marker is a MAP glyph and
            // carries no height, unlike a slot's `{x, y, z}`. Two boxes, not three, on purpose.
            <label class="mt-2 block text-label-sm text-on-surface-variant">
                "Position (x, z metres)"
            </label>
            <div class="mt-1 flex gap-1.5">
                <input
                    type="number"
                    step="0.1"
                    aria-label="Marker position x"
                    class="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 font-mono text-code-md text-on-surface outline-none focus:border-primary/60"
                    prop:value=x_value
                    on:change=move |ev| {
                        let Ok(next) = event_target_value(&ev).trim().parse::<f64>() else {
                            return;
                        };
                        if ops::set_marker_position(&f_x, &i_x, next, z_value) {
                            bump();
                        }
                    }
                />
                <input
                    type="number"
                    step="0.1"
                    aria-label="Marker position z"
                    class="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 font-mono text-code-md text-on-surface outline-none focus:border-primary/60"
                    prop:value=z_value
                    on:change=move |ev| {
                        let Ok(next) = event_target_value(&ev).trim().parse::<f64>() else {
                            return;
                        };
                        if ops::set_marker_position(&f_z, &i_z, x_value, next) {
                            bump();
                        }
                    }
                />
            </div>

            <button
                type="button"
                class="mt-2 rounded-md px-2 py-1 text-label-sm text-error transition-colors hover:bg-error/10"
                on:click=move |_| {
                    if ops::remove_marker(&f_del, &i_del) {
                        selected.set(None);
                        bump();
                    }
                }
            >
                "Delete marker"
            </button>
        </div>
    }
    .into_any()
}

/// Native shell: no document, so no markers. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn markers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}

#[cfg(test)]
mod tests {
    use super::{
        apply_eden_chip, custom_chip_visible, default_marker_icon, eden_chip_selected,
        filter_marker_icons, marker_icon_is_authorable, marker_icons, EdenChip, EdenSubmode,
        EDEN_CUSTOM_CHIP, EDEN_SIDE_CHIPS, MISSION_SCHEMA_JSON, SEARCH_GRAMMAR_HINT,
        SEARCH_PLACEHOLDER_GRAMMAR,
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
        // T-215 pinned "the Markers tab is deliberately still a stub" here, so that the assertion
        // above proved THIS tab was the one that got wired rather than any tab being live.
        // **T-069 shipped the Markers tab**, so the pin is inverted rather than deleted — the stub
        // sentence is gone, and the two tabs still arm through DIFFERENT `editor_ops` entry points
        // (a marker is not a `/registry` leaf and carries no `PlacePayload`), which is what the
        // original was really asserting.
        assert!(
            !SRC.contains(&stub("Marker", "T-069")),
            "the Markers stub must be gone now that the tab is real"
        );
        assert!(
            SRC.contains(&format!("{}{}", "begin_place_marker", "(armed.clone())")),
            "a Markers icon row must arm the marker place path"
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

    /// T-751 — T-695 kept `arm_favourite_place` textually distinct from `palette_rows` so T-215's
    /// source-inspection needle (the palette's `begin_place_vehicle` + clone call expression) keeps
    /// constraining the **palette** path specifically. That distinctness is the favourites arm
    /// MOVING its payload rather than cloning. Pin both forms: the palette clone needle must
    /// still exist exactly once, and the favourites move form (fn + `(payload),`) must exist and
    /// must not grow a clone. Needles are fragment-assembled so this module is not its own haystack.
    /// Note: `(payload)` is a prefix of the clone form, so the move needle ends at the trailing
    /// comma that only the favourites match arm writes.
    ///
    /// Wave-135 F3: Character + Object favourites arms are pinned the same way — vehicle-only left
    /// Character/Object free to grow `.clone()` while this pin stayed green.
    #[test]
    fn favourites_place_arm_stays_clone_free() {
        const SRC: &str = include_str!("eden_dock_right.rs");
        // Fragment the marker — a contiguous fn-name needle in this test would be a second hit.
        let marker = format!("{}{}", "fn arm_favourite_place", "(");
        let fav_arm = crate::arsenal::class_r_scrub::only_body(SRC, &marker);
        assert!(
            !fav_arm.contains(".clone()"),
            "T-751: arm_favourite_place must stay clone-free across Character/Object/Vehicle;              body was:\n{fav_arm}"
        );
        for (label, stem) in [
            ("Character", "begin_place"),
            ("Vehicle", "begin_place_vehicle"),
            ("Object", "begin_place_object"),
        ] {
            let palette = format!("{stem}{}", "(payload.clone())");
            let favourites = format!("{stem}{}", "(payload),");
            assert!(
                SRC.contains(&palette),
                "T-215/T-751 palette path must keep the {label} clone call expression"
            );
            assert_eq!(
                SRC.matches(&palette).count(),
                1,
                "T-751: exactly one palette-form {label} arm (clone); a second means favourites                  grew a clone"
            );
            assert!(
                SRC.contains(&favourites),
                "T-751: favourites {label} arm must MOVE the payload (trailing-comma form)"
            );
            assert_eq!(
                SRC.matches(&favourites).count(),
                1,
                "T-751: exactly one move-form {label} arm (the favourites match arm)"
            );
            assert!(
                fav_arm.contains(&format!("{stem}{}", "(payload)")),
                "T-751: arm_favourite_place {label} arm must call {stem}(payload)"
            );
        }
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
        // T-695 pinned "the Markers tab is deliberately still a stub" here, to keep the
        // "Favourites got its own tab" assertions above proving something about INDICES rather than
        // about which tab happens to be live. **T-069 shipped that tab**, so the pin is inverted
        // rather than deleted: tab 2 is a real dispatch, tab 6 is still Favourites, and the two are
        // still distinct surfaces. Deleting it would have quietly retired the index check.
        assert!(
            SRC.contains(&format!("2 => {}(", "markers_panel")),
            "tab 2 must dispatch the markers panel"
        );
        assert!(
            !SRC.contains(&format!("Marker placement {} T-069.", "lands in")),
            "the marker stub message must be gone — including from comments, where it would \
             make this test's own haystack lie"
        );
    }

    /// T-750 — Favourites has a TERMINAL failure arm with Retry, not an indefinite Resolving…
    /// spinner, when `registry_failed` is set. Call-shape pins run on `live_code` (literals blanked);
    /// user-visible Retry copy is pinned on `live_source`. Needles are fragment-assembled so this
    /// module is not its own haystack.
    #[test]
    fn favourites_panel_failure_arm_has_retry() {
        use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};
        let code = live_code(include_str!("eden_dock_right.rs"));
        let body = only_body(&code, "fn favourites_panel(");
        let failed_get = format!("{}{}", "registry_failed.", "get()");
        let bump = format!("{}{}", "registry_fetch_gen.", "update(");
        assert!(
            body.contains(&failed_get),
            "T-750: the None arm must branch on registry_failed — that is the terminal state"
        );
        assert!(
            body.contains(&bump),
            "T-750: the failure arm must bump registry_fetch_gen on Retry"
        );
        // User-visible copy: live_source keeps string literals; still cut test module.
        let sourced = live_source(include_str!("eden_dock_right.rs"));
        let sourced_body = only_body(&sourced, "fn favourites_panel(");
        let retry = format!("{}{}", "\"", "Retry\"");
        assert!(
            sourced_body.contains(&retry),
            "T-750: the failure arm must offer a Retry control"
        );
        let ellipsis = char::from_u32(0x2026).expect("horizontal ellipsis");
        let resolving = format!("Resolving {{n}} favourite(s) against the catalogue{ellipsis}");
        assert!(
            sourced_body.contains(&resolving),
            "T-750: the in-flight Resolving arm must remain; failure is an added arm, not a swap"
        );
    }

    // ── T-084 (RIGHT-SEARCH-002/003/004/005) — the grammar reaches all three search boxes ──────

    /// The grammar is a pure function in `asset_catalog` and is tested there, behaviourally. What
    /// only this file can answer is whether the three search boxes actually ADVERTISE it — a
    /// discoverable operator is the whole difference between a feature and a secret.
    ///
    /// **T-759 hollow-pin discipline, twice over:** `SRC` is TRUNCATED at the test module marker so
    /// this module is not part of its own haystack, and every needle is still assembled at run time.
    /// Delete the placeholder or the hint row and this goes red.
    #[test]
    fn every_asset_search_box_advertises_the_grammar() {
        const FULL: &str = include_str!("eden_dock_right.rs");
        let marker = format!("{}{}", "#[cfg", "(test)]");
        let src = &FULL[..FULL.find(&marker).expect("the test module marker exists")];
        // Guard the guard: the truncation must actually have removed this module.
        assert!(
            !src.contains(&format!(
                "fn every_asset_search{}",
                "_box_advertises_the_grammar"
            )),
            "SRC must be truncated before the test module, or every needle below is self-matching"
        );

        // All three palettes share ONE placeholder tail, so the operator list cannot drift.
        for noun in ["assets", "objects", "vehicles"] {
            assert!(
                src.contains(&format!("Search {noun}{}", "{SEARCH_PLACEHOLDER_GRAMMAR}")),
                "the {noun} search box must name the grammar in its placeholder"
            );
        }
        // The placeholder names every operator the parser accepts.
        for op in ["class:", "mod:", "*", "/re/"] {
            assert!(
                SEARCH_PLACEHOLDER_GRAMMAR.contains(op),
                "the placeholder must name the {op} operator"
            );
        }
        // The hint row is rendered under each of the three boxes (two call sites: the shared
        // Factions/Objects input and the Vehicles input).
        assert_eq!(
            src.matches(&format!("{}()}}", "{search_grammar_hint"))
                .count(),
            2,
            "the hint row must sit under both search inputs"
        );
        // And it shows a worked example of each operator, not just its name.
        for example in [
            "class:Character_US",
            "mod:ArmaReforger",
            "*Rifleman",
            "/^us ",
        ] {
            assert!(
                SEARCH_GRAMMAR_HINT.contains(example),
                "the hint must show a worked {example} example"
            );
        }
    }

    // ── T-069 (RIGHT-MODE-006) — the marker icon vocabulary ──────────────────────────────────

    /// **T-069 — the icon list IS the schema's, alias for alias.**
    ///
    /// `$defs/marker.icon` is a CLOSED enum, and the reason it is closed is a measured failure: a
    /// typo or empty string used to validate clean and then DEGRADE at runtime, `Resolve()`
    /// returning the fallback DOT glyph and logging once — the marker drew, but not as authored. A
    /// hand-copied `const MARKER_ICONS: [&str; 64]` in the dock would reopen that hole the first
    /// time the schema moved, so the list is PARSED from the embedded schema and this test re-reads
    /// the same bytes independently and compares in order.
    ///
    /// Perturbation RED: dropping any alias from the parse (or hard-coding the list) fails the
    /// element-wise comparison naming the index.
    #[test]
    fn the_icon_list_is_the_schemas_own() {
        let schema: serde_json::Value =
            serde_json::from_str(MISSION_SCHEMA_JSON).expect("the embedded schema must parse");
        let expected: Vec<&str> = schema["$defs"]["marker"]["properties"]["icon"]["enum"]
            .as_array()
            .expect("$defs/marker.icon declares an enum")
            .iter()
            .map(|v| v.as_str().expect("every alias is a string"))
            .collect();

        let got: Vec<&str> = marker_icons().iter().map(String::as_str).collect();
        assert_eq!(got, expected, "the panel's list must be the schema's list");
        assert_eq!(
            expected.len(),
            64,
            "the enum is closed at 64 aliases; a change here is a schema widening, which T-069 \
             is explicitly not"
        );

        // The vocabulary is not the marker SHAPE vocabulary — `shape` (T-673, ships after this) is
        // a different, four-value enum, and picking it up here would author style this slice does
        // not own.
        assert!(
            !got.contains(&"rectangle") && !got.contains(&"polyline"),
            "`$defs/marker.shape` values must not leak into the icon list: {got:?}"
        );
    }

    /// **T-069 — an alias outside the closed enum is refused, and the refusal is case-sensitive.**
    ///
    /// Every marker write in `editor_ops` gates on this predicate, so it is the whole enforcement.
    /// `hazard` is the pointed case: `store.rs`'s own T-345 tests author it, because the store
    /// mutator takes an `&str` and asks no questions — deliberately, so those pins stay green. The
    /// vocabulary is enforced at the PRODUCT boundary, which is here.
    #[test]
    fn only_schema_aliases_are_authorable() {
        assert!(marker_icon_is_authorable("dot"));
        assert!(marker_icon_is_authorable("objective"));
        assert!(marker_icon_is_authorable("rally_point"));

        assert!(!marker_icon_is_authorable(""), "empty is not an alias");
        assert!(
            !marker_icon_is_authorable("hazard"),
            "`hazard` is not in the enum, however plausible it reads"
        );
        assert!(
            !marker_icon_is_authorable("Objective"),
            "the enum is lower-case and validators do not case-fold"
        );
        assert!(
            !marker_icon_is_authorable("dot "),
            "no trimming, no guessing"
        );

        // The default a fresh place uses is itself an authorable alias, not a literal that could
        // drift out of the enum.
        assert!(
            marker_icon_is_authorable(default_marker_icon()),
            "the default icon must be in the closed list: {:?}",
            default_marker_icon()
        );
    }

    /// **T-069 — the icon search filters the closed list and can never widen it.**
    ///
    /// An empty query lists everything (RIGHT-MODE-006's "Marker icons in list"), and every result
    /// of every query is an alias the schema declares — the filter narrows, it never invents.
    #[test]
    fn the_icon_search_narrows_the_closed_list() {
        assert_eq!(
            filter_marker_icons("").len(),
            marker_icons().len(),
            "an empty query lists every icon"
        );
        assert_eq!(filter_marker_icons("   ").len(), marker_icons().len());

        let obj = filter_marker_icons("objective");
        assert!(obj.contains(&"objective"), "{obj:?}");
        assert!(obj.contains(&"objective_marker"), "{obj:?}");
        assert!(!obj.contains(&"dot"), "{obj:?}");

        // Case-insensitive, and `_` reads as a space so a typed phrase finds the token.
        assert!(filter_marker_icons("RALLY").contains(&"rally_point"));
        assert!(filter_marker_icons("rally point").contains(&"rally_point"));

        assert!(
            filter_marker_icons("zzz-not-an-icon").is_empty(),
            "a miss is empty, not a fallback"
        );

        for q in ["", "a", "point", "OBS", "medic"] {
            for hit in filter_marker_icons(q) {
                assert!(
                    marker_icon_is_authorable(hit),
                    "the filter may only return schema aliases; {q:?} yielded {hit:?}"
                );
            }
        }
    }

    /// **T-069 — markers are authored on the BRIEFING, never on the `markersById` root map.**
    ///
    /// The ticket's own registry summary says free placement needs generic add/move/remove on
    /// `markersById`. That premise is dead: `mission.schema.json` declares markers in exactly one
    /// place (`$defs/briefing.markers[]`) and no top-level `markers` property at all, and
    /// `flatten_to_mod_document` deserialises an `EditorPayload` that declares no root key — so the
    /// root map is a closed hydrate→emit loop and a marker authored there reaches no mod subsystem.
    /// `store.rs`'s `a_marker_in_the_root_map_never_reaches_the_compiled_document` proves that end
    /// of it; this pins that the PRODUCT surface never went to the dead one.
    ///
    /// Every literal is split so this test's own source cannot satisfy the search it performs.
    #[test]
    fn marker_writes_go_to_the_briefing_not_the_root_map() {
        const OPS: &str = include_str!("editor_ops.rs");

        assert!(
            OPS.contains(&format!("core.{}_faction_briefing_marker(", "set")),
            "a marker place must write the faction briefing"
        );
        assert!(
            OPS.contains(&format!("core.{}_faction_briefing_marker(", "remove")),
            "a marker delete must go through the briefing mutator"
        );
        // No mutation of the root map anywhere in the ops surface.
        assert!(
            !OPS.contains(&format!("{}.insert(", "markers_root")),
            "the `markersById` root must stay unauthored"
        );
        // The vocabulary gate is on the write path, not merely on the picker.
        assert!(
            OPS.contains(&format!("marker_icon_{}(", "is_authorable")),
            "marker writes must gate on the closed icon enum"
        );
        // SCOPE GUARD — the authored row is the four schema-carried fields and its address, and
        // nothing else. `$defs/marker` also declares `size` / `rotationDeg` / `shape` / `area`, each
        // stamped "T-673, ships after T-069" in the schema itself; authoring any of them would
        // convert this from a factory ticket into a workbench one. Pinned on the STRUCT rather than
        // by grepping the field names, because those names legitimately appear in the prose that
        // explains why they are excluded — a token search over a file that documents its own
        // boundary finds the boundary.
        let row = OPS
            .split("pub struct MarkerRow {")
            .nth(1)
            .expect("MarkerRow is declared in editor_ops")
            .split("\n}")
            .next()
            .expect("MarkerRow has a body");
        let fields: Vec<&str> = row
            .lines()
            .filter_map(|l| l.trim().strip_prefix("pub "))
            .filter_map(|l| l.split(':').next())
            .collect();
        assert_eq!(
            fields,
            vec!["faction_id", "id", "x", "z", "icon", "label"],
            "MarkerRow is the four `$defs/marker` fields plus the (factionId, id) address"
        );
    }
}

/// T-637 — **THE TAB STRIP FITS THE DOCK, AND THAT IS NOW ARITHMETIC.**
///
/// This ticket absorbed T-632, which was filed as "the right dock's fifth tab runs off the viewport".
/// That framing was a symptom: the strip had SEVEN word tabs, a `Manage` verb and a collapse chevron
/// crammed into one row, and at ~470 px of uppercase labels it did not fit the old 320 px dock either
/// — the trailing cell simply clipped at whatever edge it reached first. Equalising to 240 would have
/// made it worse.
///
/// So the strip is glyphs (Eden's own answer — its cells carry no words at all), and the labels move
/// to `title` + `aria-label` rather than disappearing. The durable half is this pin: the strip's
/// width is ADDED UP from the cell and gap classes it actually renders and checked against
/// `eden_layout::DOCK_PX` minus the dock's padding. An eighth tab, or a cell that grew, fails here
/// instead of clipping silently in a browser nobody is looking at.
#[cfg(test)]
mod t637_tab_strip_budget {
    use super::{TAB_CELL_OFF, TAB_CELL_ON, TAB_CELL_VERB, TAB_COUNT, TAB_GROUP, TAB_STRIP};
    use crate::eden_layout::{tw_len_px, DOCK_PX, DOCK_R, STUB_PX};

    /// The production half of this file — everything above the first test module, so a needle here
    /// cannot satisfy itself (the T-759 hollow-pin trap).
    fn production() -> &'static str {
        include_str!("eden_dock_right.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the production half precedes the test modules")
    }

    /// The strip's rendered width, in CSS px, from the classes themselves.
    fn strip_width_px() -> f64 {
        let cell = tw_len_px(TAB_CELL_ON, "size-").expect("a tab cell states its size");
        let verb = tw_len_px(TAB_CELL_VERB, "size-").expect("the verb cell states its size");
        let gap = tw_len_px(TAB_GROUP, "gap-").expect("the groups state their gap");
        let outer = tw_len_px(TAB_STRIP, "gap-").expect("the strip states its gap");
        let tabs = TAB_COUNT as f64;
        // Tab group: N cells + (N−1) gaps. Right group: the verb + the 24 px collapse chevron
        // (STUB_PX — its hit box must match the collapsed stub exactly, so it is not ours to shrink)
        // + one gap. Then the gap between the two groups.
        let left = tabs * cell + (tabs - 1.0) * gap;
        let right = verb + STUB_PX + gap;
        left + right + outer
    }

    /// The whole strip fits inside the equalised dock, gutters included.
    #[test]
    fn the_tab_strip_fits_the_dock() {
        let pad = tw_len_px(DOCK_R, "p-").expect("the dock states its padding");
        let budget = DOCK_PX - 2.0 * pad;
        let width = strip_width_px();
        assert!(
            width <= budget,
            "T-637/T-632: the tab strip renders {width} px inside a {budget} px dock — the trailing \
             cell clips at the panel edge, which is exactly the defect this ticket absorbed"
        );
        // …and it is not fitting by a hair, which would clip the moment a glyph gained a border.
        assert!(
            budget - width >= 8.0,
            "T-637: only {} px of slack — that is a clipping bug waiting for the next cell",
            budget - width
        );
        // The budget is computed from the SELECTED cell, so the two states must be the same box.
        // A selected tab that were wider would shuffle every cell right of it on each tab change —
        // and would make this arithmetic a lower bound rather than the width.
        assert_eq!(
            tw_len_px(TAB_CELL_ON, "size-"),
            tw_len_px(TAB_CELL_OFF, "size-"),
            "T-637: selecting a tab must not resize its cell — the strip would jitter, and the \
             width budget would be measuring the wrong state"
        );
    }

    /// **PERTURB / FAIL / RESTORE on the real failure mode.** The pre-T-637 strip labelled its cells
    /// with uppercase `text-label-sm` words. Even at a conservative 7 px per character plus the
    /// `px-1.5` gutters, those seven labels plus `Manage` blow the budget — which is why the trailing
    /// tab clipped. The budget check must REJECT that layout, or it is asserting nothing.
    #[test]
    fn the_budget_rejects_the_word_labelled_strip_it_replaced() {
        let pad = tw_len_px(DOCK_R, "p-").expect("the dock states its padding");
        let budget = DOCK_PX - 2.0 * pad;
        // The labels the strip used to render, verbatim.
        let words = [
            "Factions",
            "Vehicles",
            "Zones",
            "Compositions",
            "Triggers",
            "Favourites",
            "Markers",
            "Manage",
        ];
        // 12 px uppercase at ~7 px/char, plus `px-1.5` (12 px of gutter per cell) — deliberately
        // conservative; the real advance width of uppercase 12 px is wider.
        let word_strip: f64 = words
            .iter()
            .map(|w| w.chars().count() as f64 * 7.0 + 12.0)
            .sum::<f64>()
            + STUB_PX;
        assert!(
            word_strip > budget,
            "PERTURB: the word-labelled strip must NOT fit ({word_strip} px in {budget} px) — if it \
             did, the budget check would be passing everything"
        );
        // RESTORE: the shipped glyph strip does fit, and by a wide margin.
        assert!(
            strip_width_px() < word_strip,
            "RESTORE: glyphs must actually be narrower than the words they replaced"
        );
    }

    /// The count the budget is computed from is the count the view renders, and every cell keeps its
    /// word where a human (or a screen reader, or a gate selector) can still reach it. A glyph strip
    /// whose cells were anonymous would trade a clipping bug for an unusable one.
    #[test]
    fn every_glyph_tab_keeps_its_name() {
        let src = production();
        assert_eq!(
            src.matches("tab_btn(").count(),
            TAB_COUNT,
            "T-637: TAB_COUNT ({TAB_COUNT}) must be the number of tabs the strip actually renders — \
             the width budget is computed from it"
        );
        for label in [
            "Factions",
            "Vehicles",
            "Zones",
            "Compositions",
            "Triggers",
            "Favourites",
            "Markers",
        ] {
            assert!(
                src.contains(&format!("{:?}", label)),
                "T-637: `{label}` must survive as the cell's title/aria-label — the word moved off \
                 the glyph, it did not vanish"
            );
        }
        // The label reaches BOTH the tooltip and the accessible name, from the one `label` argument.
        assert!(
            src.contains("title=label") && src.contains("aria-label=label"),
            "T-637: one label, two consumers — a tooltip a pointer finds and a name a screen reader \
             (and every `[aria-label]` gate selector) resolves"
        );
        // Every tab index the strip renders has a glyph of its own: two tabs sharing one is a strip
        // you cannot read.
        let mut glyphs: Vec<&str> = (0..TAB_COUNT).map(super::tab_icon).collect();
        glyphs.sort_unstable();
        let n = glyphs.len();
        glyphs.dedup();
        assert_eq!(
            glyphs.len(),
            n,
            "T-637: each tab needs a DISTINCT glyph — with the words gone, the glyph is the only \
             thing telling two tabs apart"
        );
    }
}

/* ════════ T-754 — the Zones panel's selection is reachable, so a routed zone click lands ═════════
 *
 * The wave-115 MAJOR was that T-655's router could not select a zone: `zone_selected` is declared
 * inside [`DockRight`]'s body and the router lives in `mission_editor.rs`. These pin the seam that
 * closes that gap — behaviourally (an unmounted panel reports honestly; a mounted one receives the
 * id) and on the source (the hook drives the panel's OWN signal and raises the tab it is visible on).
 */
#[cfg(test)]
mod t754_zone_selection_seam {
    use super::{register_select_zone, route_select_zone, ZONES_TAB};

    /// The production half of this file — everything above the first test module, so a needle here
    /// cannot satisfy itself (the T-759 hollow-pin trap).
    fn production() -> &'static str {
        include_str!("eden_dock_right.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("the production half precedes the test modules")
    }

    /// The seam, end to end: with no panel mounted the route reports that it selected NOTHING (which
    /// is what lets the router return `false` instead of centring on a phantom selection), and with
    /// the panel mounted the zone id reaches the panel's selection hook verbatim.
    #[test]
    fn the_route_reports_honestly_and_delivers_the_id() {
        assert!(
            !route_select_zone("z-circle"),
            "T-754: no Zones panel mounted ⇒ nothing was selected, and the router must be told so"
        );
        let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let sink = std::rc::Rc::clone(&seen);
        register_select_zone(std::rc::Rc::new(move |id: &str| {
            sink.borrow_mut().push(id.to_string());
        }));
        assert!(
            route_select_zone("z-circle"),
            "T-754: a mounted panel selects the zone, so the routed click is a real selection"
        );
        assert_eq!(
            seen.borrow().as_slice(),
            ["z-circle".to_string()],
            "T-754: the id must reach the panel unchanged — no re-derivation on the way"
        );
    }

    /// The hook drives the panel's OWN selection signal (not `select_tool`'s — a zone id there reads
    /// `SEL 1` with nothing highlighted) and raises the tab it is visible on, un-collapsing the dock.
    /// A selection the author cannot see is the same dead click wearing a different costume.
    ///
    /// wave-129 F2 moved the mount-time call from `register_*` to `install_*` (register + an
    /// `on_cleanup` unregister); the needle follows it, because the thing being pinned is what the
    /// hook DOES, and the hook is the same hook.
    #[test]
    fn the_hook_selects_the_zone_and_shows_it() {
        let src = production();
        let at = src
            .find(&format!("install{}", "_select_zone(std::rc::Rc::new"))
            .expect("T-754: DockRight must install the zone-selection hook at mount");
        // Taken by CHARS, not bytes: the window runs into em-dashed prose, and a byte slice through
        // one of those is a panic that has nothing to do with what this test is asserting.
        let body: String = src[at..].chars().take(320).collect();
        assert!(
            body.contains(&format!("zone{}", "_selected.set(Some(")),
            "T-754: the hook must set the Zones panel's own selection signal"
        );
        assert!(
            body.contains(&format!("tab.set(ZONES{}", "_TAB)")),
            "T-754: and raise the Zones tab, so the selection is visible"
        );
        assert!(
            body.contains(&format!("collapsed.set({}", "false)")),
            "T-754: and un-collapse the dock (T-638), for the same reason"
        );
    }

    /// One index, three consumers. The tab button, the panel under it and the routed click all read
    /// [`ZONES_TAB`]; a literal in any of them is a silent way to select a zone on a tab nobody is
    /// looking at.
    #[test]
    fn the_zones_tab_index_is_stated_once() {
        let src = production();
        assert_eq!(ZONES_TAB, 3, "T-754: the Zones tab's index, stated once");
        assert!(
            src.contains(&format!("tab_btn(ZONES{}", "_TAB,")),
            "T-754: the tab button must read the constant"
        );
        assert!(
            src.contains(&format!("ZONES_TAB => zones{}", "_panel(")),
            "T-754: the panel arm must read the constant too"
        );
        assert!(
            !src.contains(&format!("tab_btn(3,{}", "")),
            "T-754: no literal 3 may address the Zones tab"
        );
    }
}

/* ══ wave-129 F2 — the zone hook is unregistered at unmount, and a remount is not clobbered ═══════
 *
 * T-754 made the routed zone click land by publishing the Zones panel's selection as a thread_local
 * hook. It registered at mount and never unregistered, which opened a NARROWER dead click through
 * lifecycle: Backspace hide-chrome unmounts `DockRight`, the stale closure stays callable,
 * `route_select_zone` returns `true`, and every `set` inside lands on a DISPOSED signal — a silent
 * no-op in `reactive_graph` 0.2.14. The click "succeeds" and selects nothing.
 *
 * These pin the LIFECYCLE, not the happy path (which `t754_zone_selection_seam` already covers), by
 * driving real `Owner`s through `install_select_zone` and calling `Owner::cleanup` — the same code
 * path leptos runs at unmount. Three shapes:
 *   1. registered -> cleanup -> the route reports FAILURE (not `true` over a no-op);
 *   2. install(A) -> install(B) -> A's cleanup -> B SURVIVES and still routes (the `Rc::ptr_eq`
 *      guard's entire reason for existing: unmount is not guaranteed to precede the remount);
 *   3. never installed -> failure.
 */
#[cfg(test)]
mod f2_zone_hook_lifecycle {
    use super::{install_select_zone, route_select_zone, ZoneSelectHook};
    use leptos::prelude::Owner;
    use std::{cell::RefCell, rc::Rc};

    /// A hook plus the log of every id it is handed — so "did the click actually select something"
    /// is answered by what the PANEL saw, not only by the router's boolean.
    fn spy() -> (Rc<RefCell<Vec<String>>>, ZoneSelectHook) {
        let log = Rc::new(RefCell::new(Vec::<String>::new()));
        let sink = Rc::clone(&log);
        let hook: ZoneSelectHook = Rc::new(move |id: &str| sink.borrow_mut().push(id.to_string()));
        (log, hook)
    }

    /// With nothing ever installed the route reports failure — the baseline the other two are
    /// measured against, so a green there cannot be "it was already false".
    #[test]
    fn a_route_with_no_panel_ever_installed_reports_failure() {
        assert!(
            !route_select_zone("z-nobody"),
            "F2: no panel has ever registered, so the click selected nothing and must say so"
        );
    }

    /// Unmount unregisters: after the owner is cleaned up the route must return `false`, and the
    /// dead hook must not run. Returning `true` here is the lie F2 exists to kill — the router uses
    /// that boolean to decide whether the click did anything.
    #[test]
    fn unmount_unregisters_so_the_route_stops_reporting_success() {
        let (log, hook) = spy();
        let owner = Owner::new();
        owner.with(|| install_select_zone(hook));
        assert!(
            route_select_zone("z-mounted"),
            "F2 precondition: while mounted the panel really does receive the selection"
        );
        owner.cleanup();
        assert!(
            !route_select_zone("z-after-unmount"),
            "F2: the unmounted panel's signals are DISPOSED, so every `set` is a silent no-op — the \
             route must report FAILURE, not `true` over a click that selected nothing"
        );
        assert_eq!(
            log.borrow().as_slice(),
            ["z-mounted".to_string()],
            "F2: the stale hook must not be called at all after unmount"
        );
    }

    /// The `Rc::ptr_eq` guard: a remount installs its hook BEFORE the old component's cleanup runs
    /// (leptos does not guarantee the other interleaving). The OLD cleanup must recognise that it is
    /// no longer the live registration and leave the NEW panel's hook alone — otherwise the fix for
    /// the stale hook becomes a fresh way to kill a live one.
    #[test]
    fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
        let root = Owner::new();
        // Siblings, not parent/child: two successive `DockRight` instances under the page owner. A
        // child would be cleaned up BY the parent and prove nothing about the guard.
        let old = root.child();
        let new = root.child();
        let (log_old, hook_old) = spy();
        let (log_new, hook_new) = spy();
        old.with(|| install_select_zone(hook_old));
        new.with(|| install_select_zone(hook_new));

        old.cleanup();

        assert!(
            route_select_zone("z-remounted"),
            "F2: the NEW panel is live — the old component's cleanup must not unregister it"
        );
        assert_eq!(
            log_new.borrow().as_slice(),
            ["z-remounted".to_string()],
            "F2: the id must reach the NEW panel, so the surviving registration is the new hook and \
             not a leftover that merely happens to answer `true`"
        );
        assert!(
            log_old.borrow().is_empty(),
            "F2: the superseded hook must never run again"
        );

        new.cleanup();
        assert!(
            !route_select_zone("z-gone"),
            "F2: the live panel's OWN cleanup does clear it — the guard skips losers, not everyone"
        );
    }
}
