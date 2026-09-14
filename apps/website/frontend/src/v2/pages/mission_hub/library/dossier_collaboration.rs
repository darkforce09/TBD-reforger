//! The dossier's collaboration controls and the two overlays behind them.
//!
//! **Role:** the row of collaboration buttons, the comments panel they open, and the invite
//! dialog — the parts of the dossier that are about other people rather than about the mission.
//! **Position:** a section near the foot of the dossier's scroll area; the panel and the dialog
//! render outside it, as overlays over the dossier sheet.
//! **Signals & state:** `comments_open` and `invite_open` are owned by the dossier and passed in,
//! so the buttons and the overlays they open share one piece of state. Reads the toast queue from
//! context.
//! **Invariants:** commenting and inviting have no endpoint behind them yet, so both surfaces say
//! so in place of a control that would look as if it had saved something.

use crate::v2::core::ui::Sheet;
use leptos::prelude::*;

/// The collaboration row: comments for everyone, sharing and invitations for an editor.
pub(super) fn collaboration_section(
    can_edit: bool,
    comments_open: RwSignal<bool>,
    invite_open: RwSignal<bool>,
) -> impl IntoView {
    let toasts_share = move |_| {
        #[cfg(target_arch = "wasm32")]
        crate::v2::core::ui::toast::use_toasts().success("Will allow anyone to view and comment");
    };
    view! {
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
    }
}

/// The comments panel: the shell for suggestions that do not change the mission until an editor
/// applies them. Nothing is fetched, because no endpoint serves it yet.
pub(super) fn comments_sheet(comments_open: RwSignal<bool>) -> impl IntoView {
    view! {
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
    }
}

/// The invite dialog: the field is disabled because granting another author edit access has no
/// endpoint behind it yet.
pub(super) fn invite_dialog(invite_open: RwSignal<bool>) -> impl IntoView {
    view! {
        <crate::v2::core::ui::Dialog
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
        </crate::v2::core::ui::Dialog>
    }
}
