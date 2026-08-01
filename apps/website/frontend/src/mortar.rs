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
//! **T-587 — the table can now hold the solution, so a restored one is no longer a lesser one.**
//! Until migration `0020_fire_missions_solution.sql`, `fire_missions` stored `weapon_system`, the
//! two grid strings, `distance_m`, `azimuth_deg` and `elevation_mils` — and **no**
//! `time_of_flight_s`, `charge` or `azimuth_mils`, and no numeric coordinates at all. A
//! freshly-saved solution showed the full card, because the `POST` *response* carries the live
//! `FireSolution`; the same solution after a reload showed TOF as `—`, because the row genuinely
//! did not have one. That asymmetry was visible to the operator every time and it was the schema's
//! fault, not this page's.
//!
//! The row now carries the four coordinates and all three missing numbers, so a restored solution
//! and a fresh one are the same card. What did **not** change is what `—` means: a row written
//! before that migration has `null` in all seven, and [`restore`] renders it exactly as it always
//! did rather than fabricating a `0.0 s` flight or charge zero.
//!
//! **The grid encoding is retired as a *dependency*, not deleted.** [`fmt_grid`] still writes the
//! coordinates into `fp_grid`/`target_grid` — those columns are `NOT NULL` and required by the
//! route — but [`restore`] now reads `fp_x`/`fp_y`/`tgt_x`/`tgt_y` first and falls back to
//! [`parse_grid`] only for pre-T-587 rows. The round-trip test on that encoding stays: it is what
//! keeps the fallback path honest for every row already in the table.
#![allow(dead_code)]
use crate::dto::{DataEnvelope, FireSolution, Paginated};
use crate::ui::{AuthGate, PageHeader};
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;

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
///
/// **T-587 — the seven new fields are `Option` AND `#[serde(default)]`, and they need both.**
/// `Option` because the column is nullable: a fire mission saved before migration `0020` has no
/// charge and no time of flight, and `null` is the true answer. `#[serde(default)]` because of the
/// *other* reader — this suite's `LIVE_LIST` fixture is a verbatim capture from before those
/// columns existed, and a captured response is the one thing that can prove a pre-change row still
/// decodes. Without the default that fixture stops parsing and the regression it guards goes with
/// it.
///
/// This is the one place the strictness rule above is relaxed, and the relaxation is narrow: it
/// applies only to fields the server may legitimately not send. Every field the backend marks
/// required stays required here, so a rename still fails the decode.
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
    /// The four coordinates, as real numbers (T-587). `None` for a row written before migration
    /// `0020` whose only record of them is the [`fmt_grid`] encoding in the grid strings.
    #[serde(default)]
    fp_x: Option<f64>,
    #[serde(default)]
    fp_y: Option<f64>,
    #[serde(default)]
    tgt_x: Option<f64>,
    #[serde(default)]
    tgt_y: Option<f64>,
    #[serde(default)]
    azimuth_mils: Option<i64>,
    #[serde(default)]
    charge: Option<i64>,
    #[serde(default)]
    time_of_flight_s: Option<f64>,
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

