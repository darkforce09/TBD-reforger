//! Persisting a firing solution against an operation, and reading it back.
//!
//! **Role:** owns the wire types for a stored fire mission, the request body the page posts, the
//! rules for turning a stored row back into a populated form, the browser preference that
//! remembers which operation was last worked, and the list panel those rows render in.
//! **Position:** the saved-fire-missions card floats over the bottom-left corner of the map panel
//! on `/tools/mortar`; everything else here sits behind the page's effects.
//! **Signals & state:** the list panel reads the page's `event_id` signal and the fire-mission
//! resource; [`read_event_pref`] and [`write_event_pref`] read and write `localStorage` under
//! [`EVENT_PREF_KEY`], and exist only on `wasm32` — the native build has no `window`.
//! **Invariants:** a fetched batch is tagged with the operation it was fetched for, and nothing
//! acts on a batch whose tag is not the selected operation; hydration happens at most once per
//! operation; a row that carries no coordinates restores as nothing rather than as the origin.

use super::grid::{fmt_grid, locale_int, parse_grid};
use crate::v2::core::api::dto::FireSolution;
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

/// The saved-list card's class: a glass panel pinned to the opposite corner from the solution
/// card, capped in height and scrollable — an operation accumulates fire missions and this panel
/// must never grow over the map.
const CARD_SAVED: &str = "flex flex-col gap-2 overflow-hidden rounded-xl p-4 glass absolute bottom-4 left-4 w-72 max-h-[55%] border-t-2 border-primary";

/// The `localStorage` key holding the operation the operator last saved to.
///
/// Without it the round trip only closes for whoever wants the *first* operation in the list: save
/// to the third one, reload, and the page would show a different operation's fire missions and
/// none of the operator's own.
pub(super) const EVENT_PREF_KEY: &str = "tbd-mortar-event";

/// One `fire_missions` row, as the per-operation list route returns it.
///
/// **Typed, not `serde_json::Value`, and with no `#[serde(default)]` on anything the backend marks
/// required.** A `Value` read through `.get("distance_m").and_then(as_i64).unwrap_or(0)` renders a
/// confident `0 m` when a field is renamed and the page keeps working; here a renamed column fails
/// the decode and the list goes to its error state instead.
///
/// `event_id` is genuinely absent for a fire mission saved with no operation, so it needs the
/// default. The seven columns that were added later are `Option` *and* defaulted, and they need
/// both: `Option` because the column is nullable, and defaulted because a response captured before
/// those columns existed is the one thing that can prove such a row still decodes. The relaxation
/// is narrow — every field the backend marks required stays required here.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct SavedFire {
    pub(super) id: String,
    #[serde(default)]
    pub(super) event_id: Option<String>,
    pub(super) created_by: String,
    pub(super) weapon_system: String,
    pub(super) fp_grid: String,
    pub(super) target_grid: String,
    pub(super) distance_m: i64,
    pub(super) azimuth_deg: f64,
    pub(super) elevation_mils: i64,
    /// The four coordinates, as real numbers. `None` for a row written before the columns existed,
    /// whose only record of them is the [`fmt_grid`] encoding in the grid strings.
    #[serde(default)]
    pub(super) fp_x: Option<f64>,
    #[serde(default)]
    pub(super) fp_y: Option<f64>,
    #[serde(default)]
    pub(super) tgt_x: Option<f64>,
    #[serde(default)]
    pub(super) tgt_y: Option<f64>,
    #[serde(default)]
    pub(super) azimuth_mils: Option<i64>,
    #[serde(default)]
    pub(super) charge: Option<i64>,
    #[serde(default)]
    pub(super) time_of_flight_s: Option<f64>,
    pub(super) created_at: String,
}

/// The 201 body of a save: the live solution and the row it was written as.
///
/// Both halves are used. `solution` is the full-fidelity answer that populates the card, time of
/// flight included, which no later read of the row can reproduce; `fire_mission.created_at` is
/// what lets the card claim "Saved" on the authority of a row that exists rather than on the
/// authority of a 2xx.
///
/// No `Debug`: the fire-solution DTO deliberately does not derive it, and adding one there is not
/// this page's edit to make.
#[derive(Clone, PartialEq, Deserialize)]
pub(super) struct SaveResponse {
    pub(super) solution: FireSolution,
    pub(super) fire_mission: SavedFire,
}

