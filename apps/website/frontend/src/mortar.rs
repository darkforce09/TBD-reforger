//! Mortar Calculator (/tools/mortar) — ported from pages/doctrine.tsx `MortarCalculatorPage`.
//! `<AuthGate>` → a self-contained firing-solution form (FP/TGT grid inputs + Calculate button + a
//! tactical map preview with the solution panel).
//!
//! T-159.25: live — inputs are signals and Calculate POSTs `/fire-missions/solve`
//! (useSolveFireMission port); the solution card renders the returned distance/azimuth/elevation/
//! TOF.
//!
//! **T-285 — the solution now survives a reload.** Three defects, all measured on `main` first:
//!
//! 1. **`POST /fire-missions` and `GET /events/{id}/fire-missions` had no caller.** Both exist
//!    server-side (`api/src/app.rs:826` / `:828-829` → `handlers/field_tools.rs:216` / `:281`) and
//!    have since T-145; this page was the only fire-mission caller in the SPA and it only ever hit
//!    `/fire-missions/solve`, whose result lived in an `RwSignal` and nowhere else. Every computed
//!    solution died with the tab. This slice wires the two orphans: with an operation selected,
//!    Calculate goes to `POST /fire-missions` — which computes *and* persists in one round trip —
//!    and the page hydrates itself from `GET /events/{id}/fire-missions` on load.
//! 2. **The map preview was fixed CSS.** The two markers were `top-1/4 left-1/3` and
//!    `top-1/2 left-2/3` string literals — no signal read anywhere in the subtree, so they sat
//!    still while the operator retyped every coordinate. They are now projected from the inputs
//!    (see [`preview_pos`]).
//! 3. **`weapon_system` was hardcoded** to `"M252 81mm"` in the request body while the heading
//!    rendered the *returned* system — a claim that could not be false, because since T-365 the
//!    API echoes back the weapon it was asked for (`services/mortar.rs`). It is now a real choice
//!    and the heading is a real readout.
//!
//! **What persistence can and cannot restore.** `fire_missions` (`api/src/models/admin.rs:66-80`)
//! stores `weapon_system`, `fp_grid`, `target_grid`, `distance_m`, `azimuth_deg` and
//! `elevation_mils` — and **no** `time_of_flight_s`, `charge` or `azimuth_mils`, and no numeric
//! coordinates at all. So: a freshly-saved solution shows the full card, because the `POST`
//! *response* carries the live `FireSolution`; a solution restored after a reload shows TOF as
//! `—`, because the row genuinely does not have one. The FP/TGT numbers come back because this
//! module encodes them into the one free-text field the table has for them ([`fmt_grid`]) — that
//! encoding IS the persistence of the operator's inputs, which is why it has a round-trip test.
#![allow(dead_code)]
use crate::dto::{DataEnvelope, FireSolution, Paginated};
use crate::ui::{AuthGate, PageHeader};
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

// OpsCard cn(base,'glass',className) results, tailwind-merged (deferred Rust tw_merge):
//  · inputs card: className "grid …" → grid beats base `flex` (display), `gap-4` beats `gap-3`.
//    T-285 widened this from four controls to six (weapon + operation), so the wide breakpoint is
//    `lg:grid-cols-3` (two rows of three) rather than `lg:grid-cols-4` (one row plus two orphans).
const CARD_INPUTS: &str = "relative flex-col overflow-hidden rounded-xl p-6 glass grid gap-4 sm:grid-cols-2 lg:grid-cols-3";
//  · solution card: className "absolute …" → absolute beats base `relative` (position).
const CARD_SOLUTION: &str = "flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass absolute right-4 bottom-4 w-72 border-t-2 border-tertiary";
//  · saved-list card: mirrors the solution card on the opposite corner. Capped and scrollable —
//    an operation accumulates fire missions and this panel must never grow over the map.
const CARD_SAVED: &str = "flex flex-col gap-2 overflow-hidden rounded-xl p-4 glass absolute bottom-4 left-4 w-72 max-h-[55%] border-t-2 border-primary";
const INPUT_CLASS: &str =
    "mt-1 w-full rounded-lg border border-border-subtle bg-surface px-3 py-2 text-sm";

/// The weapon keys `POST /fire-missions` and `/fire-missions/solve` accept.
///
/// **This is a second copy of a server-side table** — `charges_for` in
/// `api/src/services/mortar.rs` — and there is no endpoint that lists them, so a copy is the only
/// way to offer a choice at all. The drift is asymmetric, which is why it is tolerable: a tube
/// added there and missing here is merely unofferable, while one here and not there is a **400
/// `unknown weapon_system '…'`** the operator sees immediately. Since T-365 the API refuses an
/// unknown weapon outright rather than substituting one, so the dangerous direction — 81mm numbers
/// labelled as a 120mm tube — is closed on the server and cannot be reopened from here.
const WEAPONS: [&str; 4] = ["M252 81mm", "M821 81mm", "2B14 82mm", "M120 120mm"];

