//! Right dock favourites behavior.

use super::*;

/// Render a star toggle on an asset row. Starred rows keep the glyph visible;
/// other rows reveal it on hover or focus.
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn favourite_star(
    favourites: RwSignal<Favourites>,
    asset_id: String,
    label: String,
) -> AnyView {
    let starred_id = asset_id.clone();
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

/// arm a place from a FAVOURITES row.
///
/// The three arms are spelled out here rather than shared with `palette_rows` on purpose: the
/// gate pins the leaf's own call expression by source inspection, and folding both call sites into
/// one helper would satisfy that needle from this function instead — a check passing over an input
/// it never examined, which is exactly the defect class this programme is about. The argument is
/// **moved, not cloned**, so the two call sites stay textually distinct — pinned by
/// `favourites_place_arm_stays_clone_free` : a future tidy that cloned the payload here
/// would let the favourites path satisfy 's palette needle while the palette itself regressed.
#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn arm_favourite_place(
    palette: CatalogPalette,
    payload: crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload,
) {
    match palette {
        CatalogPalette::Character => armed_placement::begin_place(payload),
        CatalogPalette::Vehicle => armed_placement::begin_place_vehicle(payload),
        CatalogPalette::Object => armed_placement::begin_place_object(payload),
    }
}

impl PaletteKind {
    /// the catalogue-side palette a favourite resolved to, in the dock's own vocabulary
    /// (for the row glyph and title).
    pub(in crate::v2::apps::editor::ui::docks::dock_right) const fn from_catalog(
        palette: CatalogPalette,
    ) -> Self {
        match palette {
            CatalogPalette::Character => Self::Character,
            CatalogPalette::Vehicle => Self::Vehicle,
            CatalogPalette::Object => Self::Object,
        }
    }
}

/// one favourites row: a live entry that arms a place plus its unstar verb, or a stale
/// entry rendered disabled and named, whose unstar verb still works.
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn favourite_row_view(
    row: FavouriteRow,
    favourites: RwSignal<Favourites>,
) -> AnyView {
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
            let payload = crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload {
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

/// / — the Favourites tab: the starred collection over the WHOLE catalogue, resolved live.
///
/// Four states, and the middle two matter: while the registry fetch is still in flight there is
/// nothing to resolve against, so the panel says so instead of declaring every favourite stale
/// . When the fetch has *failed* (`registry_failed`), that is a terminal state with Retry —
/// not another turn of "Resolving…" ( / MINOR-2).
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn favourites_panel(
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

/// (F-22) — the **Recently-placed** subtab (Eden's History): the session's placements,
/// most-recent-first. Each row arms the identical place a fresh palette leaf would (resolved LIVE
/// against the registry, so a row whose asset left the catalogue mid-session renders disabled rather
/// than arming nothing) and re-places bump it back to the head. No star column and no unstar — this
/// list is automatic, not curated (that is Favourites' job); the only affordance is "place it again".
///
/// Session-scoped: it starts empty every mount and persists nothing (see the `record_recent` block).
/// It is fed by a merged-palette / Vehicles leaf press; a composition stamp and the ORBAT Add-Vehicle
/// commit outside this dock's reach and so are not listed (stated at `record_recent`).
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn recently_placed_panel(
    recent: RwSignal<Vec<RecentPlaced>>,
    registry_items: RwSignal<Option<Vec<RegistryItem>>>,
) -> AnyView {
    view! {
        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Recently placed"</h3>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Assets you placed this session, newest first. Click one to place it again."
        </p>
        {move || {
            let rows = recent.get();
            if rows.is_empty() {
                return view! {
                    <p class="mt-2 text-label-sm text-outline">
                        "Nothing placed yet this session."
                    </p>
                }
                .into_any();
            }
            let items = registry_items.get().unwrap_or_default();
            view! {
                <ul class="mt-2 flex flex-col gap-0.5">
                    {rows
                        .into_iter()
                        .map(|r| {
                            let palette = crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(&items, &r.asset_id)
                                .and_then(crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette);
                            let label = r.label.clone();
                            match palette {
                                Some(palette) => {
                                    let kind = PaletteKind::from_catalog(palette);
                                    let payload = crate::v2::apps::editor::arsenal::asset_catalog::PlacePayload {
                                        asset_id: r.asset_id.clone(),
                                        role: r.label.clone(),
                                    };
                                    let aria = label.clone();
                                    view! {
                                        <li class="relative flex items-center gap-1">
                                            <button
                                                type="button"
                                                aria-label=aria
                                                title=kind.leaf_title()
                                                class=format!("{PALETTE_LEAF} min-w-0 flex-1")
                                                on:pointerdown=move |_| {
                                                    #[cfg(target_arch = "wasm32")]
                                                    {
                                                        arm_favourite_place(palette, payload.clone());
                                                        record_recent(
                                                            recent,
                                                            payload.asset_id.clone(),
                                                            payload.role.clone(),
                                                        );
                                                    }
                                                    #[cfg(not(target_arch = "wasm32"))]
                                                    let _ = (&payload, recent);
                                                }
                                            >
                                                <MaterialIcon name=kind.leaf_icon() class="block text-sm" />
                                                <span class="truncate">{label}</span>
                                            </button>
                                        </li>
                                    }
                                    .into_any()
                                }
                                None => {
                                    let aria = format!("{label} — not in the current catalogue");
                                    view! {
                                        <li class="relative flex items-center gap-1">
                                            <button
                                                type="button"
                                                disabled=true
                                                aria-label=aria
                                                title="This asset is not in the catalogue the editor loaded."
                                                class="relative flex flex-1 cursor-not-allowed items-center gap-1.5 rounded px-1.5 py-1 text-left text-label-sm text-outline opacity-70"
                                            >
                                                <MaterialIcon name="warning" class="block text-sm" />
                                                <span class="truncate line-through">{label}</span>
                                            </button>
                                        </li>
                                    }
                                    .into_any()
                                }
                            }
                        })
                        .collect_view()}
                </ul>
            }
            .into_any()
        }}
    }
    .into_any()
}
