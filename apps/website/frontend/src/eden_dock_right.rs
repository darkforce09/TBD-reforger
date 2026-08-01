//! T-661 — the right dock (Factions / Vehicles / Zones / Markers palette) and the Eden side chips,
//! split from `eden_chrome.rs`.
//!
//! `palette_rows` is the drag-to-place tree the Factions/Vehicles/Objects tabs draw with; the Eden
//! side chips (`EDEN_SIDE_CHIPS` / [`EdenChip`]) drive `active_side` and the Objects place mode.
//! Not cfg-gated (the doc-driving `on:pointerdown` bodies are wasm-gated inside their closures).
#![allow(dead_code)]
use leptos::prelude::*;

use crate::asset_catalog::{CatalogNode, CatalogState};
use crate::eden_layout::DOCK_R;
use crate::eden_tree::{chevron_or_spacer, guide_spans, PALETTE_LEAF};
use crate::eden_vehicles_panel::placed_vehicles_panel;
use crate::eden_zones::zones_panel;
use crate::ui::MaterialIcon;

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
}

impl PaletteKind {
    const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
        }
    }

    const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
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

/// Right dock — the **Factions** palette (spec O2), off the live `GET /api/v1/registry`. Leaves drag
/// onto the map to place their slot. `fm_open` toggles the T-167 Faction Manager dialog.
///
/// T-180.5 — Eden side chips above search drive `active_side` / Objects stub.
///
/// T-215 — the **Vehicles** tab is a real palette off the same `/registry` fetch (`vehicle_catalog`,
/// built by `asset_catalog::build_vehicle_catalog_tree`), not the T-070 placeholder it was. Its
/// leaves arm `editor_ops::begin_place_vehicle`, so a release on the canvas writes a `vehiclesById`
/// row at that world point.
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
    view! {
        <aside class=DOCK_R>
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-1">
                    {tab_btn(0, "Factions")}
                    {tab_btn(1, "Vehicles")}
                    // T-582 — Zones sits before the Markers stub: it is a live surface and that
                    // one is still a promise (T-069).
                    {tab_btn(3, "Zones")}
                    {tab_btn(2, "Markers")}
                </div>
                <button
                    type="button"
                    aria-label="Manage factions"
                    on:click=move |_| fm_open.set(true)
                    class="rounded-md px-1.5 py-0.5 text-label-sm font-semibold uppercase tracking-wide text-primary transition-colors hover:bg-primary/15"
                >
                    "Manage"
                </button>
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
                        placeholder=move || {
                            if objects_mode.get() {
                                "Search objects…"
                            } else {
                                "Search assets…"
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
                                    return view! {
                                        <p class="text-label-sm text-outline">
                                            "No objects match."
                                        </p>
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
                                            view! {
                                                <p class="text-label-sm text-outline">
                                                    "No assets match."
                                                </p>
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
                        placeholder="Search vehicles…"
                        class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                        on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                    />
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
                                            view! {
                                                <p class="text-label-sm text-outline">
                                                    "No vehicles match."
                                                </p>
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
                _ => view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "Marker placement lands in T-069."
                    </p>
                }
                    .into_any(),
            }}
        </aside>
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_eden_chip, eden_chip_selected, EdenChip, EDEN_SIDE_CHIPS};
    use leptos::prelude::*;

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
}
