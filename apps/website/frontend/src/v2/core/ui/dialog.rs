//! The modal dialog: a backdrop and a centred panel, or nothing at all.
//!
//! **Role:** the shared modal surface — confirmations, short forms, anything that must be answered
//! before the page continues.
//! **Position:** rendered inline wherever it is declared; it paints over the page through the
//! stacking tier the shared registry hands it.
//! **Signals & state:** the open flag belongs to the caller. Registers with the overlay registry on
//! mount and drops that registration, along with its key listener, in the same cleanup.
//! **Invariants:** renders no DOM at all while closed, so a closed dialog cannot intercept a click
//! or answer a hit test. The dismiss key closes it only when the registry says it is the topmost
//! open surface, which is what stops one key press closing a stack.

use leptos::prelude::*;

use super::icons::MaterialIcon;
use super::modal_stack;
use super::page_header::cn;

/// A modal dialog.
///
/// Renders nothing at all while closed — no backdrop, no hidden node — so a closed dialog cannot
/// intercept a click or show up in a hit test. While open it renders a backdrop and a centred popup
/// holding the optional title and description and then `children`. Mounted wherever it is declared;
/// the stacking class comes from the shared registry, so the last-opened surface paints on top.
#[component]
#[allow(dead_code)]
pub fn Dialog(
    /// Whether the surface is open. Owned by the caller; the surface sets it false on dismissal.
    open: RwSignal<bool>,
    /// Heading text. Omitted entirely when empty.
    /// Supporting line under the heading. Omitted entirely when empty.
    /// Extra classes for the panel — the caller owns its maximum width.
    #[prop(optional)]
    title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] class: &'static str,
    /// The surface's body.
    children: ChildrenFn,
) -> impl IntoView {
    // An untracked read that tolerates a disposed signal, rather than one that panics: the
    // registration is dropped in the cleanup, which runs before the signal is disposed today, but a
    // disposed read must answer "not open" if that order ever changes.
    let modal_id = modal_stack::register(move || open.try_get_untracked().unwrap_or(false));
    // Opening any dialog clears the transient popovers, without per-call-site wiring.
    Effect::new(move |_| {
        if open.get() {
            modal_stack::close_registered_transients();
        }
    });
    // The dismiss key closes it, listened for at the window.
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" && modal_stack::is_topmost_open(modal_id) {
            modal_stack::mark_escape_consumed();
            open.set(false);
        }
    });
    on_cleanup(move || {
        esc.remove();
        modal_stack::unregister(modal_id);
    });
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
