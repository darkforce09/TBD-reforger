//! Mission Library (/missions) — ported from pages/missions.tsx `MissionLibraryPage`. `<AuthGate>` →
//! `/missions` Resource → a scope-tabbed, filterable library: a featured hero + a mission grid.
//!
//! T-159.25: the FULL interactive surface — live scope tabs + search/filter (query params), the
//! featured hero (global newest, stable across tabs), the mission grid, the slide-over dossier
//! Sheet (hero header, shared `dossier_body`, collaboration stubs, lifecycle archive/delete with
//! the Aegis confirm Dialog, sticky OPEN IN MISSION CREATOR footer), the transient
//! CreateMissionDialog (New Mission button + true-empty CTA + Cmd/Ctrl+N), toasts.
#![allow(dead_code)]
use crate::create_mission_dialog::CreateMissionDialog;
use crate::dto::{MissionCard, MissionDetail, Paginated};
use crate::nav::{has_min_role_authed, Role};
use crate::ui::{badge_class, AuthGate, MaterialIcon, Sheet};
use leptos::prelude::*;

const SELECT_CLASS: &str = "rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
// SCOPES: (label, scope query value). Global is scopeIdx 0.
const SCOPES: [(&str, &str); 3] = [
    ("Global Missions", "global"),
    ("My Missions", "mine"),
    ("Bookmarked", "bookmarked"),
];
// Cinematic fallback art so cards/hero never render as empty grey blocks (missions.tsx).
const PLACEHOLDER_ART: &str = "https://lh3.googleusercontent.com/aida/AP1WRLtxuwSoyDyCrRuQu8gTHWuSmoOWZq8e7gw0bSjjZCmteU96TomvCGHto-cuqHYV_0gxNUjw_Lx2SWgiEl2W3vEi6aVH84DpTky5lG8-FKDJOzH96TrwAJwGJwE3DSwSN1gRC7miWds0X7kNvMAZRBgQPu_5g2iX9RtJ3WYUlgHbfVLYcmV7TaHPUvhZHvvvKenG2B3S2CRER15d2kdG5YNFbtFwtwgzEIeYG2jP4GubWd7SMO0bADPFFA";

fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
fn game_mode_label(m: &str) -> &str {
    match m {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// T-264 — `MissionCard.bookmarked` rides the `extra` catch-all (dto.rs `MISSION_CARD_EXTRA`);
/// `MissionDetail.bookmarked` is a named field. The card control must read the wire bool the
/// Bookmarked tab filters on, not invent a second source of truth.
fn card_is_bookmarked(m: &MissionCard) -> bool {
    m.extra
        .get("bookmarked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// `POST|DELETE /missions/:id/bookmark` — pinned by the Class-R source test below.
fn bookmark_api_path(id: &str) -> String {
    format!("/missions/{id}/bookmark")
}

/// Mission visibility badge — status → (label, badge variant), missions.tsx `VISIBILITY`.
///
/// **T-389 — `rejected` was missing and fell through to the `other` arm**, which renders the raw
/// database string (`rejected`, lowercase, unlabelled) in the neutral grey used for `draft` and
/// `archived`. That is the one status an author most needs to notice, shown as the least noticeable
/// thing on the card. `missions.status` is a Postgres enum with exactly five values (`draft`,
/// `pending_approval`, `live`, `rejected`, `archived` — `migrations/01_enums.sql`), so with this arm
/// every reachable status is now named and the `other` fallback is genuinely unreachable defence
/// rather than a silent hole.
fn visibility_badge(status: &str) -> impl IntoView + use<> {
    let (label, variant) = match status {
        "draft" => ("Draft".to_string(), "neutral"),
        "pending_approval" => ("Open for review".to_string(), "warning"),
        "live" => ("Live".to_string(), "success"),
        "rejected" => ("Returned".to_string(), "error"),
        "archived" => ("Archived".to_string(), "neutral"),
        other => (other.to_string(), "neutral"),
    };
    view! { <span class=badge_class(variant)>{label}</span> }
}

/// Build the `/missions` query (useMissions params: scope + optional q/terrain/mode/player_count).
fn missions_query(scope: &str, q: &str, terrain: &str, mode: &str, players: &str) -> String {
    let mut url = format!("/missions?scope={scope}");
    #[cfg(target_arch = "wasm32")]
    let enc = |s: &str| {
        js_sys::encode_uri_component(s)
            .as_string()
            .unwrap_or_default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let enc = |s: &str| s.to_string();
    if !terrain.is_empty() {
        url.push_str(&format!("&terrain={}", enc(terrain)));
    }
    if !mode.is_empty() {
        url.push_str(&format!("&mode={}", enc(mode)));
    }
    if !players.is_empty() {
        url.push_str(&format!("&player_count={}", enc(players)));
    }
    if !q.is_empty() {
        url.push_str(&format!("&q={}", enc(q)));
    }
    url
}

#[component]
pub fn MissionLibraryPage() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // T-286 — reactive + authed: browse-mode `has_min_role(None)=>true` must NOT drive New Mission.
    // Pre-bootstrap `None` (or a guest) is false; after session lands the Memo re-reads the role.
    let is_maker = Memo::new(move |_| {
        has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)
    });
    let scope_idx = RwSignal::new(0usize);
    let q = RwSignal::new(String::new());
    let terrain = RwSignal::new(String::new());
    let mode = RwSignal::new(String::new());
    let players = RwSignal::new(String::new());
    let preview_id = RwSignal::new(None::<String>);
    let create_open = RwSignal::new(false);
    let sheet_open = RwSignal::new(false);
    // T-389 — the viewer's own discord_id, so a card can tell "my mission came back" from "someone
    // else's". `GET /missions` already refuses to list a non-live mission to anyone but its author
    // (`push_filters`: `status = 'live' OR (author_id = me AND …)`), so this is belt-and-braces
    // rather than the only guard — but the `bookmarked` scope has no status predicate at all, and
    // relying on a server-side WHERE clause to keep a reviewer's private note off someone else's
    // screen is exactly the kind of implicit coupling that breaks quietly.
    let me_id = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));

    let missions = LocalResource::new(move || {
        let url = missions_query(
            SCOPES[scope_idx.get().min(2)].1,
            &q.get(),
            &terrain.get(),
            &mode.get(),
            &players.get(),
        );
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::client::api_get::<Paginated<MissionCard>>(store, &url)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, url);
                None::<Paginated<MissionCard>>
            }
        }
    });
    // The hero always spotlights the newest GLOBAL operation so it stays stable across tabs.
    let global = LocalResource::new(move || {
        let url = missions_query(
            "global",
            &q.get(),
            &terrain.get(),
            &mode.get(),
            &players.get(),
        );
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                crate::client::api_get::<Paginated<MissionCard>>(store, &url)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, url);
                None::<Paginated<MissionCard>>
            }
        }
    });

    // Create is a transient action; close the dossier Sheet first (one overlay at a time).
    let open_create = move || {
        preview_id.set(None);
        sheet_open.set(false);
        create_open.set(true);
    };
    let open_preview = move |id: String| {
        preview_id.set(Some(id));
        sheet_open.set(true);
    };

    // Cmd/Ctrl+N opens the create dialog (mission_maker+ only), unless a field is focused.
    #[cfg(target_arch = "wasm32")]
    {
        let handle = window_event_listener(leptos::ev::keydown, move |ev| {
            if !is_maker.get_untracked() || create_open.get_untracked() {
                return;
            }
            if ev.key().to_lowercase() != "n" || !(ev.meta_key() || ev.ctrl_key()) {
                return;
            }
            if let Some(el) = document().active_element() {
                let tag = el.tag_name();
                if tag == "INPUT" || tag == "TEXTAREA" || tag == "SELECT" {
                    return;
                }
            }
            ev.prevent_default();
            open_create();
        });
        on_cleanup(move || handle.remove());
    }

    let refetch_all = Callback::new(move |()| {
        missions.refetch();
        global.refetch();
    });

    view! {
        <AuthGate>
            <div class="relative h-full w-full overflow-hidden">
                // T-173 P4 — the glass tint is baked into the static background layer instead of a
                // `backdrop-blur-xl` ON the scrollport (which re-blurred the whole page every scroll
                // frame — the page-local twin of the T-172 A3 body bug).
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="absolute inset-0 z-0 bg-surface-glass"></div>
                <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto">
                    <div class="p-6 md:p-8">
                        {library_header(is_maker, scope_idx, open_create)}
                        <Suspense fallback=move || {
                            view! { <p class="text-on-surface-variant">"Loading…"</p> }
                        }>
                            {move || {
                                missions
                                    .get()
                                    .map(|opt| match opt {
                                        Some(page) => {
                                            let featured = global
                                                .get()
                                                .flatten()
                                                .and_then(|g| g.data.first().cloned());
                                            let no_filters = q.get().is_empty()
                                                && terrain.get().is_empty() && mode.get().is_empty()
                                                && players.get().is_empty();
                                            let show_empty_cta = is_maker.get()
                                                && SCOPES[scope_idx.get().min(2)].1 == "mine"
                                                && page.data.is_empty() && no_filters;
                                            body(
                                                    page.data,
                                                    featured,
                                                    show_empty_cta,
                                                    q,
                                                    terrain,
                                                    mode,
                                                    players,
                                                    me_id,
                                                    open_preview,
                                                    open_create,
                                                    refetch_all,
                                                )
                                                .into_any()
                                        }
                                        None => {
                                            view! {
                                                <p class="text-error">"Failed to load data."</p>
                                            }
                                                .into_any()
                                        }
                                    })
                            }}
                        </Suspense>
                    </div>
                </div>
            </div>

            // Slide-over mission dossier (no full-page navigation).
            <Sheet open=sheet_open bleed=true class="w-full max-w-none md:w-[60vw]">
                {move || {
                    preview_id
                        .get()
                        .map(|id| {
                            view! {
                                <MissionDossierSheet
                                    id=id
                                    sheet_open=sheet_open
                                    changed=refetch_all
                                />
                            }
                        })
                }}
            </Sheet>

            // Transient create dialog (replaces the old /missions/create wizard).
            <CreateMissionDialog open=create_open />
        </AuthGate>
    }
}

