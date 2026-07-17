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
use crate::nav::Role;
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

/// Mission visibility badge — status → (label, badge variant), missions.tsx `VISIBILITY`.
fn visibility_badge(status: &str) -> impl IntoView + use<> {
    let (label, variant) = match status {
        "draft" => ("Draft".to_string(), "neutral"),
        "pending_approval" => ("Open for review".to_string(), "warning"),
        "live" => ("Live".to_string(), "success"),
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
    // isMaker: admin (and guest browse-mode) both satisfy mission_maker → the New Mission affordances.
    let is_maker = store.has_min_role(Role::MissionMaker);
    let scope_idx = RwSignal::new(0usize);
    let q = RwSignal::new(String::new());
    let terrain = RwSignal::new(String::new());
    let mode = RwSignal::new(String::new());
    let players = RwSignal::new(String::new());
    let preview_id = RwSignal::new(None::<String>);
    let create_open = RwSignal::new(false);
    let sheet_open = RwSignal::new(false);

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
            if !is_maker || create_open.get_untracked() {
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
                <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
                <div class="custom-scrollbar relative z-10 h-full w-full overflow-y-auto bg-surface-glass backdrop-blur-xl">
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
                                            let show_empty_cta = is_maker
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
                                                    open_preview,
                                                    open_create,
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
    is_maker: bool,
    scope_idx: RwSignal<usize>,
    open_create: impl Fn() + Copy + 'static,
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
            {is_maker
                .then(|| {
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
                })}
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
    open_preview: impl Fn(String) + Copy + 'static,
    open_create: impl Fn() + Copy + 'static,
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
            <div class="mb-6 flex flex-wrap items-center gap-2 rounded-2xl border border-white/5 bg-black/20 p-2 backdrop-blur-md">
                <input
                    type="search"
                    placeholder="Search operations..."
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
                            .map(|m| mission_card(m, open_preview))
                            .collect_view()}
                    </div>
                }
                    .into_any()
            }}
        </>
    }
}

fn mission_card(m: MissionCard, open_preview: impl Fn(String) + Copy + 'static) -> impl IntoView {
    let art = m
        .thumbnail_url
        .clone()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| PLACEHOLDER_ART.into());
    let initial = m
        .author_name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into());
    let has_avatar = !m.author_avatar.is_empty();
    let mid = m.id.clone();
    view! {
        <button
            type="button"
            on:click=move |_| open_preview(mid.clone())
            class="group overflow-hidden rounded-2xl border border-white/10 bg-surface-container/60 text-left transition-all hover:-translate-y-0.5 hover:border-white/25 hover:shadow-xl"
        >
            <div class="relative h-48 w-full overflow-hidden bg-surface-container-low">
                <img
                    src=art
                    alt=""
                    class="h-48 w-full object-cover transition-transform duration-500 group-hover:scale-105"
                />
                <span class="absolute top-3 left-3">
                    <span class=format!(
                        "{} border-white/10 bg-black/40 backdrop-blur-md",
                        badge_class("primary"),
                    )>{game_mode_label(&m.game_mode).to_string()}</span>
                </span>
                <span class="absolute top-3 right-3">{visibility_badge(&m.status)}</span>
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
    let is_maker = store.has_min_role(Role::MissionMaker);
    let is_admin = store.has_min_role(Role::Admin);
    let me = StoredValue::new(store.user.get_untracked().map(|u| u.discord_id));

    view! {
        <Suspense fallback=move || {
            view! { <p class="p-8 text-on-surface-variant">"Loading dossier…"</p> }
        }>
            {move || {
                mission
                    .get()
                    .map(|opt| match opt {
                        Some(m) => {
                            let is_owner = me.get_value().as_deref()
                                == Some(m.author_id.as_str());
                            let can_edit = is_maker && (is_owner || is_admin);
                            dossier_sheet_body(
                                    m,
                                    id_sv,
                                    can_edit,
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

#[allow(clippy::too_many_arguments)]
fn dossier_sheet_body(
    m: MissionDetail,
    id_sv: StoredValue<String>,
    can_edit: bool,
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
                {can_edit
                    .then(|| {
                        view! {
                            <section>
                                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                                    "Manage"
                                </h3>
                                <div class="flex flex-wrap gap-2">
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
                            class="flex-1 bg-action/90 py-5 font-bold tracking-wide text-on-action backdrop-blur-md transition-colors hover:bg-action"
                        >
                            "[ OPEN IN MISSION CREATOR ]"
                        </button>
                    }
                })}
            <button
                type="button"
                on:click=toasts_planner
                class="flex-1 border-t border-white/10 bg-surface-container-high/90 py-5 font-bold tracking-wide text-primary backdrop-blur-md transition-colors hover:bg-surface-container-highest"
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