/// A fetched batch of fire missions, tagged with the operation it was fetched for.
///
/// **The tag is not bookkeeping — without it the load-time hydration is a race it loses.**
/// `LocalResource` keeps serving its previous value while the next key is in flight, so the
/// instant the operator picks an operation the hydration effect sees `{new operation, old
/// operation's rows}`. Measured in a real browser against the live API: a cold session picked its
/// operation, the once-per-operation latch fired against the empty list belonging to *no*
/// operation, and the real rows that landed 40 ms later were then correctly ignored as
/// already-hydrated. The page showed its empty prompt over a saved fire mission it had in hand.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SavedFor {
    pub(super) event_id: Option<String>,
    pub(super) rows: Vec<SavedFire>,
}

/// One scheduled operation, narrowed to what the picker needs.
///
/// Deliberately *not* [`crate::v2::core::api::dto::EventListItem`]: that DTO models the whole
/// schedule card — `percent`, `filled`, `total_slots`, `mission_count` — and every one of those is
/// a field this dropdown would fail to decode over for no reason. Three fields is the honest
/// dependency.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(super) struct EventOption {
    pub(super) id: String,
    #[serde(default)]
    pub(super) name_override: Option<String>,
    pub(super) start_time: String,
}

impl EventOption {
    /// The operation's display name, falling back to a placeholder: `name_override` is nullable,
    /// and an option labelled with the empty string is an option nobody can pick on purpose.
    ///
    /// Deliberately **not** including the date. The date formatter is `js_sys::Date` all the way
    /// down and aborts the native test binary, so folding the date in here would make this whole
    /// struct untestable by `cargo test`; the view composes the two instead.
    pub(super) fn name(&self) -> &str {
        match self.name_override.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => n,
            _ => "Untitled Operation",
        }
    }
}

/// What the solution card is showing.
///
/// Not the fire-solution DTO directly, because the two sources of a solution do not carry the same
/// guarantees. A live solve always has a charge and a time of flight; a stored row has them only
/// if it was written after those columns were added. Modelling that as `Option` forces the card to
/// say so rather than print a fabricated `0.0 s` or charge `0` — numbers indistinguishable from a
/// real zero-second flight and a real charge-zero ring.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Shown {
    pub(super) weapon_system: String,
    pub(super) distance_m: i64,
    pub(super) azimuth_deg: f64,
    pub(super) elevation_mils: i64,
    /// The propellant ring. `None` only for a row written before the column existed.
    pub(super) charge: Option<i64>,
    /// `None` only for a row written before the column existed — such a row genuinely has no
    /// time of flight.
    pub(super) time_of_flight_s: Option<f64>,
    /// `Some(created_at)` once these numbers exist in the database; `None` for a solve that was
    /// computed with no operation selected and will not outlive the tab.
    pub(super) saved_at: Option<String>,
}
impl From<&FireSolution> for Shown {
    fn from(s: &FireSolution) -> Self {
        Self {
            weapon_system: s.weapon_system.clone(),
            distance_m: s.distance_m,
            azimuth_deg: s.azimuth_deg,
            elevation_mils: s.elevation_mils,
            charge: Some(s.charge),
            time_of_flight_s: Some(s.time_of_flight_s),
            saved_at: None,
        }
    }
}

/// A stored row unpacked back into the four inputs plus the card.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Restored {
    pub(super) fp: (f64, f64),
    pub(super) tgt: (f64, f64),
    pub(super) shown: Shown,
}

