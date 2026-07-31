//! Mission Overview (/missions/:id) — ported from pages/missions.tsx `MissionOverviewPage` +
//! `MissionDossierBody`. `<AuthGate>` → `useMission(id)` → `GET /missions/:id` → a `QueryState`
//! (Loading… / error / content) wrapping a PageHeader + a glass OpsCard dossier (badges, briefing,
//! a Weather/Time/Max-Players/Status detail grid, and — when present — the faction armory).
//!
//! **Gate scope:** the seeded golden `512d8658-…` (a fresh mission: auto-version v0.1.0, empty
//! armory, no briefing) → header + 3 badges + "No briefing provided." + the 4 details; the armory
//! section is hidden (no factions). The armory tabs/items are content-gated (need a golden with
//! loadouts). DTO round-trip is proven by the R-api gate (dto.rs `mission_detail`).
//!
//! T-368: the read half shipped without its write half. `PUT /missions/:id/armory` existed, worked,
//! and had **no caller anywhere in the SPA** — repo-wide the only callers were `tests/missions.rs`,
//! `tests/null_tolerance.rs` and docs, and live `mission_armories` held **zero rows**. So the
//! faction armory that this page renders, that the Event Hub dossier renders
//! (`event_hub.rs:478`) and that `GET /missions/:id/export` hands the mod could not be authored by
//! an operator at all. Same dead-endpoint shape as T-226 (event edit / mission detach) and T-232
//! (six render branches). It also means T-346's fix to that endpoint was necessarily measured on
//! synthetic data; this is what puts it on the SPA.
//!
//! The editor is an **Edit Armory** Dialog on this page (author/admin only), not in
//! [`dossier_body`] — see the note there for why the shared body must stay read-only.
#![allow(dead_code)]
use crate::dto::MissionDetail;
use crate::nav::Role;
use crate::ui::{cn, AuthGate, Dialog, MaterialIcon};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use serde_json::Value;

// Badge variants (badge.tsx cva): the base `text-label-sm` is twMerge-dropped against the trailing
// text-{color}, same as the wiki neutral badge.
const BADGE_PRIMARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-primary/30 bg-primary/10 text-primary";
const BADGE_NEUTRAL: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-outline-variant/40 bg-surface-variant/40 text-on-surface-variant";
const BADGE_TERTIARY: &str = "inline-flex items-center gap-1 rounded border px-2 py-0.5 uppercase whitespace-nowrap border-tertiary/30 bg-tertiary/10 text-tertiary";

/// `gameModeLabel` (lib/format.ts).
fn game_mode_label(mode: &str) -> &str {
    match mode {
        "pve_coop" => "COOP",
        "pvp" => "PvP",
        "zeus" => "Zeus",
        other => other,
    }
}

