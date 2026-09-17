//! T-661 — the right dock (Factions / Vehicles / Zones / Markers palette) and the Eden side chips,
//! split from `eden_chrome.rs`.
//!
//! `palette_rows` is the drag-to-place tree the Factions/Vehicles/Objects tabs draw with; the Eden
//! side chips (`EDEN_SIDE_CHIPS` / [`EdenChip`]) drive `active_side` and the Objects place mode.
//! Not cfg-gated (the doc-driving `on:pointerdown` bodies are wasm-gated inside their closures).
#![allow(dead_code)]
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::editor_context;
use leptos::prelude::*;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::tools::selection;

use serde::{Deserialize, Serialize};

use crate::v2::apps::editor::arsenal::asset_catalog::{CatalogNode, CatalogPalette, CatalogState};
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
use crate::v2::apps::editor::shell::layout::{DOCK_R, STUB_PX};
use crate::v2::apps::editor::ui::docks::dock_left::collapse_chevron;
use crate::v2::apps::editor::ui::inspector::zones_panel::zones_panel;
use crate::v2::apps::editor::ui::outliner::tree::{chevron_or_spacer, guide_spans, PALETTE_LEAF};
use crate::v2::core::api::dto::RegistryItem;
use crate::v2::core::ui::MaterialIcon;

/// T-076 (RIGHT-CREW-001) — the "place vehicle with crew" toggle rendered beside the Vehicles
/// search. A checkbox bound to `with_crew`: a change writes the [`crate::v2::apps::editor::bridge::host_state::editor_context`] placement
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
                    editor_context::set_place_with_crew(on);
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
                                    armed_placement::begin_place(payload.clone())
                                }
                                PaletteKind::Vehicle => {
                                    armed_placement::begin_place_vehicle(payload.clone())
                                }
                                PaletteKind::Object => {
                                    armed_placement::begin_place_object(payload.clone())
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

// ── T-809 (F-22) — recently-placed (Eden's Assets|History pair; History is the recent list) ────────
//
// TBD had only MANUAL Favourites; Eden also gives an AUTOMATIC recently-placed list so the asset you
// just put down is one press away from the next. It is SESSION-SCOPED on purpose (per the UX-review
// summary): it is a within-a-sitting convenience, not authored content, so it holds no localStorage
// key and starts empty every mount — the same line rulers and bookmarks draw (nothing here reaches
// the document or the wire).
//
// **WHAT FEEDS IT — and what cannot, stated because the seam is asymmetric.** The list is fed at the
// one place a placement is observable from THIS dock: a palette leaf press ([`faction_palette_rows`]
// and the standalone Vehicles view route through [`record_recent`] on `pointerdown`, right where they
// arm the place). A composition STAMP and the ORBAT-manager's Add-Vehicle both commit through
// `editor_ops` / `orbat_manager` — files this slice does not own and that expose no placement hook —
// so neither feeds the list this wave. That is the honest boundary: the list holds what the author
// dragged from the merged palette, which is exactly what the acceptance places.
//
// `FAVOURITES_MAX` doubles as the cap here for the same reason it bounds Favourites — a small,
// synchronously-read working set — and it is far past any plausible session's placements.

/// T-809 — one recently-placed entry: the asset id (`resource_name`) and the label to show. Same
/// `asset_id` a leaf carries, so a recent row arms the identical place a fresh palette leaf would.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentPlaced {
    pub asset_id: String,
    pub label: String,
}

/// T-809 — the pure list transform behind [`record_recent`]: move `asset_id` to the head
/// (most-recent-first), dedup by id so a re-place bumps the existing entry rather than duplicating it,
/// and cap at [`FAVOURITES_MAX`]. Split out from the signal write so the ordering/dedup/cap contract
/// is native-testable without a reactive runtime.
fn push_recent_into(list: &mut Vec<RecentPlaced>, asset_id: String, label: String) {
    list.retain(|r| r.asset_id != asset_id);
    list.insert(0, RecentPlaced { asset_id, label });
    list.truncate(FAVOURITES_MAX);
}

/// T-809 — push an asset to the head of the session recently-placed list. Thin signal wrapper over
/// [`push_recent_into`]; no storage, no document (session-scoped, per the UX-review summary).
fn record_recent(recent: RwSignal<Vec<RecentPlaced>>, asset_id: String, label: String) {
    recent.update(|list| push_recent_into(list, asset_id, label));
}

// ── T-809 wave-203 — the recorder SEAM so the two OFF-DOCK placement paths feed the same list ───────
//
// The boundary the block above draws is honest but incomplete: the ticket's acceptance says placing
// ANY asset heads the recent list, and two placements commit in files this dock does not own — the
// composition STAMP (`editor_ops::place_at_impl`, the `Pending::Composition` arm) and the ORBAT
// manager's Add-Vehicle (the engine's `orbat_add_vehicle`). They cannot reach `recent` (a `!Send`
// signal declared inside `DockRight`'s body), so — exactly as the Zones panel exposes its selection
// (`SELECT_ZONE` / `install_select_zone` / `route_select_zone`, above) and the transform toolbar
// exposes its verbs (`register_editor_toolbar_dispatch`) — the dock REGISTERS a recorder closure at
// mount and those paths INVOKE it through a free function. The closure still routes through
// [`record_recent`], so the pure head/dedup/cap contract is untouched: this seam only lets a caller
// that has no `recent` in scope run the same `push_recent_into` the leaf press already runs.
//
// Off-wasm, native builds, and any window where the dock is unmounted, the invoke finds no recorder
// and is a SILENT NO-OP by design — NOT a discarded ack (the T-779 lesson). The placement it follows
// has already committed and reported through its own `after_local_edit`; the recent list is a session
// convenience layered on top, so "no dock listening" means "nothing to add to", not "the place
// failed". The invoke therefore returns nothing for a caller to mis-read as success-over-a-no-op.

/// T-809 — the registered recently-placed recorder: `(asset_id, label)`, the exact
/// [`push_recent_into`] key/label. Set once at [`DockRight`] mount; `None` on the host / pre-mount /
/// while the dock is unmounted. Peer of [`ZoneSelectHook`].
type RecentRecorder = std::rc::Rc<dyn Fn(String, String)>;

thread_local! {
    /// The recently-placed recorder hook. Peer of [`SELECT_ZONE`]; thread_local for the same reason —
    /// it closes over `recent`, a `!Send` `RwSignal` owned by `DockRight` that no off-dock caller can
    /// hold.
    static RECENT_RECORDER: std::cell::RefCell<Option<RecentRecorder>> =
        const { std::cell::RefCell::new(None) };
}

/// Register the recently-placed recorder (called once at [`DockRight`] mount).
///
/// Prefer [`install_recent_recorder`] from inside a component: a bare register with no matching
/// unregister is the wave-129 F2 defect [`install_select_zone`] documents.
fn register_recent_recorder(f: RecentRecorder) {
    RECENT_RECORDER.with(|c| *c.borrow_mut() = Some(f));
}