/// The body a save or a solve is posted with — the same five fields either way, since the save
/// input flattens the solve input.
///
/// `event_id` is **omitted** rather than sent as `null` or `""` when there is no operation: the
/// handler refuses a blank string with a 400 by design, because a present-but-unparseable id used
/// to be silently demoted to null and the row then became invisible to the only route that lists
/// fire missions.
pub(super) fn save_body(
    weapon: &str,
    fp: (f64, f64),
    tgt: (f64, f64),
    event_id: Option<&str>,
) -> Value {
    let mut body = json!({
        "weapon_system": weapon,
        "fp_x": fp.0,
        "fp_y": fp.1,
        "tgt_x": tgt.0,
        "tgt_y": tgt.1,
        "fp_grid": fmt_grid(fp.0, fp.1),
        "target_grid": fmt_grid(tgt.0, tgt.1),
    });
    if let Some(id) = event_id.map(str::trim).filter(|s| !s.is_empty()) {
        body["event_id"] = json!(id);
    }
    body
}

/// Unpack a stored row back into the four inputs and the solution card.
///
/// `None` when the row has neither real coordinates nor a grid string this page wrote — the
/// numbers in the row are still real, but without coordinates there is nothing to put in the
/// fire-position and target fields, and half-restoring would leave the inputs saying one thing and
/// the card another.
///
/// The numeric columns are authoritative and are read first. The [`parse_grid`] fallback is **not**
/// belt-and-braces and must not be deleted as dead: every fire mission saved before those columns
/// existed has null in all four, and the text encoding is the only place its coordinates live.
/// Deleting the fallback would not throw — it would quietly stop restoring every historical row,
/// which reads to the operator as "nothing was ever saved" over rows sitting right there in the
/// list. The two pairs fall back independently, because the backfill filled them independently.
///
/// Everything else comes off the row and stays `Option`: an older row has no charge and no time of
/// flight, and the card is required to say so rather than invent one.
pub(super) fn restore(row: &SavedFire) -> Option<Restored> {
    let fp = match (row.fp_x, row.fp_y) {
        (Some(x), Some(y)) => (x, y),
        _ => parse_grid(&row.fp_grid)?,
    };
    let tgt = match (row.tgt_x, row.tgt_y) {
        (Some(x), Some(y)) => (x, y),
        _ => parse_grid(&row.target_grid)?,
    };
    Some(Restored {
        fp,
        tgt,
        shown: Shown {
            weapon_system: row.weapon_system.clone(),
            distance_m: row.distance_m,
            azimuth_deg: row.azimuth_deg,
            elevation_mils: row.elevation_mils,
            charge: row.charge,
            time_of_flight_s: row.time_of_flight_s,
            saved_at: Some(row.created_at.clone()),
        },
    })
}

/// Decide what the load-time hydration should do with a fetched batch.
///
/// Pure, and separate from the effect that drives it, because the two ways this goes wrong are
/// both invisible from a rendered page:
///
/// * **acting on a stale batch** — the [`SavedFor`] race above, which reads as "nothing was ever
///   saved" while the rows sit one tick away;
/// * **acting more than once** — the refetch after a save re-runs the effect, and re-hydrating
///   there would replace the fresh full-fidelity card with the row that was just written, and
///   would yank coordinates out from under anyone mid-edit.
///
/// `None` means do nothing at all. `Some(restored)` means latch this operation as hydrated and
/// apply `restored` — itself `None` when the operation has nothing saved yet, or when the newest
/// row's grids are not this page's encoding. Latching over an empty operation is deliberate: it is
/// a real answer, and re-asking on every refetch is how a later save gets clobbered.
///
/// `already_hydrated` is a set, not the last operation seen. A single slot made "once per
/// operation" true only for the most recent one: switching away from an operation and back
/// re-hydrated it, silently replacing whatever had been typed since. Nothing looked broken — the
/// in-progress edit just vanished. A latch that forgets is not a latch.
pub(super) fn hydration_step(
    batch: &SavedFor,
    want: &str,
    already_hydrated: &HashSet<String>,
) -> Option<Option<Restored>> {
    if batch.event_id.as_deref() != Some(want) {
        return None;
    }
    if already_hydrated.contains(want) {
        return None;
    }
    Some(batch.rows.last().and_then(restore))
}