/// `terrainLabel` (lib/format.ts) — capitalize first char; "—" when empty.
fn terrain_label(t: &str) -> String {
    if t.is_empty() {
        return "—".into();
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/* ═════════════════════════ Armory authoring (T-368) ═════════════════════════ */

/// Where a faction key offered by the editor came from — which is the whole reason the editor
/// offers a *choice* instead of a text field.
#[derive(Clone, Copy, PartialEq)]
enum KeySource {
    /// From this mission's own ORBAT template. [`materialize_slots`] binds these exact bytes into
    /// `orbat_slots.faction` (`handlers/events.rs:435`) when the mission is attached to an event,
    /// and [`get_event`] builds the Event Hub's `factions` list from that column
    /// (`handlers/events.rs:894`) — so an armory row carrying this key **joins**.
    Orbat,
    /// Present on a row already stored in `mission_armories`, and in no ORBAT faction. It still
    /// renders on this page (the tabs in [`dossier_body`] are built from the armory itself) but
    /// matches no Event Hub faction card. Offered anyway because the PUT is **wholesale**: a key
    /// this editor cannot represent is a row this editor silently deletes.
    StoredOnly,
}

/// One faction key the editor may file rows under.
#[derive(Clone, PartialEq)]
struct FactionKey {
    /// The bytes that go on the wire. **Never rewritten** — see [`orbat_faction_keys`].
    key: String,
    source: KeySource,
}

/// The faction keys this mission's ORBAT will materialise into `orbat_slots.faction`.
///
/// **This is the whole point of the slice.** `faction` is not a label, it is a join key, and the
/// two sides of that join live in different tables and are compared by *exact byte equality*:
/// [`armory_by_faction`] groups `mission_armories` by the row's raw value
/// (`handlers/events.rs:796`) and `event_hub.rs:415` matches those groups against a list built from
/// `orbat_slots.faction` with `.find(|f| &f.faction == faction)`. A key that does not match
/// byte-for-byte renders a dossier card with **no items at all**, while the write answers 200 and
/// echoes the author's own value back — the failure T-346 measured. A free-text box on this page
/// would be a machine for producing exactly that, so the key is *derived*, never typed.
///
/// The derivation is a faithful mirror of [`parse_orbat_template`]
/// (`crates/map-engine-core/src/mission/orbat.rs:35`), which is the **only** producer of
/// `orbat_slots.faction` through the API:
///
///  * an explicit non-empty top-level `orbat` array wins, contributing `orbat[].faction`;
///  * otherwise the keys come from `editor.factions[].key`, copied verbatim by
///    `derive_orbat_from_editor` (`orbat.rs:164`). In practice this is the live path — Save Version
///    omits the top-level `orbat[]` (T-062.1.1), and the T-357 schema note records it as absent
///    from all 128 live payloads.
///
/// Two details are load-bearing rather than incidental:
///
///  * **A faction with no materialisable slot is not offered.** `materialize_slots` inserts one row
///    per slot, so a faction whose squads resolve to zero slots produces zero `orbat_slots` rows and
///    never appears in the Event Hub's `factions` list. Offering it would be offering a key that
///    joins to nothing — the very bug this function exists to prevent.
///  * **A malformed `orbat` falls through to the editor graph, exactly as the server does.**
///    `parse_orbat_template` decodes into a struct and `unwrap_or_default()`s a failure, so an
///    `orbat` that is an object (as every compiled golden mission has) or an array holding a
///    non-object does *not* win the precedence test. Hence the shape checks below: taking the wrong
///    branch here would offer keys from one source while the server materialises from the other.
///
/// **What this does NOT cover, honestly:** `orbat_slots` rows already materialised from a
/// *superseded* version, or inserted directly by a seed (`seeds/content_golden.sql:638` does exactly
/// that, for a mission whose `json_payload` is `{}`). Those keys are not in the current payload and
/// there is no mission-scoped route that returns them — `orbat_slots` is keyed on
/// `event_mission_id`, which this page does not know. `handlers/events.rs:2009` already tells an
/// operator to re-attach the mission to re-materialise its ORBAT, and that is the same answer here.
fn orbat_faction_keys(payload: &Value) -> Vec<String> {
    fn push_unique(out: &mut Vec<String>, key: &str) {
        if !out.iter().any(|s| s == key) {
            out.push(key.to_string());
        }
    }
    /// The subset of `OrbatSquadTemplate`'s shape that decides whether serde would have accepted the
    /// whole `orbat` array — every field is `#[serde(default)]`, so only the *types* can reject.
    fn squad_decodes(sq: &Value) -> bool {
        let Some(o) = sq.as_object() else {
            return false;
        };
        for (k, v) in o {
            let ok = match k.as_str() {
                "faction" | "callsign" | "squad" => v.is_string(),
                "slots" => v
                    .as_array()
                    .is_some_and(|s| s.iter().all(|sl| sl.is_object())),
                _ => true, // unknown fields are ignored by serde, not rejected
            };
            if !ok {
                return false;
            }
        }
        true
    }

    let mut out: Vec<String> = Vec::new();

    if let Some(orbat) = payload.get("orbat").and_then(Value::as_array) {
        if !orbat.is_empty() && orbat.iter().all(squad_decodes) {
            for sq in orbat {
                let has_slot = sq
                    .get("slots")
                    .and_then(Value::as_array)
                    .is_some_and(|s| !s.is_empty());
                if has_slot {
                    let faction = sq
                        .get("faction")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    push_unique(&mut out, faction);
                }
            }
            // The array won the precedence test, so the editor graph is never consulted — even if
            // the array contributed nothing. Returning here rather than falling through is what
            // keeps this in step with `parse_orbat_template`'s early return.
            return out;
        }
    }

    let Some(editor) = payload.get("editor") else {
        return out;
    };
    let Some(factions) = editor.get("factions").and_then(Value::as_array) else {
        return out;
    };
    let empty: Vec<Value> = Vec::new();
    let squads = editor
        .get("squads")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let slots = editor
        .get("slots")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let id_of = |v: &Value| v.get("id").and_then(Value::as_str).map(str::to_owned);
    for f in factions {
        let Some(squad_ids) = f.get("squadIds").and_then(Value::as_array) else {
            continue;
        };
        let mut materialisable = 0usize;
        for sid in squad_ids.iter().filter_map(Value::as_str) {
            // `.rev()` mirrors `derive_orbat_from_editor`'s `HashMap` collect, where a duplicate id
            // resolves to the LAST squad carrying it, not the first.
            let Some(sq) = squads
                .iter()
                .rev()
                .find(|s| id_of(s).as_deref() == Some(sid))
            else {
                continue;
            };
            let Some(slot_ids) = sq.get("slotIds").and_then(Value::as_array) else {
                continue;
            };
            materialisable += slot_ids
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| slots.iter().any(|s| id_of(s).as_deref() == Some(*id)))
                .count();
        }
        if materialisable > 0 {
            let key = f.get("key").and_then(Value::as_str).unwrap_or_default();
            push_unique(&mut out, key);
        }
    }
    out
}

/// Whether `set_armory` will accept this faction key at all, using the server's own predicate
/// (`handlers/missions.rs:769` and `:777`): non-blank, and **byte-identical to its own trimmed
/// form**.
///
/// T-346 chose refuse-over-normalise deliberately, because the other side of the join
/// (`orbat_slots.faction`) normalises nothing (T-356, filed and unstarted) — a padded value on both
/// sides renders correctly *today*, and a unilateral trim would break it. So this editor must not
/// trim either: it reports the key as unstorable and names the fix, rather than quietly rewriting a
/// key into one that joins to nothing.
fn key_storable(key: &str) -> bool {
    !key.trim().is_empty() && key == key.trim()
}

/// Picker label — makes an unstorable key *visible* rather than hiding it, since the row filed
/// under it has to be findable and deletable.
fn key_label(key: &str) -> String {
    if key.is_empty() {
        "(blank)".into()
    } else if key != key.trim() {
        // Quoted so the padding the server refuses is on screen.
        format!("\u{201c}{key}\u{201d}")
    } else {
        key.to_string()
    }
}

/// One row of the draft armory. Plain data, not signals: the editor adds and removes whole rows
/// (the `event_manager.rs` staged-mission vocabulary) rather than editing them in place, so a
/// keystroke never rebuilds the list.
#[derive(Clone, PartialEq)]
struct DraftRow {
    /// Chosen from [`FactionKey`], never typed.
    faction: String,
    item_name: String,
    category: String,
    /// Blank = `null` on the wire = unlimited (`MissionArmory.quantity` is `Option<i64>`).
    quantity: String,
}

/// `"12"` → `Some(Some(12))`, `""` → `Some(None)` (unlimited), anything else → `None` (refused).
fn parse_qty(s: &str) -> Option<Option<i64>> {
    let t = s.trim();
    if t.is_empty() {
        return Some(None);
    }
    t.parse::<i64>().ok().map(Some)
}

/// The Edit Armory dialog's state. `Copy`, so the whole thing threads into [`body`] and the handlers
/// without clones.
#[derive(Clone, Copy)]
struct ArmoryEditor {
    open: RwSignal<bool>,
    /// The complete draft armory, across every faction — the PUT replaces the lot.
    rows: RwSignal<Vec<DraftRow>>,
    /// Which faction's rows are on screen. Always one of `keys`.
    faction: RwSignal<String>,
    keys: RwSignal<Vec<FactionKey>>,
    busy: RwSignal<bool>,
    new_name: RwSignal<String>,
    new_category: RwSignal<String>,
    new_qty: RwSignal<String>,
    /// The mission the draft was opened from, captured at that moment. Carried on the editor rather
    /// than re-read from the page's Resource at save time for the reason T-226 recorded: a Resource
    /// keeps serving its LAST value while the next run is in flight, so reading the id late is how
    /// one mission's armory ends up under another mission's live button.
    mission_id: RwSignal<String>,
    /// Bumped after a successful PUT. The page's Resource reads it, so the read view refreshes
    /// without this dialog needing to hold a handle on it.
    saved: RwSignal<u32>,
}

impl ArmoryEditor {
    fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            rows: RwSignal::new(Vec::new()),
            faction: RwSignal::new(String::new()),
            keys: RwSignal::new(Vec::new()),
            busy: RwSignal::new(false),
            new_name: RwSignal::new(String::new()),
            new_category: RwSignal::new(String::new()),
            new_qty: RwSignal::new(String::new()),
            mission_id: RwSignal::new(String::new()),
            saved: RwSignal::new(0),
        }
    }

    /// Snapshot the mission into the draft and open. Same shape as `event_manager.rs`'s
    /// `edit_orig`: the dialog is loaded from the row it was opened on, so nothing it sends can be
    /// carrying another mission's data.
    fn open_for(self, m: &MissionDetail) {
        let rows: Vec<DraftRow> = m
            .armory
            .iter()
            .map(|a| {
                let s = |k: &str| {
                    a.get(k)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                DraftRow {
                    faction: s("faction"),
                    item_name: s("item_name"),
                    category: s("category"),
                    quantity: a
                        .get("quantity")
                        .and_then(Value::as_i64)
                        .map(|q| q.to_string())
                        .unwrap_or_default(),
                }
            })
            .collect();
        let mut keys: Vec<FactionKey> = m
            .current_version
            .as_ref()
            .map(|v| orbat_faction_keys(&v.json_payload))
            .unwrap_or_default()
            .into_iter()
            .map(|key| FactionKey {
                key,
                source: KeySource::Orbat,
            })
            .collect();
        // Anything the stored armory already uses has to be representable, or saving would delete
        // it. Appended after the ORBAT keys so the joining ones are what the dialog opens on.
        for r in &rows {
            if !keys.iter().any(|k| k.key == r.faction) {
                keys.push(FactionKey {
                    key: r.faction.clone(),
                    source: KeySource::StoredOnly,
                });
            }
        }
        self.faction
            .set(keys.first().map(|k| k.key.clone()).unwrap_or_default());
        self.keys.set(keys);
        self.rows.set(rows);
        self.new_name.set(String::new());
        self.new_category.set(String::new());
        self.new_qty.set(String::new());
        self.mission_id.set(m.id.clone());
        self.open.set(true);
    }
}

