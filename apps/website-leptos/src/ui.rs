//! Small UI helpers ported from lib/utils.ts (`cn`) + components/MaterialIcon.tsx + AuthGate.tsx.
use crate::auth::AuthStore;
use leptos::prelude::*;

/// Minimal class-string join (clsx-like): drop empties, space-join. NOTE: unlike the React `cn`
/// (clsx + tailwind-merge), this does NOT resolve Tailwind conflicts — the V gate proves the
/// shell's class combos have none; a twMerge-equivalent lands only if a conflicting combo appears.
pub fn cn(classes: &[&str]) -> String {
    classes
        .iter()
        .filter(|c| !c.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Material Symbols icon — a font-glyph span whose text is the ligature name. Ported from
/// MaterialIcon.tsx (`<span class="material-symbols-outlined …" style?>{name}</span>`). `filled`
/// renders the FILL-1 variant; React sets it via CSSOM (`el.style.fontVariationSettings`), which the
/// browser reflects onto the style attribute as `font-variation-settings: 'FILL' 1;` — matched here.
#[component]
pub fn MaterialIcon(
    name: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] filled: bool,
) -> impl IntoView {
    let style = filled.then_some("font-variation-settings: \"FILL\" 1;");
    view! { <span class=cn(&["material-symbols-outlined", class]) style=style>{name}</span> }
}

/// Page title + optional subtitle header. Ported from components/PageHeader.tsx.
#[component]
pub fn PageHeader(title: &'static str, #[prop(optional)] subtitle: &'static str) -> impl IntoView {
    view! {
        <header class="mb-8">
            <h1 class="mb-2 text-3xl font-bold text-on-surface">{title}</h1>
            {(!subtitle.is_empty())
                .then(|| view! { <p class="max-w-3xl text-on-surface-variant">{subtitle}</p> })}
        </header>
    }
}

/// AuthGate — API-backed pages show a sign-in CTA for guests (and a "Loading session…" state while
/// bootstrapping), otherwise the children. Ported from components/AuthGate.tsx. Reactive on the
/// AuthStore so it flips to the content once a session lands.
#[component]
pub fn AuthGate(children: ChildrenFn) -> impl IntoView {
    let auth = expect_context::<AuthStore>();
    move || {
        if auth.bootstrapping.get() {
            view! {
                <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                    "Loading session…"
                </div>
            }
            .into_any()
        } else if !auth.is_authenticated() {
            view! {
                <div class="flex min-h-[40vh] flex-col items-center justify-center gap-4 text-center">
                    <p class="text-on-surface-variant">
                        "Sign in to load live data from the platform."
                    </p>
                    <a
                        href="/login"
                        class="rounded-lg bg-primary px-6 py-2.5 text-sm font-medium text-on-primary"
                    >
                        "Sign in with Discord"
                    </a>
                </div>
            }
            .into_any()
        } else {
            children().into_any()
        }
    }
}
