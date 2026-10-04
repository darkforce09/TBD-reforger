//! Right dock palette behavior.

#[cfg(target_arch = "wasm32")]
use super::*;

/// which palette a leaf belongs to. The tree machinery (guides, collapse, search) is
/// identical for both; only the glyph and which `editor_ops` arm the press calls differ, and those
/// are the two things that must not be shared — a Vehicles leaf that armed a character place would
/// silently write a `slots` row.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    /// A character asset.
    Character,
    /// A vehicle asset.
    Vehicle,
    /// Objects chip → `entitiesById`.
    Object,
}

#[cfg(target_arch = "wasm32")]
impl PaletteKind {
    /// Icon that identifies a palette leaf's placement kind.
    pub(super) const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
        }
    }

    /// Tooltip for the placement kind.
    pub(super) const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
        }
    }
}

/// Render the palette recursively. A leaf (`payload.is_some`) arms a place on `pointerdown` —
/// **pointer-drag, not HTML5 DnD**: the gates drive trusted `Input.dispatchMouseEvent`, which
/// synthesizes real pointer events into these handlers, where DnD would need `Input.setInterceptDrags`.
/// The chrome host stops `pointerdown` propagation, so this press cannot also open a map gesture; the
/// release is consumed by the container's `pointerup` (see `mission_editor`).
#[cfg(target_arch = "wasm32")]
pub(super) fn palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    prefix: &[bool],
    id_prefix: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
    kind: PaletteKind,
    favourites: RwSignal<Favourites>,
) -> AnyView {
    let len = nodes.len();
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.label.clone();
            let aria = n.label.clone();
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
                    let open = !collapsed.with_untracked(|c| c.contains(n.id.as_str()));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, n.id.as_str(), collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let mut child_ids = gids.clone();
                    child_ids.push(n.id.to_string());
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
                    let cid = n.id.to_string();
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
                            match kind {
                                PaletteKind::Character => {
                                    armed_placement::begin_place(payload.clone())
                                }
                                PaletteKind::Vehicle => {
                                    armed_placement::begin_place_vehicle(payload.clone())
                                }
                                PaletteKind::Object => {
                                    armed_placement::begin_place_object(payload.clone())
                                }
                            }
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
/// set (`buildCatalogTree` rule 3: only depth-0 faction folders start open). B6.
#[cfg(target_arch = "wasm32")]
pub(super) fn collapsed_seed(nodes: &[CatalogNode], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if n.payload.is_none() && !n.children.is_empty() && !n.default_expanded {
            out.insert(n.id.to_string());
        }
        collapsed_seed(&n.children, out);
    }
}

/// The signals a faction palette tree reads and writes: its collapsed folders, the favourites,
/// the registry rows that resolve each leaf's placement kind, and the recent placements.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
pub(super) struct FactionPaletteSignals {
    pub(super) collapsed: RwSignal<std::collections::HashSet<String>>,
    pub(super) favourites: RwSignal<Favourites>,
    pub(super) registry_items: RwSignal<Option<Vec<frontend_api_dtos::RegistryItem>>>,
    pub(super) recent: RwSignal<Vec<RecentPlaced>>,
}

/// Render a faction tree with per-leaf placement kind resolved from registry
/// rows. Recent placement records use the same asset identifiers.
#[cfg(target_arch = "wasm32")]
pub(super) fn faction_palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    prefix: &[bool],
    id_prefix: &[String],
    signals: FactionPaletteSignals,
) -> AnyView {
    let FactionPaletteSignals {
        collapsed,
        favourites,
        registry_items,
        recent,
    } = signals;
    let len = nodes.len();
    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let label = n.label.clone();
            let aria = n.label.clone();
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
                    let open = !collapsed.with_untracked(|c| c.contains(n.id.as_str()));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, n.id.as_str(), collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let mut child_ids = gids.clone();
                    child_ids.push(n.id.to_string());
                    let kids = if open {
                        faction_palette_rows(
                            &n.children,
                            depth + 1,
                            &anc,
                            &child_ids,
                            signals,
                        )
                    } else {
                        ().into_any()
                    };
                    let cid = n.id.to_string();
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
                Some(payload) => {
                    let star =
                        favourite_star(favourites, payload.asset_id.clone(), payload.role.clone());
                    let glyph_kind = registry_items
                        .with_untracked(|opt| {
                            opt.as_ref().and_then(|items| {
                                mission_creator_state::asset_catalog::find_catalog_item(items, &payload.asset_id)
                                    .and_then(mission_creator_state::asset_catalog::placeable_palette)
                                    .map(PaletteKind::from_catalog)
                            })
                        })
                        .unwrap_or(PaletteKind::Character);
                    let press_payload = payload.clone();
                    view! {
                    <div class="group relative flex items-center gap-1">
                    <button
                        type="button"
                        aria-label=aria
                        title=glyph_kind.leaf_title()
                        class=format!("{PALETTE_LEAF} min-w-0 flex-1")
                        on:pointerdown=move |_| {
                            {
                                let palette = registry_items.with_untracked(|opt| {
                                    opt.as_ref().and_then(|items| {
                                        mission_creator_state::asset_catalog::find_catalog_item(
                                            items,
                                            &press_payload.asset_id,
                                        )
                                        .and_then(mission_creator_state::asset_catalog::placeable_palette)
                                    })
                                });
                                if let Some(palette) = palette {
                                    arm_favourite_place(palette, press_payload.clone());
                                    record_recent(
                                        recent,
                                        press_payload.asset_id.clone(),
                                        press_payload.role.clone(),
                                    );
                                }
                            }
                        }
                    >
                        {guide_spans(&anc, &gids, collapsed)}
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name=glyph_kind.leaf_icon() class="block text-sm" />
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