fn library_header(
    is_maker: Memo<bool>,
    scope_idx: RwSignal<usize>,
    open_create: impl Fn() + Copy + Send + 'static,
) -> impl IntoView {
    view! {
        <header class="mb-6 flex flex-wrap items-start justify-between gap-4">
            <div>
                <h1 class="text-4xl font-bold tracking-tight text-on-surface uppercase">
                    "Mission Library"
                </h1>
                <p class="mt-1 text-body-md text-on-surface-variant">
                    "Browse, filter, and deploy active operations across the theater."
                </p>
                <div class="mt-5 inline-flex gap-1 rounded-full border border-white/5 bg-black/20 p-1">
                    {SCOPES
                        .iter()
                        .enumerate()
                        .map(|(i, (label, _))| {
                            // cn(): text-label-md twMerge-dropped vs the trailing text-{color}.
                            view! {
                                <button
                                    type="button"
                                    on:click=move |_| scope_idx.set(i)
                                    class=move || {
                                        if scope_idx.get() == i {
                                            "rounded-full px-4 py-1.5 font-medium transition-all bg-surface-glass text-on-surface shadow-md"
                                        } else {
                                            "rounded-full px-4 py-1.5 font-medium transition-all text-on-surface-variant hover:text-on-surface"
                                        }
                                    }
                                >
                                    {*label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </div>
            {move || {
                is_maker.get().then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| open_create()
                            title="New Mission (Ctrl+N)"
                            class="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90"
                        >
                            <MaterialIcon name="add" class="text-[18px]" />
                            "New Mission"
                        </button>
                    }
                })
            }}
        </header>
    }
}

#[allow(clippy::too_many_arguments)]
fn body(
    missions: Vec<MissionCard>,
    featured: Option<MissionCard>,
    show_empty_cta: bool,
    q: RwSignal<String>,
    terrain: RwSignal<String>,
    mode: RwSignal<String>,
    players: RwSignal<String>,
    me_id: StoredValue<Option<String>>,
    open_preview: impl Fn(String) + Copy + 'static,
    open_create: impl Fn() + Copy + 'static,
    changed: Callback<()>,
) -> impl IntoView {
    view! {
        <>
            // Featured Operation — cinematic hero ("LIVE OPERATION" is presentational).
            {featured
                .map(|f| {
                    let art = f
                        .thumbnail_url
                        .clone()
                        .filter(|u| !u.is_empty())
                        .unwrap_or_else(|| PLACEHOLDER_ART.into());
                    let brief = f
                        .briefing
                        .clone()
                        .filter(|b| !b.is_empty())
                        .unwrap_or_else(|| {
                            "Command has flagged this operation as the priority deployment. Review the dossier for objectives, ORBAT, and the armory loadout before committing forces to the field."
                                .into()
                        });
                    let fid = f.id.clone();
                    view! {
                        <section class="relative mb-8 flex min-h-[320px] flex-col overflow-hidden rounded-2xl border border-white/10 bg-black/30 lg:flex-row">
                            <div class="relative z-10 flex w-full flex-col justify-center gap-4 p-8 lg:w-3/5">
                                <div class="flex items-center gap-2 font-mono text-label-sm tracking-widest text-error-alert uppercase">
                                    <span class="relative flex h-2.5 w-2.5">
                                        <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-error-alert opacity-60"></span>
                                        <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-error-alert"></span>
                                    </span>
                                    "Live Operation"
                                </div>
                                <h2 class="text-4xl font-black tracking-tighter text-on-surface uppercase xl:text-5xl">
                                    {f.title.clone()}
                                </h2>
                                <p class="max-w-prose text-body-md text-on-surface-variant line-clamp-3">
                                    {brief}
                                </p>
                                <div class="flex flex-wrap items-center gap-2">
                                    <span class=badge_class(
                                        "primary",
                                    )>{game_mode_label(&f.game_mode).to_string()}</span>
                                    <span class=badge_class("neutral")>{terrain_label(&f.terrain)}</span>
                                    <span class=badge_class(
                                        "tertiary",
                                    )>{f.max_players} " OPERATORS"</span>
                                </div>
                                <div>
                                    <button
                                        type="button"
                                        on:click=move |_| open_preview(fid.clone())
                                        class="mt-2 rounded-lg bg-primary px-6 py-3 font-mono text-label-md font-semibold tracking-wider text-on-primary uppercase transition-transform hover:scale-[1.02]"
                                    >
                                        "[ View Dossier ]"
                                    </button>
                                </div>
                            </div>
                            <div class="absolute inset-0 lg:relative lg:inset-auto lg:w-2/5">
                                <img
                                    src=art
                                    alt=""
                                    class="h-full w-full object-cover opacity-60 mix-blend-luminosity"
                                />
                                <div class="absolute inset-0 bg-gradient-to-r from-surface to-transparent"></div>
                            </div>
                        </section>
                    }
                })}

            // Unified search + filter toolbar (live signals — the Resource re-keys on change).
            <div class="mb-6 flex flex-wrap items-center gap-2 rounded-2xl border border-white/5 bg-black/40 p-2">
                <input
                    type="search"
                    placeholder="Search operations..."
                    // React's controlled input reflects value="" as an attribute at rest — the
                    // frozen V golden pins it (prop:value below stays the live binding).
                    value=""
                    prop:value=move || q.get()
                    on:input=move |ev| q.set(event_target_value(&ev))
                    class="min-w-[200px] flex-1 rounded-lg border border-white/10 bg-black/30 px-4 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
                />
                <select
                    prop:value=move || terrain.get()
                    on:change=move |ev| terrain.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Terrains"</option>
                    <option value="everon">{terrain_label("everon")}</option>
                    <option value="arland">{terrain_label("arland")}</option>
                </select>
                <select
                    prop:value=move || mode.get()
                    on:change=move |ev| mode.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Modes"</option>
                    <option value="pve_coop">{game_mode_label("pve_coop")}</option>
                    <option value="pvp">{game_mode_label("pvp")}</option>
                    <option value="zeus">{game_mode_label("zeus")}</option>
                </select>
                <select
                    prop:value=move || players.get()
                    on:change=move |ev| players.set(event_target_value(&ev))
                    class=SELECT_CLASS
                >
                    <option value="">"All Players"</option>
                    <option value="1-8">"1–8"</option>
                    <option value="9-16">"9–16"</option>
                    <option value="17-32">"17–32"</option>
                    <option value="33-64">"33–64"</option>
                </select>
            </div>

            {if missions.is_empty() {
                if show_empty_cta {
                    view! {
                        <div class="mx-auto my-12 flex max-w-md flex-col items-center gap-4 rounded-2xl border border-dashed border-white/15 bg-white/5 px-8 py-16 text-center">
                            <MaterialIcon name="map" class="text-4xl text-on-surface-variant" />
                            <div>
                                <p class="text-headline-sm font-bold text-on-surface">
                                    "No missions yet"
                                </p>
                                <p class="mt-1 text-body-md text-on-surface-variant">
                                    "Create a draft to open the Mission Creator."
                                </p>
                            </div>
                            <button
                                type="button"
                                on:click=move |_| open_create()
                                class="flex items-center gap-2 rounded-full bg-action px-6 py-3 text-label-md font-bold text-on-action transition hover:bg-action/90"
                            >
                                <MaterialIcon name="add" class="text-[18px]" />
                                "New Mission"
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <p class="py-12 text-center text-on-surface-variant">"No missions found."</p>
                    }
                        .into_any()
                }
            } else {
                view! {
                    <div class="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
                        {missions
                            .into_iter()
                            .map(|m| mission_card(m, me_id, open_preview, changed))
                            .collect_view()}
                    </div>
                }
                    .into_any()
            }}
        </>
    }
}

fn mission_card(
    m: MissionCard,
    me_id: StoredValue<Option<String>>,
    open_preview: impl Fn(String) + Copy + 'static,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let art = m
        .thumbnail_url
        .clone()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| PLACEHOLDER_ART.into());
    // T-389 — the returned-mission line. `rejection_reason` is the only channel by which an author
    // ever learns why a mission came back (`handlers/approvals.rs:217` is its sole writer), and
    // `GET /approvals` is admin-tier, so the author cannot go and look. Shown here rather than only
    // in the dossier because the card is what they see first, and a "Returned" badge with no reason
    // beside it just sends them hunting.
    //
    // Own missions only, and only when there is actually a reason: an admin rejecting without one
    // leaves the empty string, which the backend then omits from the wire entirely
    // (`skip_serializing_if`), so `None` and `Some("")` both mean "no reason given" and neither
    // should render an empty box.
    let rejection_note = (m.status == "rejected"
        && me_id.get_value().as_deref() == Some(m.author_id.as_str()))
    .then(|| {
        m.rejection_reason
            .clone()
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty())
    })
    .flatten();
    let initial = m
        .author_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    let has_avatar = !m.author_avatar.is_empty();
    let mid = m.id.clone();
    let mid_bm = m.id.clone();
    // T-264 — optimistic latch so the star flips before the list refetch lands (and so the
    // Bookmarked tab can populate off the POST without waiting for a full remount).
    let bookmarked = RwSignal::new(card_is_bookmarked(&m));
    let bookmark_busy = RwSignal::new(false);
    let status = m.status.clone();
    let toggle_bookmark = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if bookmark_busy.get_untracked() {
                return;
            }
            let next = !bookmarked.get_untracked();
            bookmarked.set(next);
            bookmark_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = bookmark_api_path(&mid_bm);
            leptos::task::spawn_local(async move {
                let result = if next {
                    crate::client::api_post_ok(store, &path, serde_json::json!({})).await
                } else {
                    crate::client::api_delete(store, &path).await
                };
                match result {
                    Ok(()) => {
                        toasts.success(if next {
                            "Mission bookmarked"
                        } else {
                            "Bookmark removed"
                        });
                        changed.run(());
                    }
                    Err(e) => {
                        bookmarked.set(!next);
                        toasts.error(crate::client::api_error_message(
                            &e,
                            if next {
                                "Could not bookmark mission"
                            } else {
                                "Could not remove bookmark"
                            },
                        ));
                    }
                }
                bookmark_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&store, &changed, &mid_bm, bookmarked, bookmark_busy);
        }
    };
    view! {
        // Outer div (not a single button) so the bookmark control is a real sibling button —
        // nesting <button> inside <button> is illegal HTML and breaks the click target.
        <div class="group relative overflow-hidden rounded-2xl border border-white/10 bg-surface-container/60 transition-all hover:-translate-y-0.5 hover:border-white/25 hover:shadow-xl">
            <button
                type="button"
                on:click=move |_| open_preview(mid.clone())
                class="w-full text-left"
            >
                <div class="relative h-48 w-full overflow-hidden bg-surface-container-low">
                    <img
                        src=art
                        alt=""
                        class="h-48 w-full object-cover transition-transform duration-500 group-hover:scale-105"
                    />
                    <span class="absolute top-3 left-3">
                        <span class=format!(
                            "{} border-white/10 bg-black/70",
                            badge_class("primary"),
                        )>{game_mode_label(&m.game_mode).to_string()}</span>
                    </span>
                </div>
                <div class="p-4">
                    <div class="mb-3 flex items-center gap-2">
                        {if has_avatar {
                            view! {
                                <img
                                    src=m.author_avatar.clone()
                                    alt=""
                                    class="h-6 w-6 rounded-full object-cover"
                                />
                            }
                                .into_any()
                        } else {
                            view! {
                                <span class="flex h-6 w-6 items-center justify-center rounded-full bg-surface-container-high text-label-sm text-on-surface-variant">
                                    {initial}
                                </span>
                            }
                                .into_any()
                        }}
                        <span class="text-label-md text-on-surface-variant">
                            {m.author_name.clone()}
                        </span>
                    </div>
                    <h3 class="text-headline-sm font-bold text-on-surface">{m.title.clone()}</h3>
                    {rejection_note
                        .map(|reason| {
                            view! {
                                <div class="mt-3 flex items-start gap-2 rounded-lg border border-error-alert/30 bg-error-alert/10 px-3 py-2 text-left">
                                    <MaterialIcon
                                        name="assignment_return"
                                        class="text-[16px] leading-5 text-error-alert"
                                    />
                                    <span class="text-label-md text-on-surface-variant line-clamp-2">
                                        <span class="font-semibold text-error-alert">
                                            "Returned: "
                                        </span>
                                        {reason}
                                    </span>
                                </div>
                            }
                        })}
                    <div class="mt-3 flex flex-wrap gap-2">
                        <span class="rounded-md border border-white/5 bg-black/30 px-2 py-0.5 font-mono text-label-sm text-on-surface-variant">
                            {terrain_label(&m.terrain)}
                        </span>
                        <span class="rounded-md border border-white/5 bg-black/30 px-2 py-0.5 font-mono text-label-sm text-on-surface-variant">
                            {m.max_players} " MAX"
                        </span>
                    </div>
                </div>
            </button>
            // T-264 — bookmark control. Sibling of the open-dossier button (not nested). Status
            // badge sits beside it so the star owns the top-right hit target without covering copy.
            <div class="pointer-events-none absolute top-3 right-3 z-10 flex items-center gap-2">
                <button
                    type="button"
                    data-testid="mission-bookmark-toggle"
                    aria-label=move || {
                        if bookmarked.get() {
                            "Remove bookmark"
                        } else {
                            "Bookmark mission"
                        }
                    }
                    prop:disabled=move || bookmark_busy.get()
                    on:click=toggle_bookmark
                    class="pointer-events-auto flex h-9 w-9 items-center justify-center rounded-full border border-white/10 bg-black/70 text-on-surface backdrop-blur-md transition-colors hover:bg-black/50 disabled:opacity-60"
                >
                    {move || {
                        let filled = bookmarked.get();
                        view! {
                            <MaterialIcon
                                name="bookmark"
                                class="text-[18px] text-tactical-yellow"
                                filled=filled
                            />
                        }
                    }}
                </button>
                <span class="pointer-events-none">{visibility_badge(&status)}</span>
            </div>
        </div>
    }
}

