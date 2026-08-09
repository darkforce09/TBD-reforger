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
use crate::url_guard;
use leptos::prelude::*;
// T-282 — the version differ indexes rows by id; these are its only two containers.
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const SELECT_CLASS: &str = "rounded-lg border border-white/10 bg-black/30 px-3 py-2 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
// SCOPES: (label, scope query value). Global is scopeIdx 0.
const SCOPES: [(&str, &str); 3] = [
    ("Global Missions", "global"),
    ("My Missions", "mine"),
    ("Bookmarked", "bookmarked"),
];
// Cinematic fallback art so cards/hero never render as empty grey blocks (missions.tsx).
const PLACEHOLDER_ART: &str = "https://lh3.googleusercontent.com/aida/AP1WRLtxuwSoyDyCrRuQu8gTHWuSmoOWZq8e7gw0bSjjZCmteU96TomvCGHto-cuqHYV_0gxNUjw_Lx2SWgiEl2W3vEi6aVH84DpTky5lG8-FKDJOzH96TrwAJwGJwE3DSwSN1gRC7miWds0X7kNvMAZRBgQPu_5g2iX9RtJ3WYUlgHbfVLYcmV7TaHPUvhZHvvvKenG2B3S2CRER15d2kdG5YNFbtFwtwgzEIeYG2jP4GubWd7SMO0bADPFFA";

/// Mission card / hero / dossier art `src`. **T-413** — http(s) thumbnails only; otherwise the
/// cinematic placeholder (itself a legitimate https URL).
fn mission_art_url(stored: Option<&str>) -> String {
    stored
        .filter(|u| url_guard::is_http_url(u))
        .unwrap_or(PLACEHOLDER_ART)
        .to_string()
}

/// Author avatar on a mission card. Empty / non-http → no `<img>` (initials path).
fn author_avatar_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}

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
///
/// **T-395 — the label half moved out.** T-389 fixed this `match` and left the identical mapping
/// inline in the mission dossier's STATUS cell, which went on rendering the raw `rejected`. Two
/// copies is how that happened, so there is now one:
/// [`crate::mission_overview::mission_status_label`]. The *variant* stays here — it is a badge
/// concern and the detail grid has no chips.
fn visibility_badge(status: &str) -> impl IntoView + use<> {
    let label = crate::mission_overview::mission_status_label(status);
    let variant = match status {
        "pending_approval" => "warning",
        "live" => "success",
        "rejected" => "error",
        // draft / archived / anything unknown
        _ => "neutral",
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

/// Featured-card briefing emptiness. Whitespace-only is not authored content (same rule as
/// `mission_overview::tactical_briefing_text` / `event_hub::briefing_text` — T-494 residual → T-548).
const FEATURED_BRIEFING_FALLBACK: &str = "Command has flagged this operation as the priority deployment. Review the dossier for objectives, ORBAT, and the armory loadout before committing forces to the field.";

fn featured_briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => FEATURED_BRIEFING_FALLBACK.into(),
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
            // Featured Operation — cinematic hero. F-19 — the "Live Operation" pulse is gated on the
            // mission actually being `live`: the hero spotlights the newest GLOBAL mission, which can
            // be a DRAFT, and claiming a draft is "the priority deployment" was a lie. A non-live hero
            // shows its real status label ("Draft", "Open for review", …) in a muted chip instead.
            {featured
                .map(|f| {
                    let art = mission_art_url(f.thumbnail_url.as_deref());
                    let brief = featured_briefing_text(f.briefing.as_deref());
                    let fid = f.id.clone();
                    let is_live = f.status == "live";
                    let status_label = crate::mission_overview::mission_status_label(&f.status);
                    view! {
                        <section class="relative mb-8 flex min-h-[320px] flex-col overflow-hidden rounded-2xl border border-white/10 bg-black/30 lg:flex-row">
                            <div class="relative z-10 flex w-full flex-col justify-center gap-4 p-8 lg:w-3/5">
                                {if is_live {
                                    view! {
                                        <div class="flex items-center gap-2 font-mono text-label-sm tracking-widest text-error-alert uppercase">
                                            <span class="relative flex h-2.5 w-2.5">
                                                <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-error-alert opacity-60"></span>
                                                <span class="relative inline-flex h-2.5 w-2.5 rounded-full bg-error-alert"></span>
                                            </span>
                                            "Live Operation"
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="flex items-center gap-2 font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                                            {status_label}
                                        </div>
                                    }
                                        .into_any()
                                }}
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
    let art = mission_art_url(m.thumbnail_url.as_deref());
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
    let author_avatar = author_avatar_img_src(&m.author_avatar).map(str::to_string);
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
                        {if let Some(src) = author_avatar.clone() {
                            view! {
                                <img
                                    src=src
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

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// T-282 — MISSION VERSION HISTORY: the differ.
//
// `mission_versions` has stored an immutable full-JSON snapshot per semver since the initial
// schema, so nothing here invents versioning; it surfaces what is already retained. The whole
// question is **what a diff of two mission payloads should even compare**, and the answer is not
// "the text".
//
// WHY STRUCTURAL AND NOT TEXTUAL. The stored payload is machine-serialised by
// `map_engine_core::mission::compile::compile_payload`, and two properties of that function make a
// text diff actively wrong rather than merely slow:
//
//   1. The collections are re-emitted from *unordered* yrs maps, ordered by whatever
//      `entityOrder.slots` happened to hold (`mission/compile.rs:191`). Re-saving an untouched
//      document can legitimately permute the arrays. A line/LCS diff would then report thousands of
//      changes for a mission nobody edited — the exact false positive that makes a diff untrusted.
//   2. `serde_json` here is built without `preserve_order`, so object keys are BTreeMap-sorted on
//      the way out and carry no authorial meaning at all.
//
// So the diff is keyed on the one thing that IS stable and IS authored: the row `id`. Every
// collection (`editor.slots`, `objectives`, `markers`, … and the `loadouts` id→row object) is
// indexed by id, and each row lands in exactly one bucket: added / removed / moved / edited /
// unchanged. That is also the vocabulary the author thinks in — "I added 12 slots and moved
// Alpha 1-1", not "line 41,882 changed".
//
// PERFORMANCE, which is the real constraint. Missions in this codebase's own history reach ~367k
// slots (`map_engine_core::mission::flatten` sizes its accumulator for exactly that, flatten.rs:492)
// and the version POST route lifts the body cap because payloads run to hundreds of MB
// (`api/src/app.rs:703`). Three decisions keep this out of the "hangs the tab" regime:
//
//   * **No pairwise comparison.** An LCS/text diff is O(n·m); this is one hash insert and one hash
//     lookup per row, i.e. O(rows_a + rows_b). Against 367k slots that is the difference between
//     linear and a quarter-trillion character comparisons.
//   * **Nothing is cloned.** The index is `HashMap<&str, &Value>` borrowed straight out of the
//     parsed payloads — ~24 bytes per row of transient index, versus a second copy of a
//     hundreds-of-MB document.
//   * **The RESULT is O(1) in document size.** Counts are exact and uncapped (they are free), but
//     the human sample lines stop at [`DIFF_SAMPLE_CAP`] per collection. A 367k-slot rewrite
//     produces the same size struct as a one-slot nudge, so nothing downstream — signal, DOM,
//     memory — scales with the mission.
//
// The one thing this deliberately does NOT do is recurse into rows to render a field-level diff of
// every changed slot. That output is unbounded by construction and no author reads 40,000 field
// deltas; `edited` plus a bounded sample is the useful truth.
//
// WHAT AN AUTHOR CAN ACTUALLY REACH TODAY — read this before assuming the compare UI was forgotten.
// `GET /missions/:id` embeds `current_version` *with its full `json_payload`*
// (`api/src/handlers/missions.rs:513`), so the dossier already holds one complete snapshot. It
// cannot hold a second, because **there is no list-versions endpoint**: `/missions/:id/versions` is
// POST-only and `/missions/:id/versions/:vid` needs an id the SPA has no way to discover
// (`api/src/app.rs:702-715`). So the timeline below renders the versions it can genuinely obtain —
// one — and says so, rather than fetching a route that answers 405. The differ itself is fully
// general A-vs-B and is exercised on real data today through [`version_census`], which is the same
// code path with the empty document as the left-hand side.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

/// How many changed rows one collection names before it stops naming them.
///
/// **Counts are exact and uncapped; only this list is bounded.** That is what makes a
/// [`MissionDiff`] O(1) in the size of the mission — see the module note above.
const DIFF_SAMPLE_CAP: usize = 6;

/// The id-keyed collections of a mission editor payload, in the order an author reads them, as
/// `(label, dotted path)`. Mirrors what `compile_payload` emits: the top-level arrays, the
/// `loadouts` object (id → row), and the four `editor.*` arrays.
const DIFF_COLLECTIONS: [(&str, &str); 9] = [
    ("Slots", "editor.slots"),
    ("Squads", "editor.squads"),
    ("Factions", "editor.factions"),
    ("Editor layers", "editor.editorLayers"),
    ("Objectives", "objectives"),
    ("Vehicles", "vehicles"),
    ("Entities", "entities"),
    ("Markers", "markers"),
    ("Loadouts", "loadouts"),
];

/// Payload scalars worth naming one by one, as `(label, dotted path)`. `environment` is handled
/// separately because its key set is open-ended.
const DIFF_SCALARS: [(&str, &str); 4] = [
    ("Title", "title"),
    ("Terrain", "map.terrain"),
    ("Map bounds", "map.bounds"),
    ("Schema version", "schemaVersion"),
];

/// Placeholder for a value absent on one side of the diff.
const DIFF_ABSENT: &str = "—";

/// Resolve a dotted path (`"editor.slots"`, `"map.terrain"`) against a payload. Absent → `None`,
/// which is how a whole collection missing on one side is represented.
fn payload_node<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = payload;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Visit every row of a collection as `(id, row)`. Handles BOTH shapes `compile_payload` emits: an
/// array of rows that carry their own `id`, and the `loadouts` object whose *key* is the id.
/// Anything else (absent, null, a scalar) has no rows.
///
/// Takes a closure rather than returning an iterator so the two shapes need no boxing and no
/// intermediate `Vec` — on a 367k-slot document that allocation is the whole cost.
fn for_each_row<'a>(node: Option<&'a Value>, mut f: impl FnMut(Option<&'a str>, &'a Value)) {
    match node {
        Some(Value::Array(rows)) => {
            for row in rows {
                f(row.get("id").and_then(Value::as_str), row);
            }
        }
        Some(Value::Object(map)) => {
            for (id, row) in map {
                f(Some(id.as_str()), row);
            }
        }
        _ => {}
    }
}

/// Shorten a display string, counting CHARACTERS (a mission title is user text and can be
/// non-ASCII; byte slicing would panic mid-codepoint).
fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// One-line rendering of a payload scalar for the "from → to" column.
fn scalar_repr(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return DIFF_ABSENT.to_string();
    };
    let rendered = match v {
        Value::Null => DIFF_ABSENT.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // A blank string is emptiness, not a value — say so with the same glyph as absent, so the
        // author is not shown `Title  "" → Bridgehead` and left to decode the quotes.
        Value::String(s) if s.trim().is_empty() => DIFF_ABSENT.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            let inner: Vec<String> = a.iter().take(6).map(|e| scalar_repr(Some(e))).collect();
            let more = if a.len() > 6 { ", …" } else { "" };
            format!("[{}{}]", inner.join(", "), more)
        }
        Value::Object(o) => format!("{{{} keys}}", o.len()),
    };
    ellipsize(&rendered, 56)
}

/// The name an author would recognise a row by. Falls back to the id, which is always something.
fn row_label(id: &str, row: &Value) -> String {
    for key in ["name", "title", "label", "callsign", "role"] {
        if let Some(s) = row.get(key).and_then(Value::as_str) {
            if !s.trim().is_empty() {
                return ellipsize(s.trim(), 40);
            }
        }
    }
    ellipsize(id, 40)
}

/// How one row changed between two versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowChange {
    Same,
    /// ONLY `position` differs. Worth its own bucket: dragging a squad across the map is the most
    /// common edit in this editor and it is not the same event as re-roling a slot.
    Moved,
    Edited,
}

/// Classify a row that exists on both sides.
///
/// "Moved" requires identical key sets and equality on every key except `position` — the key the
/// editor writes for slots, entities, vehicles and markers alike
/// (`map_engine_core::mission::flatten`). A row that moved *and* changed something else is
/// `Edited`, because that is the stronger and less dismissable claim.
/// Two JSON numbers are equal when they denote the same VALUE, not the same representation.
///
/// T-584 — `serde_json::Number`'s `PartialEq` is representational: `100` and `100.0` deserialize
/// to different variants (`PosInt` vs `Float`) and compare unequal. Both sides of this differ come
/// from one pipeline today, so it is not reachable yet; but a representation flip between two
/// stored payloads (a `yrs` BigInt↔Number coercion across an editor change, a serializer swap)
/// would make the differ report untouched rows as "edited". A differ that cries wolf is worth
/// nothing, which is the whole reason it exists.
///
/// Integers are compared exactly and never widened through `f64` — that would make two distinct
/// `u64`s above 2^53 compare equal, trading a false "edited" for a false "unchanged". A silent
/// "unchanged" is the strictly worse failure, so the widening only happens when a float is
/// genuinely involved.
fn number_eq(x: &serde_json::Number, y: &serde_json::Number) -> bool {
    if let (Some(p), Some(q)) = (x.as_i64(), y.as_i64()) {
        return p == q;
    }
    if let (Some(p), Some(q)) = (x.as_u64(), y.as_u64()) {
        return p == q;
    }
    match (x.as_f64(), y.as_f64()) {
        (Some(p), Some(q)) => p == q,
        _ => x == y,
    }
}

/// Deep equality that uses [`number_eq`] at every numeric leaf. Structural for arrays and objects;
/// falls through to `Value`'s own `PartialEq` for null/bool/string, where representation IS value.
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => number_eq(x, y),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| json_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        _ => a == b,
    }
}

fn classify_row(a: &Value, b: &Value) -> RowChange {
    if json_eq(a, b) {
        return RowChange::Same;
    }
    match (a.as_object(), b.as_object()) {
        (Some(ao), Some(bo)) => {
            let same_keys = ao.len() == bo.len() && ao.keys().all(|k| bo.contains_key(k));
            let only_position_differs = same_keys
                && ao
                    .iter()
                    .all(|(k, v)| k == "position" || bo.get(k).is_some_and(|w| json_eq(v, w)));
            if only_position_differs {
                RowChange::Moved
            } else {
                RowChange::Edited
            }
        }
        _ => RowChange::Edited,
    }
}

/// What happened to one collection between two versions.
///
/// `a_rows` / `b_rows` count everything present on each side, so the census and a "142 → 150"
/// headline stay exact even when some rows could not be keyed. The invariant the tests pin is
/// `b_rows == added + moved + edited + unchanged + unkeyed_b` (and its mirror for `a_rows`), which
/// is what makes a silently-dropped row impossible to hide.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CollectionDelta {
    label: &'static str,
    a_rows: usize,
    b_rows: usize,
    added: usize,
    removed: usize,
    moved: usize,
    edited: usize,
    unchanged: usize,
    /// Rows with no usable string `id` — they cannot be matched to anything, so they are counted
    /// and REPORTED rather than quietly dropped. A differ that silently ignores what it cannot
    /// read is the "reports success over an input it never examined" failure in miniature.
    unkeyed_a: usize,
    unkeyed_b: usize,
    /// The same id appearing twice within one side. `compile_payload` builds its arrays from an
    /// id-keyed map so this cannot arise from the editor, but an imported payload can carry it and
    /// the counts would be off by the duplicate — surfaced for the same reason as `unkeyed`.
    duplicate_ids: usize,
    samples: Vec<String>,
}

impl CollectionDelta {
    fn changed(&self) -> usize {
        self.added + self.removed + self.moved + self.edited
    }
    fn is_unchanged(&self) -> bool {
        self.changed() == 0
    }
    /// Rows this differ could not account for — the honesty caveat the UI must surface.
    fn unreadable(&self) -> usize {
        self.unkeyed_a + self.unkeyed_b + self.duplicate_ids
    }
}

/// One scalar that differs between two versions.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldChange {
    label: String,
    from: String,
    to: String,
}

/// The full answer to "what changed between these two versions".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct MissionDiff {
    fields: Vec<FieldChange>,
    /// Every collection, changed or not — the view filters. Keeping the unchanged ones is what
    /// lets [`version_census`] reuse this exact struct instead of a second row-counter that could
    /// disagree with it.
    collections: Vec<CollectionDelta>,
}

