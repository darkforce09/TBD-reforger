//! The read/edit switch and the mode it toggles.
//!
//! **Role:** the two-button control an administrator uses to move a modpack between its read
//! dossier and its edit form, and the enum that names those two states.
//! **Position:** the top right of the detail pane, in both the dossier and the form.
//! **Signals & state:** writes the page's `mode` signal and reads it back to mark the active
//! button.
//! **Invariants:** the control renders the same in both modes — only the highlighted button
//! moves — so switching back and forth never shifts the layout.

use leptos::prelude::*;

/// Which half of the modpack detail pane is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MpMode {
    Read,
    Edit,
}

/// The read/edit switch bound to `mode`.
pub(super) fn read_edit_toggle(mode: RwSignal<MpMode>) -> impl IntoView {
    view! {
        <div class="flex shrink-0 items-center rounded-full border border-white/10 bg-black/30 p-1 font-mono text-xs">
            {[("read", MpMode::Read), ("edit", MpMode::Edit)]
                .into_iter()
                .map(|(m, target)| {
                    let class = move || {
                        if mode.get() == target {
                            "rounded-full px-4 py-1.5 tracking-wider uppercase transition bg-primary/20 text-primary shadow-[0_0_12px_rgba(173,198,255,0.25)]"
                        } else {
                            "rounded-full px-4 py-1.5 tracking-wider uppercase transition text-on-surface-variant hover:text-on-surface"
                        }
                    };
                    view! {
                        <button type="button" class=class on:click=move |_| mode.set(target)>
                            "[ "{m}" ]"
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}