/// `localStorage` key holding the operation the operator last saved to.
///
/// Without it the round trip only closes for whoever happens to want the *first* operation in the
/// list: save to the third one, reload, and the page would helpfully show you a different
/// operation's fire missions and none of your own. That is the "renders a solution" failure the
/// T-285 brief names — a page that looks alive over data that is not the operator's.
const EVENT_PREF_KEY: &str = "tbd-mortar-event";

/// One row of `fire_missions`, as `GET /events/{id}/fire-missions` returns it — mirrors
/// `api/src/models/admin.rs::FireMission`.
///
/// **Typed, not `serde_json::Value`, and with no `#[serde(default)]` on anything the backend marks
/// required.** A `Value` read via `.get("distance_m").and_then(as_i64).unwrap_or(0)` renders a
/// confident `0 m` when the field is renamed, and the page keeps working — which is exactly the
/// "reports success over an input it never examined" shape this program keeps finding. Here a
/// renamed column fails the decode and the list goes to its error state instead.
///
/// `event_id` carries `skip_serializing_if = "Option::is_none"` on the model, so it is genuinely
/// absent for a fire mission saved with no event and needs the default; every other field is
/// unconditional on the wire.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct SavedFire {
    id: String,
    #[serde(default)]
    event_id: Option<String>,
    created_by: String,
    weapon_system: String,
    fp_grid: String,
    target_grid: String,
    distance_m: i64,
    azimuth_deg: f64,
    elevation_mils: i64,
    created_at: String,
}

/// `POST /fire-missions` 201 body — `{solution, fire_mission}` (`field_tools.rs:272-275`).
///
/// Both halves are used: `solution` is the live full-fidelity answer that populates the card (TOF
/// included, which no later read of the row can produce), and `fire_mission.created_at` is what
/// lets the card say "Saved" on the authority of a row that exists rather than on the authority of
/// a 2xx.
// No `Debug`: `dto::FireSolution` deliberately does not derive it (its siblings do not either),
// and adding one there is a `dto.rs` edit this slice does not own.
#[derive(Clone, PartialEq, Deserialize)]
struct SaveResponse {
    solution: FireSolution,
    fire_mission: SavedFire,
}

/// One `/events` row, narrowed to what the operation picker needs.
///
/// Deliberately *not* [`crate::dto::EventListItem`]: that DTO models the whole schedule card
/// (`percent`, `filled`, `total_slots`, `mission_count`, …) and every one of those is a field this
/// dropdown would fail to decode over for no reason. Three fields is the honest dependency.
#[derive(Clone, Debug, PartialEq, Deserialize)]
struct EventOption {
    id: String,
    #[serde(default)]
    name_override: Option<String>,
    start_time: String,
}

impl EventOption {
    /// The operation's display name — `events.rs`'s "Untitled Operation" fallback, for the same
    /// reason: `name_override` is nullable and an option labelled with the empty string is an
    /// option nobody can pick on purpose.
    ///
    /// Deliberately **not** including the date: `crate::datefmt` is `js_sys::Date` all the way
    /// down and aborts the native test binary ("function not implemented on non-wasm32 targets"),
    /// so folding the date in here would make this whole struct untestable by `cargo test`. The
    /// view composes the two.
    fn name(&self) -> &str {
        match self.name_override.as_deref().map(str::trim) {
            Some(n) if !n.is_empty() => n,
            _ => "Untitled Operation",
        }
    }
}

/// What the solution card is showing.
///
/// Not `FireSolution` directly, because the two sources of a solution do not carry the same
/// fields. A live solve has all seven; a row read back out of `fire_missions` has four, and
/// `time_of_flight_s` is not one of them. Modelling that as `Option` forces the card to say `—`
/// rather than print a fabricated `0.0 s`, which would be indistinguishable from a real
/// zero-second flight and is the same defect class as the `Value`-typed decode above.
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    weapon_system: String,
    distance_m: i64,
    azimuth_deg: f64,
    elevation_mils: i64,
    /// `None` for a row restored from the database — `fire_missions` has no TOF column.
    time_of_flight_s: Option<f64>,
    /// `Some(created_at)` once these numbers exist in the database; `None` for a solve that was
    /// computed with no operation selected and will not outlive the tab.
    saved_at: Option<String>,
}

impl From<&FireSolution> for Shown {
    fn from(s: &FireSolution) -> Self {
        Self {
            weapon_system: s.weapon_system.clone(),
            distance_m: s.distance_m,
            azimuth_deg: s.azimuth_deg,
            elevation_mils: s.elevation_mils,
            time_of_flight_s: Some(s.time_of_flight_s),
            saved_at: None,
        }
    }
}

