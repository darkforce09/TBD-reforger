//! The mission grid and the card it repeats, with the formatters both share.
//!
//! **Role:** lays the library body out — hero, toolbar, then either the grid of mission cards or
//! the right empty state — and owns the card itself along with the small formatters the hero and
//! the dossier also use.
//! **Position:** the scrolling body of the `/missions` route, under the page header.
//! **Signals & state:** the filter signals are passed straight through to the toolbar; each card
//! owns its own optimistic bookmark latch and busy flag, and reads the session store and the
//! toast queue from context.
//! **Invariants:** a card is an outer element with two sibling buttons, never a button inside a
//! button, so the bookmark control has its own hit target. A returned-mission note is shown only
//! on the viewer's own missions and only when a reason was actually given. Thumbnails are
//! rendered only when they are `http` URLs; anything else takes the placeholder.

use super::featured_hero::featured_hero;
use super::filter_bar::filter_bar;
use super::search_bar::search_bar;
use crate::v2::core::api::dto::MissionCard;
use crate::v2::core::auth::url_guard;
use crate::v2::core::ui::{badge_class, MaterialIcon};
use leptos::prelude::*;

/// Cinematic fallback art, so a card or the hero never renders as an empty grey block.
pub(super) const PLACEHOLDER_ART: &str = "https://lh3.googleusercontent.com/aida/AP1WRLtxuwSoyDyCrRuQu8gTHWuSmoOWZq8e7gw0bSjjZCmteU96TomvCGHto-cuqHYV_0gxNUjw_Lx2SWgiEl2W3vEi6aVH84DpTky5lG8-FKDJOzH96TrwAJwGJwE3DSwSN1gRC7miWds0X7kNvMAZRBgQPu_5g2iX9RtJ3WYUlgHbfVLYcmV7TaHPUvhZHvvvKenG2B3S2CRER15d2kdG5YNFbtFwtwgzEIeYG2jP4GubWd7SMO0bADPFFA";

/// The `src` for a card, hero or dossier image.
///
/// Only `http` and `https` thumbnails are used; anything else takes the placeholder, which is
/// itself a legitimate URL.
pub(super) fn mission_art_url(stored: Option<&str>) -> String {
    stored
        .filter(|u| url_guard::is_http_url(u))
        .unwrap_or(PLACEHOLDER_ART)
        .to_string()
}

/// The author avatar `src` for a card, or `None` when there is no usable URL and the card should
/// fall back to initials.
pub(super) fn author_avatar_img_src(url: &str) -> Option<&str> {
    url_guard::is_http_url(url).then_some(url)
}

/// A terrain name with its first character capitalised; an em dash when there is none.
pub(super) fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The display name of a game mode; anything unrecognised is passed through unchanged.
pub(super) fn game_mode_label(m: &str) -> &str {
    match m {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// Whether a listed mission is bookmarked by the viewer.
///
/// The flag rides the card's catch-all rather than being a named field, so the control has to
/// read the same value the Bookmarked scope filters on instead of inventing a second source.
pub(super) fn card_is_bookmarked(m: &MissionCard) -> bool {
    m.extra
        .get("bookmarked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// The bookmark route for one mission, which answers both a post and a delete.
pub(super) fn bookmark_api_path(id: &str) -> String {
    format!("/missions/{id}/bookmark")
}

/// The status chip on a card and at the head of the dossier.
///
/// The label comes from the one status formatter the platform shares; only the badge variant is
/// decided here, because that is a badge concern and the dossier's detail grid has no chips. The
/// mission status column is an enum of five values, so every reachable status is named and the
/// fallback arm is unreachable defence rather than a silent hole.
pub(super) fn visibility_badge(status: &str) -> impl IntoView + use<> {
    let label = crate::v2::pages::mission_hub::overview::mission_status_label(status);
    let variant = match status {
        "pending_approval" => "warning",
        "live" => "success",
        "rejected" => "error",
        // draft, archived, or anything unknown
        _ => "neutral",
    };
    view! { <span class=badge_class(variant)>{label}</span> }
}

/// The library body: the hero, the search and filter toolbar, then the grid or an empty state.
///
/// `show_empty_cta` is the "you have no missions yet" invitation, which is only right on the
/// viewer's own unfiltered list; every other empty result says so plainly instead.
#[allow(clippy::too_many_arguments)]
pub(super) fn body(
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
            {featured_hero(featured, open_preview)}

            // The toolbar writes the filter signals live; the list resource re-keys on them.
            <div class="mb-6 flex flex-wrap items-center gap-2 rounded-2xl border border-white/5 bg-black/40 p-2">
                {search_bar(q)}
                {filter_bar(terrain, mode, players)}
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

/// One mission card: art, author, title, badges, the bookmark control and the status chip.
pub(super) fn mission_card(
    m: MissionCard,
    me_id: StoredValue<Option<String>>,
    open_preview: impl Fn(String) + Copy + 'static,
    changed: Callback<()>,
) -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let art = mission_art_url(m.thumbnail_url.as_deref());
    // The returned-mission line. The rejection reason is the only channel by which an author ever
    // learns why a mission came back — the approvals queue is administrator-only, so they cannot
    // go and look — and the card is what they see first, so a "Returned" chip with no reason
    // beside it would only send them hunting.
    //
    // Own missions only, and only when a reason was actually given: a rejection without one
    // leaves an empty string that never reaches the wire, so an absent and an empty reason both
    // mean the same thing and neither should render an empty box.
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
    // An optimistic latch, so the star flips before the list refetch lands and the Bookmarked tab
    // can populate from the write without waiting for a full remount.
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
            let toasts = crate::v2::core::ui::toast::use_toasts();
            let path = bookmark_api_path(&mid_bm);
            leptos::task::spawn_local(async move {
                let result = if next {
                    crate::v2::core::api::client::api_post_ok(store, &path, serde_json::json!({}))
                        .await
                } else {
                    crate::v2::core::api::client::api_delete(store, &path).await
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
                        toasts.error(crate::v2::core::api::client::api_error_message(
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
        // An outer element rather than one big button, so the bookmark control can be a real
        // sibling: a button nested inside a button is invalid and breaks the click target.
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
            // The bookmark control, a sibling of the open-dossier button rather than nested in it.
            // The status chip sits beside it so the star owns the top-right hit target without
            // covering any copy.
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
