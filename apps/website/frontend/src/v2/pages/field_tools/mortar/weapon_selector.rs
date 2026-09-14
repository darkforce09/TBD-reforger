//! The two dropdowns at the head of the inputs card: the tube, and the operation to save to.
//!
//! **Role:** offers the weapon systems the solver accepts and the operation a solution is
//! persisted against, and writes the operation choice through to the stored preference.
//! **Position:** the first two controls of the inputs card on `/tools/mortar`, ahead of the four
//! coordinate fields.
//! **Signals & state:** reads and writes the `weapon` and `event_id` signals the page owns, and
//! reads the operation list resource; picking an operation also writes the browser preference
//! through [`write_event_pref`](super::saved_fires::write_event_pref).
//! **Invariants:** the operation list is rebuilt once, when the fetch lands, so the freshly
//! rendered options include the one restored from the stored preference; `prop:value` is its own
//! reactive binding, so a later pick updates the selection without rebuilding the node.

use super::map_picker::INPUT_CLASS;
use super::saved_fires::{write_event_pref, EventOption};
use leptos::prelude::*;

/// The weapon keys the solve and save routes accept.
///
/// **This is a second copy of a server-side table** — `charges_for` in
/// `api/src/services/mortar.rs` — and there is no endpoint that lists them, so a copy is the only
/// way to offer a choice at all. The drift is asymmetric, which is why it is tolerable: a tube
/// added there and missing here is merely unofferable, while one here and not there is a **400
/// `unknown weapon_system '…'`** the operator sees immediately. The API refuses an unknown weapon
/// outright rather than substituting one, so the dangerous direction — 81mm numbers labelled as a
/// 120mm tube — is closed on the server and cannot be reopened from here.
pub(super) const WEAPONS: [&str; 4] = ["M252 81mm", "M821 81mm", "2B14 82mm", "M120 120mm"];

/// The tube picker: a `<label>` wrapping a `<select>` of [`WEAPONS`], bound to the page's `weapon`
/// signal.
pub(super) fn weapon_select(weapon: RwSignal<String>) -> impl IntoView {
    view! {
        <label class="text-sm">
            "Weapon"
            <select
                prop:value=move || weapon.get()
                on:change=move |ev| weapon.set(event_target_value(&ev))
                class=INPUT_CLASS
            >
                {WEAPONS
                    .iter()
                    .map(|w| view! { <option value=*w>{*w}</option> })
                    .collect_view()}
            </select>
        </label>
    }
}

/// The operation picker: the schedule, plus a "none" option that means the solution is computed
/// but not kept.
///
/// Takes the operation list resource rather than a plain list so the `<option>` set is rebuilt
/// exactly once, when the fetch lands — which is what lets an id restored from the stored
/// preference match an option that exists by the time the value is applied.
pub(super) fn operation_select(
    events: LocalResource<Option<Vec<EventOption>>>,
    event_id: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
        let rows = events.get().flatten().unwrap_or_default();
        view! {
            <label class="text-sm">
                "Operation"
                <select
                    prop:value=move || event_id.get().unwrap_or_default()
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        let v = (!v.is_empty()).then_some(v);
                        write_event_pref(v.as_deref());
                        event_id.set(v);
                    }
                    class=INPUT_CLASS
                >
                    <option value="">"— none (not saved) —"</option>
                    {rows
                        .into_iter()
                        .map(|e| {
                            let label = format!(
                                "{} — {}",
                                e.name(),
                                crate::v2::core::utils::datefmt::format_short_date(&e.start_time),
                            );
                            view! { <option value=e.id.clone()>{label}</option> }
                        })
                        .collect_view()}
                </select>
            </label>
        }
    }
}