/// A saved row unpacked back into the four inputs plus the card.
#[derive(Clone, Debug, PartialEq)]
struct Restored {
    fp: (f64, f64),
    tgt: (f64, f64),
    shown: Shown,
}

/// `Math.round(n).toLocaleString()` — default-locale thousands separators (comma).
fn locale_int(n: f64) -> String {
    let v = n.round() as i64;
    let neg = v < 0;
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        let rem = digits.len() - i;
        out.push(c);
        if rem > 1 && (rem - 1) % 3 == 0 {
            out.push(',');
        }
    }
    if neg {
        format!("-{out}")
    } else {
        out
    }
}

/// Render an FP/TGT pair into the one free-text field `fire_missions` has for it.
///
/// **`fire_missions` stores no coordinates.** The insert at `field_tools.rs:255-271` writes
/// `fp_grid` and `target_grid` and nothing else positional, so this string is the *only* place the
/// operator's four numbers survive a reload. That makes the encoding load-bearing and it has to be
/// lossless: a six-figure military grid ("012 020") would quantise 2200.4 to the nearest hundred
/// metres and hand back a target nobody aimed at.
///
/// `f64::to_string` prints `1000` for `1000.0` and `2200.5` for `2200.5`, so a whole-metre grid
/// reads exactly as the operator typed it and a fractional one is not truncated. The handler trims
/// the value before storing (`field_tools.rs:265-266`) and this format has no leading or trailing
/// space, so what comes back is byte-identical to what went out — see `parse_grid`'s round-trip
/// test.
fn fmt_grid(x: f64, y: f64) -> String {
    format!("{x}, {y}")
}

/// The inverse of [`fmt_grid`]. `None` for anything this module did not write — a grid typed by
/// hand into some other client, say — which restores as "no coordinates" rather than as `(0, 0)`.
fn parse_grid(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once(',')?;
    let x: f64 = a.trim().parse().ok()?;
    let y: f64 = b.trim().parse().ok()?;
    (x.is_finite() && y.is_finite()).then_some((x, y))
}