/// The first reason `set_armory` would refuse this draft, or `None`.
///
/// A mirror of the server's per-item guard (`handlers/missions.rs:763`-`782`) plus the one thing
/// serde decides before the guard runs: `quantity` is `Option<i64>`, so a non-numeric value fails
/// *decoding* and the 400 that comes back reads "items is required, and every item needs a faction
/// and an item_name" — a message that sends the author looking for fields their request plainly
/// has. Catching it here is the difference between a useful sentence and a misleading one.
///
/// The rows this editor *creates* cannot trip any of these (the picker supplies the key, Add
/// requires a name and a numeric quantity). What trips them is a row that was already stored —
/// which is exactly the case worth blocking loudly, because the PUT is wholesale and a refusal
/// leaves the whole armory as it was.
fn draft_problem(rows: &[DraftRow]) -> Option<String> {
    for r in rows {
        if r.item_name.trim().is_empty() {
            return Some(format!(
                "An item under {} has no name. The armory endpoint rejects a blank item_name.",
                key_label(&r.faction)
            ));
        }
        if r.faction.trim().is_empty() {
            return Some(format!(
                "\u{201c}{}\u{201d} has a blank faction key. That key matches no Event Hub faction \
                 card and the endpoint rejects it — remove the item.",
                r.item_name.trim()
            ));
        }
        if r.faction != r.faction.trim() {
            return Some(format!(
                "Faction key {} is padded with whitespace. The endpoint refuses it rather than \
                 trimming it, because the ORBAT side of the join is not trimmed either (T-346) — \
                 fix the faction key in the Mission Creator, then re-attach the mission.",
                key_label(&r.faction)
            ));
        }
        if parse_qty(&r.quantity).is_none() {
            return Some(format!(
                "\u{201c}{}\u{201d} has a non-numeric quantity. Leave it blank for unlimited.",
                r.item_name.trim()
            ));
        }
    }
    None
}

/// The `PUT /missions/:id/armory` body.
///
/// `items` is always present, including when it is empty: `{"items":[]}` is how the endpoint is
/// *told* to clear the armory, whereas `{}` fails to decode on purpose (T-315) because the handler's
/// first statement is an unconditional DELETE.
///
/// `faction` and `item_name` both go on the wire **verbatim**, for opposite reasons — the server
/// trims the label and refuses a padded key (T-346's split), and both decisions are its to make.
/// `sort_order` is the draft's own order, because every read of this table is
/// `ORDER BY sort_order ASC` and leaving them all at the default 0 makes the rendered order
/// arbitrary.
fn armory_body(rows: &[DraftRow]) -> Value {
    let items: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "faction": r.faction,
                "category": r.category,
                "item_name": r.item_name,
                "quantity": parse_qty(&r.quantity).flatten(),
                "sort_order": i as i64,
            })
        })
        .collect();
    serde_json::json!({ "items": items })
}

#[component]
pub fn MissionOverviewPage() -> impl IntoView {
    view! {
        <AuthGate>
            <MissionOverviewInner />
        </AuthGate>
    }
}