/* ───────────── Slide-over dossier + lifecycle actions (missions.tsx port) ───────────── */

#[component]
fn MissionDossierSheet(
    id: String,
    sheet_open: RwSignal<bool>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let id_sv = StoredValue::new(id);
    let mission = LocalResource::new(move || {
        let id = id_sv.get_value();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<MissionDetail>
            }
        }
    });
    let comments_open = RwSignal::new(false);
    let invite_open = RwSignal::new(false);
    let confirm_delete_open = RwSignal::new(false);
    // T-286 — same reactive + authed gate as the library header (not browse-mode None=>true).
    let is_maker = Memo::new(move |_| {
        has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)
    });
    let is_admin =
        Memo::new(move |_| has_min_role_authed(store.user.get().map(|u| u.role), Role::Admin));
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));
    // T-173 P5 — hold the heavy dossier DOM until the sheet's 300 ms slide finishes, so a fast
    // local fetch can't land a large subtree mount mid-animation (the fetch itself starts
    // immediately above; only the render is gated).
    let anim_done = RwSignal::new(false);
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::prelude::set_timeout;
        set_timeout(
            move || anim_done.set(true),
            std::time::Duration::from_millis(320),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    anim_done.set(true);

    view! {
        <Suspense fallback=dossier_loading>
            {move || {
                let fetched = mission.get();
                if !anim_done.get() {
                    // Keep the load bar up through the slide even when the fetch beat it.
                    return Some(dossier_loading().into_any());
                }
                fetched
                    .map(|opt| match opt {
                        Some(m) => {
                            let is_owner = me.get_value().as_deref()
                                == Some(m.author_id.as_str());
                            // `can_edit` gates the things that need the EDITOR: the Mission Creator
                            // CTA and the collaboration row. `/missions/:id/edit` really is
                            // `auth: "mission_maker"` (`router.rs:88`), so promising it to a
                            // non-maker would just bounce them off a role gate.
                            let can_edit = is_maker.get() && (is_owner || is_admin.get());
                            // T-389 — `can_manage` gates the LIFECYCLE (submit / archive / delete)
                            // and the rejection feedback, and it deliberately drops `is_maker` to
                            // match the API exactly: `submit_mission`, `update_mission` and
                            // `delete_mission` all take a plain `AuthUser` and test
                            // `can_edit(u, m)` = `author || admin` (`handlers/missions.rs:116`) —
                            // no maker tier anywhere. Only `create_mission` requires
                            // `MissionMakerUser`, which is why the "New Mission" button stays
                            // maker-gated.
                            //
                            // The old single `can_edit` was stricter than the backend, and the gap
                            // is reachable: Discord role sync can demote a mission_maker to
                            // enlisted while they still own missions. Such an author kept full API
                            // rights but was shown no Manage section at all — so once this slice
                            // put a rejection reason behind that same gate, the one person who
                            // needs to read it would have been the one person who could not. That
                            // is the feedback loop this ticket exists to close, so the gate moves
                            // with it rather than being left as a matching trap.
                            let can_manage = is_owner || is_admin.get();
                            dossier_sheet_body(
                                    m,
                                    id_sv,
                                    can_edit,
                                    can_manage,
                                    sheet_open,
                                    comments_open,
                                    invite_open,
                                    confirm_delete_open,
                                    changed,
                                )
                                .into_any()
                        }
                        None => {
                            view! { <p class="p-8 text-error">"Failed to load data."</p> }.into_any()
                        }
                    })
            }}
        </Suspense>
    }
}

