//! Right dock palette behavior.

use super::*;

/// which palette a leaf belongs to. The tree machinery (guides, collapse, search) is
/// identical for both; only the glyph and which `editor_ops` arm the press calls differ, and those
/// are the two things that must not be shared — a Vehicles leaf that armed a character place would
/// silently write a `slots` row.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PaletteKind {
    Character,
    Vehicle,
    /// Objects chip → `entitiesById`.
    Object,
    /// A saved multi-entity stamp placed from the compositions panel.
    Composition,
    /// An area authored and edited in the triggers panel.
    Trigger,
}

impl PaletteKind {
    /// Icon that identifies a palette leaf's placement kind.
    pub(super) const fn leaf_icon(self) -> &'static str {
        match self {
            Self::Character => "person",
            Self::Vehicle => "directions_car",
            Self::Object => "inventory_2",
            Self::Composition => "dashboard_customize",
            Self::Trigger => "sensors",
        }
    }

    /// Tooltip for the placement kind.
    pub(super) const fn leaf_title(self) -> &'static str {
        match self {
            Self::Character => "Drag onto the map to place",
            Self::Vehicle => "Drag onto the map to place this vehicle",
            Self::Object => "Drag onto the map to place this object",
            Self::Composition => "Click to arm, then click the map to place this composition",
            Self::Trigger => "Draw a trigger area on the map",
        }
    }
}

/// Render the palette recursively. A leaf (`payload.is_some`) arms a place on `pointerdown` —
/// **pointer-drag, not HTML5 DnD**: the gates drive trusted `Input.dispatchMouseEvent`, which
/// synthesizes real pointer events into these handlers, where DnD would need `Input.setInterceptDrags`.
/// The chrome host stops `pointerdown` propagation, so this press cannot also open a map gesture; the
/// release is consumed by the container's `pointerup` (see `mission_editor`).
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
                                    armed_placement::begin_place(payload.clone())
                                }
                                PaletteKind::Vehicle => {
                                    armed_placement::begin_place_vehicle(payload.clone())
                                }
                                PaletteKind::Object => {
                                    armed_placement::begin_place_object(payload.clone())
                                }
                                PaletteKind::Composition => {}
                                PaletteKind::Trigger => {}
                            }
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
/// set (`buildCatalogTree` rule 3: only depth-0 faction folders start open). B6.
pub(super) fn collapsed_seed(nodes: &[CatalogNode], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if n.payload.is_none() && !n.children.is_empty() && !n.default_expanded {
            out.insert(n.id.clone());
        }
        collapsed_seed(&n.children, out);
    }
}

/// Render a faction tree with per-leaf placement kind resolved from registry
/// rows. Recent placement records use the same asset identifiers.
pub(super) fn faction_palette_rows(
    nodes: &[CatalogNode],
    depth: usize,
    prefix: &[bool],
    id_prefix: &[String],
    collapsed: RwSignal<std::collections::HashSet<String>>,
    favourites: RwSignal<Favourites>,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    recent: RwSignal<Vec<RecentPlaced>>,
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
                    let open = !collapsed.with_untracked(|c| c.contains(&n.id));
                    let toggle =
                        chevron_or_spacer(!n.children.is_empty(), open, &n.id, collapsed);
                    let folder_icon = if open { "folder_open" } else { "folder" };
                    let mut child_ids = gids.clone();
                    child_ids.push(n.id.clone());
                    let kids = if open {
                        faction_palette_rows(
                            &n.children,
                            depth + 1,
                            &anc,
                            &child_ids,
                            collapsed,
                            favourites,
                            registry_items,
                            recent,
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
                Some(payload) => {
                    let star =
                        favourite_star(favourites, payload.asset_id.clone(), payload.role.clone());
                    let glyph_kind = registry_items
                        .with_untracked(|opt| {
                            opt.as_ref().and_then(|items| {
                                crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(items, &payload.asset_id)
                                    .and_then(crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette)
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
                            #[cfg(target_arch = "wasm32")]
                            {
                                let palette = registry_items.with_untracked(|opt| {
                                    opt.as_ref().and_then(|items| {
                                        crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(
                                            items,
                                            &press_payload.asset_id,
                                        )
                                        .and_then(crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette)
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
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&press_payload, recent, registry_items);
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
