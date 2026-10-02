//! Import and buffer outcomes and the document persistence line.

use super::*;

/// Renders action receipts, refusals, and the live write verdict.
pub(super) fn status_and_persistence(state: ArsenalTabState) -> impl IntoView {
    let ArsenalTabState {
        buffer_refusals,
        buffer_status,
        import_refusals,
        import_status,
        commits,
        persist_refused,
        ..
    } = state;
    view! {
                        // the buffer's outcome. A refusal lists EVERY reason and applied
                        // nothing; a receipt states what landed AND what it costs to undo.
                        {move || {
                            let refusals = buffer_refusals.get();
                            if !refusals.is_empty() {
                                let n = refusals.len() - 1; // the lead line is not a reason
                                return view! {
                                    <div
                                        data-loadout-refused=n.to_string()
                                        class="rounded-lg border border-error-alert/40 bg-error/10 p-2 text-label-sm normal-case text-error-alert"
                                    >
                                        <ul class="flex list-none flex-col gap-1">
                                            {refusals
                                                .into_iter()
                                                .map(|m| view! { <li>{m}</li> })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    </div>
                                }
                                    .into_any();
                            }
                            let status = buffer_status.get();
                            if status.is_empty() {
                                return ().into_any();
                            }
                            view! {
                                <p
                                    data-loadout-status
                                    class="text-label-sm normal-case text-on-surface-variant"
                                >
                                    {status}
                                </p>
                            }
                                .into_any()
                        }}
                        // the import outcome. A refusal lists EVERY reason and applied
                        // nothing, so there is no half-applied state to explain and no "partially
                        // imported" wording anywhere in it. An acceptance prints what landed.
                        {move || {
                            let refusals = import_refusals.get();
                            if !refusals.is_empty() {
                                let n = refusals.len() - 1; // the lead line is not a reason
                                return view! {
                                    <div
                                        data-import-refused=n.to_string()
                                        class="rounded-lg border border-error-alert/40 bg-error/10 p-2 text-label-sm normal-case text-error-alert"
                                    >
                                        <ul class="flex list-none flex-col gap-1">
                                            {refusals
                                                .into_iter()
                                                .map(|m| view! { <li>{m}</li> })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    </div>
                                }
                                    .into_any();
                            }
                            let status = import_status.get();
                            if status.is_empty() {
                                return ().into_any();
                            }
                            view! {
                                <p
                                    data-import-status
                                    class="text-label-sm normal-case text-on-surface-variant"
                                >
                                    {status}
                                </p>
                            }
                                .into_any()
                        }}
                        // the persistence contract, said in the panel. The platform's one
                        // "not saved yet" signal is the `•` beside the mission title, and this tab
                        // renders under a full-viewport blur scrim that dims exactly that. So the
                        // Arsenal repeats it here rather than leaving the author to guess whether a
                        // pick stuck. `data-arsenal-persist` carries the state for the gate harness.
                        // `refused` is checked FIRST and wins outright. The dirty flag is
                        // a mission-wide fact and stays perfectly accurate during a refusal; it is
                        // just the wrong question, so it must not get to answer.
                        {move || {
                            commits.track();
                            let refused = persist_refused.get();
                            let unsaved = mission_has_unsaved_work();
                            let (marker, cls, state) = if refused {
                                (
                                    "refused",
                                    "flex items-start gap-1.5 text-label-sm normal-case text-error",
                                    PERSIST_REFUSED,
                                )
                            } else if unsaved {
                                (
                                    "unsaved",
                                    "flex items-start gap-1.5 text-label-sm normal-case text-tactical-yellow",
                                    PERSIST_UNSAVED,
                                )
                            } else {
                                (
                                    "saved",
                                    "flex items-start gap-1.5 text-label-sm normal-case text-outline",
                                    PERSIST_CLEAN,
                                )
                            };
                            // The unconditional "every pick is written the moment you make it"
                            // promise is dropped on the refused branch: repeating it beside a
                            // refusal would be the contradiction the document write result.
                            let lead = if refused { "" } else { PERSIST_ALWAYS };
                            view! {
                                <p data-arsenal-persist=marker class=cls>
                                    <span class="material-symbols-outlined shrink-0 text-[14px]">
                                        {if refused {
                                            "error"
                                        } else if unsaved {
                                            "cloud_upload"
                                        } else {
                                            "check_circle"
                                        }}
                                    </span>
                                    <span>{lead} {if refused { "" } else { " " }} {state}</span>
                                </p>
                            }
                        }}
                        <p class="text-label-sm normal-case text-outline">
                            "Weapon attachments are multi-select in the compat panel — pick a weapon region on the rail to see what it accepts. Container cargo (mags, medical, throwables) lives in the Cargo panel above — seeded from the character's engine defaults. Dedicated equipment wear rows (binoculars, radios, glasses) come with the equipment slice."
                        </p>
    }
}