/// Indeterminate load gate (T-172 A7): label + sweeping bar while the dossier fetches — also
/// shown through the sheet slide-in (T-173 P5 `anim_done` gate).
fn dossier_loading() -> impl IntoView {
    view! {
        <div class="flex h-full flex-col items-center justify-center gap-4 p-8">
            <p class="font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                "Loading dossier…"
            </p>
            <div class="h-1 w-56 overflow-hidden rounded-full bg-surface-variant/40">
                <div class="animate-mc-load-bar h-full w-1/4 rounded-full bg-primary"></div>
            </div>
        </div>
    }
}

#[allow(clippy::too_many_arguments)]
fn dossier_sheet_body(
    m: MissionDetail,
    id_sv: StoredValue<String>,
    can_edit: bool,
    can_manage: bool,
    sheet_open: RwSignal<bool>,
    comments_open: RwSignal<bool>,
    invite_open: RwSignal<bool>,
    confirm_delete_open: RwSignal<bool>,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    // These feed only the wasm-gated mutation closures.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&store, id_sv, &changed);
    let art = m
        .thumbnail_url
        .clone()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| PLACEHOLDER_ART.into());
    let is_archived = m.status == "archived";
    let status_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);
    let submit_busy = RwSignal::new(false);
    // T-264 — dossier star mirrors MissionDetail.bookmarked (named field, not extra).
    let bookmarked = RwSignal::new(m.bookmarked);
    let bookmark_busy = RwSignal::new(false);
    // T-389 — the only two statuses `POST /missions/:id/submit` accepts; everything else answers 409
    // ("only draft or rejected missions can be submitted", `handlers/missions.rs:626`). Gating the
    // button on the same predicate means the author never sees an action that is guaranteed to fail.
    let can_submit = can_manage && (m.status == "draft" || m.status == "rejected");
    // "Resubmit" on a returned mission: it tells the author the queue accepts a second attempt,
    // which is the whole point of the `rejected` → `pending_approval` transition existing.
    let submit_label = if m.status == "rejected" {
        "Resubmit for review"
    } else {
        "Submit for review"
    };
    // The reviewer's note, for the author's own dossier. Empty/absent → nothing to show (an admin
    // may reject without typing a reason, and the backend omits the empty string from the wire).
    let rejection_reason = (m.status == "rejected" && can_manage)
        .then(|| {
            m.rejection_reason
                .clone()
                .map(|r| r.trim().to_string())
                .filter(|r| !r.is_empty())
        })
        .flatten();
    let reviewed_at = m.reviewed_at.clone();
    let show_returned = m.status == "rejected" && can_manage;

    // T-264 — POST when off, DELETE when on; optimistic latch + list refetch via `changed`.
    let toggle_bookmark = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if bookmark_busy.get_untracked() {
                return;
            }
            let next = !bookmarked.get_untracked();
            bookmarked.set(next);
            bookmark_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = bookmark_api_path(&id_sv.get_value());
            leptos::task::spawn_local(async move {
                let result = if next {
                    crate::client::api_post_ok(store, &path, serde_json::json!({})).await
                } else {
                    crate::client::api_delete(store, &path).await
                };
                match result {
                    Ok(()) => {
                        toasts.success(if next {
                            "Mission bookmarked"
                        } else {
                            "Bookmark removed"
                        });
                        changed.run(());
                    }
                    Err(e) => {
                        bookmarked.set(!next);
                        toasts.error(crate::client::api_error_message(
                            &e,
                            if next {
                                "Could not bookmark mission"
                            } else {
                                "Could not remove bookmark"
                            },
                        ));
                    }
                }
                bookmark_busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (&store, &changed, id_sv, bookmarked, bookmark_busy);
        }
    };

    // toggleArchive — useSetMissionStatus port (PATCH /missions/:id {status}).
    let toggle_archive = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if status_busy.get_untracked() {
                return;
            }
            status_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/missions/{}", id_sv.get_value());
            let next = if is_archived { "draft" } else { "archived" };
            leptos::task::spawn_local(async move {
                match crate::client::api_patch::<serde_json::Value>(
                    store,
                    &path,
                    serde_json::json!({ "status": next }),
                )
                .await
                {
                    Ok(_) => {
                        toasts.success(if is_archived {
                            "Mission restored to draft"
                        } else {
                            "Mission archived"
                        });
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        if is_archived {
                            "Could not unarchive mission"
                        } else {
                            "Could not archive mission"
                        },
                    )),
                }
                status_busy.set(false);
            });
        }
    };

    // T-389 — submit for review: the SPA's FIRST caller of `POST /missions/:id/submit`.
    //
    // That endpoint is the sole writer of `pending_approval` in the whole crate — `apply_status_patch`
    // refuses the value outright, so `GET /approvals` can only ever show rows this route wrote. T-234
    // proved the SPA never called it, which means `/admin/approvals` was structurally empty in
    // production no matter how many missions existed: there was no door. This button is the door.
    //
    // Shape mirrors `toggle_archive` above (and `approvals.rs:302`'s approve/reject): busy latch,
    // `api_post_ok` with an empty body, toast either way, then `changed.run(())` so the card grid and
    // the dossier both re-read the new status instead of showing a stale "Draft".
    //
    // `api_error_message` rather than a fixed string because the two failures a real author will hit
    // say different things and only the backend knows which: 409 "only draft or rejected missions can
    // be submitted" (someone else already queued it, or it was archived under them) vs 403 "not your
    // mission". Swallowing those into "Could not submit" would leave them re-clicking a button that
    // cannot work.
    let submit_for_review = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if submit_busy.get_untracked() {
                return;
            }
            submit_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/missions/{}/submit", id_sv.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_post_ok(store, &path, serde_json::json!({})).await {
                    Ok(()) => {
                        toasts.success("Submitted for review");
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not submit mission for review",
                    )),
                }
                submit_busy.set(false);
            });
        }
    };

    // confirmDelete — useDeleteMission port (DELETE /missions/:id, Aegis confirm first).
    let confirm_delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            confirm_delete_open.set(false);
            if delete_busy.get_untracked() {
                return;
            }
            delete_busy.set(true);
            let toasts = crate::toast::use_toasts();
            let path = format!("/missions/{}", id_sv.get_value());
            leptos::task::spawn_local(async move {
                match crate::client::api_delete(store, &path).await {
                    Ok(()) => {
                        toasts.success("Mission deleted");
                        sheet_open.set(false);
                        changed.run(());
                    }
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not delete mission",
                    )),
                }
                delete_busy.set(false);
            });
        }
    };

    let toasts_share = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::toast::use_toasts().success("Will allow anyone to view and comment");
    };
    let toasts_planner = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::toast::use_toasts().success("2D Tactical Planner — coming soon");
    };
    let goto_editor = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(win) = web_sys::window() {
            let _ = win
                .location()
                .set_href(&format!("/missions/{}/edit", id_sv.get_value()));
        }
    };

    view! {
        // Edge-to-edge cinematic hero header.
        <div class="relative h-64 w-full shrink-0 md:h-80">
            <img src=art alt="" class="h-full w-full object-cover" />
            <div class="absolute inset-0 bg-gradient-to-t from-surface/90 to-transparent"></div>
            <button
                type="button"
                data-testid="mission-bookmark-toggle"
                aria-label=move || {
                    if bookmarked.get() {
                        "Remove bookmark"
                    } else {
                        "Bookmark mission"
                    }
                }
                prop:disabled=move || bookmark_busy.get()
                on:click=toggle_bookmark
                class="absolute top-5 right-16 flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-black/30 text-tactical-yellow backdrop-blur-md transition-colors hover:bg-black/50 disabled:opacity-60"
            >
                {move || {
                    let filled = bookmarked.get();
                    view! {
                        <MaterialIcon name="bookmark" class="text-[20px]" filled=filled />
                    }
                }}
            </button>
            <button
                type="button"
                aria-label="Close"
                on:click=move |_| sheet_open.set(false)
                class="absolute top-5 right-5 flex h-10 w-10 items-center justify-center rounded-full border border-white/10 bg-black/30 text-on-surface backdrop-blur-md transition-colors hover:bg-black/50"
            >
                <span class="material-symbols-outlined">"close"</span>
            </button>
            <div class="absolute right-8 bottom-6 left-8">
                <span class="mb-2 inline-block">{visibility_badge(&m.status)}</span>
                <h2 class="text-4xl font-black tracking-tighter text-white uppercase">
                    {m.title.clone()}
                </h2>
                <p class="mt-1 font-mono text-label-md text-on-surface-variant">
                    {format!("Authored by {}", m.author_name)}
                </p>
            </div>
        </div>

        // Scrollable content — pb-32 clears the sticky footer.
        <div class="custom-scrollbar flex-1 overflow-y-auto px-8 pt-6 pb-32">
            <div class="space-y-8">
                // T-389 — "Returned by review", above the dossier body because it is the reason the
                // author opened this sheet. `rejection_reason` is the ONLY thing they are ever told
                // (T-313 owns any richer review history); `GET /approvals` is admin-tier, so there
                // is no queue for them to go and read instead.
                {show_returned
                    .then(|| {
                        view! {
                            <section class="rounded-xl border border-error-alert/30 bg-error-alert/10 p-5">
                                <div class="flex items-center gap-2">
                                    <MaterialIcon
                                        name="assignment_return"
                                        class="text-[20px] text-error-alert"
                                    />
                                    <h3 class="font-mono text-label-md tracking-widest text-error-alert uppercase">
                                        "Returned by review"
                                    </h3>
                                </div>
                                {match rejection_reason {
                                    Some(reason) => {
                                        view! {
                                            <p class="mt-3 text-body-md whitespace-pre-line text-on-surface">
                                                {reason}
                                            </p>
                                        }
                                            .into_any()
                                    }
                                    // Rejected with an empty reason. Saying so is strictly better than
                                    // rendering a blank panel the author reads as a loading bug.
                                    None => {
                                        view! {
                                            <p class="mt-3 text-body-md text-on-surface-variant italic">
                                                "The reviewer did not leave a reason."
                                            </p>
                                        }
                                            .into_any()
                                    }
                                }}
                                {reviewed_at
                                    .map(|at| {
                                        view! {
                                            <p class="mt-3 font-mono text-label-sm text-on-surface-variant">
                                                "Reviewed " {crate::datefmt::format_local_datetime(&at)}
                                            </p>
                                        }
                                    })}
                                <p class="mt-4 text-label-md text-on-surface-variant">
                                    "Address the notes above, then use Submit for review to put it back in the queue."
                                </p>
                            </section>
                        }
                    })}

                {crate::mission_overview::dossier_body(&m)}

                <section>
                    <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                        "Collaboration"
                    </h3>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            on:click=move |_| comments_open.set(true)
                            class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                        >
                            "Comments"
                        </button>
                        {can_edit
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        on:click=toasts_share
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                                    >
                                        "Share for review"
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| invite_open.set(true)
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                                    >
                                        "Invite editor"
                                    </button>
                                }
                            })}
                    </div>
                </section>

                // Author/admin lifecycle actions (T-130.6): archive acts directly; delete confirms.
                // T-389: gated on `can_manage` (author||admin, the API's own predicate) rather than
                // `can_edit`, and submit-for-review joins the row — see the `can_manage` comment.
                {can_manage
                    .then(|| {
                        view! {
                            <section>
                                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                                    "Manage"
                                </h3>
                                <div class="flex flex-wrap gap-2">
                                    // T-389 — the submit door. Primary-styled because on a draft it
                                    // is the only action that moves the mission forward, and it sat
                                    // unbuilt while `/admin/approvals` rendered an empty queue.
                                    {can_submit
                                        .then(|| {
                                            view! {
                                                <button
                                                    type="button"
                                                    on:click=submit_for_review
                                                    prop:disabled=move || submit_busy.get()
                                                    class="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/15 px-4 py-2 text-label-md font-semibold text-primary transition-colors hover:bg-primary/25 disabled:opacity-60"
                                                >
                                                    <MaterialIcon name="send" class="text-[16px]" />
                                                    {submit_label}
                                                </button>
                                            }
                                        })}
                                    <button
                                        type="button"
                                        on:click=toggle_archive
                                        prop:disabled=move || status_busy.get()
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10 disabled:opacity-60"
                                    >
                                        {if is_archived {
                                            "Unarchive (restore to draft)"
                                        } else {
                                            "Archive mission"
                                        }}
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| confirm_delete_open.set(true)
                                        prop:disabled=move || delete_busy.get()
                                        class="rounded-lg border border-error-alert/30 bg-error-alert/10 px-4 py-2 text-label-md text-error-alert transition-colors hover:bg-error-alert/20 disabled:opacity-60"
                                    >
                                        "Delete mission"
                                    </button>
                                </div>
                            </section>
                        }
                    })}
            </div>
        </div>

        // Sticky action footer.
        <div class="absolute right-0 bottom-0 left-0 flex">
            {can_edit
                .then(|| {
                    view! {
                        <button
                            type="button"
                            on:click=goto_editor
                            class="flex-1 bg-action py-5 font-bold tracking-wide text-on-action transition-colors hover:bg-action/80"
                        >
                            "[ OPEN IN MISSION CREATOR ]"
                        </button>
                    }
                })}
            <button
                type="button"
                on:click=toasts_planner
                class="flex-1 border-t border-white/10 bg-surface-container-high py-5 font-bold tracking-wide text-primary transition-colors hover:bg-surface-container-highest"
            >
                "[ LAUNCH TACTICAL PLANNER ]"
            </button>
        </div>

        // Comments — empty-state shell (no API yet).
        <Sheet open=comments_open class="w-full max-w-none md:w-[28rem]">
            <h2 class="text-headline-sm text-on-surface">"Comments"</h2>
            <p class="mt-1 text-label-md text-on-surface-variant">
                "Suggestions on this mission — they don't change the mission until an editor applies them."
            </p>
            <div class="mt-8 flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-white/10 bg-white/5 px-6 py-16 text-center">
                <span class="material-symbols-outlined text-4xl text-on-surface-variant">
                    "forum"
                </span>
                <p class="text-body-md text-on-surface-variant">"Comments coming soon."</p>
            </div>
        </Sheet>

        // Invite editor — stubbed dialog.
        <crate::ui::Dialog
            open=invite_open
            title="Invite editor"
            description="Grant another mission maker edit access to this mission."
        >
            <label class="mb-2 block text-label-md text-on-surface-variant">
                "Email or Discord handle"
            </label>
            <input
                type="text"
                disabled
                placeholder="name@example.com or handle#0000"
                class="mb-4 w-full cursor-not-allowed rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface-variant opacity-60"
            />
            <p class="text-label-md text-on-surface-variant">"Coming soon."</p>
            <div class="mt-6 flex justify-end">
                <button
                    type="button"
                    on:click=move |_| invite_open.set(false)
                    class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                >
                    "Close"
                </button>
            </div>
        </crate::ui::Dialog>

        // Destructive confirm (F4-04) — Aegis Dialog, not window.confirm.
        <crate::ui::Dialog
            open=confirm_delete_open
            title="Delete this mission?"
            description="The mission and its versions are removed from the library for everyone. Deletion is refused while the mission is attached to an event."
        >
            <div class="flex justify-end gap-2">
                <button
                    type="button"
                    on:click=move |_| confirm_delete_open.set(false)
                    class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                >
                    "Cancel"
                </button>
                <button
                    type="button"
                    on:click=confirm_delete
                    prop:disabled=move || delete_busy.get()
                    class="rounded-lg bg-error-alert/20 px-4 py-2 text-label-md text-error-alert transition-colors hover:bg-error-alert/30 disabled:opacity-60"
                >
                    "Delete mission"
                </button>
            </div>
        </crate::ui::Dialog>
    }
}