/// The operation the operator last saved to. Wasm-only; the native test build has no `window`.
#[cfg(target_arch = "wasm32")]
pub(super) fn read_event_pref() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(EVENT_PREF_KEY)
        .ok()?
        .filter(|s| !s.trim().is_empty())
}

/// Native builds have no `localStorage`, so there is no remembered operation to read.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read_event_pref() -> Option<String> {
    None
}

/// Remember the operation the operator is saving to, or forget it when they pick none. A blank id
/// clears the key rather than storing an empty string, so the next read cannot restore a selection
/// nobody made.
#[cfg(target_arch = "wasm32")]
pub(super) fn write_event_pref(id: Option<&str>) {
    if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
        match id {
            Some(id) if !id.trim().is_empty() => {
                let _ = storage.set_item(EVENT_PREF_KEY, id);
            }
            _ => {
                let _ = storage.remove_item(EVENT_PREF_KEY);
            }
        }
    }
}

/// Native builds have no `localStorage`, so there is nothing to remember the operation in.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write_event_pref(_id: Option<&str>) {}

/// The saved-fire-missions panel: every fire mission stored against the selected operation,
/// newest first, each one a button that loads it back into the form.
///
/// Takes the resource rather than a resolved list because the panel has to distinguish three
/// states the page cannot flatten for it — not yet fetched, fetch failed, and a batch belonging to
/// the previously selected operation.
pub(super) fn saved_list(
    saved: LocalResource<Option<SavedFor>>,
    event_id: RwSignal<Option<String>>,
    load_row: impl Fn(SavedFire) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class=CARD_SAVED>
            <h2 class="text-sm font-semibold text-primary">"Saved Fire Missions"</h2>
            {move || {
                // `LocalResource::get()` is `Option<Option<SavedFor>>`: the outer layer
                // is "the fetch has not resolved", the inner one is this module's own
                // "the fetch failed". Collapsing them would render an empty list over
                // a dead endpoint. A batch whose tag is not the selected operation is
                // the previous key's value still being served — "Loading…", never
                // another operation's gun line (see `SavedFor`).
                let batch = saved.get();
                let stale = matches!(
                    &batch,
                    Some(Some(b)) if b.event_id != event_id.get()
                );
                match batch {
                    _ if stale => {
                        view! {
                            <p class="text-xs text-on-surface-variant">"Loading…"</p>
                        }
                            .into_any()
                    }
                    None => {
                        view! {
                            <p class="text-xs text-on-surface-variant">"Loading…"</p>
                        }
                            .into_any()
                    }
                    Some(None) => {
                        view! {
                            <p class="text-xs text-error">
                                "Could not load saved fire missions."
                            </p>
                        }
                            .into_any()
                    }
                    Some(Some(SavedFor { rows, .. })) if rows.is_empty() => {
                        view! {
                            <p class="text-xs text-on-surface-variant">
                                {move || {
                                    if event_id.get().is_some() {
                                        "Nothing saved on this operation yet."
                                    } else {
                                        "Pick an operation to save and reload solutions."
                                    }
                                }}
                            </p>
                        }
                            .into_any()
                    }
                    Some(Some(SavedFor { rows, .. })) => {
                        let rows: Vec<SavedFire> = rows.iter().rev().cloned().collect();
                        view! {
                            <ul class="flex min-h-0 flex-col gap-1 overflow-y-auto font-mono text-xs">
                                {rows
                                    .into_iter()
                                    .map(|row| {
                                        let click = row.clone();
                                        view! {
                                            <li>
                                                <button
                                                    type="button"
                                                    on:click=move |_| load_row(click.clone())
                                                    class="w-full rounded px-2 py-1 text-left hover:bg-surface-variant/60"
                                                >
                                                    <span class="text-on-surface">
                                                        {row.fp_grid.clone()} " → " {row.target_grid.clone()}
                                                    </span>
                                                    <span class="block text-on-surface-variant">
                                                        {locale_int(row.distance_m as f64)} " m · "
                                                        {format!("{:.1}°", row.azimuth_deg)} " · "
                                                        {row.elevation_mils} " mils"
                                                    </span>
                                                </button>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }
            }}
        </div>
    }
}