#[component]
fn MissionOverviewInner() -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let params = use_params_map();
    // T-368. Created before the resource because the resource *depends* on it: `saved` is one of its
    // reactive inputs, so a successful PUT re-runs the GET and the read-only armory below (whose
    // faction tabs are built from that payload) shows what was just written. Owned by the component
    // rather than by the render closure, so the draft is not disposed when the resource re-runs.
    let editor = ArmoryEditor::new();
    let mission = LocalResource::new(move || {
        let id = params
            .read()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_default();
        let _ = editor.saved.get();
        async move {
            #[cfg(target_arch = "wasm32")]
            {
                let path = format!("/missions/{id}");
                crate::client::api_get::<MissionDetail>(store, &path)
                    .await
                    .ok()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (store, id);
                None::<MissionDetail>
            }
        }
    });
    let me = move || store.user.get().map(|u| u.discord_id).unwrap_or_default();

    // Mirrors `handlers::missions::can_edit` (`:58`) EXACTLY — author or admin, and deliberately not
    // `has_min_role(MissionMaker)`. The server's tier is authorship, so a role check here would hide
    // the button from someone the endpoint would serve, and show it to someone it would 403.
    let can_edit = move |m: &MissionDetail| {
        m.author_id == me() || store.user.get().map(|u| u.role) == Some(Role::Admin)
    };

    view! {
        <Suspense fallback=move || {
            view! { <p class="text-on-surface-variant">"Loading…"</p> }
        }>
            {move || {
                mission
                    .get()
                    .map(|opt| match opt {
                        Some(m) => {
                            let editable = can_edit(&m);
                            body(m, editor, editable).into_any()
                        }
                        None => view! { <p class="text-error">"Failed to load data."</p> }.into_any(),
                    })
            }}
        </Suspense>
        {armory_dialog(editor)}
    }
}

