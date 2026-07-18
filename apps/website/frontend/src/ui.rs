//! Small UI helpers ported from lib/utils.ts (`cn`) + components/MaterialIcon.tsx + AuthGate.tsx.
use crate::auth::AuthStore;
use crate::nav::Role;
use leptos::prelude::*;

/// Neutral inline avatar (data URI) shown when a user has no Discord avatar — byte-identical to
/// lib/avatar.ts `DEFAULT_AVATAR` (`encodeURIComponent`-encoded SVG).
pub const DEFAULT_AVATAR: &str = "data:image/svg+xml;utf8,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%2064%2064%22%3E%3Crect%20width%3D%2264%22%20height%3D%2264%22%20rx%3D%228%22%20fill%3D%22%23394150%22%2F%3E%3Ccircle%20cx%3D%2232%22%20cy%3D%2225%22%20r%3D%2212%22%20fill%3D%22%237a8699%22%2F%3E%3Cpath%20d%3D%22M12%2058c0-11%209-19%2020-19s20%208%2020%2019z%22%20fill%3D%22%237a8699%22%2F%3E%3C%2Fsvg%3E";

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

/// Badge variant classes — components/ui/badge.tsx `badgeVariants` (cva) with the base merged in.
/// (React's twMerge collision quirks don't apply here: no caller passes a conflicting override.)
#[allow(dead_code)]
pub fn badge_class(variant: &str) -> String {
    let v = match variant {
        "primary" => "border-primary/30 bg-primary/10 text-primary",
        "tertiary" => "border-tertiary/30 bg-tertiary/10 text-tertiary",
        "warning" => "border-tactical-yellow/30 bg-tactical-yellow/10 text-tactical-yellow",
        "success" => "border-success/30 bg-success/15 text-success",
        "error" => "border-error-alert/30 bg-error-alert/10 text-error-alert",
        _ => "border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant",
    };
    format!("inline-flex items-center gap-1 rounded border px-2 py-0.5 text-label-sm uppercase whitespace-nowrap {v}")
}

/// Frosted, centered macOS modal — the components/ui/dialog.tsx port (T-159.25). Renders **no DOM
/// while closed** (transient overlay: V captures of default states are unaffected; base-ui's
/// enter/exit transition attributes are not replicated). Esc and the backdrop close it.
#[component]
#[allow(dead_code)]
pub fn Dialog(
    open: RwSignal<bool>,
    #[prop(optional)] title: &'static str,
    #[prop(optional)] description: &'static str,
    /// Extra classes on the popup (React `className`, e.g. `max-w-lg`).
    #[prop(optional)]
    class: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    // Esc closes (base-ui behavior). Window-level like React's focus-trap dismissal.
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || esc.remove());
    move || {
        open.get().then(|| {
            view! {
                <div
                    class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
                    on:click=move |_| open.set(false)
                ></div>
                <div class=cn(
                    &[
                        "glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200",
                        class,
                    ],
                )>
                    {(!title.is_empty())
                        .then(|| {
                            view! {
                                <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                    <div class="min-w-0">
                                        <h2 class="text-headline-sm text-on-surface">{title}</h2>
                                        {(!description.is_empty())
                                            .then(|| {
                                                view! {
                                                    <p class="mt-1 text-label-md text-on-surface-variant">
                                                        {description}
                                                    </p>
                                                }
                                            })}
                                    </div>
                                    <button
                                        type="button"
                                        aria-label="Close"
                                        on:click=move |_| open.set(false)
                                        class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                    >
                                        <MaterialIcon name="close" />
                                    </button>
                                </div>
                            }
                        })}
                    <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">{children()}</div>
                </div>
            }
        })
    }
}

/// macOS slide-over panel — the components/ui/sheet.tsx port (right side; `bleed` = children own
/// the full layout). Same no-DOM-while-closed / no-transition-attrs notes as `Dialog`.
#[component]
#[allow(dead_code)]
pub fn Sheet(
    open: RwSignal<bool>,
    #[prop(optional)] title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] bleed: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || esc.remove());
    move || {
        open.get().then(|| {
            view! {
                <div
                    class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-300"
                    on:click=move |_| open.set(false)
                ></div>
                <div class=cn(
                    &[
                        "glass animate-sheet-in fixed z-50 flex flex-col border-outline-variant/30 shadow-2xl outline-none transition-transform duration-300 ease-out inset-y-0 right-0 h-full w-[92vw] max-w-md border-l",
                        class,
                    ],
                )>
                    {if bleed {
                        children().into_any()
                    } else {
                        view! {
                            {(!title.is_empty())
                                .then(|| {
                                    view! {
                                        <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                                            <div class="min-w-0">
                                                <h2 class="text-headline-sm text-on-surface">{title}</h2>
                                                {(!description.is_empty())
                                                    .then(|| {
                                                        view! {
                                                            <p class="mt-1 text-label-md text-on-surface-variant">
                                                                {description}
                                                            </p>
                                                        }
                                                    })}
                                            </div>
                                            <button
                                                type="button"
                                                aria-label="Close"
                                                on:click=move |_| open.set(false)
                                                class="shrink-0 rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                                            >
                                                <MaterialIcon name="close" />
                                            </button>
                                        </div>
                                    }
                                })}
                            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                                {children()}
                            </div>
                        }
                            .into_any()
                    }}
                </div>
            }
        })
    }
}

/// AdminGate — AuthGate + an admin-role check. Ported from components/AdminGate.tsx: authed
/// non-admins see "Admin access required." instead of the children.
#[component]
pub fn AdminGate(children: ChildrenFn) -> impl IntoView {
    view! {
        <AuthGate>
            {
                let children = children.clone();
                move || {
                    let auth = expect_context::<AuthStore>();
                    if auth.has_min_role(Role::Admin) {
                        children().into_any()
                    } else {
                        view! {
                            <div class="flex min-h-[40vh] items-center justify-center text-on-surface-variant">
                                "Admin access required."
                            </div>
                        }
                        .into_any()
                    }
                }
            }
        </AuthGate>
    }
}