/// A `GET /events/{id}/fire-missions` result, tagged with the operation it was fetched for.
///
/// **The tag is not bookkeeping — without it the load-time hydration is a race it loses.**
/// `LocalResource` keeps serving its PREVIOUS value while the next key is in flight, so the instant
/// the operator picks an operation the effect below sees `{new operation, old operation's rows}`.
/// Measured in a real browser against the live API on 2026-07-31: a cold session picked its
/// operation, the once-per-operation latch fired against the empty list belonging to *no*
/// operation, and the real rows that landed 40 ms later were then correctly ignored as
/// already-hydrated. The page showed "Enter coordinates and calculate" over a saved fire mission
/// it had in hand — this ticket's own defect, one layer up.
///
/// `events.rs` hit the identical shape on its hub Resource and answers it the identical way
/// (`Hub::Loaded(got, ev)` + `if got == want`, with the comment "the value in hand is the one asked
/// for"). Same fix here rather than a new one.
#[derive(Clone, Debug, PartialEq)]
struct SavedFor {
    event_id: Option<String>,
    rows: Vec<SavedFire>,
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
/// Not `FireSolution` directly, because the two sources of a solution still do not carry the same
/// guarantees. A live solve always has a charge and a time of flight; a row read out of
/// `fire_missions` has them only if it was written after T-587's migration. Modelling that as
/// `Option` forces the card to say `—` rather than print a fabricated `0.0 s` or charge `0` —
/// numbers that are indistinguishable from a real zero-second flight and a real charge-zero ring,
/// and are the same defect class as the `Value`-typed decode above.
///
/// **T-587 — `charge` joined the card.** It is not new information from the calculator, which has
/// always computed it; it is newly *storable*, and it is the half of a fire order an elevation is
/// useless without. Showing it only for a fresh solve, as this card would have had to before the
/// migration, is the exact asymmetry the ticket is about.
#[derive(Clone, Debug, PartialEq)]
struct Shown {
    weapon_system: String,
    distance_m: i64,
    azimuth_deg: f64,
    elevation_mils: i64,
    /// The propellant ring. `None` only for a row written before T-587's migration.
    charge: Option<i64>,
    /// `None` only for a row written before T-587's migration — that row genuinely has no TOF.
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
            charge: Some(s.charge),
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
/// **T-587 — this is no longer the only record of the coordinates, and it stays lossless anyway.**
/// Before migration `0020` the insert wrote `fp_grid`/`target_grid` and nothing else positional, so
/// this string was the sole place the operator's four numbers survived a reload. The row now
/// carries `fp_x`/`fp_y`/`tgt_x`/`tgt_y` and [`restore`] prefers them — but the encoding is still
/// what every pre-migration row is read back through, and it is what the migration's own backfill
/// parses. A lossy format here (a six-figure military grid, "012 020") would quantise 2200.4 to the
/// nearest hundred metres and hand back a target nobody aimed at — in the historical rows, now
/// permanently.
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
///
/// # T-626 — what migration `0020`'s backfill regex really accepts
///
/// `migrations/0020_fire_missions_solution.sql` calls its accept test "`parse_grid`'s,
/// deliberately character for character". **That claim is wrong, and the migration is applied and
/// checksummed, so it cannot be corrected in place** — sqlx verifies the file's hash on every boot
/// and an edited comment would refuse to start the API. The correction lives here, next to the
/// function the claim is about, and is *tested* by
/// `api/tests/t587_fire_mission_solution.rs::the_backfill_regex_is_narrower_than_parse_grid`.
///
/// The regex is `^\s*-?\d+(\.\d+)?\s*,\s*-?\d+(\.\d+)?\s*$`. This function parses with
/// `str::parse::<f64>`, which accepts strictly more syntax. Measured against a live Postgres:
///
/// ```text
///   input            regex   parse_grid
///   '1000, 2000'       t         t        agree — what `fmt_grid` writes
///   '+1000, 2000'      f         t        `-?` has no `+`
///   '.5, 2'            f         t        `\d+` requires a digit before the point
///   '5., 2'            f         t        `(\.\d+)?` requires digits after it
///   '1e3, 500'         f         t        no exponent form
/// ```
///
/// **The divergence is under-permissive, and that is the safe direction.** A backfill that
/// accepted *more* than this function would invent coordinates for rows the calculator has always
/// shown as unrestorable; one that accepts less only leaves a row where it already was. Nothing is
/// lost either: [`restore`] still falls back to this function whenever the numeric columns are
/// NULL, so a row written in one of those forms restores today exactly as it always did.
///
/// **Nothing widens the regex, because nothing can write those forms.** `fmt_grid` is the only
/// writer of this encoding, it is `format!("{x}, {y}")`, and `f64`'s `Display` never emits a `+`,
/// never a bare leading or trailing point, and never an exponent. A `0022` migration widening the
/// accept set would be dead code over rows that cannot exist — the ticket's alternative, weighed
/// and declined.
///
/// # The one input where the regex is the *wider* of the two
///
/// The accept sets are not nested. A grid whose integer part exceeds `f64::MAX` — 309 digits is
/// already enough, e.g. 309 nines — **matches the regex** and then overflows `::double precision`,
/// which raises `out of range for type double precision` and would have **aborted the whole
/// migration**. This function returns `None` for the same string, because `parse::<f64>` yields
/// `inf` and the `is_finite` guard rejects it.
///
/// No realistic row has that shape: `fmt_grid`'s longest possible output is `f64::MAX`'s own 309
/// digits, which casts cleanly. It would take a hand-written `psql` INSERT to produce one. Recorded
/// because it is the kind of thing that is only obvious once, and 0020 ran without hitting it.
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
/// `None` when the row has neither real coordinates nor a grid string this module wrote — the
/// numbers in the row are still real, but without coordinates there is nothing to put in the
/// FP/TGT inputs, and half-restoring would leave the inputs saying one thing and the card another.
///
/// # T-587 — the columns first, the encoding as the fallback
///
/// `fp_x`/`fp_y`/`tgt_x`/`tgt_y` are the authoritative record now, so they are read first. The
/// [`parse_grid`] fallback is **not** belt-and-braces and must not be deleted as dead: every fire
/// mission saved before migration `0020` has `null` in all four columns, and the text encoding is
/// the only place its coordinates exist. Deleting the fallback would not throw an error — it would
/// quietly stop restoring every historical row, which reads to the operator as "nothing was ever
/// saved" over rows that are sitting right there in the list.
///
/// The two pairs fall back independently because the migration backfills them independently (a row
/// can have one grid in this encoding and one not).
///
/// Everything else comes off the row and stays `Option`: a pre-migration row has no charge and no
/// time of flight, and the card is required to say `—` rather than invent one.
fn restore(row: &SavedFire) -> Option<Restored> {
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
///   there would replace the fresh full-fidelity card (TOF and all) with the TOF-less row that was
///   just written, and would yank coordinates out from under anyone mid-edit.
///
/// `None` means do nothing at all. `Some(restored)` means latch this operation as hydrated and
/// apply `restored` — itself `None` when the operation has nothing saved yet, or when the newest
/// row's grids are not this module's encoding. Latching over an empty operation is deliberate: it
/// is a real answer ("nothing saved here"), and re-asking on every refetch is how a later save gets
/// clobbered.
///
/// T-588 — `already_hydrated` is a SET, not the last operation seen. It used to be a single
/// `Option<&str>` slot, which made "once per operation" true only for the most recent one:
/// switching away from an operation and back re-hydrated it, silently replacing whatever the
/// operator had typed since with the saved solution. The rows were never lost, so nothing looked
/// broken — the in-progress edit just quietly vanished. A latch that forgets is not a latch.
fn hydration_step(
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
                    .map(|d| SavedFor {
                        event_id: Some(id),
                        rows: d.data,
                    }),
                    None => Some(SavedFor {
                        event_id: None,
                        rows: Vec::new(),
                    }),
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, ev);
                None::<SavedFor>
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

    // Hydrate the card from the newest saved fire mission, ONCE per operation.
    let hydrated_for = StoredValue::new(HashSet::<String>::new());
    Effect::new(move |_| {
        // Only act on a fetch that actually answered. Latching on a failed load would make the
        // page's one hydration attempt the one that read nothing.
        let Some(Some(batch)) = saved.get() else {
            return;
        };
        let Some(ev) = event_id.get() else { return };
        let Some(restored) = hydration_step(&batch, &ev, &hydrated_for.get_value()) else {
            return;
        };
        hydrated_for.update_value(|seen| {
            seen.insert(ev);
        });
        if let Some(r) = restored {
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
                                        // T-587 — charge and TOF now come off the stored row as
                                        // well as off a live solve, so both read the same on a
                                        // reload as they did the moment they were computed. `—` is
                                        // reserved for a fire mission saved before the migration,
                                        // which genuinely has neither.
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"Charge"</dt>
                                            <dd
                                                title=move || {
                                                    if s.charge.is_some() {
                                                        ""
                                                    } else {
                                                        "saved before charge was stored — recalculate for it"
                                                    }
                                                }
                                            >
                                                {match s.charge {
                                                    Some(c) => c.to_string(),
                                                    None => "—".to_string(),
                                                }}
                                            </dd>
                                        </div>
                                        <div class="flex justify-between">
                                            <dt class="text-on-surface-variant">"TOF"</dt>
                                            <dd
                                                title=move || {
                                                    if s.time_of_flight_s.is_some() {
                                                        ""
                                                    } else {
                                                        "saved before time of flight was stored — recalculate for it"
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

    /// **T-587 — the pre-existing row.** [`LIVE_LIST`] was captured before migration `0020`
    /// existed, so it is the exact wire shape of every fire mission already in the table: no
    /// `fp_x`, no `charge`, no `time_of_flight_s`. It must still decode, still restore its
    /// coordinates (through the [`fmt_grid`] fallback, the only record it has of them), and still
    /// render `—` for the two numbers it does not carry.
    ///
    /// This is the regression that the obvious version of this ticket breaks: read the new columns,
    /// delete the grid fallback as "superseded", and every historical row silently stops restoring.
    /// Nothing throws — the list still renders, the rows are still there, and clicking one just
    /// does nothing.
    #[test]
    fn a_row_saved_before_the_migration_still_restores_and_shows_no_tof_or_charge() {
        let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST).unwrap();
        let row = &list.data[0];
        // The columns are absent from the capture, not null-and-present.
        assert_eq!(
            (row.fp_x, row.fp_y, row.tgt_x, row.tgt_y),
            (None, None, None, None)
        );
        assert_eq!((row.charge, row.time_of_flight_s), (None, None));

        let r = restore(row).expect("a pre-T-587 row still restores via the grid encoding");
        assert_eq!(r.fp, (1000.0, 2000.0), "coordinates come from fp_grid");
        assert_eq!(r.tgt, (2200.0, 1800.0), "coordinates come from target_grid");
        assert_eq!(r.shown.time_of_flight_s, None);
        assert_eq!(r.shown.charge, None);
        // …while a live solve does carry both, so the two sources stay distinguishable.
        let solved: FireSolution = serde_json::from_str(
            r#"{"weapon_system":"M252 81mm","distance_m":1217,"azimuth_deg":99.5,"azimuth_mils":1768,"elevation_mils":1315,"charge":2,"time_of_flight_s":29.4}"#,
        )
        .unwrap();
        assert_eq!(Shown::from(&solved).time_of_flight_s, Some(29.4));
        assert_eq!(Shown::from(&solved).charge, Some(2));
    }

    /// **T-587 — the row a post-migration save produces.** Captured from
    /// `GET /events/{id}/fire-missions` against the live handler in
    /// `api/tests/t587_fire_mission_solution.rs`, which asserts these same values against the
    /// database row itself.
    ///
    /// The card a reload builds from this must be the card the live solve built: same charge, same
    /// TOF, no `—` anywhere. If any of the seven columns stops being written, or stops being
    /// projected by the SELECT, this goes red on the field that went missing rather than on a
    /// vague "restore returned None".
    const LIVE_LIST_T587: &str = r#"{"data":[{"id":"5c2e8b4a-1f77-4a10-9d3e-2b6c7f0a1e44","event_id":"c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7","created_by":"000000000000000001","weapon_system":"M252 81mm","fp_grid":"1000, 2000","target_grid":"2200, 1800","distance_m":1217,"azimuth_deg":99.5,"elevation_mils":1315,"fp_x":1000.0,"fp_y":2000.0,"tgt_x":2200.0,"tgt_y":1800.0,"azimuth_mils":1768,"charge":2,"time_of_flight_s":29.4,"created_at":"2026-08-01T10:04:11.512004Z"}]}"#;

    #[test]
    fn a_row_saved_after_the_migration_restores_the_whole_solution() {
        let list: DataEnvelope<SavedFire> = serde_json::from_str(LIVE_LIST_T587).unwrap();
        let row = &list.data[0];
        let r = restore(row).expect("a T-587 row restores");

        // Coordinates come from the COLUMNS now. Proven by breaking the encoding: a row whose
        // grid strings no longer parse still restores, which was impossible before this ticket.
        let mut no_grid = row.clone();
        no_grid.fp_grid = "GRID 012345".into();
        no_grid.target_grid = "GRID 012845".into();
        let r2 = restore(&no_grid).expect("the numeric columns carry it without the encoding");
        assert_eq!((r2.fp, r2.tgt), (r.fp, r.tgt));
        assert_eq!(r.fp, (1000.0, 2000.0));
        assert_eq!(r.tgt, (2200.0, 1800.0));

        // The three numbers that had no column before T-587.
        assert_eq!(r.shown.charge, Some(2), "charge did not survive the reload");
        assert_eq!(
            r.shown.time_of_flight_s,
            Some(29.4),
            "TOF did not survive the reload"
        );
        assert_eq!(
            row.azimuth_mils,
            Some(1768),
            "the sight setting is on the row"
        );

        // …and the card is now indistinguishable from the freshly-computed one, which is the
        // whole ticket. Same source numbers, same card, minus the `saved_at` a live solve has
        // not earned yet.
        let solved: FireSolution = serde_json::from_str(
            r#"{"weapon_system":"M252 81mm","distance_m":1217,"azimuth_deg":99.5,"azimuth_mils":1768,"elevation_mils":1315,"charge":2,"time_of_flight_s":29.4}"#,
        )
        .unwrap();
        let mut fresh = Shown::from(&solved);
        assert_eq!(fresh.saved_at, None);
        fresh.saved_at = r.shown.saved_at.clone();
        assert_eq!(
            fresh, r.shown,
            "a restored solution must render as the same card as a fresh one"
        );
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

    /// **The measured race, pinned.** On 2026-07-31 a real browser against the live API showed a
    /// cold session pick its operation and get "Enter coordinates and calculate to see solution."
    /// back over a fire mission it already had in hand: `LocalResource` was still serving the
    /// previous key's value, the once-per-operation latch fired against *that*, and the real rows
    /// arriving a tick later were then correctly ignored as already-hydrated.
    ///
    /// Delete the `batch.event_id != want` arm of [`hydration_step`] and the first case below goes
    /// green-then-wrong exactly as it did in the browser: it latches, restores nothing, and no
    /// render assertion anywhere can tell the difference between that and an operation with no
    /// saved fire missions.
    #[test]
    fn hydration_refuses_a_batch_fetched_for_a_different_operation() {
        let row = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
            .unwrap()
            .data
            .remove(0);
        let want = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";

        // The value in flight belongs to "no operation" — the state a cold page starts in.
        let stale = SavedFor {
            event_id: None,
            rows: Vec::new(),
        };
        assert_eq!(
            hydration_step(&stale, want, &HashSet::new()),
            None,
            "a batch fetched for another operation must not hydrate AND must not latch"
        );

        // …and the batch that actually answers for `want` still hydrates afterwards, which is the
        // half that was broken: the latch had already been spent.
        let fresh = SavedFor {
            event_id: Some(want.to_string()),
            rows: vec![row],
        };
        let applied = hydration_step(&fresh, want, &HashSet::new())
            .expect("the batch for this operation must be acted on")
            .expect("its newest row must restore");
        assert_eq!(applied.fp, (1000.0, 2000.0));
        assert_eq!(applied.shown.distance_m, 1217);

        // Once done it is done — a refetch after a save must not clobber the fresh card.
        assert_eq!(
            hydration_step(&fresh, want, &HashSet::from([want.to_string()])),
            None
        );

        // An operation with nothing saved is a real answer: latch, restore nothing.
        let empty = SavedFor {
            event_id: Some(want.to_string()),
            rows: Vec::new(),
        };
        assert_eq!(hydration_step(&empty, want, &HashSet::new()), Some(None));
    }

    /// T-588 — switching operation away and back must NOT re-hydrate.
    ///
    /// The latch used to be a single `Option<String>` slot holding the last operation hydrated, so
    /// it only ever remembered one. Sequence A → B → A: hydrating B overwrote the memory of A, and
    /// returning to A hydrated it a second time, dropping whatever the operator had typed in the
    /// meantime over the saved solution. No data was lost from the server's point of view, which is
    /// exactly why it went unnoticed.
    ///
    /// Revert `already_hydrated` to a single slot and the final assertion here goes red.
    #[test]
    fn returning_to_an_operation_does_not_re_hydrate_over_unsaved_edits() {
        let row = serde_json::from_str::<DataEnvelope<SavedFire>>(LIVE_LIST)
            .unwrap()
            .data
            .remove(0);
        let a = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";
        let b = "00000000-0000-4000-7000-000000000001";
        let batch_a = SavedFor {
            event_id: Some(a.to_string()),
            rows: vec![row],
        };
        let batch_b = SavedFor {
            event_id: Some(b.to_string()),
            rows: Vec::new(),
        };

        // This is the real effect's state, threaded by hand.
        let mut seen: HashSet<String> = HashSet::new();

        // 1. Land on A: it hydrates, and A is latched.
        assert!(
            hydration_step(&batch_a, a, &seen).is_some(),
            "the first visit to an operation must hydrate"
        );
        seen.insert(a.to_string());

        // 2. Switch to B: it hydrates (nothing saved), and B is latched.
        assert_eq!(hydration_step(&batch_b, b, &seen), Some(None));
        seen.insert(b.to_string());

        // 3. Back to A, now with unsaved edits on the card. The old single-slot latch held only
        //    B here, so this returned `Some(..)` and clobbered them.
        assert_eq!(
            hydration_step(&batch_a, a, &seen),
            None,
            "returning to an already-hydrated operation must not re-apply its saved solution —              that silently discards in-progress edits"
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