/// The Edit Armory dialog (T-368) — the SPA's only caller of `PUT /missions/:id/armory`.
///
/// **Why a Dialog on this page and not a section inside [`dossier_body`].** `dossier_body` is
/// shared: `missions.rs:789` renders it inside the Mission Library's slide-over `Sheet`. Growing an
/// editor into it would put a form in an overlay in an overlay — the two-overlays-at-once
/// anti-pattern T-048 removed when it replaced the `/missions/create` page with a Dialog that
/// *closes* the dossier Sheet first — and would change a function this slice does not own. The
/// read-only body is therefore untouched, and the write half lives here, where the route is a page
/// of its own. A Dialog rather than an inline section for the reason T-024 gave the Event Manager:
/// a half-typed row must not sit in the same column as published content and read as if it were
/// part of it. Same frosted vocabulary as `event_manager.rs`.
fn armory_dialog(ed: ArmoryEditor) -> impl IntoView {
    let store = expect_context::<crate::auth::AuthStore>();
    let on_save = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let id = ed.mission_id.get_untracked();
            if id.is_empty() || ed.busy.get_untracked() {
                return;
            }
            let rows = ed.rows.get_untracked();
            // The button is already disabled on this condition; re-checked because the disabled
            // attribute is the browser's promise, not the handler's.
            if let Some(problem) = draft_problem(&rows) {
                crate::toast::use_toasts().error(problem);
                return;
            }
            let body = armory_body(&rows);
            let count = rows.len();
            ed.busy.set(true);
            let toasts = crate::toast::use_toasts();
            leptos::task::spawn_local(async move {
                let path = format!("/missions/{id}/armory");
                match crate::client::api_put::<Value>(store, &path, body).await {
                    Ok(_) => {
                        toasts.success(if count == 0 {
                            "Armory cleared".to_string()
                        } else {
                            format!("Armory saved \u{2014} {count} items")
                        });
                        ed.open.set(false);
                        ed.saved.update(|n| *n = n.wrapping_add(1));
                    }
                    // The endpoint's 400s name the offending `items[i]` field; showing them verbatim
                    // is more use than anything this page could invent — and it is the only channel
                    // through which a guard this editor does not yet mirror can reach the operator.
                    Err(e) => toasts.error(crate::client::api_error_message(
                        &e,
                        "Could not save the armory",
                    )),
                }
                ed.busy.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (ed, store);
        }
    };
    // Rows filed under the faction currently on screen, paired with their index in the FULL draft —
    // remove has to address the real vector, not the filtered view.
    let visible = move || {
        let f = ed.faction.get();
        ed.rows
            .get()
            .into_iter()
            .enumerate()
            .filter(|(_, r)| r.faction == f)
            .collect::<Vec<_>>()
    };
    let active_key = move || {
        let f = ed.faction.get();
        ed.keys.get().into_iter().find(|k| k.key == f)
    };
    let can_add = move || {
        !ed.new_name.get().trim().is_empty()
            && parse_qty(&ed.new_qty.get()).is_some()
            && active_key().is_some_and(|k| key_storable(&k.key))
    };
    let add_row = move |_| {
        let faction = ed.faction.get_untracked();
        if faction.is_empty() || !can_add() {
            return;
        }
        ed.rows.update(|rows| {
            rows.push(DraftRow {
                faction,
                item_name: ed.new_name.get_untracked(),
                category: ed.new_category.get_untracked(),
                quantity: ed.new_qty.get_untracked(),
            })
        });
        ed.new_name.set(String::new());
        ed.new_category.set(String::new());
        ed.new_qty.set(String::new());
    };
    const PILL: &str = "rounded-full bg-white/5 px-5 py-3 text-sm text-on-surface placeholder:text-on-surface-variant/60 outline-none focus:ring-1 focus:ring-primary/50";
    view! {
        <Dialog
            open=ed.open
            title="Edit Armory"
            description="Saving replaces this mission's entire armory. Faction keys are taken from the mission's ORBAT — the Event Hub matches them byte-for-byte, so they are chosen here, never typed."
            class="max-w-2xl"
        >
            {move || {
                let keys = ed.keys.get();
                if keys.is_empty() {
                    // No ORBAT and no stored rows: there is no key that would join, and a text box
                    // here would only manufacture one that does not. Say what is missing instead.
                    return view! {
                        <div class="rounded-xl border border-tactical-yellow/20 bg-tactical-yellow/5 p-4 text-label-md text-on-surface-variant">
                            <p class="mb-2 text-on-surface">"This mission has no ORBAT factions yet."</p>
                            <p>
                                "The armory is keyed by faction, and the Event Hub builds its faction list from the ORBAT slots a mission materialises when it is attached to an operation. Author the ORBAT in the Mission Creator and save a version first — otherwise every armory row would be filed under a key that matches nothing."
                            </p>
                        </div>
                    }
                        .into_any();
                }
                let tabs = keys
                    .iter()
                    .map(|k| {
                        let key = k.key.clone();
                        let key_active = k.key.clone();
                        let label = key_label(&k.key);
                        let unstorable = !key_storable(&k.key);
                        view! {
                            <button
                                type="button"
                                on:click=move |_| ed.faction.set(key.clone())
                                class=move || {
                                    cn(
                                        &[
                                            "rounded-full px-4 py-2 text-sm font-medium transition",
                                            if ed.faction.get() == key_active {
                                                "bg-white/10 text-on-surface"
                                            } else {
                                                "text-on-surface-variant hover:text-on-surface"
                                            },
                                            if unstorable { "line-through decoration-error/70" } else { "" },
                                        ],
                                    )
                                }
                            >
                                {label}
                            </button>
                        }
                    })
                    .collect_view();
                view! {
                    <div class="inline-flex flex-wrap rounded-full bg-white/5 p-1">{tabs}</div>
                    // Why the selected key is (or is not) usable. This is the T-346 / T-356 seam, and
                    // the operator has to be told which of the two it is rather than shown a 400.
                    {move || {
                        active_key()
                            .map(|k| {
                                if !key_storable(&k.key) {
                                    view! {
                                        <p class="mt-3 rounded-lg border border-error/20 bg-error-container/10 p-3 text-label-md text-error">
                                            "The endpoint refuses this faction key: it is blank or whitespace-padded. It is not trimmed here on purpose — the ORBAT side of the join stores its value verbatim too, so trimming one side would break a pair that agrees today. Fix the faction key in the Mission Creator, re-attach the mission, then author its armory."
                                        </p>
                                    }
                                        .into_any()
                                } else if k.source == KeySource::StoredOnly {
                                    view! {
                                        <p class="mt-3 rounded-lg border border-tactical-yellow/20 bg-tactical-yellow/5 p-3 text-label-md text-on-surface-variant">
                                            "This key is on stored armory rows but is in no ORBAT faction of the current version, so its items render here and on nothing else. Kept so saving does not delete them."
                                        </p>
                                    }
                                        .into_any()
                                } else {
                                    ().into_any()
                                }
                            })
                    }}

                    <div class="mt-5 space-y-2">
                        {move || {
                            let rows = visible();
                            if rows.is_empty() {
                                return view! {
                                    <p class="px-1 text-sm text-on-surface-variant/70">
                                        "No items for this faction yet."
                                    </p>
                                }
                                    .into_any();
                            }
                            rows.into_iter()
                                .map(|(i, r)| {
                                    let name = r.item_name.clone();
                                    let aria = format!("Remove {}", r.item_name);
                                    let category = r.category.clone();
                                    let qty = parse_qty(&r.quantity)
                                        .flatten()
                                        .map(|q| format!("x{q}"))
                                        .unwrap_or_else(|| "\u{221e}".to_string());
                                    view! {
                                        <div class="flex items-center gap-3 rounded-xl border border-white/10 bg-white/[0.02] px-4 py-3">
                                            <MaterialIcon name="inventory_2" class="text-on-surface-variant" />
                                            <span class="flex-1 truncate text-sm text-on-surface">{name}</span>
                                            {(!category.is_empty())
                                                .then(|| {
                                                    view! {
                                                        <span class="shrink-0 font-mono text-xs text-on-surface-variant/70">
                                                            {category}
                                                        </span>
                                                    }
                                                })}
                                            <span class="w-12 shrink-0 text-right font-mono text-xs text-tactical-yellow">
                                                {qty}
                                            </span>
                                            <button
                                                type="button"
                                                on:click=move |_| {
                                                    ed.rows
                                                        .update(|rows| {
                                                            if i < rows.len() {
                                                                rows.remove(i);
                                                            }
                                                        })
                                                }
                                                aria-label=aria
                                                class="flex size-7 shrink-0 items-center justify-center rounded-lg text-on-surface-variant transition hover:bg-error-alert/10 hover:text-error-alert"
                                            >
                                                <MaterialIcon name="close" class="text-base" />
                                            </button>
                                        </div>
                                    }
                                })
                                .collect_view()
                                .into_any()
                        }}
                    </div>

                    // Add — a row is composed here and appended whole, so no keystroke ever
                    // re-renders the list above it.
                    <div class="mt-4 flex flex-wrap items-center gap-2">
                        <input
                            prop:value=move || ed.new_name.get()
                            on:input=move |ev| ed.new_name.set(event_target_value(&ev))
                            placeholder="Item (e.g. M4A1)"
                            class=cn(&["min-w-0 flex-1", PILL])
                        />
                        <input
                            prop:value=move || ed.new_category.get()
                            on:input=move |ev| ed.new_category.set(event_target_value(&ev))
                            placeholder="Category"
                            class=cn(&["w-32", PILL])
                        />
                        <input
                            inputmode="numeric"
                            prop:value=move || ed.new_qty.get()
                            on:input=move |ev| ed.new_qty.set(event_target_value(&ev))
                            placeholder="Qty"
                            class=cn(&["w-20 font-mono", PILL])
                        />
                        <button
                            type="button"
                            on:click=add_row
                            prop:disabled=move || !can_add()
                            class="flex items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5 disabled:opacity-40"
                        >
                            <MaterialIcon name="add" class="text-base" />
                            "Add"
                        </button>
                    </div>
                    <p class="mt-2 px-1 text-label-md text-on-surface-variant/70">
                        "Blank quantity = unlimited (\u{221e})."
                    </p>
                }
                    .into_any()
            }}

            <div class="mt-6 border-t border-outline-variant/30 pt-4">
                <p class="mb-2 font-mono text-xs tracking-wider text-on-surface-variant/70 uppercase">
                    {move || {
                        let rows = ed.rows.get();
                        let mut factions: Vec<&String> = Vec::new();
                        for r in &rows {
                            if !factions.contains(&&r.faction) {
                                factions.push(&r.faction);
                            }
                        }
                        format!(
                            "{} item{} across {} faction{}",
                            rows.len(),
                            if rows.len() == 1 { "" } else { "s" },
                            factions.len(),
                            if factions.len() == 1 { "" } else { "s" },
                        )
                    }}
                </p>
                {move || {
                    draft_problem(&ed.rows.get())
                        .map(|p| {
                            view! {
                                <p class="mb-3 rounded-lg border border-error/20 bg-error-container/10 p-3 text-label-md text-error">
                                    {p}
                                </p>
                            }
                        })
                }}
                <button
                    type="button"
                    on:click=on_save
                    prop:disabled=move || {
                        ed.busy.get() || draft_problem(&ed.rows.get()).is_some()
                    }
                    class="w-full rounded-full bg-action py-4 text-base font-bold text-on-action shadow-[0_0_30px_rgba(59,130,246,0.4)] transition hover:bg-action/90 disabled:opacity-50"
                >
                    {move || {
                        if ed.busy.get() {
                            "Saving\u{2026}".to_string()
                        } else if ed.rows.get().is_empty() {
                            "Clear Armory".to_string()
                        } else {
                            "Save Armory".to_string()
                        }
                    }}
                </button>
            </div>
        </Dialog>
    }
}