impl MissionDiff {
    fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.collections.iter().all(CollectionDelta::is_unchanged)
    }
    /// The changed collections only, in payload order.
    fn changed_collections(&self) -> impl Iterator<Item = &CollectionDelta> {
        self.collections.iter().filter(|c| !c.is_unchanged())
    }
}

/// Append a sample line while there is room. The cap is what keeps a diff's memory independent of
/// the mission's size.
fn push_sample(samples: &mut Vec<String>, line: String) {
    if samples.len() < DIFF_SAMPLE_CAP {
        samples.push(line);
    }
}

/// Diff one id-keyed collection. Three linear passes over borrowed data; see the module note.
fn diff_collection(label: &'static str, path: &str, a: &Value, b: &Value) -> CollectionDelta {
    let mut d = CollectionDelta {
        label,
        ..Default::default()
    };
    let (a_node, b_node) = (payload_node(a, path), payload_node(b, path));

    // Pass 1 — index the OLD side by id.
    let mut left: HashMap<&str, &Value> = HashMap::new();
    {
        let dr = &mut d;
        for_each_row(a_node, |id, row| {
            dr.a_rows += 1;
            match id.filter(|s| !s.is_empty()) {
                Some(id) => {
                    if left.insert(id, row).is_some() {
                        dr.duplicate_ids += 1;
                    }
                }
                None => dr.unkeyed_a += 1,
            }
        });
    }

    // Pass 2 — walk the NEW side, classifying against the index.
    let mut seen: HashSet<&str> = HashSet::with_capacity(left.len());
    {
        let dr = &mut d;
        let index = &left;
        for_each_row(b_node, |id, row| {
            dr.b_rows += 1;
            let Some(id) = id.filter(|s| !s.is_empty()) else {
                dr.unkeyed_b += 1;
                return;
            };
            if !seen.insert(id) {
                dr.duplicate_ids += 1;
            }
            match index.get(id) {
                None => {
                    dr.added += 1;
                    push_sample(&mut dr.samples, format!("+ {}", row_label(id, row)));
                }
                Some(prev) => match classify_row(prev, row) {
                    RowChange::Same => dr.unchanged += 1,
                    RowChange::Moved => {
                        dr.moved += 1;
                        push_sample(&mut dr.samples, format!("~ {} moved", row_label(id, row)));
                    }
                    RowChange::Edited => {
                        dr.edited += 1;
                        push_sample(&mut dr.samples, format!("~ {} edited", row_label(id, row)));
                    }
                },
            }
        });
    }

    // Pass 3 — removals, walked in OLD-document order rather than by draining the HashMap, so the
    // sample lines are deterministic. (HashMap iteration order is not, and a nondeterministic
    // sample makes a flaky test, which teaches people to ignore the differ.)
    {
        let dr = &mut d;
        let index = &mut left;
        for_each_row(a_node, |id, row| {
            let Some(id) = id.filter(|s| !s.is_empty()) else {
                return;
            };
            // `remove` makes this exactly-once per unique id even if the OLD side repeats one.
            if !seen.contains(id) && index.remove(id).is_some() {
                dr.removed += 1;
                push_sample(&mut dr.samples, format!("− {}", row_label(id, row)));
            }
        });
    }
    d
}

/// **The differ.** What changed between mission editor payload `a` (older) and `b` (newer).
fn diff_mission_payloads(a: &Value, b: &Value) -> MissionDiff {
    let mut fields = Vec::new();
    for (label, path) in DIFF_SCALARS {
        let (l, r) = (payload_node(a, path), payload_node(b, path));
        if l != r {
            fields.push(FieldChange {
                label: label.to_string(),
                from: scalar_repr(l),
                to: scalar_repr(r),
            });
        }
    }
    // `environment` is an open map (weather, timeOfDay, wind…). Walk the UNION of both sides' keys:
    // iterating only the new side would make a *deleted* environment key invisible, which is
    // precisely the class of change an author most needs to be told about.
    let (ea, eb) = (
        payload_node(a, "environment"),
        payload_node(b, "environment"),
    );
    let mut env_keys: Vec<&str> = Vec::new();
    for node in [ea, eb] {
        if let Some(Value::Object(o)) = node {
            for k in o.keys() {
                if !env_keys.contains(&k.as_str()) {
                    env_keys.push(k.as_str());
                }
            }
        }
    }
    env_keys.sort_unstable();
    for k in env_keys {
        let (l, r) = (ea.and_then(|o| o.get(k)), eb.and_then(|o| o.get(k)));
        if l != r {
            fields.push(FieldChange {
                label: format!("Environment · {k}"),
                from: scalar_repr(l),
                to: scalar_repr(r),
            });
        }
    }
    let collections = DIFF_COLLECTIONS
        .iter()
        .map(|(label, path)| diff_collection(label, path, a, b))
        .collect();
    MissionDiff {
        fields,
        collections,
    }
}

/// "What is in this version" — `(label, rows)` for every non-empty collection.
///
/// This is the SAME code path as the A-vs-B compare with the empty document on the left, not a
/// second row-counter that could drift from it. It is also why the differ is live code in the
/// shipped bundle rather than something only its own tests execute: the dossier holds exactly one
/// real payload (`current_version.json_payload`), and this is the true thing that can be said about
/// it without a second snapshot to compare against.
fn version_census(payload: &Value) -> Vec<(&'static str, usize)> {
    diff_mission_payloads(&Value::Null, payload)
        .collections
        .into_iter()
        .filter(|c| c.b_rows > 0)
        .map(|c| (c.label, c.b_rows))
        .collect()
}