/// Unregister the recorder at [`DockRight`] unmount — but ONLY if `f` is still the LIVE registration.
/// The `Rc::ptr_eq` guard is [`unregister_select_zone`]'s: a remount can install its newer recorder
/// before the old component's cleanup runs, and an unconditional clear would delete the live one.
fn unregister_recent_recorder(f: &RecentRecorder) -> bool {
    let taken = RECENT_RECORDER.with(|c| {
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

/// Install the recorder for the CURRENT reactive owner: register now, unregister at unmount. Mirrors
/// [`install_select_zone`] — the `StoredValue` clone keeps the `Rc` alive so the `ptr_eq` identity is
/// meaningful, and `on_cleanup` drops the registration when Backspace hide-chrome (or a mission
/// switch) unmounts the dock, so a later placement finds `None` and no-ops rather than writing into a
/// disposed `recent` signal.
fn install_recent_recorder(f: RecentRecorder) {
    let mine = StoredValue::new_local(std::rc::Rc::clone(&f));
    register_recent_recorder(f);
    on_cleanup(move || {
        let _ = mine.try_with_value(unregister_recent_recorder);
    });
}

/// T-809 — record a placement made OUTSIDE this dock (composition stamp / ORBAT Add-Vehicle) into the
/// session recently-placed list, through the mount-registered recorder. A no-op when no dock is
/// mounted (host / pre-mount / hidden chrome) — that is not a dropped ack (see the seam note): the
/// placement itself already committed, and the recent list is a convenience with nothing to add to
/// when nothing is listening. Routes through [`record_recent`], so the head/dedup/cap contract holds.
pub(crate) fn record_placed(asset_id: String, label: String) {
    let hook = RECENT_RECORDER.with(|c| c.borrow().clone());
    if let Some(f) = hook {
        f(asset_id, label);
    }
}

/// T-809 — render the **merged Factions tree** ([`crate::v2::apps::editor::arsenal::asset_catalog::build_faction_catalog_tree`]):
/// characters, vehicles and objects filed under one faction, so a vehicle leaf is reachable inside
/// NATO. It is [`palette_rows`] with ONE difference — a leaf carries no fixed [`PaletteKind`], because
/// the merged tree mixes kinds. Each leaf resolves its place-arm at press time from the live registry
/// row ([`crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette`]), the SAME resolution the Favourites collection
/// uses ([`arm_favourite_place`]) — so a character leaf writes a `slots` row and a vehicle leaf writes
/// a `vehiclesById` row, decided by the row's own kind, never by which folder it sits in.
///
/// `registry_items` is the raw `/registry` rows (already in the dock); `recent` is the session
/// recently-placed list, fed on every leaf press. Folders reuse [`palette_rows`]' exact chrome via a
/// shared inner helper is deliberately NOT done here — the two trees' leaves differ (fixed vs resolved
/// kind), and the T-215 palette gate pins `palette_rows`' own leaf call expression by source, so a
/// merge would let this path satisfy that needle. The folder arm is therefore spelled out again.
///
/// The recursion carries the same guide/collapse/id state `palette_rows` threads plus two extra
/// signals (`registry_items` for per-leaf resolution, `recent` for the feed); all are `Copy`
/// `RwSignal`s or slices, so the arg count is a threaded-context artefact, not shared mutable state.
#[allow(clippy::too_many_arguments)]
fn faction_palette_rows(
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
                // A merged-tree leaf: resolve its palette from the live row so the press arms the
                // right `editor_ops` verb (character vs vehicle vs object), then record it as
                // recently-placed. A row whose id is no longer in the catalogue (a modpack switched
                // off mid-session) resolves to no palette and renders as an inert label — the same
                // honesty the Favourites stale row shows, rather than a leaf that arms nothing.
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
                            // Resolve the palette LIVE at press time (not the cached glyph): the arm
                            // must reflect the catalogue as it is now, and `placeable_palette` is the
                            // same gate the tree offered the leaf through.
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

/// Apply a chip click to the shared place signals (same `active_side` EditorContext / `place_at` read).
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

/// T-800 — the shared Factions/Vehicles tree failure view: name the cause and offer Retry.
///
/// The main tree used to render one flat line ("Could not load the catalog.") with no cause and
/// no recovery, while the search-grammar hint kept rendering above it (F-05/F-21). This replaces
/// that dead end. Two causes, distinguished by `no_modpack` (raised by the dock's probe when the
/// `/registry` failure was the API's 404 "no current modpack configured", the swallowed cause):
///   • no current modpack  → the editor has nothing to load, and says so — not a transient error.
///   • request failed       → a network/server fault; the same rows are expected back on Retry.
/// Retry reuses the T-750 Favourites mechanism verbatim: bump `registry_fetch_gen`, which re-enters
/// the cold `/registry` fetch and puts both palettes back into Loading. `noun` names the palette so
/// the copy reads in place ("vehicle catalog" / "asset catalog").
fn catalog_failure_view(
    noun: &'static str,
    no_modpack: RwSignal<bool>,
    registry_fetch_gen: RwSignal<u64>,
) -> AnyView {
    let cause = if no_modpack.get() {
        format!(
            "No modpack is configured, so the {noun} is empty. Set a current modpack, then retry."
        )
    } else {
        format!("Could not load the {noun}. The request to the registry failed.")
    };
    view! {
        <div class="flex flex-col gap-2" data-testid="catalog-failure">
            <p class="text-label-sm text-error" data-testid="catalog-failure-cause">
                {cause}
            </p>
            <button
                type="button"
                data-testid="catalog-failure-retry"
                class="self-start rounded border border-outline-variant/40 px-2 py-1 text-label-sm text-on-surface transition hover:bg-surface-container-high"
                on:click=move |_| {
                    // T-750 pattern — re-kick the cold /registry fetch; the effect clears the
                    // failure and returns both palettes to Loading. A populated seed then Readies
                    // the tree with no page reload.
                    registry_fetch_gen.update(|n| *n = n.wrapping_add(1));
                }
            >
                "Retry"
            </button>
        </div>
    }
    .into_any()
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

    /// How many assets are starred.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when nothing is starred, which is what the empty-state row renders from.
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
            let live = crate::v2::apps::editor::arsenal::asset_catalog::find_catalog_item(
                items,
                &f.asset_id,
            )
            .and_then(|it| {
                crate::v2::apps::editor::arsenal::asset_catalog::placeable_palette(it)
                    .map(|p| (it, p))
            });
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
fn arm_favourite_place(
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

/// T-809 (F-22) — the **Recently-placed** subtab (Eden's History): the session's placements,
/// most-recent-first. Each row arms the identical place a fresh palette leaf would (resolved LIVE
/// against the registry, so a row whose asset left the catalogue mid-session renders disabled rather
/// than arming nothing) and re-places bump it back to the head. No star column and no unstar — this
/// list is automatic, not curated (that is Favourites' job); the only affordance is "place it again".
///
/// Session-scoped: it starts empty every mount and persists nothing (see the `record_recent` block).
/// It is fed by a merged-palette / Vehicles leaf press; a composition stamp and the ORBAT Add-Vehicle
/// commit outside this dock's reach and so are not listed (stated at `record_recent`).
fn recently_placed_panel(
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
                            // Resolve LIVE: a recent entry arms only if its row is still placeable.
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
                                // The asset left the catalogue this session — show it named and
                                // disabled, not a leaf that arms nothing (the Favourites-stale idiom).
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

// ── T-637 — the tab strip, as a WIDTH BUDGET ─────────────────────────────────────────────────────
//
// The strip's cells and its gaps are consts rather than inline class strings so
// `the_tab_strip_fits_the_dock` can ADD THEM UP against `eden_layout::DOCK_PX` and fail if the
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
const TAB_CELL_VERB: &str = "flex size-5 shrink-0 items-center justify-center rounded text-primary transition-colors hover:bg-primary/15";
/// T-809 — the Favourites|History subtab pair's selected pill (a text pill, not a glyph cell: two
/// words fit inside the tab body where the glyph strip above does not have the room).
const SUBTAB_ON: &str = "rounded px-2 py-0.5 text-label-sm font-medium text-primary bg-primary/15";
/// T-809 — the subtab pill at rest.
const SUBTAB_OFF: &str = "rounded px-2 py-0.5 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface";
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
        6 => "star",           // Favourites + Recently-placed (Assets|History pair, T-809)
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
/// leaves arm `armed_placement::begin_place_vehicle`, so a release on the canvas writes a `vehiclesById`
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
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
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
    let place_with_crew = RwSignal::new(editor_context::place_with_crew());
    // T-254 — Objects chip palette (entities[]): own collapse + search, built from registry_items.
    let object_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    let object_search = RwSignal::new(String::new());
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
    // T-069 (RIGHT-MODE-006) — the selected marker's `(factionId, id)` address (Attributes target),
    // or `None`. Its own signal for the same reason `zone_selected` / `trigger_selected` are: a
    // marker is not a slot, so putting its id in `select_tool`'s selection would show `SEL 1` with
    // nothing highlighted. Selection is the full address pair — not id alone — because a hydrated
    // foreign payload can carry the same marker id under two factions (T-763 / wave-116 finding 7).
    // This editor's own minting pairs `(factionId, id)` already; the Attributes lookup must too.
    let marker_selected = RwSignal::new(None::<(String, String)>);
    // T-695 (NEW-F5 / 3den E3) — the starred-asset collection, seeded from localStorage on mount so
    // it survives a catalogue reload, and written back on every star/unstar. It is dock-local
    // because it is a per-user editor preference, not mission state: nothing in the document, in
    // `editor_ops` or on the wire knows or should know what an author has starred.
    let favourites = RwSignal::new(load_favourites());
    // T-809 (F-22) — the session recently-placed list (Eden's History half of the Assets|History
    // pair). Dock-local and NOT persisted: it is within-a-sitting convenience, not authored state.
    // Fed by a merged-palette leaf press (see `record_recent`) AND — wave-203 — by the two off-dock
    // placement paths through the recorder seam below; read by the History subtab.
    let recent_placed = RwSignal::new(Vec::<RecentPlaced>::new());
    // T-809 wave-203 — publish a recorder so the composition stamp and ORBAT Add-Vehicle (which commit
    // in `editor_ops`, out of this signal's reach) can head the SAME list a leaf press feeds. The
    // closure routes through `record_recent`, so the pure head/dedup/cap contract is unchanged; the
    // `install_*` half unregisters on unmount so a later off-dock placement finds no recorder and
    // no-ops rather than writing into a disposed `recent_placed` (the T-754 lesson). Mirrors
    // `install_select_zone`.
    install_recent_recorder(std::rc::Rc::new(move |asset_id: String, label: String| {
        record_recent(recent_placed, asset_id, label);
    }));
    // T-809 — which half of the Favourites|History tab pair is showing (Eden's Assets|History). The
    // pair lives under ONE tab rather than an eighth strip cell: the strip is glyph-tight (T-637), and
    // Eden itself pairs the starred collection with the recent list under one surface.
    let history_open = RwSignal::new(false);
    // T-809 — the merged Factions tree's collapse set, seeded (rule 3) whenever the side-filtered tree
    // rebuilds. Its own set, not `palette_collapsed`: that one is seeded from the character-only
    // `catalog` signal, whose folder ids are a subset of the merged tree's.
    let faction_collapsed = RwSignal::new(std::collections::HashSet::<String>::new());
    Effect::new(move |_| {
        // Re-seed on a chip flip or a registry (re)load — the merged tree is derived from the raw
        // rows the dock already holds, so it need not wait on the character-only `catalog` signal.
        let side = active_side.get();
        if let Some(items) = registry_items.get() {
            let nodes = crate::v2::apps::editor::arsenal::asset_catalog::build_faction_catalog_tree(
                &items, &side,
            );
            let mut set = std::collections::HashSet::new();
            collapsed_seed(&nodes, &mut set);
            faction_collapsed.set(set);
        }
    });
    // T-800 — the CAUSE of a `/registry` failure, so the Factions/Vehicles Failed arms can name it
    // instead of a flat "Could not load the catalog." The editor's cold fetch (mission_editor.rs,
    // T-788-owned) collapses every error to a unit `CatalogState::Failed` + `registry_failed`, so
    // the 404 "no current modpack configured" is thrown away before it reaches this dock. Rather
    // than reach into that file, the dock re-asks: when `registry_failed` latches, a one-row probe
    // reads the status the fetch discarded — a 404 IS the "no current modpack" case (the API's only
    // 404 on this route: handlers/registry.rs:70), anything else is a genuine request failure. The
    // probe is a wasm-only network call, so it lives behind the arch gate exactly like the fetch it
    // shadows; the native `live_code` harness scrubs the body (pins gate on `live_source`).
    let registry_no_modpack = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    {
        let auth = use_context::<crate::v2::core::auth::AuthStore>();
        Effect::new(move |_| {
            if !registry_failed.get() {
                // Loading or Ready — no failure to attribute; a successful Retry clears the cause.
                registry_no_modpack.set(false);
                return;
            }
            let Some(auth) = auth else {
                return;
            };
            leptos::task::spawn_local(async move {
                // A single row is enough to learn the status; success and any non-404 both mean
                // "not the no-modpack case", so the arm falls back to the request-failed wording.
                let got: Result<
                    crate::v2::core::api::dto::RegistryResponse,
                    crate::v2::core::api::client::ApiErr,
                > = crate::v2::core::api::client::api_get(auth, "/registry?limit=1&offset=0").await;
                registry_no_modpack.set(matches!(got, Err((404, _))));
            });
        });
    }
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
                        // T-800 — the grammar hint doubles as filter help while healthy, so it
                        // stays in every non-failed state; but above a named failure + Retry it was
                        // just chrome on a corpse (F-05/F-21), so the Factions-catalog Failed state
                        // (Objects has its own registry path) drops it.
                        {move || {
                            (objects_mode.get()
                                || !matches!(catalog.get(), CatalogState::Failed))
                                .then(search_grammar_hint)
                        }}
                        <div class="mt-2">
                            {move || {
                                if objects_mode.get() {
                                    let items = registry_items.get().unwrap_or_default();
                                    let nodes =
                                        crate::v2::apps::editor::arsenal::asset_catalog::build_object_catalog_tree(&items);
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
                                    let filtered = crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                                    if filtered.is_empty() {
                                        // T-646 — a `class:` with an empty operand says so; a genuine
                                        // miss reads "No objects match."
                                        let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(&q, "objects");
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
                                    // T-800 — name the cause (no modpack vs request failed) and
                                    // offer Retry, instead of the old flat dead-end line.
                                    CatalogState::Failed => catalog_failure_view(
                                        "asset catalog",
                                        registry_no_modpack,
                                        registry_fetch_gen,
                                    ),
                                    // T-809 (F-22) — the MERGED Factions tree: build it from the raw
                                    // rows the dock holds, side-filtered by the chips, so characters
                                    // and vehicles (and catalogued objects) sit under one faction.
                                    // `catalog` (the character-only signal) still drives the STATE
                                    // arms above — it is the fetch's Loading/Failed/Ready oracle; only
                                    // the tree the Ready arm DRAWS comes from `build_faction_catalog_tree`.
                                    CatalogState::Ready(_) => {
                                        let items = registry_items.get().unwrap_or_default();
                                        let side = active_side.get();
                                        let nodes = crate::v2::apps::editor::arsenal::asset_catalog::build_faction_catalog_tree(
                                            &items, &side,
                                        );
                                        if nodes.is_empty() {
                                            return view! {
                                                <p class="text-label-sm text-outline">"No placeable assets."</p>
                                            }
                                            .into_any();
                                        }
                                        let q = search.get();
                                        if q.trim().is_empty() {
                                            // Track the collapse set so a chevron toggle re-renders the
                                            // tree (faction_palette_rows reads it untracked).
                                            faction_collapsed.track();
                                            faction_palette_rows(
                                                &nodes,
                                                0,
                                                &[],
                                                &[],
                                                faction_collapsed,
                                                favourites,
                                                registry_items,
                                                recent_placed,
                                            )
                                        } else {
                                            // Search spans the MERGED tree — `class:`/`mod:` reach the
                                            // vehicle leaves now inside the faction (filter_catalog is
                                            // kind-agnostic; it runs over whatever tree it is handed).
                                            let filtered =
                                                crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                                            if filtered.is_empty() {
                                                // T-646 — `class:` empty operand says so (see
                                                // `search_empty_message`); a real miss reads "No assets match."
                                                let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
                                                    &q, "assets",
                                                );
                                                view! {
                                                    <p class="text-label-sm text-outline">{msg}</p>
                                                }
                                                    .into_any()
                                            } else {
                                                faction_palette_rows(
                                                    &filtered,
                                                    0,
                                                    &[],
                                                    &[],
                                                    no_collapse,
                                                    favourites,
                                                    registry_items,
                                                    recent_placed,
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
                    //
                    // T-809 (F-22) — DECISION: the Vehicles tab is KEPT as a FILTERED VIEW, not
                    // retired. Every vehicle is now also reachable inside its faction on the Factions
                    // tab (the merged tree), so this tab's job narrows to "all placeable vehicles at
                    // once, across factions" — a kind-filtered cross-cut of the same catalogue, which
                    // is exactly what `vehicle_catalog` (the `kind == "vehicle"` half of the fetch)
                    // already is. T-818 deleted the Placed strip (crew/heading/cargo now live in the
                    // vehicle Attributes modal); this tab is catalog tree + search only. The ORBAT
                    // picker reads vehicles through `registry_vehicle_options` off the RAW rows, not
                    // through this tab. Its leaves render through `faction_palette_rows` too, so
                    // placing from here also feeds recently-placed and resolves its arm the same way
                    // the merged tree does.
                    1 => view! {
                        <h3 class="mt-2 text-label-md font-semibold text-on-surface">"Vehicles"</h3>
                        <p class="mt-0.5 text-label-sm normal-case text-outline">
                            "Every placeable vehicle, across factions — a filtered view of the catalog. Drag one onto the map to place it."
                        </p>
                        <input
                            type="search"
                            aria-label="Search vehicles"
                            // T-084 — same grammar, same copy, on all three palettes.
                            placeholder=format!("Search vehicles{SEARCH_PLACEHOLDER_GRAMMAR}")
                            class="mt-2 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-sm text-on-surface outline-none transition-colors placeholder:text-outline focus:border-primary/60"
                            on:input=move |ev| vehicle_search.set(event_target_value(&ev))
                        />
                        // T-800 — same rule as the Factions tab: the hint is filter help while
                        // healthy, so it stays in Loading/Ready, but it does not sit above a named
                        // vehicle-catalog failure + Retry.
                        {move || {
                            (!matches!(vehicle_catalog.get(), CatalogState::Failed))
                                .then(search_grammar_hint)
                        }}
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
                                    // T-800 — name the cause (no modpack vs request failed) and
                                    // offer Retry, matching the Factions tab.
                                    CatalogState::Failed => catalog_failure_view(
                                        "vehicle catalog",
                                        registry_no_modpack,
                                        registry_fetch_gen,
                                    ),
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
                                            // T-809 — render through the merged-tree path so a place
                                            // from here feeds recently-placed and resolves its arm the
                                            // same way (every leaf here is a vehicle → Vehicle arm).
                                            faction_palette_rows(
                                                &nodes,
                                                0,
                                                &[],
                                                &[],
                                                vehicle_collapsed,
                                                favourites,
                                                registry_items,
                                                recent_placed,
                                            )
                                        } else {
                                            let filtered =
                                                crate::v2::apps::editor::arsenal::asset_catalog::filter_catalog(&nodes, &q);
                                            if filtered.is_empty() {
                                                // T-646 — `class:` empty operand says so; else "No vehicles match."
                                                let msg = crate::v2::apps::editor::arsenal::asset_catalog::search_empty_message(
                                                    &q, "vehicles",
                                                );
                                                view! {
                                                    <p class="text-label-sm text-outline">{msg}</p>
                                                }
                                                    .into_any()
                                            } else {
                                                faction_palette_rows(
                                                    &filtered,
                                                    0,
                                                    &[],
                                                    &[],
                                                    no_collapse,
                                                    favourites,
                                                    registry_items,
                                                    recent_placed,
                                                )
                                            }
                                        }
                                    }
                                }
                            }}
                        </div>
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
                    // T-695 / T-809 — the Favourites + Recently-placed PAIR (Eden's Assets|History).
                    // One tab, two subtabs: Favourites is the manual starred collection (T-695);
                    // History is the automatic session recently-placed list (T-809, F-22). The pair
                    // shares a tab rather than adding an eighth strip cell — the strip is glyph-tight
                    // (T-637), and Eden itself pairs these two under one surface.
                    6 => view! {
                        <div class="mt-2 flex items-center gap-0.5" role="tablist" aria-label="Assets and history">
                            <button
                                type="button"
                                role="tab"
                                aria-selected=move || (!history_open.get()).to_string()
                                class=move || if history_open.get() { SUBTAB_OFF } else { SUBTAB_ON }
                                on:click=move |_| history_open.set(false)
                            >
                                "Favourites"
                            </button>
                            <button
                                type="button"
                                role="tab"
                                aria-selected=move || history_open.get().to_string()
                                class=move || if history_open.get() { SUBTAB_ON } else { SUBTAB_OFF }
                                on:click=move |_| history_open.set(true)
                            >
                                "Recently placed"
                            </button>
                        </div>
                        {move || {
                            if history_open.get() {
                                recently_placed_panel(recent_placed, registry_items)
                            } else {
                                favourites_panel(
                                    favourites,
                                    registry_items,
                                    registry_failed,
                                    registry_fetch_gen,
                                )
                            }
                        }}
                    }
                        .into_any(),
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
// panel is one function (native-stubbed with the same signature, like `zones_panel`) with three jobs:
//   • SAVE (COMP-SAVE-001): a "Save composition…" header affordance, shown only when a selection
//     exists, that opens a small INLINE title/category form (not a new dialog file) and writes the
//     row from the current selection.
//   • LIST + PLACE (COMP-PLACE-001): the saved compositions grouped by category, each row showing
//     its title, an author line and an entity count; a row press ARMS the place (the T-647 armed
//     state — the canvas release stamps it as one undo step via `place_composition`).
//   • EDIT (COMP-EDIT-001 + the three ATTR-FIELD-COMP-* metadata fields): inline rename /
//     recategorize / delete (the T-666 hover-actions + inline-input idiom).
//
// T-781 widened what a stamp can hold, and the panel says so because neither fact is visible from a
// row's entity count: a selected COMMENT is captured as authoring metadata (the composable clause of
// PLACE-COMMENT-001) and is stamped back into the editor-only root that never compiles, and every
// placed entity keeps the ELEVATION it was saved at instead of landing at ground level. Both live at
// the capture (`editor_ops::capture_selection_entities`) and the place
// (`MissionDocCore::place_composition`); `a_composition_captures_comments_and_authored_elevation`
// below pins that seam, because both ends are unreachable from a native test in this crate.

/// T-650 — the Compositions panel. `editing` holds the composition id currently in inline-edit
/// (rename/recategorize), or `None`.
#[cfg(target_arch = "wasm32")]
pub(crate) fn compositions_panel(
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

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
                    engine_ops::composition_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Reusable multi-entity stamps. Select entities and Save; click a saved row to arm, then click the map to place."
        </p>
        // T-781 — the two properties an author cannot see from the row summary, said once here: a
        // placement restores the elevation each entity was saved at, and a comment carried in the
        // selection is captured too (the composable clause of PLACE-COMMENT-001) and stays
        // editor-only.
        //
        // **The comment clause is CONDITIONAL ("any comment in the selection") on purpose, and the
        // no-selection line below no longer says "or comments".** The capture and place halves are
        // built and pinned, but there is currently NO UI lane that puts a comment id into a
        // selection: the Outliner comment row is `ROW_STATIC`, the map glyph has no pick path
        // (`route_target` has no comment arm, so even a dock-left search hit for a comment renders
        // inert), and the T-697 selection-filter apply only NARROWS a selection that already exists.
        // Copy in the imperative — "select comments" — would name a gesture an author cannot
        // perform. **T-784 (Outliner comment row + map glyph pick) is what closes that gap**; this
        // wording is true on both sides of it, so it does not want rewriting when T-784 lands.
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "A save takes the whole selection: placed entities keep the elevation they were saved at, and any comment in the selection is captured and stays editor-only — comments never reach the compiled mission."
        </p>

        // ── Armed state (T-791) — LIVE while a composition row is armed for placement ─────────────
        // Mirrors the Markers panel's KeepArmed block: it appears the instant a row is armed
        // (`begin_place_composition` bumps `doc_tick`) and vanishes when the arm clears — a canvas
        // click that stamps, RMB, a release over chrome, OR Esc (all funnel through
        // `cancel_pending`, which bumps `doc_tick`). Without this the panel's copy above only ever
        // PROMISED the place; nothing told the author the arm was live or that Esc had cleared it
        // (the F-30 "the hint keeps promising the thing that never happens" report).
        {move || {
            let _ = doc_tick.get();
            let Some(armed) = armed_placement::armed_composition_id() else {
                return ().into_any();
            };
            // The row's title for the readout; fall back to the id if the row was deleted out from
            // under a live arm (an edge, but a deleted row still leaves a stale `Pending`). Never an
            // invented placeholder — an untitled row already reads "Untitled" everywhere else.
            let label = engine_ops::composition_rows()
                .into_iter()
                .find(|r| r.id == armed)
                .map(|r| {
                    if r.title.trim().is_empty() {
                        "Untitled".to_string()
                    } else {
                        r.title
                    }
                })
                .unwrap_or(armed);
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {format!("Placing \u{201c}{label}\u{201d}")}
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Click the map to stamp it at the cursor. Esc or right-click to cancel."
                    </p>
                </div>
            }
                .into_any()
        }}

        // ── Save affordance (shown only when a selection exists) ──────────────────────────────
        {move || {
            let _ = doc_tick.get();
            if website_map_engine::editing::host::selection_len() == 0 {
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
                                    let author = use_context::<crate::v2::core::auth::AuthStore>()
                                        .and_then(|s| s.user.get_untracked().map(|u| u.username))
                                        .filter(|u| !u.is_empty())
                                        .unwrap_or_else(|| "You".to_string());
                                    let _ = engine_ops::save_composition(title, category, author);
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
                        {move || format!("Save composition… ({} selected)", website_map_engine::editing::host::selection_len())}
                    </button>
                }
                    .into_any()
            }
        }}

        // ── The saved compositions, grouped by category ───────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let rows = engine_ops::composition_rows();
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
            let mut groups: Vec<(String, Vec<engine_ops::CompositionRow>)> = Vec::new();
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
    c: engine_ops::CompositionRow,
    doc_tick: RwSignal<u64>,
    editing: RwSignal<Option<String>>,
    row: &'static str,
    row_active: &'static str,
) -> AnyView {
    use crate::v2::apps::editor::bridge::host_state::editor_context;

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
    let (del_id, save_id) = (id.clone(), id.clone());

    view! {
        <li>
            {move || {
                if is_editing() {
                    let (save_id, edit_title, edit_category, edit_author) = (
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
                                        engine_ops::rename_composition(save_id.clone(), t);
                                        engine_ops::recategorize_composition(
                                            save_id.clone(),
                                            edit_category.get_untracked(),
                                        );
                                        engine_ops::set_composition_author(
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
                                    armed_placement::begin_place_composition(arm_id.clone());
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
                                    if engine_ops::delete_composition(&del_id) {
                                        editor_context::cancel_armed_composition(&del_id);
                                    }
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
// draw controls call `armed_placement::begin_zone_draw(&activation, shape, DrawTarget::Trigger)` and the
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
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_token, DrawTarget, ZoneShape,
    };
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

    // The activation the NEXT draw will carry, seeded to the first of the three (presence).
    let draw_activation = RwSignal::new(
        engine_ops::TRIGGER_ACTIVATIONS
            .first()
            .copied()
            .unwrap_or("presence")
            .to_string(),
    );

    let arm = move |shape: ZoneShape| {
        let activation = draw_activation.get_untracked();
        // SECOND CONSUMER: identical call to the Zones panel's `arm`, targeting triggers.
        armed_placement::begin_zone_draw(&activation, shape, DrawTarget::Trigger);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Triggers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::trigger_count()
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
            {engine_ops::TRIGGER_ACTIVATIONS
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
            let Some(d) = armed_placement::zone_draft() else {
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
            let can_close = is_poly && crate::v2::apps::editor::ui::inspector::zones_panel::polygon_is_committable(&d.verts);
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
                                            armed_placement::close_zone_polygon();
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
                                            armed_placement::zone_draw_pop_vertex();
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
                                armed_placement::cancel_zone_draw();
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
            let rows = engine_ops::trigger_rows();
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
            let Some(t) = engine_ops::trigger_rows().into_iter().find(|r| r.id == id) else {
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
/// (CONN-TRG-OWNER-001), reshape, schema-driven rules, delete. The [`crate::v2::apps::editor::ui::inspector::zones_panel`]
/// `zone_attributes` twin, with the owner picker + activation in place of zone label/faction/type.
#[cfg(target_arch = "wasm32")]
fn trigger_attributes(
    t: engine_ops::TriggerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_token, DrawTarget, ZoneShape,
    };

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
    let owner_opts = engine_ops::placed_owner_options();
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
                    engine_ops::set_trigger_name(&id_name, next);
                    bump();
                }
            />

            <label class=field_label>"Activation"</label>
            <select
                aria-label="Trigger activation"
                class=input_class
                on:change=move |ev| {
                    engine_ops::set_trigger_activation(&id_activation, &event_target_value(&ev));
                    bump();
                }
            >
                {
                    let current = t.activation.clone();
                    engine_ops::TRIGGER_ACTIVATIONS
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
                    engine_ops::set_trigger_owner(&id_owner, next);
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
                                armed_placement::begin_zone_reshape(&a, ZoneShape::Circle, DrawTarget::Trigger);
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
                                armed_placement::begin_zone_reshape(&b, ZoneShape::Polygon, DrawTarget::Trigger);
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
            {crate::v2::apps::editor::ui::inspector::zones_panel::zone_rule_fields()
                .into_iter()
                .map(|f| trigger_rule_control(tid.clone(), f, rules.clone(), doc_tick))
                .collect_view()}

            <button
                type="button"
                class="mt-3 w-full rounded-md border border-error/40 px-2 py-1.5 text-label-sm text-error transition-colors hover:bg-error/15"
                on:click=move |_| {
                    engine_ops::delete_trigger(&id_delete);
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
    f: crate::v2::apps::editor::ui::inspector::zones_panel::ZoneRuleField,
    rules: serde_json::Value,
    doc_tick: RwSignal<u64>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_key, humanize_token, ZoneRuleKind,
    };

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
                            engine_ops::set_trigger_rule(&trigger_id, &k, Some(serde_json::Value::Bool(on)));
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
                            engine_ops::set_trigger_rule(&trigger_id, &k, next);
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
                                engine_ops::set_trigger_rule(&trigger_id, &k, next);
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
                            engine_ops::set_trigger_rule(&trigger_id, &k, next);
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
/// [`crate::v2::apps::editor::ui::inspector::zones_panel::project_owner_line`]. Nothing renders when no trigger is selected or the
/// owner is dangling (`owner_line_world` returns `None`).
#[cfg(target_arch = "wasm32")]
#[component]
fn TriggerOwnerLine(selected: RwSignal<Option<String>>, doc_tick: RwSignal<u64>) -> impl IntoView {
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

    let projected =
        move || -> Option<crate::v2::apps::editor::ui::inspector::zones_panel::ProjectedOwnerLine> {
            // Subscribe to selection, doc edits (owner assign / geometry / delete) and the pan heartbeat
            // (`tick`, bumped per rAF while selected). The camera is read live off the snapshot.
            let _ = doc_tick.get();
            let _ = tick.get();
            let sel = selected.get();
            let (world_a, world_b) = engine_ops::owner_line_world(sel.as_deref())?;
            let (tx, ty, zoom) = website_map_engine::streaming::host::camera_snapshot()?;
            let win = web_sys::window()?;
            let vw = win.inner_width().ok().and_then(|v| v.as_f64())?;
            let vh = win.inner_height().ok().and_then(|v| v.as_f64())?;
            if vw <= 0.0 || vh <= 0.0 {
                return None;
            }
            // Full-bleed canvas → the camera viewport IS the whole window, built exactly as the ruler
            // overlay does (`select_tool::frozen_camera`).
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
            let project = move |x: f64, y: f64| {
                let p = cam.project([x, y, 0.0]);
                (p[0], p[1])
            };
            Some(
                crate::v2::apps::editor::ui::inspector::zones_panel::project_owner_line(
                    world_a, world_b, project,
                ),
            )
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
// that body. (Pre-T-069 the T-215 / T-695 pins passed on that LIVE stub — the sentence lived in the
// rendered view and in the pins' own split literals, never as a comment decoy. Wave-116 finding 5
// struck that decoy narrative. The inverted pins still refuse a contiguous reintroduction of the
// stub text anywhere in this module — including comments — so this file cannot become its own
// haystack.)
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
// Read the shared crate embed (`eden_zones::MISSION_SCHEMA`) rather than a second `include_str!`
// (wave-135 H2 / T-757 follow-through). ONE embed, ONE vocabulary.
//
// SCOPE: the four schema-carried fields. `$defs/marker` also declares `size` / `rotationDeg` /
// `shape` / `area`, each stamped "T-673, lands after T-069" in its own schema description — marker
// STYLE and Eden's second Area-marker model. This panel authors none of them.

/// `mission.schema.json` via the crate's single embed — the ONE source of the marker icon vocabulary.
const MISSION_SCHEMA_JSON: &str =
    crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA;

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
/// Every marker write in [`website_map_engine::editing::hosted_commands::map_markers`] passes through this. It is exact and
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

// ── T-806 (F-08) — one picker row per CANONICAL icon ─────────────────────────────────────────────
//
// F-08: the picker was 64 rows of raw schema slugs, every one wearing the same generic `place` pin,
// the human name truncated to "Ob…" while the slug got the width, and case-duplicates
// (Waypoint/waypoint, Objective/Obj/Target, mark/marker/point) rendered as separate rows. Choosing
// by sight was impossible. The fix collapses the DISPLAY onto T-790's canonical glyph families: one
// row per [`map_engine_render::scene::MarkerGlyph`], each drawing that glyph's real shape (a distinct
// inline SVG, mirroring the map's vocabulary), the human name taking the width, the slug demoted to
// the tooltip + a search-match key. The DOCUMENT still stores the canonical slug (collapse at display
// AND at write) — a canonical write is what makes the post-T-790 map draw the matching glyph.
//
// SOURCE OF TRUTH for the family set + the folding is `scene::marker_glyph_for_alias` /
// `scene::MarkerGlyph` (T-790, a wasm32-only dep). The one representative slug per family is named
// here so the native surface (row order, count, labels) can be tested without linking that dep; the
// wasm side (`canonical_marker_rows`) folds the live schema list through the real mapper. Two ties
// keep this list from silently drifting from the glyphs the map draws, and neither needs the (absent)
// wasm-bindgen-test harness: a COMPILE-TIME `const _` assert pins the count to
// `scene::MARKER_GLYPH_COUNT` (fails the wasm32 build on drift), and a runtime `debug_assert` +
// self-healing fallback in `canonical_marker_rows` guarantees each row stores a slug that folds back
// to its own glyph.

/// One representative schema alias per canonical [`map_engine_render::scene::MarkerGlyph`] family,
/// in the schema-order the families first appear in (the browse order the picker keeps).
///
/// Each entry is (a) a member of the closed `$defs/marker.icon` enum — so a pick validates and saves —
/// and (b) folds back to its own family via `scene::marker_glyph_for_alias`, so picking it makes the
/// map draw that family's glyph. Invariant (a) is asserted natively
/// ([`tests::picker_has_one_row_per_canonical_icon`]); invariant (b) is enforced on the wasm side by
/// the runtime `debug_assert` + self-healing fallback in [`canonical_marker_rows`] (the mapper is a
/// wasm32-only dep the native tests cannot link).
///
/// The label a row shows is `humanize_token` of the slug, so the human-readable stem was chosen over
/// the schema's paired base glyph where they differ (`objective`, not `objective_marker`; `medical`,
/// not `cross`) — the base still lives in that family and matches search via the alias list.
const CANONICAL_MARKER_SLUGS: [&str; CANONICAL_MARKER_GLYPH_COUNT] = [
    "dot",               // Disc      — dot / point / mark / marker (mod FALLBACK_ICON) family
    "objective",         // Square    — objective(_marker) / obj / target / task family
    "point_of_interest", // Diamond   — point_of_interest / poi / intel / contact family
    "observation_post",  // Target    — observation_post / op / observe / overwatch / recon family
    "destroy",           // Ex        — destroy / demolish / demo / sabotage family
    "attack",   // TriangleUp — attack / assault / capture / seize / advance / ambush family
    "defend",   // TriangleDown — defend / hold / garrison / fallback family
    "waypoint", // Chevron   — waypoint / move / wp / route / phase_line family
    "flag",     // Flag      — flag / rally / rally_point / base / hq / spawn family
    "medical",  // Cross     — cross / medical / medic / aid / casevac / medevac family
    "circle",   // Ring      — circle / area / zone / ao family
];

/// The picker's row count: the number of canonical marker glyphs. Mirrors
/// `map_engine_render::scene::MARKER_GLYPH_COUNT` (the source of truth, a wasm32-only dep this native
/// const cannot reference directly); the wasm-side [`canonical_marker_rows`] asserts they agree.
/// Well under the 64 raw schema aliases — that shrink IS the F-08 fix.
const CANONICAL_MARKER_GLYPH_COUNT: usize = 11;

/// Compile-time tie between the mirrored count above and its source of truth. This fails the
/// `wasm32` build the moment T-790's glyph set changes without this picker following — the native
/// test cannot link `scene`, so THIS is what keeps [`CANONICAL_MARKER_GLYPH_COUNT`] honest. (The
/// per-slug glyph round-trip is a runtime `debug_assert` + self-healing fallback in
/// [`canonical_marker_rows`], since `marker_glyph_for_alias` is not a `const fn`.)
#[cfg(target_arch = "wasm32")]
const _: () = assert!(
    CANONICAL_MARKER_GLYPH_COUNT
        == website_map_engine::overlay::symbology::markers::MARKER_GLYPH_COUNT,
    "picker row count must equal scene::MARKER_GLYPH_COUNT (T-790 source of truth)"
);

/// A canonical picker row: the glyph to draw, the slug to STORE on pick (and show in the tooltip),
/// its human label, and every schema alias that folds into this family (the search-match set).
#[cfg(target_arch = "wasm32")]
struct CanonicalMarkerRow {
    glyph: website_map_engine::overlay::symbology::markers::MarkerGlyph,
    /// The canonical slug written to the document on pick — a closed-enum member.
    slug: &'static str,
    /// `humanize_token(slug)`, the label that takes the row width.
    label: String,
    /// Every `$defs/marker.icon` alias that folds to this glyph, for search + the tooltip.
    aliases: Vec<&'static str>,
}

/// The canonical picker rows, built by folding the live schema alias list through the real
/// [`map_engine_render::scene::marker_glyph_for_alias`] so DISPLAY and MAP can never disagree.
///
/// Row order is schema-first-seen (the same browse order [`marker_icons`] documents). Each row's
/// `slug` is [`CANONICAL_MARKER_SLUGS`] chosen for that glyph, its `aliases` are every schema alias
/// that folds into it, and `filter` (empty ⇒ all) keeps a row when the human label, the slug, or any
/// alias substring-matches — so "Search icons" still matches slugs and names.
#[cfg(target_arch = "wasm32")]
fn canonical_marker_rows(filter: &str) -> Vec<CanonicalMarkerRow> {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;
    use website_map_engine::overlay::symbology::markers::marker_glyph_for_alias;
    use website_map_engine::overlay::symbology::markers::MarkerGlyph;
    use website_map_engine::overlay::symbology::markers::MARKER_GLYPH_COUNT;

    // The mirrored count and the source-of-truth count must agree — a wasm build fails loudly here
    // if T-790 ever changes the glyph set without this picker following.
    debug_assert_eq!(
        CANONICAL_MARKER_GLYPH_COUNT, MARKER_GLYPH_COUNT,
        "picker row count must track scene::MARKER_GLYPH_COUNT"
    );

    // Fold every authored alias into its glyph, first-seen order fixes the row order.
    let mut order: Vec<MarkerGlyph> = Vec::with_capacity(MARKER_GLYPH_COUNT);
    let mut aliases_by_glyph: Vec<(MarkerGlyph, Vec<&'static str>)> = Vec::new();
    for alias in marker_icons() {
        let g = marker_glyph_for_alias(alias);
        if let Some(slot) = aliases_by_glyph.iter_mut().find(|(gg, _)| *gg == g) {
            slot.1.push(alias.as_str());
        } else {
            order.push(g);
            aliases_by_glyph.push((g, vec![alias.as_str()]));
        }
    }

    let q = filter.trim().to_ascii_lowercase();
    order
        .into_iter()
        .map(|g| {
            let aliases = aliases_by_glyph
                .iter()
                .find(|(gg, _)| *gg == g)
                .map(|(_, a)| a.clone())
                .unwrap_or_default();
            // The stored slug is this glyph's canonical representative; if (impossibly) it did not
            // fold back to this glyph, fall back to the first alias that did, so a pick always
            // stores something the map will render as THIS row.
            let slug = *CANONICAL_MARKER_SLUGS
                .iter()
                .find(|s| marker_glyph_for_alias(s) == g)
                .unwrap_or_else(|| aliases.first().unwrap_or(&"dot"));
            CanonicalMarkerRow {
                glyph: g,
                slug,
                label: humanize_token(slug),
                aliases,
            }
        })
        .filter(|row| {
            q.is_empty()
                || row.label.to_ascii_lowercase().contains(&q)
                || row.slug.contains(&q)
                || row.slug.replace('_', " ").contains(&q)
                || row
                    .aliases
                    .iter()
                    .any(|a| a.contains(&q) || a.replace('_', " ").contains(&q))
        })
        .collect()
}

/// A small inline SVG drawing one canonical [`map_engine_render::scene::MarkerGlyph`] — a distinct
/// DOM shape per glyph, mirroring the map's glyph vocabulary (the enum names the shapes). This is the
/// picker preview the acceptance calls "a glyph pixel signature that differs from the generic pin":
/// the previous rows all rendered the same `place` Material pin; each shape below is drawn from
/// different SVG primitives, so no two rows (and none vs. the old pin) share a DOM signature.
#[cfg(target_arch = "wasm32")]
fn marker_glyph_svg(
    glyph: website_map_engine::overlay::symbology::markers::MarkerGlyph,
) -> AnyView {
    use website_map_engine::overlay::symbology::markers::MarkerGlyph;

    // 16×16 viewBox; `currentColor` so the shape inherits the row's text colour on hover.
    let inner = match glyph {
        // Hollow ring.
        MarkerGlyph::Ring => view! {
            <circle cx="8" cy="8" r="5.5" fill="none" stroke="currentColor" stroke-width="1.6" />
        }
        .into_any(),
        // Solid disc (the fallback).
        MarkerGlyph::Disc => view! {
            <circle cx="8" cy="8" r="4" fill="currentColor" />
        }
        .into_any(),
        // Filled square.
        MarkerGlyph::Square => view! {
            <rect x="3.5" y="3.5" width="9" height="9" fill="currentColor" />
        }
        .into_any(),
        // Filled diamond.
        MarkerGlyph::Diamond => view! {
            <polygon points="8,2.5 13.5,8 8,13.5 2.5,8" fill="currentColor" />
        }
        .into_any(),
        // Upward triangle.
        MarkerGlyph::TriangleUp => view! {
            <polygon points="8,2.5 14,13.5 2,13.5" fill="currentColor" />
        }
        .into_any(),
        // Downward triangle.
        MarkerGlyph::TriangleDown => view! {
            <polygon points="2,2.5 14,2.5 8,13.5" fill="currentColor" />
        }
        .into_any(),
        // Plus / medical cross.
        MarkerGlyph::Cross => view! {
            <path
                d="M6.5 2.5 h3 v4 h4 v3 h-4 v4 h-3 v-4 h-4 v-3 h4 z"
                fill="currentColor"
            />
        }
        .into_any(),
        // Diagonal X.
        MarkerGlyph::Ex => view! {
            <path
                d="M3.5 3.5 L12.5 12.5 M12.5 3.5 L3.5 12.5"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                fill="none"
            />
        }
        .into_any(),
        // Pennant flag on a staff.
        MarkerGlyph::Flag => view! {
            <path
                d="M4 2 v12"
                stroke="currentColor"
                stroke-width="1.4"
                stroke-linecap="round"
                fill="none"
            />
            <polygon points="4,2.5 13,4.5 4,7.5" fill="currentColor" />
        }
        .into_any(),
        // Chevron (waypoint / move).
        MarkerGlyph::Chevron => view! {
            <path
                d="M3 10 L8 4 L13 10"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                fill="none"
            />
        }
        .into_any(),
        // Concentric target (ring + centre dot).
        MarkerGlyph::Target => view! {
            <circle cx="8" cy="8" r="5.5" fill="none" stroke="currentColor" stroke-width="1.4" />
            <circle cx="8" cy="8" r="1.8" fill="currentColor" />
        }
        .into_any(),
    };

    view! {
        <svg
            class="block h-3.5 w-3.5 shrink-0"
            viewBox="0 0 16 16"
            aria-hidden="true"
            fill="none"
        >
            {inner}
        </svg>
    }
    .into_any()
}

/// T-069 (RIGHT-MODE-006) — the Markers panel: the icon list that arms a place, the authored-marker
/// list, and the three-field Attributes block for the selected one.
///
/// One function with a native stub, exactly like [`zones_panel`] / [`triggers_panel`] /
/// [`compositions_panel`] — the doc reads and writes go through `editor_ops`, which is wasm-only.
#[cfg(target_arch = "wasm32")]
pub(crate) fn markers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

    let icon_search = RwSignal::new(String::new());

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Markers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::marker_count()
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
            // T-806 (F-08): one row per CANONICAL glyph, not one per raw alias. Each row draws its
            // real glyph shape, the human name takes the width, and the canonical SLUG moves to the
            // tooltip (still searchable). Picking arms the canonical slug — a closed-enum member the
            // post-T-790 map renders as this row's glyph.
            let rows = canonical_marker_rows(&icon_search.get());
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
                        .map(|row| {
                            let armed = row.slug.to_string();
                            let glyph = row.glyph;
                            let label = row.label;
                            let aria = format!("{label} marker");
                            // Slug + folded aliases in the tooltip: the demoted slug stays
                            // discoverable on hover without stealing the row width from the name.
                            let tip = if row.aliases.len() > 1 {
                                format!(
                                    "{} — arm, then click the map to place. Slug: {}. Also: {}",
                                    label,
                                    row.slug,
                                    row.aliases.join(", "),
                                )
                            } else {
                                format!(
                                    "{} — arm, then click the map to place. Slug: {}",
                                    label, row.slug,
                                )
                            };
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class=PALETTE_LEAF
                                        title=tip
                                        aria-label=aria
                                        // `pointerdown`, not `click`: the palette arm/release
                                        // contract — the chrome host stops propagation here and the
                                        // map container's `pointerup` commits the drop.
                                        on:pointerdown=move |_| {
                                            armed_placement::begin_place_marker(armed.clone());
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        {marker_glyph_svg(glyph)}
                                        <span class="truncate">{label}</span>
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
            let Some(icon) = armed_placement::armed_marker_icon() else {
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
            let rows = engine_ops::marker_rows();
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
                            let addr = (m.faction_id.clone(), m.id.clone());
                            let sel_addr = addr.clone();
                            let sel_addr2 = addr.clone();
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
                                        aria-pressed=move || selected.get().as_ref() == Some(&sel_addr)
                                        class=move || {
                                            if selected.get().as_ref() == Some(&sel_addr2) {
                                                ROW_ACTIVE
                                            } else {
                                                ROW
                                            }
                                        }
                                        on:click=move |_| selected.set(Some(addr.clone()))
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
            let Some((faction_id, id)) = selected.get() else {
                return ().into_any();
            };
            // T-763 — match on the full `(factionId, id)` address. Id-alone would edit the first
            // faction's row when a hydrated foreign payload reused the same marker id on two sides.
            let Some(m) = engine_ops::marker_rows()
                .into_iter()
                .find(|r| r.faction_id == faction_id && r.id == id)
            else {
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
    m: engine_ops::MarkerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;

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
                    if engine_ops::set_marker_icon(&f_icon, &i_icon, &next, marker_icon_is_authorable) {
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
                    if engine_ops::set_marker_label(&f_label, &i_label, &next) {
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
                        if engine_ops::set_marker_position(&f_x, &i_x, next, z_value) {
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
                        if engine_ops::set_marker_position(&f_z, &i_z, x_value, next) {
                            bump();
                        }
                    }
                />
            </div>

            <button
                type="button"
                class="mt-2 rounded-md px-2 py-1 text-label-sm text-error transition-colors hover:bg-error/10"
                on:click=move |_| {
                    if engine_ops::remove_marker(&f_del, &i_del) {
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
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}

#[cfg(test)]
#[path = "tests/dock_right/mod.rs"]
mod tests;

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
#[path = "tests/dock_right/tab_strip_budget.rs"]
mod t637_tab_strip_budget;

/* ════════ T-754 — the Zones panel's selection is reachable, so a routed zone click lands ═════════
 *
 * The wave-115 MAJOR was that T-655's router could not select a zone: `zone_selected` is declared
 * inside [`DockRight`]'s body and the router lives in `mission_editor.rs`. These pin the seam that
 * closes that gap — behaviourally (an unmounted panel reports honestly; a mounted one receives the
 * id) and on the source (the hook drives the panel's OWN signal and raises the tab it is visible on).
 */
#[cfg(test)]
#[path = "tests/dock_right/zone_selection_seam.rs"]
mod t754_zone_selection_seam;

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
#[path = "tests/dock_right/zone_hook_lifecycle.rs"]
mod f2_zone_hook_lifecycle;