#[cfg(test)]
mod tests {
    use super::{bookmark_api_path, card_is_bookmarked};
    use crate::dto::MissionCard;
    use serde_json::json;

    /// T-286 Class-R — New Mission must not use browse-mode `has_min_role(None)=>true`.
    /// Source guard goes red if the one-shot store role read returns or the authed helper /
    /// Memo wiring is dropped.
    #[test]
    fn maker_affordance_uses_authed_reactive_role() {
        const SRC: &str = include_str!("missions.rs");
        assert!(
            SRC.contains("has_min_role_authed"),
            "Mission Library maker gate must use has_min_role_authed (not browse-mode None=>true)"
        );
        assert!(
            SRC.contains("Memo::new(move |_|")
                && SRC.contains(
                    "has_min_role_authed(store.user.get().map(|u| u.role), Role::MissionMaker)"
                ),
            "is_maker must be a Memo that re-reads AuthStore.user after bootstrap"
        );
        // The old one-shot browse-mode path must stay gone. Split the needle so this assert's
        // own source text cannot false-red the include_str scan.
        let one_shot = format!("store.has_min_role({}::MissionMaker)", "Role");
        assert!(
            !SRC.contains(&one_shot),
            "one-shot store.has_min_role(MissionMaker) freezes pre-bootstrap None as maker"
        );
    }