fn body(m: MissionDetail, ed: ArmoryEditor, editable: bool) -> impl IntoView {
    let version_suffix = m
        .current_version
        .as_ref()
        .map(|v| format!(" — v{}", v.semver))
        .unwrap_or_default();
    let subtitle = format!(
        "by {} — Terrain: {}{}",
        m.author_name,
        terrain_label(&m.terrain),
        version_suffix
    );
    // Snapshot for the dialog, taken at click time (T-368) — the same "open from the row you
    // clicked" discipline `event_manager.rs` uses for `edit_orig`.
    let snapshot = m.clone();
    view! {
        <div class="mx-auto w-full max-w-3xl">
            <header class="mb-8 flex flex-wrap items-start justify-between gap-4">
                <div class="min-w-0">
                    <h1 class="mb-2 text-3xl font-bold text-on-surface">{m.title.clone()}</h1>
                    <p class="max-w-3xl text-on-surface-variant">{subtitle}</p>
                </div>
                {editable
                    .then(move || {
                        view! {
                            <button
                                type="button"
                                on:click=move |_| ed.open_for(&snapshot)
                                class="flex shrink-0 items-center gap-1.5 rounded-full border border-white/10 px-4 py-2 text-sm text-on-surface transition hover:bg-white/5"
                            >
                                <MaterialIcon name="inventory_2" class="text-base" />
                                "Edit Armory"
                            </button>
                        }
                    })}
            </header>
            <div class="relative flex flex-col gap-3 overflow-hidden rounded-xl p-6 glass">
                {dossier_body(&m)}
            </div>
        </div>
    }
}

/// Shared dossier content — used here and by the library slide-over (missions.rs), like React's
/// `MissionDossierBody`. T-159.25 adds the interactive Armory faction tabs (rows stay `Value`-read
/// so the R-api golden shape is untouched).
///
/// **Read-only, deliberately (T-368).** `missions.rs:789` renders this inside a `Sheet`, so an
/// editor here would be a form inside a slide-over that a Dialog then has to stack on top of — the
/// arrangement T-048 removed. The armory *write* path lives in [`armory_dialog`], reachable only
/// from the `/missions/:id` page. Note also that the faction tabs here are built from the armory's
/// own rows, so they render whatever key is stored, joining or not; the Event Hub's are built from
/// `orbat_slots` and do not. That asymmetry is why the editor derives its keys from the ORBAT.
///
/// **T-407 — briefing emptiness.** Whitespace-only briefings are not authored content (same rule
/// as `approvals.rs` and `event_hub::briefing_text`). They take the empty affordance rather than
/// a blank "Tactical Briefing" heading over empty `whitespace-pre-wrap` space.
fn tactical_briefing_text(briefing: Option<&str>) -> String {
    match briefing {
        Some(b) if !b.trim().is_empty() => b.to_string(),
        _ => "No briefing provided.".into(),
    }
}

