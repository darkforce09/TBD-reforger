//! The account settings route: the viewer's profile, their Arma identity, and their service
//! record.
//!
//! **Role:** fetches the signed-in user and the state of their game-account link, renders the
//! three cards, and owns the two mutations that link and unlink that account.
//! **Position:** the `/settings` route, behind the authentication gate, inside the navigation
//! frame.
//! **Signals & state:** two `LocalResource`s — the user and the link status — plus the
//! `pending_code`, `gen_busy` and `unlink_busy` signals carried in [`ArmaLinkCtx`]; the
//! authentication store and the toast queue come from context.
//! **Invariants:** the cards render only once *both* fetches have settled, so the link status is
//! never shown as unlinked merely because it has not arrived. Both mutations are `wasm32`-only;
//! natively the resources resolve to `None` and the page renders its failure line.
#![allow(dead_code)]
use crate::v2::core::api::dto::{LinkStatus, MeResponse};
use crate::v2::core::ui::{AuthGate, MaterialIcon, PageHeader};
use leptos::prelude::*;

#[cfg(test)]
#[path = "tests/settings.rs"]
mod tests;

/// The role chip's class. The base label size is deliberately absent: it collides with the
/// variant's own `text-*` utility, and the merge the design system performs keeps the later of the
/// two, so the chip takes the default size. Spelling the collision out here is what keeps the
/// rendered chip identical to the primitive's.
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";

/// The settings page, behind the authentication gate.
#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <SettingsInner />
        </AuthGate>
    }
}

/// The mutation handles threaded into the settled render.
///
/// All three are `Copy` signals created *above* the suspense boundary, so a freshly generated code
/// survives the re-render a refetch causes; created inside, it would be discarded the moment the
/// link status it triggered came back.
#[derive(Clone, Copy)]
struct ArmaLinkCtx {
    pending_code: RwSignal<Option<String>>,
    gen_busy: RwSignal<bool>,
    unlink_busy: RwSignal<bool>,
}

/// Fetches the viewer and their link status, owns the two mutations, and renders the cards, a
/// loading line or a failure line.
#[component]
fn SettingsInner() -> impl IntoView {
    let store = expect_context::<crate::v2::core::auth::AuthStore>();
    let me = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<MeResponse>(store, "/me")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<MeResponse>
        }
    });
    let link = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::v2::core::api::client::api_get::<LinkStatus>(store, "/me/link/status")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<LinkStatus>
        }
    });
    let ctx = ArmaLinkCtx {
        pending_code: RwSignal::new(None),
        gen_busy: RwSignal::new(false),
        unlink_busy: RwSignal::new(false),
    };

    // Requesting a code shows it, toasts, and refetches the link status. The handler is built
    // here and passed down so both buttons stay inside the settled render.
    let on_generate = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            if ctx.gen_busy.get_untracked() {
                return;
            }
            ctx.gen_busy.set(true);
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_post::<
                    crate::v2::core::api::dto::LinkCodeResponse,
                >(store, "/me/link", serde_json::json!({}))
                .await
                {
                    Ok(resp) => {
                        ctx.pending_code.set(Some(resp.code));
                        toasts.success("Link code generated — enter it in-game");
                        link.refetch();
                    }
                    Err(_) => toasts.error("Failed to generate link code"),
                }
                ctx.gen_busy.set(false);
            });
        }
    };
    // Unlinking clears the code panel and refetches both the user and the link status.
    let on_unlink = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let toasts = crate::v2::core::ui::toast::use_toasts();
            if ctx.unlink_busy.get_untracked() {
                return;
            }
            ctx.unlink_busy.set(true);
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_delete(store, "/me/link").await {
                    Ok(()) => {
                        ctx.pending_code.set(None);
                        toasts.success("Arma identity unlinked");
                        me.refetch();
                        link.refetch();
                    }
                    Err(_) => toasts.error("Failed to unlink"),
                }
                ctx.unlink_busy.set(false);
            });
        }
    };

    // Gate on BOTH resources, so what renders is the settled page with the link status resolved
    // rather than a profile above an "Unlinked" line that has not been fetched yet.
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || match (me.get(), link.get()) {
                (Some(Some(me)), Some(link_opt)) => {
                    body(me, link_opt, ctx, on_generate, on_unlink).into_any()
                }
                (Some(None), _) => {
                    view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                }
                _ => ().into_any(),
            }}
        </Suspense>
    }
}