    /// T-264 Class-R — the Bookmarked tab is dead unless this file both *renders* a control and
    /// *calls* POST/DELETE `/missions/{id}/bookmark`. Source guards go red under perturbation
    /// (delete the path format, the testids, or the api_post_ok/api_delete arms).
    #[test]
    fn bookmark_control_and_handlers_are_wired() {
        const SRC: &str = include_str!("missions.rs");
        assert!(
            SRC.contains("data-testid=\"mission-bookmark-toggle\""),
            "bookmark control must be present on the Mission Library surface \
             (perturbation: remove data-testid=\"mission-bookmark-toggle\")"
        );
        assert!(
            SRC.contains("Bookmark mission"),
            "bookmark control needs an accessible name when off"
        );
        assert!(
            SRC.contains("Remove bookmark"),
            "bookmark control needs an accessible name when on"
        );
        assert!(
            SRC.contains(r#"name="bookmark""#),
            "bookmark control must render the bookmark Material icon"
        );
        // POST when off — must not be a toast-only stub.
        assert!(
            SRC.contains("api_post_ok(store, &path, serde_json::json!({}))")
                && SRC.contains("bookmark_api_path"),
            "toggling on must POST via api_post_ok(bookmark_api_path(...))"
        );
        // DELETE when on.
        assert!(
            SRC.contains("api_delete(store, &path)"),
            "toggling off must DELETE via api_delete"
        );
        assert_eq!(
            bookmark_api_path("abc"),
            "/missions/abc/bookmark",
            "path helper must match Axum POST|DELETE /missions/{{id}}/bookmark"
        );
        // Live router registration — same shape as personnel.rs ADMIN_ROLES_SYNC_PATH pin.
        const APP_RS: &str =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../api/src/app.rs"));
        assert!(
            APP_RS.contains(r#""/missions/{id}/bookmark""#),
            "apps/website/api/src/app.rs must still register /missions/{{id}}/bookmark"
        );
    }

    #[test]
    fn card_is_bookmarked_reads_wire_extra_bool() {
        // MissionCard.bookmarked lives in `extra` until a dto promotion; the card control must
        // still see the same bool the Bookmarked scope filter uses.
        let on: MissionCard = serde_json::from_value(json!({
            "id": "1",
            "title": "t",
            "author_id": "a",
            "terrain": "everon",
            "game_mode": "pve_coop",
            "weather": "clear",
            "time_of_day": "dawn",
            "max_players": 16,
            "status": "live",
            "author_name": "x",
            "author_avatar": "",
            "bookmarked": true
        }))
        .unwrap();
        let off: MissionCard = serde_json::from_value(json!({
            "id": "1",
            "title": "t",
            "author_id": "a",
            "terrain": "everon",
            "game_mode": "pve_coop",
            "weather": "clear",
            "time_of_day": "dawn",
            "max_players": 16,
            "status": "live",
            "author_name": "x",
            "author_avatar": "",
            "bookmarked": false
        }))
        .unwrap();
        assert!(card_is_bookmarked(&on));
        assert!(!card_is_bookmarked(&off));
        assert!(
            on.extra.get("bookmarked").and_then(|v| v.as_bool()) == Some(true),
            "bookmarked must remain on the extra catch-all for MissionCard (dto owns naming)"
        );
    }
}
