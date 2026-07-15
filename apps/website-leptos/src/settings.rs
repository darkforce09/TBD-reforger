//! Settings (/settings) — ported from pages/Settings.tsx. `<AuthGate>` → `/me` + `/me/link/status`
//! Resources → `QueryState` → Profile / Arma Identity / Service Stats cards. This is a POPULATED
//! render: the dev goldens carry a linked Arma identity + a real profile, so every field (username,
//! handle, role badge, linked status string, deployment + attendance stats) is byte-exact-verified.
//!
//! The mutation buttons (Generate Link Code / Unlink) render but their click handlers + the
//! generated-code / pending-code panels are behavior (T-toast + mutations) — a follow-up; the static
//! render (buttons present, no pending panel since the golden's pending_code is false) is gated here.
#![allow(dead_code)]
use crate::dto::{LinkStatus, MeResponse};
use crate::ui::{AuthGate, MaterialIcon, PageHeader, DEFAULT_AVATAR};
use leptos::prelude::*;

/// Badge (variant="primary") class from ui/badge.tsx. React's `cn` (tailwind-merge) DROPS the base
/// `text-label-sm`: twMerge reads it + the variant's `text-primary` as colliding `text-*` utilities
/// and keeps the last, so the chip inherits the default 16px/400 — matched here by omitting it (same
/// twMerge quirk as the nav links; a general Rust tw_merge is still deferred).
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";

#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <AuthGate>
            <SettingsInner />
        </AuthGate>
    }
}

#[component]
fn SettingsInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let me = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<MeResponse>(store, "/me")
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
            crate::client::api_get::<LinkStatus>(store, "/me/link/status")
                .await
                .ok()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<LinkStatus>
        }
    });
    // Gate on BOTH resources so the settled DOM (linked status resolved) is what renders — matches
    // React's final render after useMe + useLinkStatus both land.
    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || match (me.get(), link.get()) {
                (Some(Some(me)), Some(link_opt)) => body(me, link_opt).into_any(),
                (Some(None), _) => {
                    view! { <p class="text-error">"Failed to load data."</p> }.into_any()
                }
                _ => ().into_any(),
            }}
        </Suspense>
    }
}

fn body(me: MeResponse, link: Option<LinkStatus>) -> impl IntoView {
    let user = me.user;
    let avatar = if user.avatar_url.is_empty() {
        DEFAULT_AVATAR.to_string()
    } else {
        user.avatar_url.clone()
    };
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
                // pendingCode (mutation result) is null on load; only the pending_code branch can show.
                {pending
                    .then(|| {
                        view! {
                            <p class="mb-4 rounded-lg border border-primary/30 bg-primary/10 p-3 text-sm">
                                "A link code is already pending. Generate a new one to display it, then enter it in-game."
                            </p>
                        }
                    })}
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                    >
                        "Generate Link Code"
                    </button>
                    {linked
                        .then(|| {
                            view! {
                                <button
                                    type="button"
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