pub fn dossier_body(m: &MissionDetail) -> impl IntoView {
    let briefing = tactical_briefing_text(m.briefing.as_deref());
    let v_badge = m
        .current_version
        .as_ref()
        .map(|v| view! { <span class=BADGE_TERTIARY>"v"{v.semver.clone()}</span> });
    // Armory faction tabs (React `factions = [...new Set(mission.armory.map(a => a.faction))]`).
    let armory = m.armory.clone();
    let factions: Vec<String> = {
        let mut seen = Vec::new();
        for a in &armory {
            if let Some(f) = a.get("faction").and_then(|v| v.as_str()) {
                if !seen.iter().any(|s: &String| s == f) {
                    seen.push(f.to_string());
                }
            }
        }
        seen
    };
    let faction_sel = RwSignal::new(None::<String>);
    // StoredValue keeps the resolver closure Copy (used by both the tabs and the rows renders).
    let default_faction = StoredValue::new(factions.first().cloned());
    let factions_for_tabs = factions.clone();
    let active_faction = move || faction_sel.get().or_else(|| default_faction.get_value());
    view! {
        <div class="space-y-8">
            <div class="flex flex-wrap gap-2">
                <span class=BADGE_PRIMARY>{game_mode_label(&m.game_mode).to_string()}</span>
                <span class=BADGE_NEUTRAL>{terrain_label(&m.terrain)}</span>
                {v_badge}
            </div>

            <section>
                <h3 class="mb-2 font-mono text-label-md tracking-widest text-on-surface-variant uppercase">
                    "Tactical Briefing"
                </h3>
                <p class="whitespace-pre-wrap text-body-md leading-relaxed text-on-surface-variant">
                    {briefing}
                </p>
            </section>

            <dl class="grid grid-cols-1 gap-8 md:grid-cols-2">
                {detail("Weather", m.weather.clone())} {detail("Time", m.time_of_day.clone())}
                {detail("Max Players", m.max_players.to_string())}
                {detail("Status", m.status.clone())}
            </dl>

            {(!factions.is_empty())
                .then(move || {
                    let af_tabs = active_faction;
                    let af_rows = active_faction;
                    let armory_rows = armory.clone();
                    view! {
                        <section>
                            <h3 class="mb-2 text-label-md text-on-surface-variant uppercase">
                                "The Armory"
                            </h3>
                            <div class="mb-3 flex gap-2">
                                {factions_for_tabs
                                    .iter()
                                    .map(|f| {
                                        let f_click = f.clone();
                                        let f_active = f.clone();
                                        view! {
                                            <button
                                                type="button"
                                                on:click=move |_| faction_sel.set(Some(f_click.clone()))
                                                class=move || {
                                                    crate::ui::cn(
                                                        &[
                                                            "rounded-lg px-3 py-1.5 text-label-md",
                                                            if af_tabs().as_deref() == Some(f_active.as_str()) {
                                                                "bg-primary text-on-primary"
                                                            } else {
                                                                "bg-surface-container text-on-surface-variant"
                                                            },
                                                        ],
                                                    )
                                                }
                                            >
                                                {f.clone()}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="grid gap-2">
                                {move || {
                                    let af = af_rows();
                                    armory_rows
                                        .iter()
                                        .filter(|a| {
                                            a.get("faction").and_then(|v| v.as_str())
                                                == af.as_deref()
                                        })
                                        .map(|item| {
                                            let name = item
                                                .get("item_name")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or_default()
                                                .to_string();
                                            let qty = match item.get("quantity") {
                                                Some(Value::Number(n)) => format!("x{n}"),
                                                _ => "∞".to_string(),
                                            };
                                            view! {
                                                <div class="flex justify-between rounded-lg border border-outline-variant/30 bg-surface-container p-3 text-label-md">
                                                    <span class="text-on-surface">{name}</span>
                                                    <span class="text-on-surface-variant">{qty}</span>
                                                </div>
                                            }
                                        })
                                        .collect_view()
                                }}
                            </div>
                        </section>
                    }
                })}
        </div>
    }
}

fn detail(label: &'static str, value: String) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-white/10 bg-white/5 p-4">
            <dt class="font-mono text-label-sm tracking-widest text-on-surface-variant uppercase">
                {label}
            </dt>
            <dd class="mt-1 text-headline-sm text-on-surface">{value}</dd>
        </div>
    }
}

/// This module is **not** `#[cfg(target_arch = "wasm32")]` in `main.rs`, so unlike `sse.rs` these
/// run under `cargo test -p website-frontend`. Everything tested here is the pure half of the
/// T-368 editor: the faction-key derivation, the guard mirror and the request body. The view half
/// is proven in the browser.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(faction: &str, item: &str, qty: &str) -> DraftRow {
        DraftRow {
            faction: faction.into(),
            item_name: item.into(),
            category: String::new(),
            quantity: qty.into(),
        }
    }

    /// The live path: Save Version omits the top-level `orbat[]` (T-062.1.1), so keys come from
    /// `editor.factions[].key` — copied verbatim into `orbat_slots.faction` by
    /// `derive_orbat_from_editor`.
    #[test]
    fn derives_editor_faction_keys_verbatim() {
        let p = json!({
            "editor": {
                "factions": [
                    {"key": "BLUFOR", "squadIds": ["sq1"]},
                    {"key": "  USA  ", "squadIds": ["sq2"]},
                ],
                "squads": [
                    {"id": "sq1", "slotIds": ["s1", "s2"]},
                    {"id": "sq2", "slotIds": ["s3"]},
                ],
                "slots": [{"id": "s1"}, {"id": "s2"}, {"id": "s3"}],
            }
        });
        // Padding is carried, not trimmed: the other side of the join carries it too (T-346/T-356).
        assert_eq!(orbat_faction_keys(&p), vec!["BLUFOR", "  USA  "]);
    }

    /// `materialize_slots` inserts one row per slot, so a faction whose squads resolve to no slot
    /// produces no `orbat_slots` row and appears in no Event Hub faction list. Offering it would be
    /// offering a key that joins to nothing.
    #[test]
    fn omits_factions_that_materialise_nothing() {
        let p = json!({
            "editor": {
                "factions": [
                    {"key": "EMPTY_SQUAD", "squadIds": ["sq1"]},
                    {"key": "NO_SQUADS", "squadIds": []},
                    {"key": "DANGLING_SQUAD", "squadIds": ["nope"]},
                    {"key": "DANGLING_SLOTS", "squadIds": ["sq2"]},
                    {"key": "REAL", "squadIds": ["sq3"]},
                ],
                "squads": [
                    {"id": "sq1", "slotIds": []},
                    {"id": "sq2", "slotIds": ["ghost"]},
                    {"id": "sq3", "slotIds": ["s1"]},
                ],
                "slots": [{"id": "s1"}],
            }
        });
        assert_eq!(orbat_faction_keys(&p), vec!["REAL"]);
    }

    /// An explicit non-empty `orbat` array wins the precedence test, exactly as
    /// `parse_orbat_template`'s early return does — the editor graph is then never consulted.
    #[test]
    fn explicit_orbat_array_wins_and_dedupes() {
        let p = json!({
            "orbat": [
                {"faction": "USA", "slots": [{"role": "SL"}]},
                {"faction": "USA", "slots": [{"role": "RTO"}]},
                {"faction": "RU", "slots": []},
            ],
            "editor": {
                "factions": [{"key": "NEVER_REACHED", "squadIds": ["sq1"]}],
                "squads": [{"id": "sq1", "slotIds": ["s1"]}],
                "slots": [{"id": "s1"}],
            }
        });
        // `RU` has no slot, so it materialises nothing; `USA` appears once.
        assert_eq!(orbat_faction_keys(&p), vec!["USA"]);
    }

    /// A malformed `orbat` fails to decode server-side and `unwrap_or_default()` falls through to
    /// the editor graph. Taking the other branch here would offer keys from a source the server
    /// never reads.
    #[test]
    fn malformed_orbat_falls_through_like_serde() {
        let editor = json!({
            "factions": [{"key": "BLUFOR", "squadIds": ["sq1"]}],
            "squads": [{"id": "sq1", "slotIds": ["s1"]}],
            "slots": [{"id": "s1"}],
        });
        // An object (every compiled golden mission's shape), an array of non-objects, and a
        // wrongly-typed field all fail `Top`'s decode.
        for bad in [
            json!({"blufor": {}}),
            json!(["BLUFOR"]),
            json!([{"faction": 7}]),
            json!([{"slots": "many"}]),
        ] {
            let p = json!({ "orbat": bad, "editor": editor });
            assert_eq!(
                orbat_faction_keys(&p),
                vec!["BLUFOR"],
                "should have fallen through for {bad}"
            );
        }
        // An EMPTY array also loses the precedence test, matching `if !top.orbat.is_empty()`.
        let p = json!({ "orbat": [], "editor": editor });
        assert_eq!(orbat_faction_keys(&p), vec!["BLUFOR"]);
    }

    #[test]
    fn no_payload_shape_yields_no_keys() {
        for p in [
            json!({}),
            json!({"editor": {}}),
            json!({"editor": {"factions": []}}),
            json!({"editor": {"factions": [{"key": "X"}]}}), // no squadIds at all
        ] {
            assert!(orbat_faction_keys(&p).is_empty(), "for {p}");
        }
    }

    /// The server's predicate, not an approximation of it (`handlers/missions.rs:769`, `:777`).
    #[test]
    fn key_storable_matches_the_server_guard() {
        assert!(key_storable("USA"));
        assert!(key_storable("BLU FOR")); // interior space is fine; only the ends are refused
        assert!(!key_storable(""));
        assert!(!key_storable("   "));
        assert!(!key_storable("  USA  "));
        assert!(!key_storable("USA "));
        assert!(!key_storable("\tUSA"));
    }

    #[test]
    fn parse_qty_blank_is_unlimited_and_junk_is_refused() {
        assert_eq!(parse_qty(""), Some(None));
        assert_eq!(parse_qty("   "), Some(None));
        assert_eq!(parse_qty("12"), Some(Some(12)));
        assert_eq!(parse_qty(" 12 "), Some(Some(12)));
        assert_eq!(parse_qty("-3"), Some(Some(-3))); // the server accepts it; we do not invent a rule
        assert_eq!(parse_qty("1.5"), None);
        assert_eq!(parse_qty("lots"), None);
    }

    #[test]
    fn draft_problem_flags_exactly_what_the_endpoint_refuses() {
        assert!(draft_problem(&[]).is_none());
        assert!(draft_problem(&[row("BLUFOR", "L85A3", "12")]).is_none());
        assert!(draft_problem(&[row("BLUFOR", "L85A3", "")]).is_none());

        assert!(draft_problem(&[row("BLUFOR", "  ", "1")])
            .unwrap()
            .contains("no name"));
        assert!(draft_problem(&[row("", "L85A3", "1")])
            .unwrap()
            .contains("blank faction key"));
        let padded = draft_problem(&[row("  USA  ", "L85A3", "1")]).unwrap();
        assert!(padded.contains("padded with whitespace"), "{padded}");
        // Names the reason it is refused rather than trimmed, so the operator fixes the ORBAT.
        assert!(padded.contains("Mission Creator"), "{padded}");
        assert!(draft_problem(&[row("BLUFOR", "L85A3", "many")])
            .unwrap()
            .contains("non-numeric quantity"));
    }

    /// `items` is always present — `{}` is a decode failure by design (T-315), because the handler's
    /// first statement is an unconditional DELETE.
    #[test]
    fn armory_body_always_states_items() {
        assert_eq!(armory_body(&[]), json!({ "items": [] }));
    }

    #[test]
    fn armory_body_sends_the_key_verbatim_and_orders_rows() {
        let rows = vec![row("BLUFOR", " L85A3 ", "12"), row("OPFOR", "AK-74", "")];
        let body = armory_body(&rows);
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        // Verbatim on both, for opposite reasons: the server trims the label and refuses a padded
        // key, and both of those decisions are its to make.
        assert_eq!(items[0]["faction"], json!("BLUFOR"));
        assert_eq!(items[0]["item_name"], json!(" L85A3 "));
        assert_eq!(items[0]["quantity"], json!(12));
        assert_eq!(items[0]["sort_order"], json!(0));
        // Blank quantity is `null`, which the column reads as unlimited — not 0.
        assert_eq!(items[1]["quantity"], Value::Null);
        assert_eq!(items[1]["sort_order"], json!(1));
    }

    /// T-407 — whitespace-only briefings must not render as blank "authored" prose under
    /// "Tactical Briefing". Same trim rule as `approvals.rs` / `event_hub::briefing_text`.
    #[test]
    fn tactical_briefing_trims_whitespace_only_to_empty_affordance() {
        for cleared in [None, Some(""), Some("   \n\n  "), Some("\t")] {
            assert_eq!(
                tactical_briefing_text(cleared),
                "No briefing provided.",
                "whitespace-only briefing must take the empty affordance ({cleared:?})"
            );
        }
        let authored = "Hold the ridge.\n\nSecond wave at H+20.";
        assert_eq!(tactical_briefing_text(Some(authored)), authored);
        // Leading/trailing space alone is not emptiness — only all-whitespace is.
        assert_eq!(tactical_briefing_text(Some(" Hold. ")), " Hold. ");
    }

    /// Guard against reverting to the pre-T-407 emptiness check (is_empty without trim).
    ///
    /// **T-494** — the filter-only ratchet stayed GREEN while a match-arm `!b.is_empty()`
    /// (no `trim`) made the behavioral trim test go RED. Ban both shapes, and pin the
    /// trim-aware arm so a rewrite cannot drop `trim` unnoticed.
    #[test]
    fn dossier_body_uses_trim_aware_briefing_helper() {
        const SRC: &str = include_str!("mission_overview.rs");
        assert!(
            SRC.contains("tactical_briefing_text(m.briefing.as_deref())"),
            "dossier_body must route briefing through tactical_briefing_text"
        );
        // concat! so this test body does not match itself.
        let old_filter = concat!(".filter(|b| !b.", "is_empty())");
        assert!(
            !SRC.contains(old_filter),
            "the pre-T-407 is_empty-only filter must not return — whitespace-only \
             briefings would blank the Tactical Briefing section again"
        );
        let old_arm = concat!("Some(b) if !b.", "is_empty()");
        assert!(
            !SRC.contains(old_arm),
            "match-arm !b.is_empty() without trim must not return on briefing paths — \
             whitespace-only briefings would blank the Tactical Briefing section again"
        );
        let trim_arm = concat!("Some(b) if !b.trim().", "is_empty()");
        assert!(
            SRC.contains(trim_arm),
            "tactical_briefing_text must keep the trim-aware match arm"
        );
    }
}
