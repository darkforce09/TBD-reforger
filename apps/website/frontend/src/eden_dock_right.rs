//! T-661 — the right dock (Factions / Vehicles / Zones / Markers palette) and the Eden side chips,
//! split from `eden_chrome.rs`.
//!
//! `palette_rows` is the drag-to-place tree the Factions/Vehicles/Objects tabs draw with; the Eden
//! side chips (`EDEN_SIDE_CHIPS` / [`EdenChip`]) drive `active_side` and the Objects place mode.
//! Not cfg-gated (the doc-driving `on:pointerdown` bodies are wasm-gated inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::asset_catalog::{CatalogNode, CatalogState};
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
}

impl PaletteKind {
    const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
            Self::Composition => "dashboard_customize",
        }
    }

    const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
            Self::Composition => "Click to arm, then click the map to place this composition",
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
                        palette_rows(&n.children, depth + 1, &anc, &child_ids, collapsed, kind)
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
                Some(payload) => view! {
                    <button
                        type="button"
                        aria-label=aria
                        title=kind.leaf_title()
                        class=PALETTE_LEAF
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
                }
                .into_any(),
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
}

impl EdenSubmode {
    /// Map a DockRight tab index (`0` Factions, `1` Vehicles, `2` Markers, `3` Zones, `4`
    /// Compositions) plus the Objects-chip flag to the sub-mode. The Objects chip lives on the
    /// Factions tab but is its own place surface, so it reports [`EdenSubmode::Objects`], not
    /// `Groups` — which is exactly why the Custom slot hides the moment the operator flips to Objects.
    #[must_use]
    pub fn from_tab(tab: usize, objects_mode: bool) -> Self {
        match tab {
            1 => Self::Vehicles,
            2 => Self::Markers,
            3 => Self::Zones,
            // T-650 — tab 4 is Compositions.
            4 => Self::Compositions,
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

        // Tab → sub-mode: Factions (tab 0) is Groups unless the Objects chip is on.
        assert_eq!(EdenSubmode::from_tab(0, false), EdenSubmode::Groups);
        assert_eq!(EdenSubmode::from_tab(0, true), EdenSubmode::Objects);
        assert_eq!(EdenSubmode::from_tab(1, false), EdenSubmode::Vehicles);
        assert_eq!(EdenSubmode::from_tab(2, false), EdenSubmode::Markers);
        assert_eq!(EdenSubmode::from_tab(3, false), EdenSubmode::Zones);
        // T-650 — tab 4 is the Compositions surface.
        assert_eq!(EdenSubmode::from_tab(4, false), EdenSubmode::Compositions);

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
        for tab in [1usize, 2, 3] {
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
}