/// The `POST /fire-missions` body (or `/fire-missions/solve`'s, which is the same five fields
/// flattened — `SaveFireInput` `#[serde(flatten)]`s `SolveInput`).
///
/// `event_id` is **omitted** rather than sent as `null` or `""` when there is no operation: the
/// handler refuses a blank string with a 400 by design (`field_tools.rs:246-254`), because a
/// present-but-unparseable id used to be silently demoted to NULL and the row then became
/// invisible to the only endpoint that lists fire missions.
fn save_body(weapon: &str, fp: (f64, f64), tgt: (f64, f64), event_id: Option<&str>) -> Value {
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

/// Unpack a saved row back into the four inputs and the solution card.
///
/// `None` when the grids are not this module's encoding — the numbers in the row are still real,
/// but without coordinates there is nothing to put in the FP/TGT inputs, and half-restoring would
/// leave the inputs saying one thing and the card another.
fn restore(row: &SavedFire) -> Option<Restored> {
    Some(Restored {
        fp: parse_grid(&row.fp_grid)?,
        tgt: parse_grid(&row.target_grid)?,
        shown: Shown {
            weapon_system: row.weapon_system.clone(),
            distance_m: row.distance_m,
            azimuth_deg: row.azimuth_deg,
            elevation_mils: row.elevation_mils,
            // Not stored. See the module note.
            time_of_flight_s: None,
            saved_at: Some(row.created_at.clone()),
        },
    })
}

/// Project FP and TGT into the preview box as `(left%, top%)` pairs.
///
/// **This is the whole of the claim-2 fix**, so it is a pure function with a test rather than
/// inline arithmetic in the view: the defect was a subtree that read no signal, and the only way
/// to prove that is fixed is to perturb an input and watch the output move.
///
/// The two points are *fitted* to the box rather than projected onto a terrain extent. There is no
/// terrain on this page — the four inputs are unbounded game-world metres with no mission and no
/// map behind them — so there is no absolute frame to project into, and a fitted frame has the
/// property that matters operationally: both markers are always on screen, at any separation.
///
/// The frame is square (so the gun-target line's bearing is not sheared), centred on the midpoint,
/// and 1.6× the larger span, which leaves the pair occupying the middle ~62% with margin for the
/// marker glyphs. `top` is inverted because north is +y on the map and −y in CSS.
fn preview_pos(fp: (f64, f64), tgt: (f64, f64)) -> ((f64, f64), (f64, f64)) {
    let mid = ((fp.0 + tgt.0) / 2.0, (fp.1 + tgt.1) / 2.0);
    let span = (fp.0 - tgt.0).abs().max((fp.1 - tgt.1).abs());
    // Coincident FP and TGT (or a non-finite input) has no scale to fit; both markers stack in the
    // centre, which is the truth — the gun is on the target.
    let side = if span.is_finite() && span > 0.0 {
        span * 1.6
    } else {
        return ((50.0, 50.0), (50.0, 50.0));
    };
    let place = |p: (f64, f64)| {
        (
            ((p.0 - (mid.0 - side / 2.0)) / side * 100.0).clamp(0.0, 100.0),
            (((mid.1 + side / 2.0) - p.1) / side * 100.0).clamp(0.0, 100.0),
        )
    };
    (place(fp), place(tgt))
}

/// The operation the operator last saved to. Wasm-only; the native test build has no `window`.
#[cfg(target_arch = "wasm32")]
fn read_event_pref() -> Option<String> {
    let storage = web_sys::window()?.local_storage().ok()??;
    storage
        .get_item(EVENT_PREF_KEY)
        .ok()?
        .filter(|s| !s.trim().is_empty())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_event_pref() -> Option<String> {
    None
}

#[cfg(target_arch = "wasm32")]
fn write_event_pref(id: Option<&str>) {
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

#[cfg(not(target_arch = "wasm32"))]
fn write_event_pref(_id: Option<&str>) {}

fn num_input(label: &'static str, sig: RwSignal<f64>) -> impl IntoView {
    view! {
        <label class="text-sm">
            {label}
            <input
                type="number"
                // React reflects the controlled value as an attribute at rest ("1000" etc.) — the
                // frozen V golden pins it; prop:value stays the live binding.
                value=move || sig.get().to_string()
                prop:value=move || sig.get().to_string()
                on:input=move |ev| sig.set(event_target_value(&ev).parse().unwrap_or(0.0))
                class=INPUT_CLASS
            />
        </label>
    }
}

#[component]
pub fn MortarCalculatorPage() -> impl IntoView {
    view! {
        <AuthGate>
            <MortarInner />
        </AuthGate>
    }
}

#[component]
fn MortarInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = &store;
    let fp_x = RwSignal::new(1000.0);
    let fp_y = RwSignal::new(2000.0);
    let tgt_x = RwSignal::new(2200.0);
    let tgt_y = RwSignal::new(1800.0);
    let weapon = RwSignal::new(WEAPONS[0].to_string());
    // Seeded from localStorage so a reload lands on the operation the operator was working, not on
    // whichever one sorts first.
    let event_id = RwSignal::new(read_event_pref());
    let solution = RwSignal::new(None::<Shown>);
    let busy = RwSignal::new(false);

    let events = LocalResource::new(move || async move {
        #[cfg(target_arch = "wasm32")]
        {
            crate::client::api_get::<Paginated<EventOption>>(store, "/events")
                .await
                .ok()
                .map(|p| p.data)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = store;
            None::<Vec<EventOption>>
        }
    });

    // The reload half of the round trip. Re-keys on the selected operation, so switching
    // operations swaps the history rather than merging two gun lines' work.
    let saved = LocalResource::new(move || {
        let ev = event_id.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                match ev {
                    Some(id) => crate::client::api_get::<DataEnvelope<SavedFire>>(
                        store,
                        &format!("/events/{id}/fire-missions"),
                    )
                    .await
                    .ok()
                    .map(|d| d.data),
                    None => Some(Vec::new()),
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, ev);
                None::<Vec<SavedFire>>
            }
        }
    });

    // Reconcile the remembered operation against the live schedule.
    //
    // **`fire_missions.event_id` has no foreign key** — T-262 abstained on it deliberately, because
    // there is no 23503 handler and a 500 on an ingest path loses data. So a stale id out of
    // `localStorage` is not rejected by anything: `POST /fire-missions` would happily write the row
    // against an operation that no longer exists, and `GET /events/{id}/fire-missions` would
    // happily return `{"data":[]}` for it. The fire mission would be saved, reported saved, and
    // unreachable — the exact failure this ticket exists to close, reintroduced through the back
    // door. Nothing validates the id for us, so this does.
    Effect::new(move |_| {
        let Some(Some(rows)) = events.get() else {
            return;
        };
        let Some(want) = event_id.get() else { return };
        if !rows.iter().any(|e| e.id == want) {
            write_event_pref(None);
            event_id.set(None);
        }
    });

    // Hydrate the card from the newest saved fire mission, ONCE per operation. The latch is what
    // stops the refetch after a save from overwriting the fresh full-fidelity card (which has a
    // TOF) with the row that was just written (which does not), and stops any later refetch from
    // yanking coordinates out from under someone mid-edit.
    let hydrated_for = StoredValue::new(None::<String>);
    Effect::new(move |_| {
        // Only latch on a fetch that actually answered. Latching on a failed load would make the
        // page's one hydration attempt the one that read nothing.
        let Some(Some(rows)) = saved.get() else {
            return;
        };
        let Some(ev) = event_id.get() else { return };
        if hydrated_for.get_value().as_deref() == Some(ev.as_str()) {
            return;
        }
        hydrated_for.set_value(Some(ev));
        let Some(row) = rows.last() else {
            return;
        };
        if let Some(r) = restore(row) {
            fp_x.set(r.fp.0);
            fp_y.set(r.fp.1);
            tgt_x.set(r.tgt.0);
            tgt_y.set(r.tgt.1);
            weapon.set(r.shown.weapon_system.clone());
            solution.set(Some(r.shown));
        }
    });

    // Load any saved fire mission back into the form — the newest one runs automatically on load,
    // the rest are one click away.
    let load_row = move |row: SavedFire| {
        if let Some(r) = restore(&row) {
            fp_x.set(r.fp.0);
            fp_y.set(r.fp.1);
            tgt_x.set(r.tgt.0);
            tgt_y.set(r.tgt.1);
            weapon.set(r.shown.weapon_system.clone());
            solution.set(Some(r.shown));
        }
    };

    let on_solve = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if busy.get_untracked() {
                return;
            }
            busy.set(true);
            let toasts = crate::toast::use_toasts();
            let ev = event_id.get_untracked();
            let body = save_body(
                &weapon.get_untracked(),
                (fp_x.get_untracked(), fp_y.get_untracked()),
                (tgt_x.get_untracked(), tgt_y.get_untracked()),
                ev.as_deref(),
            );
            leptos::task::spawn_local(async move {
                match ev {
                    // With an operation: ONE call that computes and persists
                    // (`POST /fire-missions`). Solving first and saving second would leave a
                    // window where the operator is looking at numbers that failed to save.
                    Some(_) => {
                        match crate::client::api_post::<SaveResponse>(store, "/fire-missions", body)
                            .await
                        {
                            Ok(r) => {
                                let mut shown = Shown::from(&r.solution);
                                shown.saved_at = Some(r.fire_mission.created_at.clone());
                                solution.set(Some(shown));
                                saved.refetch();
                                toasts.success("Firing solution saved to the operation");
                            }
                            Err(e) => toasts.error(crate::client::api_error_message(
                                &e,
                                "Could not compute firing solution",
                            )),
                        }
                    }
                    // No operation: solve only, and say so. There is no endpoint that persists a
                    // fire mission the operator can find again without one.
                    None => {
                        match crate::client::api_post::<FireSolution>(
                            store,
                            "/fire-missions/solve",
                            body,
                        )
                        .await
                        {
                            Ok(s) => {
                                solution.set(Some(Shown::from(&s)));
                                toasts
                                    .message("Not saved — pick an operation to keep this solution");
                            }
                            Err(e) => toasts.error(crate::client::api_error_message(
                                &e,
                                "Could not compute firing solution",
                            )),
                        }
                    }
                }
                busy.set(false);
            });
        }
    };

    view! {
        <div class="relative flex h-full w-full flex-col overflow-hidden">
            <div class="bg-topo-map bg-grid-overlay absolute inset-0 z-0"></div>
            <div class="relative z-10 flex h-full w-full flex-col gap-4 bg-surface-glass p-6 backdrop-blur-xl md:p-8">
                <PageHeader
                    title="Mortar Calculator"
                    subtitle="Enter grid coordinates, pick a tube, and save the solution to an operation."
                />
                <div class=CARD_INPUTS>
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
                    // Rebuilt once when the events land so the freshly-rendered <option> set
                    // includes the localStorage-seeded id; `prop:value` is its own reactive
                    // binding, so a later pick updates the value without rebuilding the node.
                    {move || {
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
                                                crate::datefmt::format_short_date(&e.start_time),
                                            );
                                            view! { <option value=e.id.clone()>{label}</option> }
                                        })
                                        .collect_view()}
                                </select>
                            </label>
                        }
                    }} {num_input("FP X", fp_x)} {num_input("FP Y", fp_y)}
                    {num_input("TGT X", tgt_x)} {num_input("TGT Y", tgt_y)}
                </div>
                <button
                    type="button"
                    on:click=on_solve
                    prop:disabled=move || busy.get()
                    class="self-start rounded-lg bg-primary px-4 py-2 text-sm font-medium text-on-primary disabled:opacity-50"
                >
                    {move || {
                        if busy.get() {
                            "Computing…".to_string()
                        } else if event_id.get().is_some() {
                            "Calculate & Save".to_string()
                        } else {
                            "Calculate Solution".to_string()
                        }
                    }}
                </button>
                <div class="relative min-h-0 flex-1 overflow-hidden rounded-xl border border-border-subtle bg-surface-container-lowest">
                    <div
                        class="absolute inset-0 opacity-30"
                        style="background-image: linear-gradient(rgba(59, 130, 246, 0.08) 1px, transparent 1px), linear-gradient(90deg, rgba(59, 130, 246, 0.08) 1px, transparent 1px); background-size: 40px 40px;"
                    ></div>
                    // Gun-target line. `preserveAspectRatio=none` puts the SVG on the same 0–100
                    // percentage frame as the two markers below, so the line lands on their
                    // centres at any box shape; `non-scaling-stroke` keeps it 1px anyway.
                    <svg
                        class="pointer-events-none absolute inset-0 h-full w-full"
                        viewBox="0 0 100 100"
                        preserveAspectRatio="none"
                    >
                        <line
                            x1=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).0.0
                            y1=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).0.1
                            x2=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).1.0
                            y2=move || preview_pos((fp_x.get(), fp_y.get()), (tgt_x.get(), tgt_y.get())).1.1
                            stroke="currentColor"
                            stroke-width="1"
                            stroke-dasharray="3 3"
                            vector-effect="non-scaling-stroke"
                            class="text-tertiary/60"
                        />
                    </svg>
                    <div
                        class="absolute h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-success bg-success/30"
                        style=move || {
                            let (fp, _) = preview_pos(
                                (fp_x.get(), fp_y.get()),
                                (tgt_x.get(), tgt_y.get()),
                            );
                            format!("left:{}%;top:{}%", fp.0, fp.1)
                        }
                        title="Fire Position"
                    ></div>
                    <div
                        class="absolute h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-error bg-error/30"
                        style=move || {
                            let (_, tgt) = preview_pos(
                                (fp_x.get(), fp_y.get()),
                                (tgt_x.get(), tgt_y.get()),
                            );
                            format!("left:{}%;top:{}%", tgt.0, tgt.1)
                        }
                        title="Target"
                    ></div>
                    <div class=CARD_SAVED>
                        <h2 class="text-sm font-semibold text-primary">"Saved Fire Missions"</h2>
                        {move || {
                            // `LocalResource::get()` is `Option<Option<Vec<_>>>`: the outer layer
                            // is "the fetch has not resolved", the inner one is this module's own
                            // "the fetch failed". Collapsing them would render an empty list over
                            // a dead endpoint.
                            match saved.get() {
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
                                Some(Some(rows)) if rows.is_empty() => {
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
                                Some(Some(rows)) => {
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
                    <div class=CARD_SOLUTION>
                        <h2 class="text-sm font-semibold text-primary">
                            "Firing Solution — "
                            {move || {
                                solution.get().map(|s| s.weapon_system).unwrap_or_else(|| weapon.get())
                            }}
                        </h2>
                        {move || match solution.get() {
                            Some(s) => {
                                view! {
                                    <p class=move || {
                                        if s.saved_at.is_some() {
                                            "text-xs text-success"
                                        } else {
                                            "text-xs text-tactical-yellow"
                                        }
                                    }>
                                        {if s.saved_at.is_some() {
                                            "Saved — survives a reload"
                                        } else {
                                            "Not saved — lost on reload"
                                        }}
                                    </p>
                                    <dl class="mt-3 space-y-2 font-mono text-sm">
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"Distance"</dt>
                                            <dd>{locale_int(s.distance_m as f64)} " m"</dd>
                                        </div>
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"Azimuth"</dt>
                                            <dd>{format!("{:.1}°", s.azimuth_deg)}</dd>
                                        </div>
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"Elevation"</dt>
                                            <dd class="text-primary">{s.elevation_mils} " mils"</dd>
                                        </div>
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"TOF"</dt>
                                            <dd
                                                title=move || {
                                                    if s.time_of_flight_s.is_some() {
                                                        ""
                                                    } else {
                                                        "fire_missions stores no time of flight — recalculate for it"
                                                    }
                                                }
                                            >
                                                {match s.time_of_flight_s {
                                                    Some(t) => format!("{t:.1} s"),
                                                    None => "—".to_string(),
                                                }}
                                            </dd>
                                        </div>
                                    </dl>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <p class="mt-3 text-xs text-on-surface-variant">
                                        "Enter coordinates and calculate to see solution."
                                    </p>
                                }
                                    .into_any()
                            }
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact body `GET /events/{id}/fire-missions` returned on the live dev API on
    /// 2026-07-31, for the fire mission posted by the round-trip probe in the T-285 report:
    /// FP (1000, 2000) → TGT (2200, 1800), `M252 81mm`. Captured verbatim — the field names, the
    /// envelope shape and the absence of `time_of_flight_s` are all server truth, not a guess.
    const LIVE_LIST: &str = r#"{"data":[{"id":"97176662-5589-4831-85e5-61f2a7bd8597","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"created_at":"2026-07-31T02:16:52.695935Z"}]}"#;

    /// **The round trip, over real server bytes.** Build the body the page posts, hand the grids
    /// it produced to the response the server actually gave back, and require the four inputs and
    /// the three persisted numbers to come out the far side unchanged.
    ///
    /// This is the assertion the T-285 brief asks for and it is not satisfiable by a page that
    /// renders: perturb `fmt_grid` (drop the separator, quantise to a six-figure grid, swap the
    /// axes) and the restored coordinates stop matching the posted ones while every view in this
    /// module still builds and still shows a solution.
    #[test]
    fn a_posted_solution_comes_back_from_the_server_with_the_same_numbers() {
        let fp = (1000.0, 2000.0);
        let tgt = (2200.0, 1800.0);
        let body = save_body(
            "M252 81mm",
            fp,
            tgt,
            Some("c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7"),
        );

        // What went out.
        assert_eq!(body["fp_grid"], "1000, 2000");
        assert_eq!(body["target_grid"], "2200, 1800");

        // What the server stored and handed back.
        let list: DataEnvelope<SavedFire> =
            serde_json::from_str(LIVE_LIST).expect("live list decodes");
        let row = list.data.last().expect("one saved fire mission");
        assert_eq!(row.fp_grid, body["fp_grid"].as_str().unwrap());
        assert_eq!(row.target_grid, body["target_grid"].as_str().unwrap());

        // What the page shows after a reload.
        let r = restore(row).expect("a row this module wrote restores");
        assert_eq!(r.fp, fp, "FP did not survive the round trip");
        assert_eq!(r.tgt, tgt, "TGT did not survive the round trip");
        assert_eq!(r.shown.weapon_system, "M252 81mm");
        assert_eq!(r.shown.distance_m, 1217);
        assert_eq!(r.shown.azimuth_deg, 99.5);
        assert_eq!(r.shown.elevation_mils, 1315);
        assert_eq!(
            r.shown.saved_at.as_deref(),
            Some("2026-07-31T02:16:52.695935Z")
        );
    }

    /// TOF is `None` on a restored row and the card must say so. `fire_missions` has no such
    /// column (`api/src/models/admin.rs:66-80`), and a `0.0` here would render `0.0 s` — a
    /// plausible, wrong, unfalsifiable number.
    #[test]
    fn a_restored_row_has_no_time_of_flight_because_the_table_has_no_column_for_it() {
        let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST).unwrap();
        let r = restore(&list.data[0]).unwrap();
        assert_eq!(r.shown.time_of_flight_s, None);
        // …while a live solve does carry one, so the two sources are distinguishable.
        let solved: FireSolution = serde_json::from_str(
            r#"{"weapon_system":"M252 81mm","distance_m":1217,"azimuth_deg":99.5,"azimuth_mils":1768,"elevation_mils":1315,"charge":2,"time_of_flight_s":29.4}"#,
        )
        .unwrap();
        assert_eq!(Shown::from(&solved).time_of_flight_s, Some(29.4));
    }

    /// The grid encoding is the persistence of the operator's inputs, so it has to be lossless
    /// over everything the number inputs can hold — negatives, fractions, zero and a coordinate
    /// far outside any terrain.
    #[test]
    fn every_coordinate_the_inputs_accept_round_trips_through_the_grid_string() {
        for (x, y) in [
            (0.0, 0.0),
            (1000.0, 2000.0),
            (2200.5, 1800.25),
            (-750.0, 12800.0),
            (0.1, -0.1),
            (123456.789, 987654.321),
        ] {
            let s = fmt_grid(x, y);
            assert_eq!(
                parse_grid(&s),
                Some((x, y)),
                "({x}, {y}) did not survive as {s:?}"
            );
            assert_eq!(s.trim(), s, "the handler trims before storing: {s:?}");
        }
    }

    /// A grid this module did not write restores as "no coordinates", never as `(0, 0)`.
    #[test]
    fn a_foreign_grid_reference_restores_as_nothing_rather_than_as_the_origin() {
        for s in ["012345", "", "1000", "AB, CD", "1000, ", "NaN, 3"] {
            assert_eq!(parse_grid(s), None, "{s:?} should not parse");
        }
        let mut row: SavedFire = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
            .unwrap()
            .data
            .remove(0);
        row.fp_grid = "012345".into();
        assert_eq!(restore(&row), None);
    }

    /// `POST /fire-missions` requires all seven fields (`field_tools.rs:224-233`) and refuses a
    /// blank `event_id` with a 400 (`:246-254`), so the no-operation body must omit the key
    /// entirely rather than send `""`.
    #[test]
    fn the_post_body_carries_every_field_the_route_requires() {
        let b = save_body("M120 120mm", (1.0, 2.0), (3.0, 4.0), Some("  ev-1  "));
        for k in [
            "weapon_system",
            "fp_x",
            "fp_y",
            "tgt_x",
            "tgt_y",
            "fp_grid",
            "target_grid",
            "event_id",
        ] {
            assert!(b.get(k).is_some(), "missing {k} in {b}");
        }
        assert_eq!(b["weapon_system"], "M120 120mm");
        assert_eq!(b["fp_x"], 1.0);
        assert_eq!(b["tgt_y"], 4.0);
        assert_eq!(b["event_id"], "ev-1", "the id must be trimmed, not refused");

        for none in [None, Some(""), Some("   ")] {
            let b = save_body("M252 81mm", (1.0, 2.0), (3.0, 4.0), none);
            assert!(
                b.get("event_id").is_none(),
                "a blank event_id must be omitted, not sent: {b}"
            );
        }
    }

    /// **Claim 2's perturbation test.** The defect was a marker subtree that read no input, so the
    /// assertion is that every input moves it: change one coordinate at a time and require the
    /// projected position to change. Restore the fixed CSS (`top-1/4 left-1/3`) and this is the
    /// test that goes red — a render assertion would not, because the markers rendered fine.
    #[test]
    fn both_preview_markers_move_when_any_input_moves() {
        let base = preview_pos((1000.0, 2000.0), (2200.0, 1800.0));
        for (fp, tgt, what) in [
            ((1500.0, 2000.0), (2200.0, 1800.0), "FP X"),
            ((1000.0, 2500.0), (2200.0, 1800.0), "FP Y"),
            ((1000.0, 2000.0), (2800.0, 1800.0), "TGT X"),
            ((1000.0, 2000.0), (2200.0, 1200.0), "TGT Y"),
        ] {
            assert_ne!(
                preview_pos(fp, tgt),
                base,
                "{what} did not move the preview"
            );
        }
        // Both markers stay inside the box at any separation, and north is up.
        for (fp, tgt) in [
            ((0.0, 0.0), (12800.0, 12800.0)),
            ((6400.0, 6400.0), (6401.0, 6400.5)),
            ((-5000.0, 9000.0), (5000.0, -9000.0)),
        ] {
            let (a, b) = preview_pos(fp, tgt);
            for (l, t) in [a, b] {
                assert!(
                    (0.0..=100.0).contains(&l) && (0.0..=100.0).contains(&t),
                    "{l},{t}"
                );
            }
            // Whichever point is further north gets the smaller `top`.
            if fp.1 > tgt.1 {
                assert!(a.1 < b.1, "north must be up");
            }
        }
        // A gun sitting on its own target has no scale to fit; both markers centre.
        assert_eq!(
            preview_pos((500.0, 500.0), (500.0, 500.0)),
            ((50.0, 50.0), (50.0, 50.0))
        );
    }

    /// The weapon list duplicates `api/src/services/mortar.rs::charges_for`. Nothing can check
    /// that from here, so this pins the copy: it fails the moment someone edits the list without
    /// reading the note above it, which is the only warning this drift can get.
    #[test]
    fn the_offered_weapons_are_the_keys_the_api_accepts() {
        assert_eq!(
            WEAPONS,
            ["M252 81mm", "M821 81mm", "2B14 82mm", "M120 120mm"],
            "mirror of services/mortar.rs charges_for — update both or neither"
        );
        assert!(WEAPONS.iter().all(|w| *w == w.trim() && !w.is_empty()));
    }

    /// The `/events` rows this page decodes. `name_override` is genuinely optional on the model;
    /// nothing else is, and the picker must not silently drop an operation it failed to read.
    #[test]
    fn the_operation_picker_decodes_a_real_events_row() {
        let page: Paginated<EventOption> = serde_json::from_str(
            r#"{"data":[{"id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","name_override":"Operation Byte Parity Night","start_time":"2026-08-01T19:00:00Z","status":"scheduled","registration_locked":false,"max_slots":0,"mission_count":1,"registered":5,"filled":5,"total_slots":16,"percent":31}],"total":1,"limit":50,"offset":0}"#,
        )
        .expect("a live /events row decodes");
        assert_eq!(page.data[0].id, "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7");
        assert_eq!(page.data[0].name(), "Operation Byte Parity Night");
        for blank in ["", r#","name_override":null"#, r#","name_override":"  ""#] {
            let anon: EventOption = serde_json::from_str(&format!(
                r#"{{"id":"x","start_time":"2026-08-01T19:00:00Z"{blank}}}"#
            ))
            .unwrap();
            assert_eq!(anon.name(), "Untitled Operation", "over {blank:?}");
        }
    }
}
