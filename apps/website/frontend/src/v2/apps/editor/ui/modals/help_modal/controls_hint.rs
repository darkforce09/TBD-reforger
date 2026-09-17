//! Controls Hint state and floating reference card.

use super::*;

/// Close-button hit area for the floating hint card.
pub(super) const HINT_CLOSE_BTN: &str = "shrink-0 rounded p-1.5 text-on-surface";
thread_local! {
    static HINT_SHOWN: Cell<bool> = const { Cell::new(false) };
}

#[must_use]
/// Returns the persisted hint visibility across chrome remounts.
pub fn hint_shown() -> bool {
    HINT_SHOWN.with(Cell::get)
}

/// Persists the hint visibility across chrome remounts.
pub fn set_hint_shown(v: bool) {
    HINT_SHOWN.with(|c| c.set(v));
}

/// Renders the floating keyboard shortcut reference.
#[component]
pub fn ControlsHint(open: RwSignal<bool>) -> impl IntoView {
    view! {
        {move || {
            open.get()
                .then(|| {
                    view! {
                        <div
                            data-controls-hint
                            class="pointer-events-none fixed inset-0 z-50 flex items-start justify-center pt-16"
                        >
                            <div class="glass animate-menu-in pointer-events-auto max-h-[70vh] w-[38rem] max-w-[92vw] overflow-y-auto rounded-xl p-4 shadow-lg">
                                <div class="mb-3 flex items-center justify-between gap-3">
                                    <span class="text-label-md font-semibold text-on-surface">
                                        "Controls — keyboard shortcuts"
                                    </span>
                                    <button
                                        type="button"
                                        aria-label="Close the Controls Hint"
                                        title="Close (Esc)"
                                        class=cn(&[HINT_CLOSE_BTN, HOVER_FILL])
                                        on:click=move |_| {
                                            open.set(false);
                                            set_hint_shown(false);
                                        }
                                    >
                                        <MaterialIcon name="close" class="block text-base" />
                                    </button>
                                </div>
                                {GROUPS
                                    .iter()
                                    .map(|g| {
                                        let rows = SHORTCUTS
                                            .iter()
                                            .filter(|s| s.group == *g)
                                            .map(|s| {
                                                view! {
                                                    <li
                                                        class="flex items-baseline gap-3 py-0.5"
                                                        data-codes=s.codes.join(" ")
                                                    >
                                                        <kbd class="w-52 shrink-0 text-right font-mono text-code-md text-on-surface">
                                                            {s.chord}
                                                        </kbd>
                                                        <span class="text-label-sm text-on-surface-variant">
                                                            {s.action}
                                                        </span>
                                                    </li>
                                                }
                                            })
                                            .collect_view();
                                        view! {
                                            <div class="mb-3">
                                                <div class="mb-1 text-label-sm font-semibold uppercase tracking-wide text-outline">
                                                    {*g}
                                                </div>
                                                <ul class="flex flex-col">{rows}</ul>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                                <div class="border-t border-white/10 pt-2 text-label-sm text-outline">
                                    "This list is pinned against the editor's real key handlers — a new binding cannot ship undocumented."
                                </div>
                            </div>
                        </div>
                    }
                })
        }}
    }
}