/// The three cards, rendered from settled data: the profile, the Arma identity with its two
/// actions, and the service statistics.
fn body(
    me: MeResponse,
    link: Option<LinkStatus>,
    ctx: ArmaLinkCtx,
    on_generate: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
    on_unlink: impl Fn(leptos::ev::MouseEvent) + Copy + 'static,
) -> impl IntoView {
    let user = me.user;
    let avatar = crate::v2::core::utils::safe_avatar_url(&user.avatar_url);
    let linked = link.as_ref().map(|l| l.linked).unwrap_or(false);
    let pending = link.as_ref().and_then(|l| l.pending_code).unwrap_or(false);
    let status_class = if linked {
        "text-success"
    } else {
        "text-on-surface-variant"
    };
    let link_label = link
        .as_ref()
        .filter(|l| l.linked)
        .map(|l| {
            let ident = l
                .arma_character
                .clone()
                .filter(|s| !s.is_empty())
                .or_else(|| l.arma_id.clone())
                .unwrap_or_default();
            format!("Linked ({ident})")
        })
        .unwrap_or_else(|| "Unlinked".to_string());

    view! {
        <div class="mx-auto w-full max-w-2xl">
            <PageHeader
                title="Settings"
                subtitle="Account profile, Arma identity, and service statistics."
            />

            // ── Profile ──
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass mb-6">
                <h2 class="mb-4 text-lg font-semibold">"Profile"</h2>
                <div class="flex items-center gap-4">
                    <img
                        src=avatar
                        alt=""
                        class="h-16 w-16 rounded-full border border-border-subtle object-cover"
                    />
                    <div>
                        <p class="text-lg font-semibold">{user.username.clone()}</p>
                        <p class="text-sm text-on-surface-variant">{user.discord_handle.clone()}</p>
                        <span class="mt-2 inline-block">
                            <span class=BADGE_PRIMARY>{user.role.as_str()}</span>
                        </span>
                    </div>
                </div>
            </div>

            // ── Arma Identity ──
            <div
                id="arma-link"
                class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass mb-6 scroll-mt-24"
            >
                <h2 class="mb-4 text-lg font-semibold">"Arma Identity"</h2>
                <p class="mb-4 text-sm text-on-surface-variant">
                    "Status: "
                    <span class=status_class>{link_label}</span>
                </p>
                // A code generated in this session outranks the server's "one is already
                // pending" notice; with neither, no panel shows at all.
                {move || match ctx.pending_code.get() {
                    Some(code) => {
                        view! {
                            <p class="mb-4 rounded-lg border border-primary/30 bg-primary/10 p-3 font-mono text-sm">
                                "Link code: "
                                {code}
                            </p>
                        }
                            .into_any()
                    }
                    None => {
                        pending
                            .then(|| {
                                view! {
                                    <p class="mb-4 rounded-lg border border-primary/30 bg-primary/10 p-3 text-sm">
                                        "A link code is already pending. Generate a new one to display it, then enter it in-game."
                                    </p>
                                }
                            })
                            .into_any()
                    }
                }}
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        on:click=on_generate
                        prop:disabled=move || ctx.gen_busy.get()
                        class="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                    >
                        "Generate Link Code"
                    </button>
                    {linked
                        .then(|| {
                            view! {
                                <button
                                    type="button"
                                    on:click=on_unlink
                                    prop:disabled=move || ctx.unlink_busy.get()
                                    class="rounded-lg border border-border-subtle px-4 py-2 text-sm disabled:opacity-50"
                                >
                                    "Unlink Arma ID"
                                </button>
                            }
                        })}
                </div>
            </div>

            // ── Service Stats ──
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass">
                <h2 class="mb-4 flex items-center gap-2 text-lg font-semibold">
                    <MaterialIcon name="military_tech" class="text-primary" />
                    "Service Stats"
                </h2>
                <div class="grid grid-cols-2 gap-4 text-sm">
                    <div>
                        <span class="text-on-surface-variant">"Total Operations"</span>
                        <p class="font-mono text-headline-lg text-primary">
                            {user.total_deployments}
                        </p>
                    </div>
                    <div>
                        <span class="text-on-surface-variant">"Attendance"</span>
                        <p class="font-mono text-headline-lg text-success">
                            {user.attendance_rate}
                            "%"
                        </p>
                    </div>
                </div>
            </div>
        </div>
    }
}
