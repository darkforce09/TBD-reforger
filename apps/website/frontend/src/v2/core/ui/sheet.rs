//! The side sheet: the dialog's shape, laid against an edge.
//!
//! **Role:** the shared edge-anchored surface — dossiers, detail panels, anything that wants the
//! full height of the viewport.
//! **Position:** rendered inline wherever it is declared; it paints over the page through the
//! stacking tier the shared registry hands it.
//! **Signals & state:** the open flag belongs to the caller. Registers with the same overlay
//! registry the dialog uses, and releases the registration and its key listener together.
//! **Invariants:** renders no DOM while closed. It shares the dialog's registry because the two
//! stack on each other in practice, and a registry covering only one of them would let the dismiss
//! key close the sheet out from under an open dialog.

use leptos::prelude::*;

use super::icons::MaterialIcon;
use super::modal_stack;
use super::page_header::cn;

/// A side sheet.
///
/// The same shape as the dialog — nothing rendered while closed, backdrop plus panel while open — laid
/// out against an edge rather than centred, with `bleed` removing the panel's own padding for children
/// that own the full layout.
///
/// It shares the dialog's registry deliberately. The two stack on each other in practice: a dossier
/// opens in a sheet and its confirmations are dialogs over it, so a registry that covered only dialogs
/// would leave the dismiss key closing the dossier out from under an open confirmation — the same
/// defect, one component over.
#[component]
#[allow(dead_code)]
pub fn Sheet(
    /// Whether the surface is open. Owned by the caller; the surface sets it false on dismissal.
    open: RwSignal<bool>,
    /// Heading text. Omitted entirely when empty.
    /// Supporting line under the heading. Omitted entirely when empty.
    /// Extra classes for the panel — the caller owns its maximum width.
    /// Drop the panel's own padding, for children that own the full layout.
    /// The surface's body.
    #[prop(optional)]
    title: &'static str,
    #[prop(optional)] description: &'static str,
    #[prop(optional)] class: &'static str,
    #[prop(optional)] bleed: bool,
    children: ChildrenFn,
) -> impl IntoView {
    let modal_id = modal_stack::register(move || open.try_get_untracked().unwrap_or(false));
    let esc = leptos::prelude::window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" && modal_stack::is_topmost_open(modal_id) {
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
                // No backdrop blur on either the scrim or the sliding panel: two stacked blurs
                // recomputed on every frame of the slide were what made the entrance stutter. The
                // scrim carries the dimming and the panel sits on an opaque surface instead.
                <div
                    class="animate-overlay-fade fixed inset-0 z-50 bg-black/60 transition-opacity duration-300"
                    on:click=move |_| open.set(false)
                ></div>
                <div class=cn(
                    &[
                        "animate-sheet-in fixed z-50 flex flex-col border border-outline-variant/30 bg-surface-container shadow-2xl outline-none transition-transform duration-300 ease-out inset-y-0 right-0 h-full w-[92vw] max-w-md border-l",
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