/// Render a census as `"142 slots · 8 objectives · 12 markers"`.
fn census_line(census: &[(&'static str, usize)]) -> String {
    census
        .iter()
        .map(|(label, n)| format!("{n} {}", label.to_lowercase()))
        .collect::<Vec<_>>()
        .join(" · ")
}

// ═══════════════════════════════════════════════════════════════════════════════════════════════
// T-117 — MISSION DOCUMENT UPLOAD. Read this before touching anything below.
//
// The ticket says "(API exists)". It does — but NOT as a file upload, and the distinction decides
// the whole design. Verified against the running dev API on 2026-07-31, not inferred:
//
//   * There is **no multipart mission route anywhere**. `/cms/uploads` is the only multipart
//     endpoint in the crate. So "upload" here means POSTing a JSON mission document.
//   * `POST /missions/:id/versions` takes `{semver, editor_notes, payload}`, validates `payload`
//     against `mission-editor-payload.schema.json` + the wire-safety/cargo/zone scans, and on
//     failure answers **400 `{"error":"invalid mission payload","details":[…]}`** — measured:
//         /schemaVersion: "nope" is not of type "integer"
//         /markers: "not-an-array" is not of type "array"
//         /editor: this payload does not match the shape the mission compiler reads …
//     A valid document answers **201** and becomes `current_version_id`, so it shows up in the
//     library and in this dossier immediately. That is the whole feature.
//   * It is also the **only** mission-write route that can carry a real payload: it is the one
//     route with a lifted body cap (256 MB, `app.rs:703-706`). `POST /missions` runs under the
//     global **1 MiB** `MAX_JSON_BODY`, so "create a new mission from a document" cannot work at
//     mission scale through the API as it stands. Upload therefore targets an EXISTING mission,
//     which is also the surface an author is already looking at when they have a document.
//   * `GET /missions/:id/versions` is **405** — there is no list route (the T-282 note above says
//     the same thing; it is still true).
//
// THE ENVELOPE TRAP, which is why this is not a two-line file picker. Both exporters in this
// codebase — the editor's Export button (`map_engine_core::mission::compile::compile_export`) and
// `GET /missions/:id/export` (`handlers/missions.rs::build_mission_doc`) — emit a WRAPPER:
//
//     {"exportFormatVersion":1,"missionId":…,"title":…,"payload":{ …the editor payload… },…}
//
// The editor payload the API validates is the value of `payload`, not the wrapper. Posting an
// exported file verbatim is measured to answer:
//
//     400 {"error":"payload must include editor content (refusing empty payload as current version)"}
//
// — which is actively misleading, because the file is full of editor content one level down. So
// the natural round-trip an author will try first (Export → Upload) fails, and fails with a
// message that sends them looking in the wrong place. [`unwrap_export_envelope`] is what makes the
// round-trip work; the same file with `.payload` lifted out is measured to answer 201.
//
// SIZE, deliberately. This SPA is `wasm32` — a 32-bit address space, and browsers grant far less
// than its 4 GiB ceiling, on top of whatever the wgpu editor is already holding. A mission in this
// codebase's history reaches ~367k slots and hundreds of MB, and that document **cannot** come
// through a browser JSON parse no matter how the button is written. So [`UPLOAD_MAX_BYTES`]
// refuses over-budget files up front, by name and by size, BEFORE reading a byte — an honest
// refusal beats a dead tab.
//
// The cost is per-JSON-OBJECT, not per-byte (T-591: a parsed tree is several times its own source
// text, because every object carries map overhead regardless of how few keys it has). So what sets
// the ceiling is not the file size but **how many times the document is simultaneously resident**.
// T-593 cut that from four copies to one:
//
//   1. the signal's stored `Value` — unavoidable, the panel is holding the document
//   2. `up_doc.get_untracked()`'s clone         → REMOVED, `with_untracked` reads by reference
//   3. `version_body`'s clone under `"payload"` → REMOVED, `version_body_to_writer` borrows
//   4. `request`'s per-attempt `Body::Json`     → REMOVED, `api_post_raw` takes a `String`
//
// (The `File.text()` JS string and the Rust `String` both drop at parse and are not live at the
// fetch.) MEASURED with a counting allocator over a 170k-slot document, x86_64 / `preserve_order`
// — the numbers that justify the constant below:
//
//   32.07 MiB source, old path (api_post)          peak 1356.94 MiB   42.31x source
//   32.07 MiB source, this path (to_writer + raw)  peak  285.98 MiB    8.92x source
//   64.44 MiB source, this path (to_writer + raw)  peak  572.33 MiB    8.88x source
//
// The remaining fix is a streaming/multipart mission route (T-591 item 3), which is the only thing
// that removes the ceiling rather than raising it — the document still has to be parsed to be
// validated, and that parse is the 8.9x.
// ═══════════════════════════════════════════════════════════════════════════════════════════════

/// Largest mission document this browser upload will accept, in bytes.
///
/// Not a policy number — a memory one; see the module note above for the copies that make it so.
///
/// **Anchored on the peak that already ships, not on a hoped-for one.** T-117 shipped 32 MiB
/// through the old four-copy path, which measures a **1356.94 MiB** peak. 64 MiB through this
/// path measures **572.33 MiB** — so doubling the ceiling still leaves the worst case at well
/// under half of what the operator is already running. That is the whole argument: this is not a
/// new risk being taken, it is an old one being paid down and partly spent.
///
/// **256 MB — the server's cap — is NOT reachable and this must not pretend otherwise.** At the
/// measured 8.9x that is ~2.2 GiB of heap on x86_64, and while `wasm32`'s narrower pointers make
/// the trees smaller, it stays far past what a 32-bit tab holding the wgpu editor can serve.
/// Raising this number without cutting the amplification would only move an honest refusal into a
/// dead tab, which is strictly worse: the refusal names the limit and the workaround, and a tab
/// that dies during a parse tells the author nothing.
///
/// The server's own cap is 256 MB (`config.rs:186`) and stays the authority: a file under this
/// budget can still be refused by the server with a 413, and that message is surfaced verbatim
/// rather than pre-empted here (this client does not get to invent the server's limit — T-585-era
/// lesson, and `create_version` already words it precisely).
const UPLOAD_MAX_BYTES: usize = 64 << 20;

/// Refuse an over-budget file before it is read. `None` = accept.
///
/// Names BOTH numbers, because "too large" without them is unactionable: the author cannot tell
/// whether they need to trim one squad or that this door is closed to them entirely.
fn oversize_refusal(bytes: usize) -> Option<String> {
    (bytes > UPLOAD_MAX_BYTES).then(|| {
        format!(
            "That document is {} — this browser upload accepts up to {}. A mission that large has \
             to be saved from the Mission Creator, which builds the payload in memory instead of \
             parsing a file.",
            crate::mission_size::format_bytes(bytes),
            crate::mission_size::format_bytes(UPLOAD_MAX_BYTES)
        )
    })
}

/// Name a JSON value's kind for an error message an author can act on.
fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a true/false value",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Accept **both** shapes an author can plausibly have on disk and return the editor payload the
/// API validates.
///
/// * An **export envelope** — anything carrying `exportFormatVersion`, which both exporters emit
///   unconditionally (`compile_export` writes the literal key; `MissionJson` serialises
///   `export_format_version`). Its `payload` is lifted out.
/// * A **bare editor payload** — passed through untouched.
///
/// Keyed on `exportFormatVersion` rather than "has a `payload` key" on purpose: the editor payload
/// schema has no top-level `payload` property, so both tests happen to work today, but only the
/// version marker is a thing the producers promise. Guessing from shape is how a future top-level
/// key would silently start eating documents.
fn unwrap_export_envelope(doc: Value) -> Result<Value, String> {
    let Value::Object(mut obj) = doc else {
        return Err(format!(
            "A mission document must be a JSON object; this file's top level is {}.",
            json_kind(&doc)
        ));
    };
    if !obj.contains_key("exportFormatVersion") {
        return Ok(Value::Object(obj));
    }
    match obj.remove("payload") {
        Some(payload @ Value::Object(_)) => Ok(payload),
        Some(other) => Err(format!(
            "This looks like an exported mission file, but its \"payload\" is {} rather than an \
             object, so there is no editor document inside it to upload.",
            json_kind(&other)
        )),
        None => Err(
            "This looks like an exported mission file, but it has no \"payload\" — there is no \
             editor document inside it to upload."
                .to_string(),
        ),
    }
}

/// Parse a picked file into the editor payload to POST, or the reason it cannot be one.
///
/// The syntax error is kept verbatim: `serde_json`'s `Display` already ends in
/// `at line L column C`, which is the single most useful thing anyone can be told about a broken
/// 40 MB document, and the server's own message for the same file is a flat
/// `payload is not valid JSON` with no position at all.
fn parse_uploaded_document(text: &str) -> Result<Value, String> {
    if text.trim().is_empty() {
        return Err("That file is empty.".to_string());
    }
    let doc: Value =
        serde_json::from_str(text).map_err(|e| format!("That file is not valid JSON — {e}."))?;
    unwrap_export_envelope(doc)
}

/// Suggested next version number: bump the patch of the mission's current version.
///
/// `create_version` enforces real SemVer 2.0 (T-363) and 409s on a duplicate, so the suggestion has
/// to be both valid and unused — a patch bump of the current tip is the only value guaranteed to be
/// neither of the mission's known-taken ones. Pre-release / build metadata on the current version is
/// dropped rather than carried: `1.2.3-rc1` bumps to `1.2.4`, because incrementing inside a
/// pre-release tag is a guess about the author's release scheme.
fn next_semver(current: Option<&str>) -> String {
    const FALLBACK: &str = "0.1.0";
    let Some(cur) = current else {
        return FALLBACK.to_string();
    };
    let core = cur.split(['-', '+']).next().unwrap_or("");
    let mut parts = core.split('.');
    let (Some(maj), Some(min), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return FALLBACK.to_string();
    };
    let (Ok(maj), Ok(min), Ok(patch)) =
        (maj.parse::<u64>(), min.parse::<u64>(), patch.parse::<u64>())
    else {
        return FALLBACK.to_string();
    };
    format!("{maj}.{min}.{}", patch + 1)
}

/// Turn a failed upload into `(headline, findings)` — **the function that decides whether this
/// feature is worth having.**
///
/// `create_version` answers 400 with the exact list of everything wrong with the document, and
/// `client::error_body_message` has folded that `details` array into the error string as extra
/// lines since T-181.44. Collapsing it back to one line is the whole failure mode this ticket
/// exists to avoid: "invalid mission payload" names a verdict, not a cause, and an author who is
/// told only the verdict cannot fix the file. So the generic arm ALWAYS returns the split rows, and
/// the specific arms are only for the statuses that genuinely carry no findings.
///
/// The status-specific arms exist because those four failures are things the author does something
/// different about — pick another version number, use the editor, sign in, check the connection —
/// and none of them is a defect in the document they just picked.
fn upload_failure(status: u16, msg: Option<&str>, semver: &str) -> (String, Vec<String>) {
    let (head, rows) = crate::client::split_error_lines(msg);
    let head = head.filter(|h| !h.trim().is_empty());
    match status {
        409 => (
            format!(
                "Version {semver} already exists on this mission. Versions are immutable — choose \
                 a different number."
            ),
            Vec::new(),
        ),
        // The backend names its own limit in MB; echo it rather than restating a number this file
        // would have to keep in sync with `MISSION_VERSION_MAX_BODY_BYTES`.
        413 => (
            head.unwrap_or_else(|| "The server refused the document as too large.".to_string()),
            Vec::new(),
        ),
        401 => (
            "Your session expired — sign in again and re-pick the document.".to_string(),
            Vec::new(),
        ),
        0 => (
            "The upload could not reach the server. Nothing was saved; try again.".to_string(),
            Vec::new(),
        ),
        _ => match (&head, rows.len()) {
            (Some(h), 0) => (format!("Rejected ({status}): {h}"), rows),
            (Some(h), n) => (
                format!("Rejected ({status}): {h} — {n} problem(s) listed below"),
                rows,
            ),
            (None, _) => (format!("Upload failed ({status})."), rows),
        },
    }
}

/// What the picked document would do to this mission, in the author's vocabulary — the T-282
/// differ, finally with two payloads to compare.
///
/// Until now the differ had exactly one snapshot to work with and could only be run against the
/// empty document (see [`version_census`]); an uploaded file is the second side it was written for.
/// Bounded by construction: the per-collection counts are exact, the named samples stop at
/// [`DIFF_SAMPLE_CAP`], so this stays O(1) in the size of the mission (module note above).
fn diff_summary_lines(diff: &MissionDiff) -> Vec<String> {
    let mut out: Vec<String> = diff
        .fields
        .iter()
        .map(|f| format!("{}: {} → {}", f.label, f.from, f.to))
        .collect();
    for c in diff.changed_collections() {
        let mut parts: Vec<String> = Vec::new();
        for (n, word) in [
            (c.added, "added"),
            (c.removed, "removed"),
            (c.moved, "moved"),
            (c.edited, "edited"),
        ] {
            if n > 0 {
                parts.push(format!("{n} {word}"));
            }
        }
        out.push(format!(
            "{}: {} → {} ({})",
            c.label,
            c.a_rows,
            c.b_rows,
            parts.join(", ")
        ));
    }
    // Rows the differ could not key are counted in the totals but classified nowhere. Saying so is
    // the same honesty `CollectionDelta::unreadable` was added for: a summary that silently drops
    // what it could not read is a check reporting success over an input it never examined.
    let unreadable: usize = diff
        .collections
        .iter()
        .map(CollectionDelta::unreadable)
        .sum();
    if unreadable > 0 {
        out.push(format!(
            "{unreadable} row(s) have no usable id — they are counted in the totals above but \
             could not be matched to anything."
        ));
    }
    out
}

/// The dossier's version-history rail (T-282).
///
/// Renders the versions this SPA can genuinely obtain. Today that is exactly one — the mission
/// detail's embedded `current_version` — because no list-versions route exists; see the module
/// note. The closing line says so in the author's own terms instead of leaving them to conclude
/// their history was lost.
fn version_history_section(m: &MissionDetail) -> Option<impl IntoView + use<>> {
    let v = m.current_version.as_ref()?;
    let semver = v.semver.clone();
    let saved = crate::datefmt::format_local_datetime(&v.created_at);
    // `created_by` is a raw Discord snowflake, not a display name, and the dossier has no directory
    // to resolve one against. Name the author when the ids match; otherwise say nothing rather than
    // print an 18-digit number at them.
    let by = (v.created_by == m.author_id).then(|| format!(" by {}", m.author_name));
    let census = version_census(&v.json_payload);
    let contents = if census.is_empty() {
        // Reachable and true: a mission saved before the editor wrote any content, or the golden
        // seed whose `json_payload` is literally `{}`. Saying so beats a blank line read as a bug.
        "This version stores no editor content.".to_string()
    } else {
        census_line(&census)
    };
    Some(view! {
        <section>
            <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                "Version history"
            </h3>
            <ol class="border-l border-white/10 pl-5">
                <li class="relative">
                    <span class="absolute top-1.5 -left-[23px] size-2.5 rounded-full bg-primary ring-4 ring-surface-container-high"></span>
                    <div class="flex flex-wrap items-center gap-2">
                        <span class="font-mono text-label-lg font-semibold text-on-surface">
                            {format!("v{semver}")}
                        </span>
                        <span class="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 font-mono text-label-sm tracking-widest text-primary uppercase">
                            "Current"
                        </span>
                    </div>
                    <p class="mt-1 font-mono text-label-sm text-on-surface-variant">
                        {format!("Saved {saved}")}
                        {by}
                    </p>
                    <p class="mt-1 text-label-md text-on-surface-variant">{contents}</p>
                </li>
            </ol>
            <p class="mt-3 flex items-start gap-2 text-label-md text-on-surface-variant">
                <MaterialIcon name="history" class="mt-0.5 shrink-0 text-[16px]" />
                <span>
                    "Earlier versions of this mission are kept, but the library cannot list them yet — it can load only the current one, so there is no earlier snapshot to compare against."
                </span>
            </p>
        </section>
    })
}

#[allow(clippy::too_many_arguments)]
fn dossier_sheet_body(
    mut m: MissionDetail,
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
    let art = mission_art_url(m.thumbnail_url.as_deref());
    let is_archived = m.status == "archived";
    let status_busy = RwSignal::new(false);
    let delete_busy = RwSignal::new(false);
    let submit_busy = RwSignal::new(false);

    // ── T-117 upload state ───────────────────────────────────────────────────────────────────
    // Everything that needs the whole `&m` runs FIRST, so the current payload can then be MOVED
    // out of `m` rather than cloned. That is not fussiness: `current_version.json_payload` is the
    // one value in this SPA that reaches hundreds of MB, the T-282 differ went out of its way to
    // borrow rather than copy it, and a `.clone()` here would double the dossier's peak for a
    // panel that is idle until someone picks a file.
    let overview_body = crate::mission_overview::dossier_body(&m);
    let version_rail = version_history_section(&m);
    // Suggested next version, derived from the tip so it is both valid SemVer (T-363) and not one
    // of the mission's known-taken numbers. Read before the payload is moved out below.
    let up_semver = RwSignal::new(next_semver(
        m.current_version.as_ref().map(|v| v.semver.as_str()),
    ));
    let current_payload = StoredValue::new(m.current_version.take().map(|v| v.json_payload));
    // The picked file's name — also the `editor_notes` provenance line on the stored version.
    let up_name = RwSignal::new(Option::<String>::None);
    let up_size = RwSignal::new(0usize);
    // The parsed, envelope-unwrapped editor payload, held so a 409 can be retried with a different
    // semver without making the author re-pick and re-parse the file.
    let up_doc = RwSignal::new(Option::<Value>::None);
    let up_busy = RwSignal::new(false);
    let up_status = RwSignal::new(String::new());
    // The backend's `details` rows, rendered as a persistent list. Deliberately NOT a toast: a
    // schema finding is a work item an author reads while editing the document, and a toast that
    // vanishes in four seconds is exactly the "swallowed the reason" failure this ticket is about.
    let up_findings = RwSignal::new(Vec::<String>::new());
    let up_preview = RwSignal::new(Vec::<String>::new());
    // The upload pipeline is browser-only (file picker → Blob::text → POST); on the native test
    // build these three are read by nothing. The pure functions behind them are what the suite
    // exercises — see the tests at the foot of this file.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (current_payload, up_name, up_size);
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

    // T-117 — pick a mission document. Same programmatic-picker idiom as the CMS hero upload
    // (`content.rs:632`): an off-DOM `<input type=file>` + a one-shot `Closure`, so there is no
    // dead control in the DOM when the author never uses it.
    //
    // Reading and parsing happen HERE rather than at Upload time, for two reasons: the author gets
    // told the file is unusable before they have chosen a version number, and the preview diff
    // below can tell them what the document would actually do while there is still time not to.
    let pick_document = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::closure::Closure;
            use wasm_bindgen::JsCast;

            if up_busy.get_untracked() {
                return;
            }
            let toasts = crate::toast::use_toasts();
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                toasts.error("Could not open the file picker");
                return;
            };
            let Ok(input) = document
                .create_element("input")
                .map_err(|_| ())
                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().map_err(|_| ()))
            else {
                toasts.error("Could not open the file picker");
                return;
            };
            input.set_type("file");
            input.set_accept("application/json,.json");

            let input_for_cb = input.clone();
            let on_change = Closure::once(move |_ev: web_sys::Event| {
                let Some(file) = input_for_cb.files().and_then(|list| list.item(0)) else {
                    return;
                };
                let name = file.name();
                // `File::size()` is f64 (JS Number). Clamp rather than cast blind: a negative or
                // NaN size must not wrap into a huge usize and sail past the budget check.
                let size = file.size().max(0.0).min(usize::MAX as f64) as usize;
                up_findings.set(Vec::new());
                up_preview.set(Vec::new());
                up_doc.set(None);
                up_name.set(Some(name.clone()));
                up_size.set(size);
                // Refuse BEFORE reading — see the module note. A tab that dies mid-read cannot
                // tell anybody why.
                if let Some(refusal) = oversize_refusal(size) {
                    up_status.set(refusal);
                    return;
                }
                up_status.set(format!("Reading {name}…"));
                leptos::task::spawn_local(async move {
                    // `Blob::text()` is a Promise: the browser reads off disk on its own thread and
                    // this task is suspended, so the tab stays interactive through the read. The
                    // parse that follows is synchronous — which is exactly why the budget above is
                    // a hard gate and not a warning.
                    let text = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                        Ok(v) => v.as_string().unwrap_or_default(),
                        Err(_) => {
                            up_status.set(format!("Could not read {name}."));
                            return;
                        }
                    };
                    match parse_uploaded_document(&text) {
                        Ok(doc) => {
                            let census = census_line(&version_census(&doc));
                            let census = if census.is_empty() {
                                "no editor content".to_string()
                            } else {
                                census
                            };
                            // The T-282 differ, with a real second payload for the first time.
                            let preview = current_payload.with_value(|cur| match cur {
                                Some(a) => {
                                    let d = diff_mission_payloads(a, &doc);
                                    if d.is_empty() {
                                        vec!["Identical to the current version — uploading it \
                                             would only add a version number."
                                            .to_string()]
                                    } else {
                                        diff_summary_lines(&d)
                                    }
                                }
                                None => Vec::new(),
                            });
                            up_preview.set(preview);
                            up_doc.set(Some(doc));
                            up_status.set(format!("{name} — {census}. Ready to upload."));
                        }
                        Err(why) => up_status.set(why),
                    }
                });
            });
            let _ = input
                .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
            // One-shot listener outlives this frame — the picker is fire-and-forget (content.rs).
            on_change.forget();
            input.click();
        }
    };

    // T-117 — POST the parsed document as a new version. Same wire shape as the editor's own Save
    // (`mission_commands::save_now`), through the SAME `version_body` builder in map-engine-core, so
    // the two doors onto `create_version` cannot drift into sending different JSON.
    let upload_document = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if up_busy.get_untracked() {
                return;
            }
            if up_doc.with_untracked(Option::is_none) {
                up_status.set("Choose a mission document first.".to_string());
                return;
            }
            let semver = up_semver.get_untracked().trim().to_string();
            if semver.is_empty() {
                up_status.set("A version number is required (e.g. 1.2.3).".to_string());
                return;
            }
            let notes = up_name
                .get_untracked()
                .map(|n| format!("Uploaded from {n}"))
                .unwrap_or_else(|| "Uploaded document".to_string());
            up_busy.set(true);
            up_findings.set(Vec::new());
            up_status.set(format!("Uploading v{semver}…"));
            // Serialise the request bytes straight out of the stored document — see
            // [`UPLOAD_MAX_BYTES`] for why the copies this avoids set the ceiling.
            //
            // `with_untracked` reads the signal BY REFERENCE: `get_untracked()` would clone the
            // whole parsed tree just to hand it to the builder. `version_body_to_writer` then
            // walks that borrowed tree once, so the wrapper costs no second tree either, and
            // `api_post_raw` takes the finished `String` so `request`'s per-attempt body clone
            // never happens. One live tree at the fetch instead of four.
            //
            // The `Vec` is pre-sized from the picked file's size because the growth doubling is
            // itself a transient copy of everything written so far — at these sizes that realloc
            // is the peak. Compact re-serialisation is ~never larger than the source JSON, and
            // the `Vec` still grows correctly if it is.
            // Read outside the borrow below — nothing else should touch a signal while
            // `up_doc`'s storage is held open.
            let cap = up_size.get_untracked().saturating_add(1024);
            let body = up_doc.with_untracked(|slot| {
                let doc = slot.as_ref()?;
                let mut buf: Vec<u8> = Vec::with_capacity(cap);
                map_engine_core::mission::compile::version_body_to_writer(
                    &mut buf, &semver, &notes, doc,
                )
                .ok()?;
                String::from_utf8(buf).ok()
            });
            // Unreachable in practice (a `Value` always serialises, and serde_json always emits
            // UTF-8) — but sending an empty body would earn a 400 the author would read as "my
            // document is broken", so refuse in our own words instead.
            let Some(body) = body else {
                up_status.set(
                    "That document could not be prepared for upload — nothing was sent."
                        .to_string(),
                );
                up_busy.set(false);
                return;
            };
            let path = format!("/missions/{}/versions", id_sv.get_value());
            let toasts = crate::toast::use_toasts();
            leptos::task::spawn_local(async move {
                // `api_post_raw`, not `api_post`: the 201 echoes the entire `json_payload` back
                // (`models/mission.rs:128`), so a `T`-generic post would parse a whole extra tree
                // out of the response — and the `Ok` arm below throws it away. Non-2xx bodies are
                // still read and still fold `details` into the message (`client.rs:220`), which is
                // what `upload_failure` needs.
                match crate::client::api_post_raw(store, &path, body).await {
                    Ok(()) => {
                        up_status.set(format!(
                            "Uploaded v{semver} — it is now this mission's current version."
                        ));
                        up_doc.set(None);
                        up_name.set(None);
                        up_size.set(0);
                        up_preview.set(Vec::new());
                        up_semver.set(next_semver(Some(&semver)));
                        toasts.success("Mission document uploaded");
                        // Re-read the dossier + the card grid so the rail shows the new tip rather
                        // than the version this panel just replaced.
                        changed.run(());
                    }
                    Err((status, msg)) => {
                        let (head, rows) = upload_failure(status, msg.as_deref(), &semver);
                        up_status.set(head);
                        up_findings.set(rows);
                    }
                }
                up_busy.set(false);
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

                {overview_body}

                // T-282 — the version rail. Above Collaboration because "what is in the saved
                // version" is dossier fact, not a collaboration action.
                {version_rail}

                // T-117 — upload a mission document as the next version. Directly under the rail
                // because that is what it writes; gated on `can_edit`, the same predicate
                // `create_version` itself enforces (plus the MissionMakerUser tier), so nobody is
                // shown a control guaranteed to answer 403.
                {can_edit
                    .then(|| {
                        view! {
                            <section data-testid="mission-upload-section">
                                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                                    "Upload mission document"
                                </h3>
                                <p class="mb-3 text-label-md text-on-surface-variant">
                                    "Accepts an exported mission file or a bare editor payload. The document is validated before it is stored — if it is rejected you get the list of what is wrong with it, not just a refusal."
                                </p>
                                <div class="flex flex-wrap items-center gap-2">
                                    <button
                                        type="button"
                                        data-testid="mission-upload-pick"
                                        on:click=pick_document
                                        prop:disabled=move || up_busy.get()
                                        class="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10 disabled:opacity-60"
                                    >
                                        <MaterialIcon name="upload_file" class="text-[16px]" />
                                        "Choose document…"
                                    </button>
                                    <label class="text-label-md text-on-surface-variant" for="mission-upload-semver">
                                        "Version"
                                    </label>
                                    <input
                                        id="mission-upload-semver"
                                        type="text"
                                        data-testid="mission-upload-semver"
                                        placeholder="1.2.3"
                                        prop:value=move || up_semver.get()
                                        on:input=move |ev| up_semver.set(event_target_value(&ev))
                                        class="w-28 rounded-lg border border-white/10 bg-black/30 px-3 py-2 font-mono text-label-md text-on-surface outline-none transition-colors focus:border-primary/60"
                                    />
                                    <button
                                        type="button"
                                        data-testid="mission-upload-submit"
                                        on:click=upload_document
                                        prop:disabled=move || up_busy.get() || up_doc.with(Option::is_none)
                                        class="flex items-center gap-2 rounded-lg border border-primary/30 bg-primary/15 px-4 py-2 text-label-md font-semibold text-primary transition-colors hover:bg-primary/25 disabled:opacity-40"
                                    >
                                        <MaterialIcon name="cloud_upload" class="text-[16px]" />
                                        "Upload as new version"
                                    </button>
                                </div>

                                // Status line — the headline of whatever just happened (read, parse
                                // refusal, oversize refusal, upload verdict).
                                {move || {
                                    let s = up_status.get();
                                    (!s.is_empty())
                                        .then(|| {
                                            view! {
                                                <p
                                                    data-testid="mission-upload-status"
                                                    class="mt-3 text-label-md text-on-surface"
                                                >
                                                    {s}
                                                </p>
                                            }
                                        })
                                }}

                                // The backend's `details` — the reason the document was refused,
                                // one work item per row. Losing these is the defect this ticket
                                // exists to prevent, so they render as their own labelled block.
                                {move || {
                                    let rows = up_findings.get();
                                    (!rows.is_empty())
                                        .then(|| {
                                            view! {
                                                <div
                                                    data-testid="mission-upload-findings"
                                                    class="mt-3 rounded-xl border border-error-alert/30 bg-error-alert/10 p-4"
                                                >
                                                    <p class="font-mono text-label-sm tracking-widest text-error-alert uppercase">
                                                        "What is wrong with this document"
                                                    </p>
                                                    <ul class="mt-2 space-y-1">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| {
                                                                view! {
                                                                    <li class="font-mono text-label-sm break-words text-on-surface">
                                                                        {r}
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                </div>
                                            }
                                        })
                                }}

                                // What uploading this file would change, before it is uploaded.
                                {move || {
                                    let rows = up_preview.get();
                                    (!rows.is_empty())
                                        .then(|| {
                                            view! {
                                                <div
                                                    data-testid="mission-upload-preview"
                                                    class="mt-3 rounded-xl border border-white/10 bg-white/5 p-4"
                                                >
                                                    <p class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                                                        "Against the current version"
                                                    </p>
                                                    <ul class="mt-2 space-y-1">
                                                        {rows
                                                            .into_iter()
                                                            .map(|r| {
                                                                view! {
                                                                    <li class="text-label-md text-on-surface-variant">
                                                                        {r}
                                                                    </li>
                                                                }
                                                            })
                                                            .collect_view()}
                                                    </ul>
                                                </div>
                                            }
                                        })
                                }}
                            </section>
                        }
                    })}

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
    use super::{
        author_avatar_img_src, bookmark_api_path, card_is_bookmarked, featured_briefing_text,
        mission_art_url, FEATURED_BRIEFING_FALLBACK, PLACEHOLDER_ART,
    };
    use crate::dto::MissionCard;
    use serde_json::json;
    // T-117 upload pipeline. `diff_mission_payloads` / `Value` come in via the T-282 differ's own
    // `use super::{…}` further down this module — deliberately not re-imported here.
    use super::{
        diff_summary_lines, next_semver, oversize_refusal, parse_uploaded_document,
        unwrap_export_envelope, upload_failure, UPLOAD_MAX_BYTES,
    };

    /// T-548 — whitespace-only featured briefings take the cinematic fallback (trim-aware,
    /// same rule as mission_overview / event_hub after T-494).
    #[test]
    fn featured_briefing_trims_whitespace_only_to_fallback() {
        for cleared in [None, Some(""), Some("   \n\n  "), Some("\t")] {
            assert_eq!(
                featured_briefing_text(cleared),
                FEATURED_BRIEFING_FALLBACK,
                "whitespace-only briefing must take the featured fallback ({cleared:?})"
            );
        }
        let authored = "Hold the ridge.\n\nSecond wave at H+20.";
        assert_eq!(featured_briefing_text(Some(authored)), authored);
        // Leading/trailing space alone is not emptiness — only all-whitespace is.
        assert_eq!(featured_briefing_text(Some(" Hold. ")), " Hold. ");
    }

    /// T-548 Class-R source ratchet — ban the pre-T-548 `!b.is_empty()` filter (no trim) that
    /// left whitespace-only briefings on the featured hero. Pin the trim-aware helper arm.
    #[test]
    fn featured_briefing_source_ratchet_requires_trim() {
        const SRC: &str = include_str!("missions.rs");
        assert!(
            SRC.contains("featured_briefing_text(f.briefing.as_deref())"),
            "featured hero must route briefing through featured_briefing_text"
        );
        // concat! so this test body does not match itself.
        let old_filter = concat!(".filter(|b| !b.", "is_empty())");
        assert!(
            !SRC.contains(old_filter),
            "is_empty-only briefing filter must not return — whitespace-only briefings \
             would keep the empty string on the featured hero instead of the fallback"
        );
        let old_arm = concat!("Some(b) if !b.", "is_empty()");
        assert!(
            !SRC.contains(old_arm),
            "match-arm !b.is_empty() without trim must not return on featured briefing paths"
        );
        let trim_arm = concat!("Some(b) if !b.trim().", "is_empty()");
        assert!(
            SRC.contains(trim_arm),
            "featured_briefing_text must keep the trim-aware match arm"
        );
    }

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

    include!("../../shared/is_http_url_cases.rs");

    #[test]
    fn mission_art_falls_back_for_non_http_thumbnails() {
        let mut wrong = Vec::new();
        for (input, ok) in IS_HTTP_URL_CASES {
            let got = mission_art_url(Some(input));
            if *ok {
                if got != *input {
                    wrong.push(format!("  dropped a legitimate thumb {input:?}"));
                }
            } else if got != PLACEHOLDER_ART {
                wrong.push(format!("  kept a non-http thumb {input:?} (got {got:?})"));
            }
        }
        assert!(
            wrong.is_empty(),
            "mission art sink wrong on {} of {} cases:\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
        assert_eq!(mission_art_url(None), PLACEHOLDER_ART);
        assert_eq!(mission_art_url(Some("")), PLACEHOLDER_ART);
    }

    #[test]
    fn author_avatar_emits_src_only_for_http_urls() {
        let mut wrong = Vec::new();
        for (input, should_img) in IS_HTTP_URL_CASES {
            match (author_avatar_img_src(input), should_img) {
                (Some(_), false) => wrong.push(format!("  RENDERED AN IMG FOR {input:?}")),
                (None, true) => wrong.push(format!("  refused a legitimate avatar {input:?}")),
                _ => {}
            }
        }
        assert!(
            wrong.is_empty(),
            "mission author avatar sink wrong on {} of {} cases:\n{}",
            wrong.len(),
            IS_HTTP_URL_CASES.len(),
            wrong.join("\n")
        );
    }

    // ───────────────────────────────────────────────────────────────────────────────────────
    // T-282 — the version differ.
    //
    // NON-VACUITY IS THE WHOLE POINT HERE. A differ that answers "nothing changed" to every
    // question renders perfectly and passes any test that only asserts a section appears. So every
    // test below names the SPECIFIC change it expects, and `differ_is_not_vacuous_on_identical_input`
    // is its paired control: the same helper pair must also report *silence* when there is nothing
    // to say, or "reports everything" would pass just as cheaply as "reports nothing".
    // ───────────────────────────────────────────────────────────────────────────────────────
    use super::{
        census_line, classify_row, diff_mission_payloads, version_census, RowChange,
        DIFF_SAMPLE_CAP,
    };
    use serde_json::Value;

    /// Mission "Bridgehead at Levie" v0.1.0 — three slots in one squad, one objective, one
    /// loadout. Shapes copied from `map_engine_core::mission::flatten`'s own fixtures so the
    /// differ is tested against the rows this editor really writes.
    fn levie_v1() -> Value {
        json!({
            "schemaVersion": 1,
            "title": "Bridgehead at Levie",
            "map": { "terrain": "everon", "bounds": [0, 0, 12800, 12800] },
            "environment": { "timeOfDay": "dawn", "weather": "clear" },
            "objectives": [{ "id": "o1", "name": "Seize the bridge" }],
            "markers": [],
            "vehicles": [],
            "entities": [],
            "loadouts": { "l1": { "primary": "L85A3" } },
            "editor": {
                "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army" }],
                "squads": [{ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1" }],
                "slots": [
                    { "id": "s1", "name": "Alpha SL",  "squadId": "sq1", "index": 0, "role": "SL",
                      "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                    { "id": "s2", "name": "Alpha TL",  "squadId": "sq1", "index": 1, "role": "TL",
                      "position": { "x": 110.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                    { "id": "s3", "name": "Alpha RFL", "squadId": "sq1", "index": 2, "role": "RFL",
                      "position": { "x": 120.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } }
                ],
                "editorLayers": [{ "id": "L1", "name": "Default" }]
            }
        })
    }

    /// v0.2.0 — FIVE named changes against [`levie_v1`], and nothing else:
    ///   1. `s1` **edited** (role SL → PL)
    ///   2. `s2` **moved** (position only)
    ///   3. `s3` **removed**
    ///   4. `s4` **added**
    ///   5. terrain everon → arland, `environment.timeOfDay` dawn → dusk, `environment.weather`
    ///      **deleted**
    /// The slot array is also written in a different order than v1, so any test that passes here
    /// has also proved the diff is order-insensitive.
    fn levie_v2() -> Value {
        json!({
            "schemaVersion": 1,
            "title": "Bridgehead at Levie",
            "map": { "terrain": "arland", "bounds": [0, 0, 12800, 12800] },
            "environment": { "timeOfDay": "dusk" },
            "objectives": [{ "id": "o1", "name": "Seize the bridge" }],
            "markers": [],
            "vehicles": [],
            "entities": [],
            "loadouts": { "l1": { "primary": "L85A3" } },
            "editor": {
                "factions": [{ "id": "f1", "key": "BLUFOR", "name": "US Army" }],
                "squads": [{ "id": "sq1", "factionId": "f1", "callsign": "Alpha", "name": "Alpha 1-1" }],
                "slots": [
                    { "id": "s1", "name": "Alpha SL",  "squadId": "sq1", "index": 0, "role": "PL",
                      "position": { "x": 100.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } },
                    { "id": "s2", "name": "Alpha TL",  "squadId": "sq1", "index": 1, "role": "TL",
                      "position": { "x": 480.5, "y": 902.5, "z": 0.0, "rotation": 90.0 } },
                    { "id": "s4", "name": "Alpha MED", "squadId": "sq1", "index": 3, "role": "MED",
                      "position": { "x": 130.0, "y": 200.0, "z": 0.0, "rotation": 0.0 } }
                ],
                "editorLayers": [{ "id": "L1", "name": "Default" }]
            }
        })
    }

    fn slots_delta(d: &super::MissionDiff) -> &super::CollectionDelta {
        d.collections
            .iter()
            .find(|c| c.label == "Slots")
            .expect("Slots must be a diffed collection")
    }

    /// **The non-vacuity test.** Five known changes in, five specifically-named changes out.
    /// Every number here is asserted exactly — `assert!(delta.changed() > 0)` would pass for a
    /// differ that mislabelled all four slot events as the same thing.
    #[test]
    fn differ_names_the_specific_change_between_two_versions() {
        let d = diff_mission_payloads(&levie_v1(), &levie_v2());
        let slots = slots_delta(&d);

        assert_eq!(slots.added, 1, "s4 was added: {slots:#?}");
        assert_eq!(slots.removed, 1, "s3 was removed: {slots:#?}");
        assert_eq!(slots.moved, 1, "s2 changed only its position: {slots:#?}");
        assert_eq!(slots.edited, 1, "s1 changed its role: {slots:#?}");
        assert_eq!(slots.unchanged, 0, "no slot survived untouched: {slots:#?}");
        assert_eq!(slots.a_rows, 3);
        assert_eq!(slots.b_rows, 3);

        // The sample lines name the rows by the label an author would recognise, in document
        // order on the new side, then removals in document order on the old side.
        assert_eq!(
            slots.samples,
            vec![
                "~ Alpha SL edited".to_string(),
                "~ Alpha TL moved".to_string(),
                "+ Alpha MED".to_string(),
                "− Alpha RFL".to_string(),
            ],
            "the differ must name WHICH rows changed, not just how many"
        );

        // Untouched collections must stay silent — a differ that flags everything is as useless
        // as one that flags nothing.
        for c in &d.collections {
            if c.label != "Slots" {
                assert!(
                    c.is_unchanged(),
                    "{} reported a change and nothing in it changed: {c:#?}",
                    c.label
                );
            }
        }

        // Scalars, including the DELETED environment key — the case a new-side-only walk misses.
        let fields: Vec<(String, String, String)> = d
            .fields
            .iter()
            .map(|f| (f.label.clone(), f.from.clone(), f.to.clone()))
            .collect();
        assert_eq!(
            fields,
            vec![
                ("Terrain".into(), "everon".into(), "arland".into()),
                (
                    "Environment · timeOfDay".into(),
                    "dawn".into(),
                    "dusk".into()
                ),
                ("Environment · weather".into(), "clear".into(), "—".into()),
            ],
            "scalar changes must name the field and both values"
        );
    }

    /// The paired control. Without this, "report every field as changed" would satisfy the test
    /// above; with it, the differ has to be right in both directions.
    #[test]
    fn differ_is_not_vacuous_on_identical_input() {
        let d = diff_mission_payloads(&levie_v1(), &levie_v1());
        assert!(
            d.is_empty(),
            "a version compared with itself has no changes: {d:#?}"
        );
        assert_eq!(d.changed_collections().count(), 0);
        assert_eq!(slots_delta(&d).unchanged, 3);
    }

    /// **The claim that justifies a structural diff over a textual one.** `compile_payload`
    /// re-emits collections in whatever order `entityOrder` held, so a re-save can permute the
    /// arrays with no authorial change. A line diff would scream; this must not.
    #[test]
    fn reordering_rows_is_not_a_change() {
        let v1 = levie_v1();
        let mut v2 = levie_v1();
        let slots = v2["editor"]["slots"].as_array_mut().unwrap();
        slots.reverse();
        assert_ne!(
            serde_json::to_string(&v1).unwrap(),
            serde_json::to_string(&v2).unwrap(),
            "the two payloads must differ TEXTUALLY, or this test proves nothing"
        );
        let d = diff_mission_payloads(&v1, &v2);
        assert!(
            d.is_empty(),
            "permuting the emit order is not an edit — a textual diff would have reported 3 \
             changed rows here: {d:#?}"
        );
    }

    /// Moving a slot is not the same event as re-roling it, and the differ must not collapse them.
    #[test]
    fn position_only_change_is_moved_not_edited() {
        let a = json!({ "id": "s1", "role": "SL", "position": { "x": 1.0, "y": 2.0 } });
        let b = json!({ "id": "s1", "role": "SL", "position": { "x": 9.0, "y": 2.0 } });
        assert_eq!(classify_row(&a, &b), RowChange::Moved);
        // Same position, different role → edited.
        let c = json!({ "id": "s1", "role": "PL", "position": { "x": 1.0, "y": 2.0 } });
        assert_eq!(classify_row(&a, &c), RowChange::Edited);
        // Moved AND re-roled → the stronger claim wins.
        let e = json!({ "id": "s1", "role": "PL", "position": { "x": 9.0, "y": 2.0 } });
        assert_eq!(classify_row(&a, &e), RowChange::Edited);
        // A key appearing is an edit even though every shared key is equal.
        let f =
            json!({ "id": "s1", "role": "SL", "position": { "x": 1.0, "y": 2.0 }, "tag": "CMD" });
        assert_eq!(classify_row(&a, &f), RowChange::Edited);
        assert_eq!(classify_row(&a, &a), RowChange::Same);
    }

    /// T-584 — a representation flip is not an edit.
    ///
    /// `serde_json` parses `100` as `PosInt` and `100.0` as `Float`, and `Value`'s own `PartialEq`
    /// calls those unequal. Before this fix `classify_row` inherited that, so a payload whose
    /// integers had been re-serialized as floats (a `yrs` BigInt↔Number coercion across an editor
    /// change) reported every touched row as **edited** while nothing had changed.
    #[test]
    fn a_number_representation_flip_is_not_an_edit() {
        // Positive control FIRST: the two payloads really do differ under `Value`'s equality, so
        // this test is exercising the flip and not comparing a value with itself.
        let int = json!({ "id": "s1", "count": 100, "position": { "x": 1, "y": 2 } });
        let float = json!({ "id": "s1", "count": 100.0, "position": { "x": 1.0, "y": 2.0 } });
        assert_ne!(
            int, float,
            "serde_json must still consider these unequal, or this test proves nothing"
        );

        assert_eq!(
            classify_row(&int, &float),
            RowChange::Same,
            "100 and 100.0 are the same number; reporting an edit here is crying wolf"
        );
        // And a flip confined to `position` must not read as a MOVE either.
        let moved = json!({ "id": "s1", "count": 100, "position": { "x": 9.0, "y": 2.0 } });
        assert_eq!(classify_row(&int, &moved), RowChange::Moved);

        // The negative half: widening must not swallow a real difference, including two distinct
        // u64s above 2^53 that would collide if compared as f64.
        let other = json!({ "id": "s1", "count": 101.0, "position": { "x": 1, "y": 2 } });
        assert_eq!(classify_row(&int, &other), RowChange::Edited);
        let big_a = json!({ "id": "s1", "count": 9_007_199_254_740_993_u64 });
        let big_b = json!({ "id": "s1", "count": 9_007_199_254_740_992_u64 });
        assert_eq!(
            classify_row(&big_a, &big_b),
            RowChange::Edited,
            "two distinct u64s above 2^53 must not be collapsed by an f64 comparison"
        );
    }

    /// Every row on each side lands in exactly one bucket. This is the invariant that makes a
    /// silently-dropped row impossible: if the differ ever ignores an input, these sums stop
    /// matching the raw row counts.
    #[test]
    fn every_row_is_accounted_for_on_both_sides() {
        let d = diff_mission_payloads(&levie_v1(), &levie_v2());
        for c in &d.collections {
            assert_eq!(
                c.b_rows,
                c.added + c.moved + c.edited + c.unchanged + c.unkeyed_b,
                "{} lost a NEW-side row between the walk and the counters: {c:#?}",
                c.label
            );
            assert_eq!(
                c.a_rows,
                c.removed + c.moved + c.edited + c.unchanged + c.unkeyed_a,
                "{} lost an OLD-side row between the walk and the counters: {c:#?}",
                c.label
            );
        }
    }

    /// A row with no `id` cannot be matched to anything. It must be COUNTED and reported, never
    /// dropped — dropping it is "a tool reports success over an input it never examined".
    #[test]
    fn unkeyed_rows_are_reported_not_silently_dropped() {
        let mut v2 = levie_v1();
        v2["editor"]["slots"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "name": "orphan", "role": "RFL" }));
        let d = diff_mission_payloads(&levie_v1(), &v2);
        let slots = slots_delta(&d);
        assert_eq!(slots.b_rows, 4, "the id-less row is still a row");
        assert_eq!(slots.unkeyed_b, 1);
        assert_eq!(
            slots.added, 0,
            "an unkeyable row must not be guessed as added"
        );
        assert_eq!(
            slots.unreadable(),
            1,
            "the UI needs a caveat count it can surface"
        );
    }

    /// `loadouts` is the one collection whose id is the OBJECT KEY, not a field in the row. A
    /// differ that only understands arrays would report it as permanently unchanged.
    #[test]
    fn object_keyed_loadouts_are_diffed_by_their_map_key() {
        let mut v2 = levie_v1();
        v2["loadouts"]["l1"]["primary"] = json!("M4A1");
        v2["loadouts"]["l2"] = json!({ "primary": "AKM" });
        let d = diff_mission_payloads(&levie_v1(), &v2);
        let lo = d
            .collections
            .iter()
            .find(|c| c.label == "Loadouts")
            .expect("Loadouts must be a diffed collection");
        assert_eq!(lo.edited, 1, "l1 changed its primary: {lo:#?}");
        assert_eq!(lo.added, 1, "l2 is new: {lo:#?}");
        assert_eq!(lo.a_rows, 1);
        assert_eq!(lo.b_rows, 2);
    }

    /// **The performance contract, as an assertion.** Counts are exact at any size; the sample
    /// list is what stays bounded. If this ever fails by `samples.len()` growing, a diff of the
    /// 367k-slot missions this codebase really has would build a 367k-entry `Vec<String>`.
    #[test]
    fn counts_are_exact_at_scale_while_samples_stay_bounded() {
        const N: usize = 5_000;
        let mut big = levie_v1();
        {
            let slots = big["editor"]["slots"].as_array_mut().unwrap();
            slots.clear();
            for i in 0..N {
                slots.push(json!({
                    "id": format!("bulk-{i}"),
                    "name": format!("Rifleman {i}"),
                    "squadId": "sq1",
                    "position": { "x": i as f64, "y": 0.0, "z": 0.0, "rotation": 0.0 }
                }));
            }
        }
        let empty = json!({});
        let d = diff_mission_payloads(&empty, &big);
        let slots = slots_delta(&d);
        assert_eq!(slots.added, N, "every bulk row must be counted");
        assert_eq!(slots.b_rows, N);
        assert_eq!(
            slots.samples.len(),
            DIFF_SAMPLE_CAP,
            "the sample list is the only thing allowed to grow with the document, and it must not"
        );
    }

    /// The census must be the differ's own output, not a parallel row-counter that can drift.
    #[test]
    fn census_is_the_diff_from_the_empty_document() {
        let census = version_census(&levie_v1());
        assert_eq!(
            census,
            vec![
                ("Slots", 3),
                ("Squads", 1),
                ("Factions", 1),
                ("Editor layers", 1),
                ("Objectives", 1),
                ("Loadouts", 1),
            ],
            "census must list every non-empty collection, in payload order"
        );
        assert_eq!(
            census_line(&census),
            "3 slots · 1 squads · 1 factions · 1 editor layers · 1 objectives · 1 loadouts"
        );
        // Empty collections are omitted, not rendered as "0 markers".
        assert!(!census.iter().any(|(l, _)| *l == "Markers"));
        // The seeded golden mission's payload is literally `{}` — the dossier's empty state.
        assert!(version_census(&json!({})).is_empty());
    }

    /// T-282 Class-R source ratchet. Pins the two structural decisions this slice exists to make,
    /// so a later "simplification" into a whole-document string compare goes red here rather than
    /// in a browser at 367k slots.
    #[test]
    fn differ_source_ratchet_is_structural_and_wired_into_the_dossier() {
        const SRC: &str = include_str!("missions.rs");
        assert!(
            SRC.contains("let mut left: HashMap<&str, &Value> = HashMap::new();"),
            "the id index must stay a BORROWED map — cloning rows doubles a hundreds-of-MB payload"
        );
        // concat! so this assertion does not match its own source text.
        let textual = concat!("serde_json::to_string(a) ", "== serde_json::to_string(b)");
        assert!(
            !SRC.contains(textual),
            "a whole-document string compare is O(n) memory and answers the wrong question — \
             re-emit order is not an edit (see reordering_rows_is_not_a_change)"
        );
        assert!(
            SRC.contains("{version_history_section(&m)}"),
            "the version rail must be MOUNTED in the dossier — a differ nothing calls is dead \
             code that #![allow(dead_code)] at the top of this file would hide"
        );
        assert!(
            SRC.contains("fn version_census(payload: &Value) -> Vec<(&'static str, usize)> {\n    diff_mission_payloads(&Value::Null, payload)"),
            "the census must route through the differ, not count rows a second way"
        );
    }

    // ── T-117 — mission document upload ──────────────────────────────────────────────────────

    /// **The test this ticket lives or dies on.**
    ///
    /// `create_version` answers a bad document with 400 + a `details` array naming every finding.
    /// This drives the FULL client chain the browser drives — the raw response body through
    /// [`crate::client::error_body_message`] (which folds `details` into the message as extra
    /// lines) and out through [`upload_failure`] — and demands every finding survive.
    ///
    /// The body is **not invented**: it is the response measured from the running dev API on
    /// 2026-07-31 for `POST /missions/{id}/versions` with `schemaVersion:"nope"`,
    /// `markers:"not-an-array"` and a numeric slot `role`.
    ///
    /// RED under perturbation: return `Vec::new()` from `upload_failure`'s generic arm, or route
    /// 400 through one of the detail-less status arms, and this fails naming the lost finding.
    #[test]
    fn a_rejected_document_surfaces_every_finding_the_api_sent() {
        let measured = json!({
            "error": "invalid mission payload",
            "details": [
                "/schemaVersion: \"nope\" is not of type \"integer\"",
                "/markers: \"not-an-array\" is not of type \"array\"",
                "/editor: this payload does not match the shape the mission compiler reads, so it cannot be compiled — invalid type: integer `123`, expected a string at line 1 column 83"
            ]
        });
        let msg = crate::client::error_body_message(&measured)
            .expect("the client must extract a message from a 400 body carrying `error`");
        let (head, findings) = upload_failure(400, Some(&msg), "0.2.0");

        assert_eq!(
            findings.len(),
            3,
            "all three findings must reach the author; got {findings:?}"
        );
        for needle in [
            "/schemaVersion",
            "is not of type \"integer\"",
            "/markers",
            "/editor",
            "expected a string at line 1 column 83",
        ] {
            assert!(
                findings.iter().any(|f| f.contains(needle)),
                "the author must be told {needle:?} — an upload UI that swallows the reason is \
                 worse than none, because the document cannot be fixed from a verdict. \
                 findings={findings:?}"
            );
        }
        assert!(
            head.contains("invalid mission payload") && head.contains("400"),
            "the headline must name the verdict and the status; got {head:?}"
        );
        assert!(
            head.contains("3 problem(s)"),
            "the headline must point at the list rather than pretend there is one problem; got {head:?}"
        );
    }

    /// A systematic defect yields one finding per slot; the API caps its own list at 20 and the
    /// client folds at [`crate::client::MAX_ERROR_DETAILS`] with a `… and N more` tail. That tail
    /// is itself a finding row and must reach the author — otherwise a 20-problem document reads
    /// as a 6-problem one and the author "fixes" it and re-uploads into the same wall.
    #[test]
    fn a_truncated_finding_list_still_tells_the_author_how_many_there_are() {
        let details: Vec<String> = (0..20)
            .map(|i| format!("/editor/slots/{i}/role: 42 is not of type \"string\""))
            .collect();
        let body = json!({ "error": "invalid mission payload", "details": details });
        let msg = crate::client::error_body_message(&body).expect("message");
        let (_head, findings) = upload_failure(400, Some(&msg), "1.0.0");
        assert_eq!(
            findings.len(),
            crate::client::MAX_ERROR_DETAILS + 1,
            "six shown findings plus the count tail; got {findings:?}"
        );
        assert!(
            findings.last().is_some_and(|t| t.contains("14 more")),
            "the tail must name the 14 findings not shown; got {:?}",
            findings.last()
        );
    }

    /// The four failures that are NOT a defect in the document get their own words, because the
    /// author does something different about each — and none of them carries `details`.
    #[test]
    fn non_validation_failures_say_what_to_do_instead() {
        let (head, rows) = upload_failure(409, Some("version already exists"), "0.2.0");
        assert!(rows.is_empty());
        assert!(
            head.contains("0.2.0") && head.contains("already exists"),
            "409 must name the taken version; got {head:?}"
        );

        // The backend computes its own MB figure from MISSION_VERSION_MAX_BODY_BYTES. Echo it —
        // restating a number here would be a second source of truth that silently goes stale.
        let (head, _) = upload_failure(413, Some("payload too large (max 256 MB)"), "0.2.0");
        assert!(
            head.contains("256 MB"),
            "413 must carry the server's own limit verbatim; got {head:?}"
        );

        let (head, _) = upload_failure(401, None, "0.2.0");
        assert!(head.to_lowercase().contains("sign in"), "got {head:?}");

        // status 0 is the client's transport failure (`ApiErr` uses 0 for network/serde).
        let (head, _) = upload_failure(0, None, "0.2.0");
        assert!(
            head.contains("Nothing was saved"),
            "a network failure must say the mission is unchanged; got {head:?}"
        );

        // A 403 has no `details` but does have a message, and must not be swallowed.
        let (head, _) = upload_failure(403, Some("not your mission"), "0.2.0");
        assert!(
            head.contains("not your mission"),
            "the backend's own reason must survive; got {head:?}"
        );
    }

    /// **The round-trip.** Both exporters wrap the editor payload in an envelope
    /// (`compile_export` and the API's `build_mission_doc`), and posting that envelope verbatim
    /// is MEASURED to answer `400 payload must include editor content` — a message that sends the
    /// author looking in entirely the wrong place. Export → Upload only works if the envelope is
    /// unwrapped, so this pins both shapes.
    ///
    /// The envelope below is the key set captured from `GET /missions/{id}/export` on 2026-07-31.
    #[test]
    fn both_an_exported_file_and_a_bare_payload_upload() {
        let payload = json!({
            "schemaVersion": 3,
            "editor": { "slots": [{ "id": "s1", "callsign": "Alpha 1-1", "role": "Rifleman" }] },
            "markers": []
        });
        let envelope = json!({
            "exportFormatVersion": 1,
            "missionId": "8881e97e-b348-4052-bdea-4a45c8e962a7",
            "title": "T-117 scratch probe",
            "terrain": "everon",
            "gameMode": "pve_coop",
            "weather": "clear",
            "timeOfDay": "14:00:00",
            "maxPlayers": 8,
            "version": "0.2.0",
            "armory": [],
            "payload": payload.clone(),
            "exportedAt": "2026-07-31T05:00:00Z"
        });
        assert_eq!(
            unwrap_export_envelope(envelope).expect("an exported mission file must upload"),
            payload,
            "the envelope's `payload` is the editor document the API validates"
        );
        // A bare editor payload has no `exportFormatVersion` and passes through untouched.
        assert_eq!(
            unwrap_export_envelope(payload.clone()).expect("a bare payload must upload"),
            payload
        );
    }

    /// The two ways an envelope can be unusable are named as envelope problems, not as schema
    /// errors 200 lines deep — the author needs to know the file is the wrong *kind* of thing.
    #[test]
    fn a_broken_envelope_is_named_as_an_envelope_problem() {
        let no_payload = json!({ "exportFormatVersion": 1, "title": "x" });
        let err = unwrap_export_envelope(no_payload).expect_err("no payload must be refused");
        assert!(
            err.contains("exported mission file") && err.contains("payload"),
            "got {err:?}"
        );

        let bad_payload = json!({ "exportFormatVersion": 1, "payload": [1, 2, 3] });
        let err = unwrap_export_envelope(bad_payload).expect_err("array payload must be refused");
        assert!(
            err.contains("an array"),
            "the kind must be named; got {err:?}"
        );

        // A top-level array is the shape someone gets by exporting the wrong thing entirely.
        let err = unwrap_export_envelope(json!([1, 2])).expect_err("array doc must be refused");
        assert!(
            err.contains("JSON object") && err.contains("an array"),
            "got {err:?}"
        );
        let err = unwrap_export_envelope(json!("hello")).expect_err("string doc must be refused");
        assert!(err.contains("a string"), "got {err:?}");
    }

    /// A syntax error keeps `serde_json`'s line/column. The server's own answer for the same file
    /// is a flat `payload is not valid JSON` with no position, so parsing client-side is the only
    /// way an author locates the break in a 30 MB document.
    #[test]
    fn a_syntax_error_keeps_its_line_and_column() {
        let err = parse_uploaded_document("{\n  \"schemaVersion\": 3,\n  \"editor\": {,\n}")
            .expect_err("malformed JSON must be refused");
        assert!(err.contains("not valid JSON"), "got {err:?}");
        assert!(
            err.contains("line 3") && err.contains("column"),
            "the position is the whole point of parsing here; got {err:?}"
        );
        assert_eq!(
            parse_uploaded_document("   \n\t ").expect_err("blank file"),
            "That file is empty."
        );
        // The happy path goes through the envelope unwrap, so a pasted export parses to its payload.
        let got = parse_uploaded_document(
            r#"{"exportFormatVersion":1,"payload":{"schemaVersion":3,"markers":[]}}"#,
        )
        .expect("an exported file must parse");
        assert_eq!(got, json!({"schemaVersion": 3, "markers": []}));
    }

    /// The size gate refuses BEFORE the file is read, and names both numbers. `usize` at the exact
    /// budget is accepted — an off-by-one here would reject a file the tab can handle.
    #[test]
    fn the_size_gate_names_both_numbers_and_is_inclusive_at_the_budget() {
        assert!(oversize_refusal(0).is_none());
        assert!(
            oversize_refusal(UPLOAD_MAX_BYTES).is_none(),
            "a document exactly at the budget must be accepted"
        );
        let refusal = oversize_refusal(UPLOAD_MAX_BYTES + 1).expect("over budget must be refused");
        assert!(
            refusal.contains("67.1 MB"),
            "the budget must be named; got {refusal:?}"
        );
        let huge = oversize_refusal(400 << 20).expect("400 MiB must be refused");
        // BOTH numbers, and this is the case that can actually prove it: one byte over the budget
        // rounds to the same text as the budget, so the assertion above cannot tell the two apart
        // and must not be asked to. (It used to be written as `contains(X) && contains(X)` — the
        // same needle twice — which read as a two-number check and was a one-number check.)
        assert!(
            huge.contains("419.4 MB") && huge.contains("67.1 MB"),
            "both the author's file size and the budget must be named — 'too large' without them \
             is unactionable; got {huge:?}"
        );
        assert!(
            huge.contains("Mission Creator"),
            "a refusal must say what to do instead; got {huge:?}"
        );
    }

    /// The suggested version has to be valid SemVer 2.0 (`create_version` parses it, T-363) AND
    /// unused (a duplicate is a 409), so it is a patch bump of the tip. Pre-release / build
    /// metadata is dropped rather than incremented inside.
    #[test]
    fn the_suggested_version_bumps_the_patch_of_the_tip() {
        assert_eq!(next_semver(Some("0.1.0")), "0.1.1");
        assert_eq!(next_semver(Some("1.2.3")), "1.2.4");
        assert_eq!(next_semver(Some("2.0.9")), "2.0.10");
        assert_eq!(next_semver(Some("1.2.3-rc1")), "1.2.4");
        assert_eq!(next_semver(Some("1.2.3+build.7")), "1.2.4");
        // No current version, or something this parser cannot read, falls back to the number
        // `create_mission` itself writes first — always valid, and only taken if the mission
        // already has one, in which case the 409 says so precisely.
        assert_eq!(next_semver(None), "0.1.0");
        for junk in ["", "1", "1.2", "1.2.3.4", "banana", "1.2.x", " 1.2.3"] {
            assert_eq!(next_semver(Some(junk)), "0.1.0", "junk semver {junk:?}");
        }
    }

    /// The preview runs the T-282 differ with a real second payload — the case it was written for
    /// and could not reach until an uploaded document existed. Counts are exact; unkeyed rows are
    /// reported rather than dropped.
    #[test]
    fn the_preview_names_what_the_document_would_change() {
        let current = json!({
            "schemaVersion": 3,
            "map": { "terrain": "everon" },
            "editor": { "slots": [
                { "id": "a", "callsign": "Alpha 1-1", "role": "Rifleman", "position": [1.0, 1.0] },
                { "id": "b", "callsign": "Alpha 1-2", "role": "Medic",    "position": [2.0, 2.0] }
            ]}
        });
        let uploaded = json!({
            "schemaVersion": 3,
            "map": { "terrain": "arland" },
            "editor": { "slots": [
                { "id": "a", "callsign": "Alpha 1-1", "role": "Rifleman", "position": [9.0, 9.0] },
                { "id": "b", "callsign": "Alpha 1-2", "role": "Grenadier","position": [2.0, 2.0] },
                { "id": "c", "callsign": "Alpha 1-3", "role": "Rifleman", "position": [3.0, 3.0] },
                { "callsign": "no id at all" }
            ]}
        });
        let lines = diff_summary_lines(&diff_mission_payloads(&current, &uploaded));
        let joined = lines.join("\n");
        assert!(
            joined.contains("Terrain: everon → arland"),
            "a changed scalar must be named; got\n{joined}"
        );
        assert!(
            joined.contains("Slots: 2 → 4"),
            "the row census must be exact on both sides; got\n{joined}"
        );
        assert!(
            joined.contains("1 added") && joined.contains("1 moved") && joined.contains("1 edited"),
            "added/moved/edited must be distinguished; got\n{joined}"
        );
        assert!(
            joined.contains("no usable id"),
            "a row the differ could not key must be reported, not silently dropped — that is the \
             'reports success over an input it never examined' failure in miniature; got\n{joined}"
        );

        // Identical payloads produce nothing to say, which is what lets the UI offer the
        // "uploading this would only add a version number" line honestly.
        assert!(diff_mission_payloads(&current, &current).is_empty());
        assert!(diff_summary_lines(&diff_mission_payloads(&current, &current)).is_empty());
        // The empty document on the left is the census path — same code, so they cannot drift.
        assert!(!diff_mission_payloads(&Value::Null, &uploaded).is_empty());
    }

    /// T-117 Class-R source ratchet. The pure functions above can all pass while the panel is
    /// wired to nothing, so pin the wiring itself: the control, the route, and — above all —
    /// that the failure path goes through [`upload_failure`] into a rendered findings list.
    ///
    /// RED under perturbation: delete the section, point the POST somewhere else, drop the
    /// `up_findings.set(rows)` assignment, or delete the findings block from the view.
    #[test]
    fn the_upload_panel_is_wired_to_the_versions_route() {
        const SRC: &str = include_str!("missions.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");

        for (needle, why) in [
            (
                "data-testid=\"mission-upload-pick\"",
                "the dossier must render a control that opens the file picker",
            ),
            (
                "data-testid=\"mission-upload-findings\"",
                "the rejection findings must have a rendered home; a toast is not one",
            ),
            (
                "up_findings.set(rows)",
                "the rows from upload_failure must reach the rendered list — dropping this \
                 assignment is exactly the 'swallowed the reason' defect",
            ),
            (
                "upload_failure(status, msg.as_deref(), &semver)",
                "the error path must route through upload_failure, not a fixed string",
            ),
            (
                "format!(\"/missions/{}/versions\", id_sv.get_value())",
                "the upload must POST the versions route — the only mission-write route with a \
                 lifted body cap",
            ),
            (
                "parse_uploaded_document(&text)",
                "the picked file must be parsed (and envelope-unwrapped) before it is posted",
            ),
            (
                "oversize_refusal(size)",
                "the size gate must run on the picked file's size before the read",
            ),
            (
                "version_body_to_writer(",
                "the wire body must be built by map-engine-core, not hand-rolled here, so the \
                 two doors onto create_version cannot drift — `version_body_to_writer` and the \
                 editor Save's `version_body` are both wrappers over one `VersionBody` struct, \
                 and compile.rs's `both_doors_onto_create_version_serialise_identical_bytes` \
                 pins them byte-identical",
            ),
            (
                "api_post_raw(store, &path, body)",
                "the upload must hand over an already-serialised String — `api_post` takes a \
                 `Value` and clones it again per attempt, which is the amplification that sets \
                 UPLOAD_MAX_BYTES",
            ),
            (
                "up_doc.with_untracked(|slot|",
                "the document must be read BY REFERENCE to build the body — `get_untracked()` \
                 clones the whole parsed tree, and on wasm32 that clone is a whole extra copy of \
                 the mission for no reason",
            ),
        ] {
            assert!(production.contains(needle), "{why} (missing: {needle})");
        }

        // `File`/`FileList` are what `HtmlInputElement::files()` needs; without them this compiles
        // on native and dies on the wasm build (the T-446 lesson, pinned the same way).
        let cargo = include_str!("../Cargo.toml");
        for feat in ["\"File\"", "\"FileList\"", "\"Blob\""] {
            assert!(
                cargo.contains(feat),
                "Cargo.toml must keep the web-sys {feat} feature for the document picker"
            );
        }
    }
}
