//! T-159.20 — Save Version + Export commands for the Leptos Mission Creator.
//!
//! The compile itself is pure Rust in `map-engine-core` (`mission::compile`, unit-tested natively);
//! this module is the thin wasm glue that (a) reads the hosted `MissionDocCore`, (b) POSTs the Save
//! Version body through the authed `api_post`, (c) triggers the Export file download, and (d) installs
//! the `window.__editorCommands` smoke bridge (peer of `__missionDoc`).
//!
//! The doc/auth/mission-id live in a `thread_local` [`EditorCtx`] set from the editor's `on_load` —
//! the wasm-only `DocHandle` type can't cross the `#[cfg(target_arch = "wasm32")]` boundary into the
//! native view shell, so the buttons reach it through here instead of a hoisted handle. Every read is
//! taken as an owned snapshot before any `.await`, so no `RefCell` borrow is ever held across a yield.
//!
//! **T-417 — Class-R helpers** ([`compiled_export_text`], [`row_meta_missing_message`]) are ungated so
//! native `cargo test` can pin them. The wasm transport body lives in [`imp`].

/// Format the compact `/compiled` bytes for the Export Compiled download.
///
/// Returns the wire UTF-8 text **byte-identical** to `flatten_mod_document_json` / the compiled
/// route. Deliberately does **not** re-parse through `serde_json::Value` for a "pretty" download:
/// that round-trip is not byte-identical (whitespace), and without `preserve_order` it also
/// BTreeMap-sorts keys — the false "whitespace-only" comment on the old path set a trap for any
/// harness that compares against `GET /missions/:id/compiled`.
pub(crate) fn compiled_export_text(doc: &[u8]) -> Result<String, String> {
    String::from_utf8(doc.to_vec()).map_err(|e| format!("compiled document is not UTF-8: {e}"))
}

/// T-690 — the one-line author-facing summary of a compile's structured findings.
///
/// ## Why this replaces the toast rather than joining it
///
/// The compile used to say one of two things: "Downloaded the compiled mission document." or an
/// error. Everything it LEARNED — which authored values it discarded, and whose — was thrown away.
/// FNF v4's `init3DEN.sqf` is rated the single thing that framework does better than anyone else
/// precisely because Export is a build step there; TBD had the build step and none of the build
/// system. The findings now ride alongside the bytes
/// (`flatten::flatten_mod_document_json_with_diagnostics`) and are PUBLISHED to the T-655 validation
/// panel, which is the render surface and is not duplicated here. This string is only the pointer:
/// it names the count by severity so the toast says something true and finite, and sends the author
/// to the list rather than trying to be the list.
///
/// Empty findings → `None`: a clean compile gets the plain success message it always had, never a
/// celebratory "0 issues" (the panel's own empty-state doctrine).
///
/// Class-R / ungated so native `cargo test` can pin the wording without a browser (the
/// [`compiled_export_text`] precedent).
pub(crate) fn compile_diagnostics_summary(
    findings: &[map_engine_core::mission::validate::Finding],
) -> Option<String> {
    use map_engine_core::mission::validate::Severity;
    if findings.is_empty() {
        return None;
    }
    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let label = |n: usize, noun: &str| {
        if n == 1 {
            format!("1 {noun}")
        } else {
            format!("{n} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    for (sev, noun) in [
        (Severity::Error, "error"),
        (Severity::Warning, "warning"),
        (Severity::Info, "note"),
    ] {
        let n = count(sev);
        if n > 0 {
            parts.push(label(n, noun));
        }
    }
    Some(format!(
        "The compile reported {} — see the validation panel.",
        parts.join(" · ")
    ))
}

/// Author-facing message when [`ROW_META`] never arrived.
///
/// `authenticated == false` means the session is missing/expired (hydrate 401 never sets the row);
/// do not tell the author to "save a version first" — that path would also 401.
pub(crate) fn row_meta_missing_message(authenticated: bool) -> &'static str {
    if authenticated {
        "This mission has no saved server row yet — save a version first, then export."
    } else {
        "Sign in to export the compiled mission — your session is missing or expired."
    }
}

/// T-693 (MENU-SCEN-011) — format a [`MissionDocCore::merge_mission_payload_json`] report for the
/// author: a one-line counts summary + a per-row skipped list. Class-R / ungated so native
/// `cargo test` can pin the wording without a browser (the [`compiled_export_text`] precedent).
///
/// Returns `(summary, skipped_lines)`. `summary` names only the non-zero counts (a merge that added
/// nothing says so), and pluralizes. `skipped_lines` is one `"kind id — reason"` per tolerated
/// malformed row (empty when the merge was clean). A report string that does not parse yields a
/// single skipped line naming that, so the caller never silently swallows a broken report.
pub(crate) fn format_merge_report(report_json: &str) -> (String, Vec<String>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(report_json) else {
        return (
            "Merge produced no readable report.".to_string(),
            vec![format!("report — could not parse: {report_json}")],
        );
    };
    let n = |k: &str| v.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
    let plural = |count: u64, noun: &str| {
        if count == 1 {
            format!("1 {noun}")
        } else {
            format!("{count} {noun}s")
        }
    };
    let mut parts: Vec<String> = Vec::new();
    let slots = n("slots_added");
    if slots > 0 {
        parts.push(plural(slots, "slot"));
    }
    // Squads / factions: report merged + created distinctly so "grew a side" vs "added a side" reads.
    let sq_created = n("squads_created");
    let sq_merged = n("squads_merged");
    if sq_created > 0 {
        parts.push(plural(sq_created, "squad"));
    }
    if sq_merged > 0 {
        parts.push(format!(
            "{} merged into existing",
            plural(sq_merged, "squad")
        ));
    }
    let fac_created = n("factions_created");
    if fac_created > 0 {
        parts.push(plural(fac_created, "faction"));
    }
    for (key, noun) in [
        ("vehicles_added", "vehicle"),
        ("entities_added", "object"),
        ("zones_added", "zone"),
        ("triggers_added", "trigger"),
        ("compositions_added", "composition"),
        ("markers_added", "marker"),
    ] {
        let c = n(key);
        if c > 0 {
            parts.push(plural(c, noun));
        }
    }

    let summary = if parts.is_empty() {
        "Merge added nothing — the source mission had no mergeable content.".to_string()
    } else {
        format!("Merged {}.", parts.join(", "))
    };

    let skipped: Vec<String> = v
        .get("skipped")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|s| {
                    let kind = s
                        .get("kind")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("row");
                    let id = s
                        .get("id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    let reason = s
                        .get("reason")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("malformed");
                    if id.is_empty() {
                        format!("{kind} — {reason}")
                    } else {
                        format!("{kind} {id} — {reason}")
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    (summary, skipped)
}

/* ───────────────────────── T-698 — clipboard exporters (3den E5) ─────────────────────────
 *
 * WOG's `wog3_3den` Log entries and 3den Enhanced both ship clipboard exporters, and the
 * milsim-relevant one is the grid reference: a mission maker reads a grid off the editor and types
 * it into a briefing or says it on the radio. Three exporters ship here — grid position, classnames,
 * and a selection summary — because the data is already in the document and the target is
 * `navigator.clipboard`.
 *
 * ## The grid format is NOT invented here
 *
 * T-667 already shipped grid-reference labels on the map-pane edges, and their text comes from
 * `eden_toolbelt::grid_ref_3digit` — the 3-digit hundreds-of-metres Arma ref, one axis at a time.
 * [`format_grid_ref`] CALLS that function rather than re-deriving the rule. This is the whole point:
 * if the furniture prints `032` down the top edge and this exporter put `0320` on the clipboard, the
 * two would disagree and the exporter would be a confident wrong answer — worse than no exporter,
 * because a briefing built on it looks authoritative. The agreement is pinned by
 * `the_exporter_grid_ref_is_the_map_furnitures_own_label_text`, which reads the labels
 * `edge_eastings` / `edge_northings` actually emit and compares them to this exporter's output for
 * an entity standing on that line. The separator is a single space — the `mortar.rs` "012 020"
 * convention this codebase already writes.
 *
 * ## Everything below the wasm boundary is a pure string function
 *
 * Resolution (ids → rows) and composition (rows → clipboard text) are ungated so native
 * `cargo test` pins them by VALUE, the [`compiled_export_text`] precedent. Only the two things that
 * genuinely need a browser — reading the live selection and the clipboard write itself — live in
 * [`imp`].
 */

/// T-698 — one selected entity, resolved out of the document into the four things a clipboard
/// exporter needs: which id, what kind of thing it is, what it is made of, and where it stands.
///
/// `kind` is the same three-way split `editor_ops::capture_selection_entities` uses, and for the
/// same reason: the document stores slots, vehicles and objects in three different maps with three
/// different classname keys, and a reader that guessed one would silently export nothing for the
/// other two.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SelectedEntity {
    /// The app-side entity id (the selection's own vocabulary).
    pub id: String,
    /// `"slot"` | `"vehicle"` | `"object"`.
    pub kind: &'static str,
    /// The Enfusion prefab path — a slot's `assetId`, a vehicle's or object's `resourceName`.
    /// Empty when the entity expresses no asset (a slot placed from the "+ button", which the
    /// compile resolves to a faction default later); an empty classname is REPORTED, never silently
    /// exported as a blank line.
    pub classname: String,
    /// The authored human label — a slot's role (or tag), an object's alias. Empty when unset.
    pub label: String,
    /// World easting, metres.
    pub x: f64,
    /// World northing, metres.
    pub y: f64,
}

/// T-698 — resolve selection ids against the document's own maps, in selection order.
///
/// `slots_json` is `MissionDocCore::slots_json` (an id→row map) and `small_maps_json` is
/// `MissionDocCore::small_maps_json`, whose `vehiclesById` / `entitiesById` hold the other two
/// kinds. The lookup order (slot → vehicle → object) mirrors `editor_ops::capture_selection_entities`
/// so the exporters and the composition capture agree about what an id IS.
///
/// Ids the document does not know are DROPPED rather than exported as a placeholder: a stale id in
/// the selection is not an entity, and inventing a `000 000` row for it would put a false grid in a
/// briefing. The caller compares `out.len()` against the id count when it needs to know.
pub(crate) fn resolve_selected_entities(
    slots_json: &str,
    small_maps_json: &str,
    ids: &[String],
) -> Vec<SelectedEntity> {
    let slots = serde_json::from_str::<serde_json::Value>(slots_json).unwrap_or_default();
    let small = serde_json::from_str::<serde_json::Value>(small_maps_json).unwrap_or_default();
    let vehicles = small.get("vehiclesById").cloned().unwrap_or_default();
    let entities = small.get("entitiesById").cloned().unwrap_or_default();

    let s = |row: &serde_json::Value, k: &str| {
        row.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    // The document stores world position under `position.{x,y}` for all three kinds (the shape
    // `capture_selection_entities` reads). `y` is the NORTHING — the map is north-up and the engine
    // carries elevation separately in `z`.
    let axis = |row: &serde_json::Value, k: &str| {
        row.get("position")
            .and_then(|p| p.get(k))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };

    let mut out = Vec::new();
    for id in ids {
        let (kind, row) = if let Some(r) = slots.get(id) {
            ("slot", r)
        } else if let Some(r) = vehicles.get(id) {
            ("vehicle", r)
        } else if let Some(r) = entities.get(id) {
            ("object", r)
        } else {
            continue;
        };
        let classname = if kind == "slot" {
            s(row, "assetId")
        } else {
            s(row, "resourceName")
        };
        let label = match kind {
            "slot" => {
                let role = s(row, "role");
                if role.is_empty() {
                    s(row, "tag")
                } else {
                    role
                }
            }
            "object" => s(row, "alias"),
            _ => String::new(),
        };
        out.push(SelectedEntity {
            id: id.clone(),
            kind,
            classname,
            label,
            x: axis(row, "x"),
            y: axis(row, "y"),
        });
    }
    out
}

/// The readable leaf of an Enfusion prefab path:
/// `{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et` → `UAZ469`. Anything that is not a
/// prefab path comes back unchanged, so this can never turn a name into a wrong one.
pub(crate) fn prefab_leaf(classname: &str) -> String {
    let after_guid = classname.rsplit('}').next().unwrap_or(classname);
    let file = after_guid.rsplit('/').next().unwrap_or(after_guid);
    file.strip_suffix(".et").unwrap_or(file).to_string()
}

/// What to CALL an entity in a human-facing line: the authored label if there is one, else the
/// prefab leaf, else the raw id. Never empty — a nameless row in a summary is a row the reader
/// cannot match back to the map.
pub(crate) fn entity_display_name(e: &SelectedEntity) -> String {
    if !e.label.is_empty() {
        return e.label.clone();
    }
    let leaf = prefab_leaf(&e.classname);
    if leaf.is_empty() {
        e.id.clone()
    } else {
        leaf
    }
}

/// `1 slot` / `3 slots` — the pluralising counter the exporter messages share.
pub(crate) fn count_noun(n: usize, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {plural}")
    }
}

/// T-698 — the six-figure grid reference of a world position, in the map furniture's own format.
///
/// **Both halves come from [`crate::eden_toolbelt::grid_ref_3digit`]** — the T-667 formatter whose
/// output is literally the text printed on the map-pane edge labels. Do not re-derive the rule here:
/// a second convention that disagreed with the on-screen labels would be the confident-wrong-answer
/// defect this exporter exists to avoid. Separator is one space (`mortar.rs`'s "012 020").
pub(crate) fn format_grid_ref(x: f64, y: f64) -> String {
    format!(
        "{} {}",
        crate::eden_toolbelt::grid_ref_3digit(x),
        crate::eden_toolbelt::grid_ref_3digit(y)
    )
}

/// T-698 exporter 1 — **grid position**, the milsim-relevant one.
///
/// A single selection yields the BARE reference (`"032 048"`) and nothing else, because that is what
/// gets typed into a briefing line or read over the radio — a decorated string would have to be
/// hand-edited at the paste site every time. A multi-selection yields one line per entity,
/// `"<grid>  <name>"`, so the list stays attributable.
pub(crate) fn grid_position_text(entities: &[SelectedEntity]) -> String {
    match entities {
        [] => String::new(),
        [one] => format_grid_ref(one.x, one.y),
        many => many
            .iter()
            .map(|e| format!("{}  {}", format_grid_ref(e.x, e.y), entity_display_name(e)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// T-698 exporter 2 — **classnames**, one prefab path per line in selection order.
///
/// Returns `(text, skipped)`. Duplicates are KEPT: two of the same prefab is two entities, and a
/// silent dedupe would change the count a reader pastes into a config. Entities with no classname
/// contribute no line — and are COUNTED into `skipped` so the caller can say so, rather than
/// emitting a blank line that reads as a real (empty) classname.
pub(crate) fn classnames_text(entities: &[SelectedEntity]) -> (String, usize) {
    let mut lines: Vec<&str> = Vec::new();
    let mut skipped = 0usize;
    for e in entities {
        if e.classname.is_empty() {
            skipped += 1;
        } else {
            lines.push(&e.classname);
        }
    }
    (lines.join("\n"), skipped)
}

/// T-698 exporter 3 — **selection summary**, a human-readable digest.
///
/// A headline (`"3 entities selected — 2 slots, 1 vehicle"`) plus one `- name (kind) at grid —
/// classname` line per entity. The grid on every row is [`format_grid_ref`]'s, so the summary and
/// the grid exporter can never disagree about where something stands. A missing classname is
/// spelled `(no classname)` rather than left blank — the digest says what it does not know.
pub(crate) fn selection_summary_text(entities: &[SelectedEntity]) -> String {
    if entities.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (kind, singular, plural) in [
        ("slot", "slot", "slots"),
        ("vehicle", "vehicle", "vehicles"),
        ("object", "object", "objects"),
    ] {
        let n = entities.iter().filter(|e| e.kind == kind).count();
        if n > 0 {
            parts.push(count_noun(n, singular, plural));
        }
    }
    let mut out = format!(
        "{} selected — {}",
        count_noun(entities.len(), "entity", "entities"),
        parts.join(", ")
    );
    for e in entities {
        let class = if e.classname.is_empty() {
            "(no classname)"
        } else {
            &e.classname
        };
        out.push_str(&format!(
            "\n- {} ({}) at {} — {}",
            entity_display_name(e),
            e.kind,
            format_grid_ref(e.x, e.y),
            class
        ));
    }
    out
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::RefCell;

    use leptos::prelude::{GetUntracked, RwSignal, Set};
    use leptos::task::spawn_local;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    use map_engine_core::mission::compile::{compile_export, compile_payload, version_body};
    use map_engine_core::mission::flatten::{
        flatten_mod_document_json_with_diagnostics, MissionMeta,
    };
    use map_engine_core::mission::validate::Finding;

    /// T-690 — what a compile hands the command layer: the download text and the structured
    /// findings, from one compile. Aliased so the entry point's signature stays on one line, which
    /// is what `class_r_source_forbids_value_pretty_on_compiled_export` locates it by.
    type CompiledWithDiagnostics = (String, Vec<Finding>);

    use crate::auth::AuthStore;
    use crate::mission_doc::DocHandle;

    use super::{
        compiled_export_text, format_merge_report, resolve_selected_entities,
        row_meta_missing_message, SelectedEntity,
    };

    /// Editor context shared from `mission_editor::on_load` to the Save/Export buttons. `AuthStore` is
    /// `Copy`; `doc` is the same shared `Rc` the persistence layer may swap on IDB restore (reads see the
    /// swap). Held in a `thread_local` because `DocHandle` is `!Send` + wasm-only.
    struct EditorCtx {
        doc: DocHandle,
        auth: AuthStore,
        mission_id: String,
        /// T-159.26 — the adopted server semver signal, updated on a successful Save (the saved
        /// version becomes the version local now derives from).
        current_semver: RwSignal<Option<String>>,
    }

    thread_local! {
        static EDITOR_CTX: RefCell<Option<EditorCtx>> = const { RefCell::new(None) };

        /// **T-243 — the mission ROW, as `GET /missions/:id` last served it.**
        ///
        /// Deliberately NOT part of [`EditorCtx`]: `set_ctx` runs synchronously at mount, and this
        /// arrives later from `mission_hydrate::hydrate_from_server`'s `await`. Folding it in would
        /// have meant either an `Option` field nobody could keep honest or an ordering assumption
        /// between a mount and a fetch.
        ///
        /// **`None` is load-bearing and must stay refusable.** It means the row never arrived — a
        /// local-only / non-UUID id (the `smoke` gate route), a 404, an offline boot, **or a 401 /
        /// expired session** (hydrate never got the row; see [`row_meta_missing_message`]). There is
        /// no server document for those, and `MissionMeta::default()` would happily compile one with a
        /// blank author and `playerRange: [1, 1]`. Emitting that under the name "the document the game
        /// server will receive" is the confident-wrong-answer failure this whole ticket exists to
        /// avoid, so [`export_compiled_now`] refuses instead — and names auth failure separately from
        /// "no saved version".
        static ROW_META: RefCell<Option<MissionMeta>> = const { RefCell::new(None) };
    }

    /// Record the mission row for the server-truth Export (T-243). Called by
    /// `mission_hydrate::hydrate_from_server` on every successful `GET /missions/:id`, including the
    /// fresh-mission and warm-IDB branches — the row is what the compile needs, and it is equally real
    /// whichever way the payload half was resolved.
    pub fn set_row_meta(detail: &crate::dto::MissionDetail) {
        ROW_META.with(|r| *r.borrow_mut() = Some(detail.compiled_meta()));
    }

    /// Install the editor context (called once from `on_load`, after the doc is seeded/registered).
    pub fn set_ctx(
        doc: DocHandle,
        auth: AuthStore,
        mission_id: String,
        current_semver: RwSignal<Option<String>>,
    ) {
        EDITOR_CTX.with(|c| {
            *c.borrow_mut() = Some(EditorCtx {
                doc,
                auth,
                mission_id,
                current_semver,
            });
        });
    }

    /// The current-semver signal, for the save-success adopt. `None` when the editor isn't mounted.
    fn semver_signal() -> Option<RwSignal<Option<String>>> {
        EDITOR_CTX.with(|c| c.borrow().as_ref().map(|ctx| ctx.current_semver))
    }

    /// An owned snapshot of everything a command needs — taken synchronously so no borrow spans an
    /// `.await`. `None` when the editor isn't mounted / the doc Option is empty.
    struct Snap {
        small: String,
        slots: String,
        auth: AuthStore,
        mission_id: String,
    }

    fn snapshot() -> Option<Snap> {
        EDITOR_CTX.with(|c| {
            let ctx = c.borrow();
            let ctx = ctx.as_ref()?;
            let doc = ctx.doc.borrow();
            let core = doc.as_ref()?;
            Some(Snap {
                small: core.small_maps_json(),
                slots: core.slots_json(),
                auth: ctx.auth,
                mission_id: ctx.mission_id.clone(),
            })
        })
    }

    /// **T-243 — download the document the game server will actually receive.**
    ///
    /// Returns the compact compiled mod document (byte-identical to `GET /compiled`'s body when the
    /// local doc matches the saved version), or a message fit to show an author.
    ///
    /// ## Why this exists at all
    ///
    /// `GET /missions/:id/compiled` takes a `ServiceAuth` (`handlers::missions::get_compiled_mission`)
    /// — it answers game servers, not browsers — so an author has no way to fetch it. Until this,
    /// "Export JSON" downloaded [`compile_export`]'s `MissionExport` envelope: the editor SUPERSET,
    /// `{exportFormatVersion, missionId, title, …, payload}`, whose `payload` is the editor graph. That
    /// is the right file for re-importing into the editor and it is **not** the mod document — it has
    /// no `slots[]`, no `orbat`, no `radioPlan`, no `winConditions`, and the mod cannot load it. So
    /// there was no way to see the compiled document before a game server did.
    ///
    /// ## Why the answer can be trusted
    ///
    /// This runs `flatten_mod_document_json` — the same `map-engine-core` compile `/compiled` runs,
    /// over the same two inputs:
    ///
    ///   * the **row**, from `GET /missions/:id` ([`ROW_META`]), which is where the server gets
    ///     `author`, `maxPlayers` and the fallback time/weather;
    ///   * the **save-shaped payload** (`include_orbat = false`) — byte-for-byte what
    ///     `POST /missions/:id/versions` stores and therefore what `/compiled` later reads. `orbat` is
    ///     omitted for the same reason the save omits it: the flatten derives its own from `editor`,
    ///     and including it would put a key in the preview's input that the stored version never has.
    ///
    /// Their agreement is not asserted here — it is pinned natively, on both halves, by
    /// `website-api`'s `client_twin_is_byte_identical_to_the_compiled_route` (the compile) and
    /// `dto::r_api::compiled_meta_is_the_row_the_server_compiles_from` (the row).
    ///
    /// ## The one honest difference, and it is the point
    ///
    /// `/compiled` serves the last **saved** version; this compiles the document **as it is now**,
    /// unsaved edits included. That is what makes it useful — you can see what a save would ship
    /// before shipping it — but it means a dirty document previews something the server does not yet
    /// have. The caller says so in the toast rather than hiding it.
    ///
    /// # Errors
    /// Returns a display message when the row never arrived (local-only id, 404, offline, **401 /
    /// expired session** — see [`ROW_META`] + [`row_meta_missing_message`]), when the editor is not
    /// mounted, or when the compile refuses (no placed slots is the common one, and it is the same
    /// `409` a game server would get).
    pub fn compiled_document_json() -> Result<String, String> {
        compiled_document_json_with_diagnostics().map(|(text, _)| text)
    }

    /// T-690 — the compile, with the structured result it produced ALONGSIDE the bytes.
    ///
    /// This is the body [`compiled_document_json`] projects: one compile, one document, one finding
    /// list. Splitting it the other way round (a second compile just for the findings) is the shape
    /// `flatten_mod_document_json_full` exists to forbid — two compiles are two things that can
    /// disagree about what was compiled.
    ///
    /// The findings are `map_engine_core::mission::validate::Finding`s — the T-657 vocabulary
    /// (`rule_id` / `severity` / `primitive` / `message` / `subject` / `subject_id`), reused so the
    /// T-655 panel renders a compile finding through exactly the same row as a validation finding
    /// and click-to-select works on both.
    ///
    /// # Errors
    /// Same three refusals as [`compiled_document_json`] — a missing row, an unmounted editor, or a
    /// compile that produced no document. A FINDING is never one of them.
    pub fn compiled_document_json_with_diagnostics() -> Result<CompiledWithDiagnostics, String> {
        let Some(snap) = snapshot() else {
            return Err("Editor not ready".to_string());
        };
        let authenticated = snap.auth.access_token.get_untracked().is_some()
            && snap.auth.user.get_untracked().is_some();
        let Some(mut meta) = ROW_META.with(|r| r.borrow().as_ref().map(clone_meta)) else {
            return Err(row_meta_missing_message(authenticated).to_string());
        };
        // Carry the mission id from the route when the row's is blank, matching the envelope export's
        // `meta.id`-then-route fallback (`compile_export`). `mission_doc_id` in the flatten normalizes
        // whatever lands here into the schema's id space either way.
        if meta.id.is_empty() {
            meta.id = snap.mission_id.clone();
        }
        let payload = compile_payload(&snap.small, &snap.slots, false);
        let payload_bytes = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        let meta_bytes = serde_json::to_vec(&meta).map_err(|e| e.to_string())?;

        let (doc, findings) =
            flatten_mod_document_json_with_diagnostics(&meta_bytes, &payload_bytes)?;
        // T-417 — ship the compact wire bytes (byte-identical to `/compiled`). Do not re-parse to
        // `serde_json::Value` for a "pretty" download — that is not whitespace-only vs the route.
        compiled_export_text(&doc).map(|text| (text, findings))
    }

    /// `MissionMeta` is a plain data carrier in core and deliberately not `Clone` (it is an input type
    /// built once per compile); this is the local copy out of the `thread_local` so no borrow is held
    /// across the compile below.
    fn clone_meta(m: &MissionMeta) -> MissionMeta {
        MissionMeta {
            id: m.id.clone(),
            title: m.title.clone(),
            author: m.author.clone(),
            terrain: m.terrain.clone(),
            custom_terrain_name: m.custom_terrain_name.clone(),
            max_players: m.max_players,
            time_of_day: m.time_of_day.clone(),
            weather_preset: m.weather_preset.clone(),
        }
    }

    /// Trigger the server-truth download and report the outcome (T-243).
    ///
    /// **T-690 — this is where the compile stops being a pass/fail.** The compile now returns a
    /// structured finding list alongside the bytes; this publishes that list to the T-655 validation
    /// panel ([`crate::validation_panel::publish_compile_findings`]) and lets the toast shrink back
    /// to what a toast is good at — a one-line verdict with a pointer. The panel is the render
    /// surface and is deliberately not duplicated here.
    ///
    /// The publish happens even when the list is EMPTY, and that is load-bearing: a clean compile
    /// must CLEAR the previous compile's findings, or the panel would show a stale build report
    /// after the author fixed everything in it.
    ///
    /// `toasts` is resolved at component setup by the caller — `use_toasts()` is an `expect_context`
    /// and would panic from a DOM handler, the `RowMirror` precedent.
    pub fn export_compiled_now(toasts: crate::toast::Toasts) {
        let mission_id = EDITOR_CTX
            .with(|c| c.borrow().as_ref().map(|ctx| ctx.mission_id.clone()))
            .unwrap_or_default();
        match compiled_document_json_with_diagnostics() {
            Ok((json, findings)) => {
                let filename = format!("mission-{mission_id}.compiled.json");
                if let Err(e) = download_json(&filename, &json) {
                    toasts.error(format!("Could not start the download: {e:?}"));
                    return;
                }
                // The findings reach the panel through the engine's own row type, so a compile
                // finding renders — and click-to-selects on its `subject_id` — exactly like a
                // validation finding. Published AFTER the download starts: a diagnostic is not a
                // refusal, and the file the author asked for is not held back by one.
                let summary = super::compile_diagnostics_summary(&findings);
                crate::validation_panel::publish_compile_findings(
                    findings
                        .iter()
                        .map(crate::validation_panel::PanelFinding::from_finding)
                        .collect(),
                );
                // Naming the staleness is the whole reason this is a toast and not a silent download:
                // the file is the CURRENT document, which is only what a game server would fetch once
                // this state is saved.
                let staleness = if crate::mission_history::is_dirty() {
                    Some(
                        "Downloaded the compiled mission document — compiled from your unsaved \
                         changes, so the server still serves the last saved version.",
                    )
                } else {
                    None
                };
                match (staleness, summary) {
                    (Some(stale), Some(s)) => toasts.message(format!("{stale} {s}")),
                    (Some(stale), None) => toasts.message(stale.to_string()),
                    (None, Some(s)) => {
                        toasts.message(format!("Downloaded the compiled mission document. {s}"))
                    }
                    (None, None) => toasts.success("Downloaded the compiled mission document."),
                }
            }
            Err(e) => toasts.error(e),
        }
    }

    /// Export the current mission as a downloaded `mission-<id>.json` (React `exportJson`): compile with
    /// `orbat` included, wrap in the `MissionExport` envelope, pretty-print, and trigger the browser
    /// download. `version` is the current semver (envelope `version` field).
    ///
    /// **This is the editor SUPERSET, not the mod document** — it round-trips back into the editor and
    /// the mod cannot load it. The compiled document an author ships is
    /// [`export_compiled_now`] (T-243); both are kept because they answer different questions.
    pub fn export_now(version: &str) {
        let Some(snap) = snapshot() else {
            return;
        };
        let payload = compile_payload(&snap.small, &snap.slots, true);
        let doc = compile_export(
            &payload,
            &snap.small,
            &snap.mission_id,
            version,
            &js_date_iso(),
        );
        let json = serde_json::to_string_pretty(&doc).unwrap_or_default();
        let filename = format!("mission-{}.json", snap.mission_id);
        let _ = download_json(&filename, &json);
    }

    /// Save a new immutable version (React `saveVersion`): compile with `orbat` omitted (the server
    /// re-derives), POST `{semver, editor_notes, payload}` to `/missions/:id/versions`, and reflect the
    /// outcome in `status`. 409 = dup semver, 413 = too large, 401 = not signed in.
    ///
    /// T-181.44 — a 400 from `create_version` carries the *list* of things wrong with the payload
    /// (schema violations plus the wire-safety findings), and this used to collapse all of it into
    /// "Save failed (400)". `findings` takes the per-problem lines so the dialog can name them; the
    /// headline stays short because `status` is also rendered in the top strip.
    pub fn save_now(
        semver: String,
        notes: String,
        status: RwSignal<String>,
        findings: RwSignal<Vec<String>>,
    ) {
        findings.set(Vec::new());
        let Some(snap) = snapshot() else {
            status.set("Editor not ready".to_string());
            return;
        };
        let payload = compile_payload(&snap.small, &snap.slots, false);
        let body = version_body(&semver, &notes, &payload);
        let auth = snap.auth;
        let path = format!("/missions/{}/versions", snap.mission_id);
        let mission_id = snap.mission_id.clone();
        status.set(format!("Saving v{semver}…"));
        spawn_local(async move {
            match crate::client::api_post::<serde_json::Value>(auth, &path, body).await {
                Ok(_) => {
                    status.set(format!("Saved v{semver}"));
                    // T-159.26 — the saved version is now what local derives from: clear the dirty
                    // flag and update the current-semver signal so a later Export/adopt uses it.
                    // (T-370 removed the `editor_session::mark_adopted` call that sat here: T-352
                    // had already emptied it, and T-223 replaced the semver marker it once wrote
                    // with the content test in `mission_hydrate::classify_local`.)
                    crate::mission_history::set_dirty(false);
                    // T-191 fix — expire the conflict backup pair. This 201 is the one moment those
                    // whole-document IDB records stop being anybody's last copy, and nothing else ever
                    // deleted them: they accumulated one doc per mission ever conflicted, forever, while
                    // `__missionBackup.has()` kept offering a weeks-old document that a restore would
                    // swap over current work. Rationale in `mission_hydrate::clear_local_backups`.
                    crate::mission_hydrate::clear_local_backups(&mission_id);
                    if let Some(sig) = semver_signal() {
                        sig.set(Some(semver.clone()));
                    }
                }
                Err((409, _)) => status.set(format!("Version {semver} already exists")),
                Err((413, _)) => status.set("Payload too large".to_string()),
                Err((401, _)) => status.set("Sign in to save".to_string()),
                Err((s, msg)) => {
                    let (head, rows) = crate::client::split_error_lines(msg.as_deref());
                    let head = head.filter(|h| !h.is_empty());
                    status.set(match (&head, rows.len()) {
                        (Some(h), 0) => format!("Save rejected ({s}): {h}"),
                        (Some(h), n) => format!("Save rejected ({s}): {h} — {n} problem(s) below"),
                        (None, _) => format!("Save failed ({s})"),
                    });
                    findings.set(rows);
                }
            }
        });
    }

    /// Current wall-clock ISO-8601 (`new Date().toISOString()`) — the one clock read, kept out of the
    /// pure core (which takes `exported_at` as a param, so the smoke can pin it).
    fn js_date_iso() -> String {
        js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default()
    }

    /// The `Blob → URL.createObjectURL → <a download> → click → revokeObjectURL` download dance
    /// (mirrors the React `exportJson` DOM path).
    pub(crate) fn download_json(filename: &str, contents: &str) -> Result<(), JsValue> {
        let win = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = win
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        let parts = js_sys::Array::new();
        parts.push(&JsValue::from_str(contents));
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/json");
        let blob = web_sys::Blob::new_with_str_sequence_and_options(parts.as_ref(), &opts)?;

        let url = web_sys::Url::create_object_url_with_blob(&blob)?;

        let anchor = document
            .create_element("a")?
            .dyn_into::<web_sys::HtmlAnchorElement>()?;
        anchor.set_href(&url);
        anchor.set_download(filename);
        let el: &web_sys::HtmlElement = anchor.as_ref();
        el.click();

        web_sys::Url::revoke_object_url(&url)?;
        Ok(())
    }

    /// T-693 (MENU-SCEN-011) — one row in the "Merge Mission…" picker: a mission the author can merge
    /// FROM. `id` feeds [`merge_mission_now`]; `title` is the label.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct MissionPick {
        /// The mission id (`GET /missions/:id`).
        pub id: String,
        /// The mission's display title.
        pub title: String,
    }

    /// T-693 — the author's OTHER missions, for the "Merge Mission…" picker.
    ///
    /// Reuses the SPA's own list client (`GET /missions?scope=mine`, the same call
    /// `missions::MissionLibraryPage` makes) through [`crate::client::api_get`], which owns the
    /// single-flight refresh — so this adds no second auth path. The CURRENT mission is filtered out
    /// (you cannot merge a mission into itself). Titles come straight off the `MissionCard` rows.
    ///
    /// # Errors
    /// A display string when the list request fails (offline / 401 / server error).
    pub async fn other_missions(
        auth: AuthStore,
        exclude_id: &str,
    ) -> Result<Vec<MissionPick>, String> {
        use crate::dto::{MissionCard, Paginated};
        match crate::client::api_get::<Paginated<MissionCard>>(auth, "/missions?scope=mine").await {
            Ok(page) => Ok(page
                .data
                .into_iter()
                .filter(|c| c.id != exclude_id)
                .map(|c| MissionPick {
                    id: c.id,
                    title: c.title,
                })
                .collect()),
            Err((401, _)) => Err("Sign in to list your missions.".to_string()),
            Err((s, msg)) => Err(match msg {
                Some(m) if !m.is_empty() => format!("Could not load your missions ({s}): {m}"),
                _ => format!("Could not load your missions ({s})."),
            }),
        }
    }

    /// T-693 (MENU-SCEN-011) — merge another mission (`source_id`) into the CURRENT document.
    ///
    /// Fetches the source's latest payload (`GET /missions/:id` → `current_version.json_payload`, the
    /// same superset [`crate::mission_hydrate`] loads), runs [`MissionDocCore::merge_mission_payload_json`]
    /// on the hosted doc, and reports the outcome via toasts: a counts line plus, when the merge
    /// tolerated malformed rows, an error toast listing each skipped row (the T-657 totality contract
    /// made visible to the author). `offset` is the optional template placement delta.
    ///
    /// The whole merge is one undo step (the core opens one txn), so a mistaken merge is one Ctrl+Z.
    /// The borrow of the hosted `MissionDocCore` is taken and released synchronously AFTER the
    /// `.await` — never held across the yield (the module's borrow-safety contract).
    pub fn merge_mission_now(
        source_id: String,
        offset: Option<(f64, f64)>,
        toasts: crate::toast::Toasts,
    ) {
        let Some((doc, auth)) =
            EDITOR_CTX.with(|c| c.borrow().as_ref().map(|ctx| (ctx.doc.clone(), ctx.auth)))
        else {
            toasts.error("Editor not ready.");
            return;
        };
        let path = format!("/missions/{source_id}");
        spawn_local(async move {
            let detail =
                match crate::client::api_get::<crate::dto::MissionDetail>(auth, &path).await {
                    Ok(d) => d,
                    Err((401, _)) => {
                        toasts.error("Sign in to merge a mission.");
                        return;
                    }
                    Err((404, _)) => {
                        toasts.error("That mission no longer exists.");
                        return;
                    }
                    Err((s, _)) => {
                        toasts.error(format!("Could not load the mission to merge ({s})."));
                        return;
                    }
                };
            // The editor superset lives in `current_version.json_payload`; an empty `{}` (a
            // never-saved source) has nothing to merge — say so rather than run an empty merge.
            let payload = detail.current_version.as_ref().map(|v| &v.json_payload);
            let is_empty =
                payload.is_none_or(|p| p.as_object().is_none_or(serde_json::Map::is_empty));
            if is_empty {
                toasts.message(format!(
                    "\"{}\" has no saved content to merge yet.",
                    detail.title
                ));
                return;
            }
            let payload_json = payload
                .map(std::string::ToString::to_string)
                .unwrap_or_default();

            // Borrow the doc only now (post-await), run the merge, drop the borrow before toasting.
            let report_json = {
                let borrow = doc.borrow();
                let Some(core) = borrow.as_ref() else {
                    toasts.error("Editor not ready.");
                    return;
                };
                core.merge_mission_payload_json(&payload_json, offset)
            };
            // A merge is a document mutation, so it must run the SAME post-mutation tail every editor
            // mutator ends on (`editor_ops` mutators all call this): materialize → prune the selection
            // → rebind the engine slot/vehicle glyphs so the merged rows reach the GPU (they are
            // invisible on the map otherwise) → bump `doc_ver` (which drives the validation panel's
            // re-check and the attributes re-read) → set dirty → schedule the IDB persist → refresh the
            // HUD counts. `set_dirty(true)` alone (the prior code) did only the last-but-two of those,
            // so a wired merge reported success by toast while the map showed nothing. The `EDITOR_CTX`
            // doc borrow was dropped at the end of the block above; `after_local_edit` takes its own
            // `HISTORY_CTX` borrow, so this is not held across the earlier `.await`.
            crate::mission_history::after_local_edit();

            let (summary, skipped) = format_merge_report(&report_json);
            toasts.success(format!("{summary} (Ctrl+Z to undo.)"));
            if !skipped.is_empty() {
                let head = if skipped.len() == 1 {
                    "1 row was skipped:".to_string()
                } else {
                    format!("{} rows were skipped:", skipped.len())
                };
                toasts.error(format!("{head} {}", skipped.join("; ")));
            }
        });
    }

    /* ─────────────── T-698 — the browser half of the clipboard exporters ─────────────── */

    /// The author-facing refusal when a clipboard exporter runs with nothing selected. A copy that
    /// quietly did nothing is indistinguishable from a copy that worked until the paste lands empty.
    const NOTHING_SELECTED: &str = "Nothing is selected — select an entity on the map first.";

    /// The live selection ids.
    ///
    /// **Why through the `window.__editorSelection` bridge rather than a Rust call.** The selection
    /// is app-side state held in `select_tool`'s leaked `SelectionHandle` and mirrored in
    /// `editor_ops`'s `OPS_CTX`; neither exposes a Rust ids accessor (`editor_ops::selection_len`
    /// returns only the count, and `attrs_multi_ids` needs an anchor id and refuses below two). The
    /// one exported reader is `__editorSelection.ids()`, which `select_tool::register_editor_selection`
    /// installs over the same handle — so this reads the real selection, not a copy that can drift.
    /// An `editor_ops::selection_ids()` would be the better seam and is reported as residue.
    ///
    /// Every failure along the way yields an EMPTY selection, which the callers turn into the
    /// [`NOTHING_SELECTED`] refusal — never into a copy of something else.
    fn selected_ids() -> Vec<String> {
        let Some(win) = web_sys::window() else {
            return Vec::new();
        };
        let Ok(bridge) = js_sys::Reflect::get(&win, &JsValue::from_str("__editorSelection")) else {
            return Vec::new();
        };
        let Ok(f) = js_sys::Reflect::get(&bridge, &JsValue::from_str("ids")) else {
            return Vec::new();
        };
        let Ok(f) = f.dyn_into::<js_sys::Function>() else {
            return Vec::new();
        };
        let Ok(raw) = f.call0(&bridge) else {
            return Vec::new();
        };
        raw.as_string()
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .unwrap_or_default()
    }

    /// The live selection resolved against the hosted document — the input every exporter shares.
    fn selection_entities() -> Vec<SelectedEntity> {
        let ids = selected_ids();
        if ids.is_empty() {
            return Vec::new();
        }
        EDITOR_CTX.with(|c| {
            let guard = c.borrow();
            let Some(ctx) = guard.as_ref() else {
                return Vec::new();
            };
            let d = ctx.doc.borrow();
            let Some(core) = d.as_ref() else {
                return Vec::new();
            };
            resolve_selected_entities(&core.slots_json(), &core.small_maps_json(), &ids)
        })
    }

    /// Resolve `navigator.clipboard`, REFUSING rather than throwing when the browser does not expose
    /// it. The property is absent on an insecure origin (plain http on a non-localhost host), and
    /// calling `writeText` on `undefined` would raise a JS exception straight through the wasm
    /// boundary instead of producing a message an author can act on.
    fn clipboard_api() -> Result<web_sys::Clipboard, String> {
        let win = web_sys::window().ok_or_else(|| "there is no browser window".to_string())?;
        let nav: JsValue = win.navigator().into();
        let raw = js_sys::Reflect::get(&nav, &JsValue::from_str("clipboard"))
            .map_err(|_| "this browser exposes no navigator.clipboard".to_string())?;
        if raw.is_undefined() || raw.is_null() {
            return Err(
                "the Clipboard API is unavailable here — it needs a secure context (https, or \
                 localhost)"
                    .to_string(),
            );
        }
        Ok(raw.unchecked_into::<web_sys::Clipboard>())
    }

    /// Best-effort human text for a rejected clipboard promise (a `DOMException` carries `message`).
    fn js_error_text(e: &JsValue) -> String {
        if let Some(s) = e.as_string() {
            return s;
        }
        if let Ok(m) = js_sys::Reflect::get(e, &JsValue::from_str("message")) {
            if let Some(s) = m.as_string() {
                return s;
            }
        }
        format!("{e:?}")
    }

    /// **T-698 — write to the clipboard and REPORT the outcome. Never fire-and-forget.**
    ///
    /// `navigator.clipboard.writeText` returns a promise that rejects on an insecure context, on an
    /// unfocused document, and on a denied permission. Dropping that promise and toasting success
    /// anyway is the "reported success over something it never did" defect: the author walks away
    /// believing a grid reference is on their clipboard and pastes whatever was there before. So the
    /// promise is AWAITED, and the success toast is on the resolve arm only — the failure arm names
    /// the browser's own reason.
    ///
    /// **T-773 promoted this to the crate's ONE clipboard path.** `server_intel::server_panel`'s
    /// Copy button carried the very defect this function was written against — a dropped
    /// `write_text` promise followed by an unconditional "copied" toast — and it was the live
    /// in-repo precedent any new exporter would have copied. It now calls through here (reachable
    /// as `crate::mission_commands::write_clipboard` via the `pub use imp::*` re-export below).
    /// A second clipboard path is a defect in itself: two vocabularies for "did the copy land"
    /// means one of them is eventually wrong and nobody notices. If another surface needs to copy,
    /// call this — do not re-derive it.
    pub(crate) fn write_clipboard(text: String, ok_message: String, toasts: crate::toast::Toasts) {
        let clipboard = match clipboard_api() {
            Ok(c) => c,
            Err(why) => {
                toasts.error(format!("Could not copy — {why}."));
                return;
            }
        };
        let promise = clipboard.write_text(&text);
        spawn_local(async move {
            match wasm_bindgen_futures::JsFuture::from(promise).await {
                Ok(_) => toasts.success(ok_message),
                Err(e) => toasts.error(format!(
                    "Could not copy to the clipboard — {}. Click the map and try again.",
                    js_error_text(&e)
                )),
            }
        });
    }

    /// **T-698 exporter 1 — copy the selection's grid position.**
    ///
    /// `#[allow(dead_code)]`: this verb has no UI entry point yet. The menu bar (`eden_top_strip.rs`,
    /// where `export_compiled_now`'s button lives) and the context menu (`context_menu.rs`) are both
    /// outside this slice's owns, so the three exporters ship as commands and the missing "Copy grid
    /// reference" row is reported as residue rather than reached across for. They are harness-drivable
    /// today through `__editorCommands.clipboard_grid_json()` and its two peers.
    #[allow(dead_code)]
    pub fn copy_grid_position_now(toasts: crate::toast::Toasts) {
        let entities = selection_entities();
        if entities.is_empty() {
            toasts.error(NOTHING_SELECTED);
            return;
        }
        let text = super::grid_position_text(&entities);
        let ok = if entities.len() == 1 {
            format!("Copied the grid reference {text}.")
        } else {
            format!(
                "Copied {}.",
                super::count_noun(entities.len(), "grid reference", "grid references")
            )
        };
        write_clipboard(text, ok, toasts);
    }

    /// **T-698 exporter 2 — copy the selection's classnames.** Same missing-menu-entry residue note
    /// as [`copy_grid_position_now`].
    ///
    /// A selection whose every entity is classname-less copies NOTHING and says so: putting an empty
    /// string on the clipboard while reporting success is the same silent-failure shape the awaited
    /// promise exists to prevent.
    #[allow(dead_code)]
    pub fn copy_classnames_now(toasts: crate::toast::Toasts) {
        let entities = selection_entities();
        if entities.is_empty() {
            toasts.error(NOTHING_SELECTED);
            return;
        }
        let (text, skipped) = super::classnames_text(&entities);
        if text.is_empty() {
            toasts.error("Nothing in the selection carries a classname — nothing was copied.");
            return;
        }
        let copied = entities.len() - skipped;
        let ok = if skipped == 0 {
            format!(
                "Copied {}.",
                super::count_noun(copied, "classname", "classnames")
            )
        } else {
            format!(
                "Copied {}, skipping {} with no classname.",
                super::count_noun(copied, "classname", "classnames"),
                super::count_noun(skipped, "entity", "entities")
            )
        };
        write_clipboard(text, ok, toasts);
    }

    /// **T-698 exporter 3 — copy a human-readable digest of the selection.** Same missing-menu-entry
    /// residue note as [`copy_grid_position_now`].
    #[allow(dead_code)]
    pub fn copy_selection_summary_now(toasts: crate::toast::Toasts) {
        let entities = selection_entities();
        if entities.is_empty() {
            toasts.error(NOTHING_SELECTED);
            return;
        }
        let text = super::selection_summary_text(&entities);
        let ok = format!(
            "Copied a summary of {}.",
            super::count_noun(entities.len(), "entity", "entities")
        );
        write_clipboard(text, ok, toasts);
    }

    /// T-698 — what a clipboard exporter WOULD put on the clipboard, for the harness.
    ///
    /// `{"text":…,"count":n,"skipped":k}` on success, `{"error":…}` on a refusal — the two are
    /// distinguishable by shape, the `compiled_diagnostics_json` precedent. This deliberately does
    /// NOT touch the clipboard: a headless gate has no clipboard permission, and a reader that had
    /// to grant one would test the browser rather than the exporter. The clipboard write itself is
    /// [`write_clipboard`], and its contract (await, then report) is prose the author can check
    /// against the toast.
    fn export_preview_json(kind: &str) -> String {
        let entities = selection_entities();
        if entities.is_empty() {
            return serde_json::json!({ "error": NOTHING_SELECTED }).to_string();
        }
        let (text, skipped) = match kind {
            "classnames" => super::classnames_text(&entities),
            "summary" => (super::selection_summary_text(&entities), 0),
            _ => (super::grid_position_text(&entities), 0),
        };
        serde_json::json!({
            "text": text,
            "count": entities.len(),
            "skipped": skipped,
        })
        .to_string()
    }

    /// Install `window.__editorCommands` — the read-only compile smoke bridge (peer of `__missionDoc`,
    /// same leaked-closure `js_sys::Object` idiom as `register_mission_doc`). `compile_save_json()` and
    /// `compile_export_json()` return the compiled JSON strings; the export path pins `exportedAt` +
    /// `missionId`/`version` to fixed values so the gate output is byte-deterministic.
    ///
    /// **T-243 adds `compiled_document_json()`** — the same bytes the "Export Compiled" button
    /// downloads. It is on the bridge for the reason the other two are: a compile whose only entry
    /// point is a `<button>` and a `Blob` download is a compile no harness can read back, and this one
    /// makes a claim worth checking against a live `GET /missions/:id/compiled`. Unlike its two peers
    /// it pins nothing: its whole value is being the real output. On a failure it returns the same
    /// author-facing message the toast shows (a plain string either way — the caller can tell them
    /// apart by parsing).
    pub fn register_editor_commands(doc: DocHandle) {
        let obj = js_sys::Object::new();

        let compiled_doc = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&compiled_document_json().unwrap_or_else(|e| e))
        }) as Box<dyn FnMut() -> JsValue>);

        // T-690 — the compile's structured findings as JSON, so a harness can read back what the
        // build step LEARNED and not only what it emitted. Same argument as `compiled_document_json`
        // above: a result whose only exit is a floating card is a result no harness can check.
        // Returns `[{ruleId, severity, primitive, message, subject, subjectId}]`, `[]` on a clean
        // compile, and `{"error": "…"}` on a refusal — the three cases are distinguishable by shape.
        let compiled_diags = Closure::wrap(Box::new(move || -> JsValue {
            let out = match compiled_document_json_with_diagnostics() {
                Ok((_, findings)) => {
                    let rows: Vec<serde_json::Value> = findings
                        .iter()
                        .map(|f| {
                            serde_json::json!({
                                "ruleId": f.rule_id,
                                "severity": f.severity.as_str(),
                                "primitive": f.primitive.tag(),
                                "message": f.message,
                                "subject": f.subject,
                                "subjectId": f.subject_id,
                            })
                        })
                        .collect();
                    serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => serde_json::json!({ "error": e }).to_string(),
            };
            JsValue::from_str(&out)
        }) as Box<dyn FnMut() -> JsValue>);

        let compile_save = {
            let doc = doc.clone();
            Closure::wrap(Box::new(move || -> JsValue {
                let json = doc
                    .borrow()
                    .as_ref()
                    .map(|c| {
                        let payload = compile_payload(&c.small_maps_json(), &c.slots_json(), false);
                        serde_json::to_string(&payload).unwrap_or_default()
                    })
                    .unwrap_or_default();
                JsValue::from_str(&json)
            }) as Box<dyn FnMut() -> JsValue>)
        };
        let compile_export_fn = {
            let doc = doc.clone();
            Closure::wrap(Box::new(move || -> JsValue {
                let json = doc
                    .borrow()
                    .as_ref()
                    .map(|c| {
                        let small = c.small_maps_json();
                        let payload = compile_payload(&small, &c.slots_json(), true);
                        let env = compile_export(
                            &payload,
                            &small,
                            "smoke",
                            "0.1.0",
                            "1970-01-01T00:00:00.000Z",
                        );
                        serde_json::to_string(&env).unwrap_or_default()
                    })
                    .unwrap_or_default();
                JsValue::from_str(&json)
            }) as Box<dyn FnMut() -> JsValue>)
        };
        // T-693 — merge a payload JSON string into the hosted doc and return the report JSON. Unlike
        // its read-only peers this one MUTATES (the merge is one undo step in-core), so a smoke that
        // calls it should undo after. Takes the payload as a JS string arg; no offset (the harness
        // exercises the authored-position path). Returns the [`MergeReport`] JSON.
        let merge_fn = {
            let doc = doc.clone();
            Closure::wrap(Box::new(move |payload: JsValue| -> JsValue {
                let payload_json = payload.as_string().unwrap_or_default();
                let out = doc
                    .borrow()
                    .as_ref()
                    .map(|c| c.merge_mission_payload_json(&payload_json, None))
                    .unwrap_or_default();
                JsValue::from_str(&out)
            }) as Box<dyn FnMut(JsValue) -> JsValue>)
        };

        // T-698 — one closure per exporter over the shared [`export_preview_json`] reader.
        let clipboard_grid = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("grid"))
        }) as Box<dyn FnMut() -> JsValue>);
        let clipboard_classnames = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("classnames"))
        }) as Box<dyn FnMut() -> JsValue>);
        let clipboard_summary = Closure::wrap(Box::new(move || -> JsValue {
            JsValue::from_str(&export_preview_json("summary"))
        }) as Box<dyn FnMut() -> JsValue>);

        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compile_save_json"),
            compile_save.as_ref(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compile_export_json"),
            compile_export_fn.as_ref(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compiled_document_json"),
            compiled_doc.as_ref(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("compiled_diagnostics_json"),
            compiled_diags.as_ref(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &JsValue::from_str("merge_mission_json"),
            merge_fn.as_ref(),
        );
        // T-698 — the three clipboard exporters, readable. Same argument as `compiled_document_json`
        // above: an exporter whose only exit is a `navigator.clipboard` write is an exporter no
        // harness can read back, and the clipboard is not readable in a headless gate.
        for (name, closure) in [
            ("clipboard_grid_json", &clipboard_grid),
            ("clipboard_classnames_json", &clipboard_classnames),
            ("clipboard_summary_json", &clipboard_summary),
        ] {
            let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(name), closure.as_ref());
        }
        if let Some(win) = web_sys::window() {
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCommands"), &obj);
        }
        // Leaked like the other editor bridges (harness reads them across the page lifetime).
        compile_save.forget();
        compile_export_fn.forget();
        compiled_doc.forget();
        compiled_diags.forget();
        merge_fn.forget();
        clipboard_grid.forget();
        clipboard_classnames.forget();
        clipboard_summary.forget();
    }
}

#[cfg(target_arch = "wasm32")]
pub use imp::*;

#[cfg(test)]
mod tests {
    use super::{compiled_export_text, format_merge_report, row_meta_missing_message};

    /// Measured wire key order from `GET /compiled` (ticket T-417 / ModMissionDocument field order).
    const WIRE_ORDER_COMPACT: &[u8] = br#"{"schemaVersion":"1.1","meta":{"id":"m","title":"t","author":"a","terrain":"everon","playerRange":[1,1]},"environment":{"timeOfDay":"0800","weatherPreset":"clear"},"factions":[],"orbat":{},"slots":[{"id":"s1"}],"radioPlan":{"nets":[]},"zones":[],"flow":{"briefingDurationSec":0},"winConditions":{"mode":"none"}}"#;

    #[test]
    fn class_r_compiled_export_is_byte_identical_to_wire() {
        let out = compiled_export_text(WIRE_ORDER_COMPACT).expect("utf-8");
        assert_eq!(
            out.as_bytes(),
            WIRE_ORDER_COMPACT,
            "Export Compiled must ship compact wire bytes — not a Value pretty-print"
        );
        // Top-level key order pin (the measured /compiled order from T-417).
        // Read order from the raw text — `serde_json::Value` would BTreeMap-sort without preserve_order.
        assert_eq!(
            raw_top_level_keys(&out),
            [
                "schemaVersion",
                "meta",
                "environment",
                "factions",
                "orbat",
                "slots",
                "radioPlan",
                "zones",
                "flow",
                "winConditions",
            ]
        );
    }

    #[test]
    fn class_r_value_pretty_print_is_not_byte_identical() {
        // Even with serde_json `preserve_order` (unified from map-engine-core / T-220), a
        // Value→pretty round-trip changes bytes (whitespace). The old comment claiming
        // "whitespace-only" while inviting a live `/compiled` compare set a trap; shipping
        // compact makes the download byte-identical to the flatten/`/compiled` body.
        let value: serde_json::Value = serde_json::from_slice(WIRE_ORDER_COMPACT).unwrap();
        let pretty = serde_json::to_string_pretty(&value).unwrap();
        assert_ne!(
            pretty.as_bytes(),
            WIRE_ORDER_COMPACT,
            "pretty-print must differ from compact wire bytes"
        );
        let shipped = compiled_export_text(WIRE_ORDER_COMPACT).unwrap();
        assert_eq!(shipped.as_bytes(), WIRE_ORDER_COMPACT);
        // Key order must still match wire (preserve_order keeps it; compact never reorders).
        assert_eq!(raw_top_level_keys(&shipped), raw_top_level_keys(&pretty));
        assert_eq!(
            raw_top_level_keys(&shipped)[0],
            "schemaVersion",
            "wire order starts with schemaVersion, not alpha environment"
        );
    }

    #[test]
    fn class_r_auth_failure_is_not_no_saved_row() {
        let auth = row_meta_missing_message(false);
        let no_row = row_meta_missing_message(true);
        assert!(
            auth.contains("Sign in") || auth.contains("session"),
            "unauthenticated must name auth, got: {auth}"
        );
        assert!(
            !auth.contains("save a version first"),
            "auth failure must not suggest save-first: {auth}"
        );
        assert!(
            no_row.contains("save a version first"),
            "authenticated-but-no-row keeps the save-first remedy: {no_row}"
        );
    }

    /// T-417 Class-R — the Export Compiled download must ship the compact wire bytes.
    ///
    /// # Cure 2 (scrub-then-grep), and why not cure 1 (T-601)
    ///
    /// This is the closest call of the six pins T-601 converted, because the invariant *does* have
    /// a runtime signature: [`compiled_export_text`]. But that half is already pinned by value —
    /// [`class_r_compiled_export_is_byte_identical_to_wire`] feeds it the golden wire bytes and
    /// asserts the output is byte-identical. What is left over is only "the download path calls
    /// it", and [`compiled_document_json`] lives inside `#[cfg(target_arch = "wasm32")] mod imp`
    /// on top of `snapshot()`, `ROW_META`, `compile_payload` and `flatten_mod_document_json`. A
    /// cure-1 harness would have to model four crate seams, and a pin whose preamble is bigger
    /// than the code under test is a pin nobody will keep honest. Recorded as a deliberate choice,
    /// not an oversight: if the wasm boundary ever moves, this is the pin to promote.
    ///
    /// T-601 replaced the ad-hoc `split(…).nth(1).split("fn clone_meta")` slice — which silently
    /// depended on `clone_meta` staying the next item, and would have returned the *first* of two
    /// `compiled_document_json` definitions without a word — with the shared scrubber's
    /// ambiguity-refusing extractor.
    ///
    /// **T-690 moved the needle one function down, and did not loosen it.** The transport body now
    /// lives in `compiled_document_json_with_diagnostics` (the compile returns findings alongside
    /// the bytes, and `compiled_document_json` is a projection of it — one compile, not two). So the
    /// pin reads the body that actually calls the compile, AND asserts the projection is thin: if
    /// `compiled_document_json` ever grows its own compile again, the second assertion fires and the
    /// two paths can no longer drift into shipping different bytes.
    #[test]
    fn class_r_source_forbids_value_pretty_on_compiled_export() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(
            &production,
            "pub fn compiled_document_json_with_diagnostics()",
        );
        assert!(
            code.contains("compiled_export_text(&doc)"),
            "the compile path must ship via compiled_export_text"
        );
        assert!(
            !code.contains("to_string_pretty"),
            "the compile path must not pretty-print the compiled doc"
        );
        // A live `serde_json::Value` binding in the return path is the old defect.
        assert!(
            !code.contains("let value: serde_json::Value"),
            "the compile path must not re-parse through Value for download"
        );
        // …and the plain entry point is a PROJECTION of it, never a second compile.
        let plain = only_body(&production, "pub fn compiled_document_json()");
        assert!(
            plain.contains("compiled_document_json_with_diagnostics()"),
            "compiled_document_json must project the diagnostics body, not compile again; got:\n{plain}"
        );
        assert!(
            !plain.contains("flatten_mod_document_json"),
            "compiled_document_json must not run its own compile; got:\n{plain}"
        );
        // The ROW_META docs are PROSE, so they are read from the raw file on purpose — the
        // scrubber's whole job is to delete prose, and asserting a doc string against scrubbed
        // source would be a pin that can only ever fail.
        let prose = SRC.split("mod tests {").next().expect("tests marker");
        assert!(
            prose.contains("401") && prose.contains("expired session"),
            "ROW_META docs must name 401 / expired session"
        );
    }

    /// **T-601 — calibration for the export pin above.**
    ///
    /// The needle it cannot do without is `compiled_export_text(&doc)`. Every wrapper in the
    /// battery must stop satisfying it, or a `to_string_pretty` download could ship while this
    /// pin reported the compact wire path was live.
    #[test]
    fn the_export_pin_rejects_every_dead_code_wrapper() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let needle = "compiled_export_text(&doc)";
        let attacks: [(&str, String); 12] = [
            (
                "if true == false",
                format!("if true == false {{ {needle}; }}"),
            ),
            ("loop { break; … }", format!("loop {{ break; {needle}; }}")),
            (
                "#[cfg(any())]",
                format!("#[cfg(any())] fn d() {{ {needle}; }}"),
            ),
            ("while false", format!("while false {{ {needle}; }}")),
            ("if !true", format!("if !true {{ {needle}; }}")),
            ("if 1 > 2", format!("if 1 > 2 {{ {needle}; }}")),
            (
                "if std::hint::black_box(false)",
                format!("if std::hint::black_box(false) {{ {needle}; }}"),
            ),
            (
                "const C: bool = false; if C",
                format!("const C: bool = false;\nfn d() {{ if C {{ {needle}; }} }}"),
            ),
            ("return; above", format!("fn d() {{ return; {needle}; }}")),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod shadow {{ fn d() {{ {needle}; }} }}"),
            ),
            (
                "match guard",
                format!("match () {{ _ if false => {{ {needle}; }} _ => {{}} }}"),
            ),
            ("comment", format!("// {needle}")),
        ];
        for (label, body) in attacks {
            let forged =
                format!("pub fn compiled_document_json() {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the compact-bytes needle survived scrubbing — this pin would report a \
                 live wire-bytes download over code the build never runs"
            );
        }
        for (label, forged) in [
            (
                "shadow copy in a live mod, no cfg",
                "pub fn compiled_document_json() { good(); }\n\
                 mod real { pub fn compiled_document_json() { bad(); } }\n#[cfg(test)]\n",
            ),
            (
                "shadow copy in an impl",
                "pub fn compiled_document_json() { good(); }\n\
                 impl T { pub fn compiled_document_json() { bad(); } }\n#[cfg(test)]\n",
            ),
        ] {
            let scrubbed = live_code(forged);
            let caught = std::panic::catch_unwind(|| {
                only_body(&scrubbed, "pub fn compiled_document_json()")
            })
            .is_err();
            assert!(
                caught,
                "{label}: the old `split(…).nth(1)` slice would have taken the first of two \
                 definitions without saying so"
            );
        }
        let live = format!("pub fn compiled_document_json() {{\n    {needle}\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }

    /// First-level object key order from a JSON object string (no full parse → no Map reorder).
    fn raw_top_level_keys(json: &str) -> Vec<&str> {
        let mut keys = Vec::new();
        let bytes = json.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        assert_eq!(bytes.get(i), Some(&b'{'));
        i += 1;
        let mut depth = 1u32;
        let mut in_string = false;
        let mut escape = false;
        let mut key_start: Option<usize> = None;
        let mut expect_key = true;
        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if escape {
                    escape = false;
                } else if b == b'\\' {
                    escape = true;
                } else if b == b'"' {
                    in_string = false;
                    if expect_key && depth == 1 {
                        if let Some(s) = key_start {
                            keys.push(&json[s..i]);
                            expect_key = false;
                        }
                    }
                    key_start = None;
                }
                i += 1;
                continue;
            }
            match b {
                b'"' => {
                    in_string = true;
                    if expect_key && depth == 1 {
                        key_start = Some(i + 1);
                    }
                }
                b'{' | b'[' => depth += 1,
                b'}' | b']' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                b',' if depth == 1 => expect_key = true,
                b':' if depth == 1 => expect_key = false,
                _ => {}
            }
            i += 1;
        }
        keys
    }

    /// T-693 Class-R — the report formatter names the non-zero counts, distinguishes squads
    /// merged-into-existing from squads created, and lists each skipped row.
    #[test]
    fn class_r_merge_report_formats_counts_and_skips() {
        let report = r#"{
            "slots_added": 3, "squads_merged": 1, "squads_created": 2,
            "factions_merged": 1, "factions_created": 0, "vehicles_added": 1,
            "entities_added": 0, "zones_added": 0, "triggers_added": 1,
            "compositions_added": 0, "markers_added": 0,
            "skipped": [{"kind":"slot","id":"","reason":"missing id"}]
        }"#;
        let (summary, skipped) = format_merge_report(report);
        assert!(summary.contains("3 slots"), "counts slots: {summary}");
        assert!(
            summary.contains("2 squads"),
            "counts created squads: {summary}"
        );
        assert!(
            summary.contains("1 squad merged into existing"),
            "names merged squads distinctly: {summary}"
        );
        assert!(summary.contains("1 vehicle"), "singular vehicle: {summary}");
        assert!(summary.contains("1 trigger"), "counts triggers: {summary}");
        assert!(
            !summary.contains("faction"),
            "zero created factions omitted from the summary: {summary}"
        );
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0], "slot — missing id");
    }

    /// T-693 Class-R — an empty merge says so rather than "Merged .".
    #[test]
    fn class_r_merge_report_empty_is_named() {
        let report = r#"{"slots_added":0,"squads_merged":0,"squads_created":0,
            "factions_merged":0,"factions_created":0,"vehicles_added":0,"entities_added":0,
            "zones_added":0,"triggers_added":0,"compositions_added":0,"markers_added":0,"skipped":[]}"#;
        let (summary, skipped) = format_merge_report(report);
        assert!(
            summary.contains("added nothing"),
            "empty merge named: {summary}"
        );
        assert!(skipped.is_empty());
    }

    /// T-693 Class-R — an unparseable report degrades to a named skip, never a silent success.
    #[test]
    fn class_r_merge_report_unparseable_degrades() {
        let (summary, skipped) = format_merge_report("{not json");
        assert!(
            summary.contains("no readable report"),
            "unparseable named: {summary}"
        );
        assert_eq!(skipped.len(), 1);
        assert!(skipped[0].contains("could not parse"));
    }

    /// T-693 Class-R (MAJOR-1) — **`merge_mission_now` runs the full post-mutation tail.** A merge is a
    /// document mutation; ending it on `set_dirty(true)` alone (the prior code) skipped the engine
    /// rebind (merged rows never reached the GPU — invisible on the map), the `doc_ver` bump (the
    /// validation panel never re-checked), and the persist schedule. The tail is `after_local_edit`,
    /// the same call every `editor_ops` mutator ends on. This pins the live source (the function is
    /// wasm-gated, so there is no native runtime seam — the source is the contract) through the
    /// `class_r_scrub` extractor, so a regression to a bare `set_dirty` re-arms the bug loudly. The
    /// scrubber blanks comments, so the `set_dirty(true)` named in this function's doc-comment cannot
    /// satisfy the needle — only a live call can.
    #[test]
    fn class_r_merge_mission_now_runs_the_after_local_edit_tail() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(&production, "pub fn merge_mission_now");
        assert!(
            code.contains("after_local_edit"),
            "merge_mission_now must run the post-mutation tail (after_local_edit), not just set_dirty"
        );
        // The prior defect: the tail was a bare `set_dirty(true)` and nothing else. `after_local_edit`
        // already sets dirty (via `after_doc_change`), so a live `set_dirty(true)` here would be the
        // regression — the merge doing only the dirty flag again.
        assert!(
            !code.contains("set_dirty(true)"),
            "merge_mission_now must not end on a bare set_dirty(true) — after_local_edit sets dirty"
        );
    }

    /* ══════════ T-690 — the compile's structured result ══════════ */

    use map_engine_core::mission::validate::{Finding, Primitive, Severity};

    fn finding(rule_id: &'static str, severity: Severity, subject_id: Option<&str>) -> Finding {
        Finding {
            rule_id,
            severity,
            primitive: Primitive::PerObjectInvariant,
            message: "the compile dropped a value".to_string(),
            subject: "/editor/slots/0/rank".to_string(),
            subject_id: subject_id.map(ToString::to_string),
        }
    }

    /// A clean compile gets the message it always had — never a celebratory "0 issues".
    #[test]
    fn a_clean_compile_produces_no_diagnostics_summary() {
        assert_eq!(super::compile_diagnostics_summary(&[]), None);
    }

    /// The toast shrinks to a verdict + a pointer: counts by severity, worst first, only the
    /// non-zero rungs, correctly pluralised, and it names where the list actually lives.
    #[test]
    fn the_diagnostics_summary_counts_by_severity_and_points_at_the_panel() {
        let findings = [
            finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
            finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
            finding("COMPILE-DROP-SLOT-TAG", Severity::Info, Some("s1")),
        ];
        let s = super::compile_diagnostics_summary(&findings).expect("some findings");
        assert!(s.contains("1 warning"), "{s}");
        assert!(s.contains("2 notes"), "{s}");
        assert!(!s.contains("error"), "no zero-count rung may appear: {s}");
        assert!(
            s.contains("validation panel"),
            "the toast must point at the render surface rather than try to be it: {s}"
        );
    }

    /// **The feed.** The compile's findings reach the T-655 panel through the panel's OWN row type,
    /// so a compile finding renders — and click-to-selects on its `subject_id` — exactly like a
    /// validation finding. No second panel, no parallel vocabulary.
    #[test]
    fn compile_findings_reach_the_validation_panel() {
        use crate::validation_panel::{
            evaluate_now, publish_compile_findings, PanelFinding, Rollup,
        };

        // Baseline: nothing published, nothing shown (no payload source is registered on the host).
        publish_compile_findings(Vec::new());
        assert!(
            Rollup::of(&evaluate_now()).is_empty(),
            "the panel starts empty"
        );

        let findings = [
            finding("COMPILE-DROP-SQUAD-LEADER", Severity::Warning, Some("sq1")),
            finding("COMPILE-DROP-SLOT-RANK", Severity::Info, Some("s1")),
        ];
        publish_compile_findings(findings.iter().map(PanelFinding::from_finding).collect());

        let rows = evaluate_now();
        assert_eq!(rows.len(), 2, "{rows:?}");
        let rollup = Rollup::of(&rows);
        assert_eq!((rollup.errors, rollup.warnings, rollup.infos), (0, 1, 1));
        assert_eq!(rollup.chip_text(), "1 warning · 1 info");
        // The owning entity id survived, which is what makes the row clickable — the T-657
        // `subject_id` vocabulary reused rather than a parallel one invented.
        let leader = rows
            .iter()
            .find(|r| r.rule_id == "COMPILE-DROP-SQUAD-LEADER")
            .expect("the leader finding rendered");
        assert_eq!(leader.subject_id.as_deref(), Some("sq1"));
        assert!(leader.is_selectable());

        // A clean compile CLEARS the previous build report — otherwise the panel would show a stale
        // list after the author fixed everything in it.
        publish_compile_findings(Vec::new());
        assert!(
            Rollup::of(&evaluate_now()).is_empty(),
            "a clean compile must clear the previous compile's findings"
        );
    }

    /// Class-R — the export path FEEDS the shipped panel and does not grow one of its own.
    ///
    /// The ticket's own constraint ("owns only the compiler and the command layer so the panel stays
    /// a single claimant") is the kind that decays silently: a second list rendered next to the
    /// download button would look fine and would be a second claimant. This reads the live body.
    #[test]
    fn class_r_the_export_publishes_to_the_panel_and_builds_no_second_one() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let code = only_body(&production, "pub fn export_compiled_now(");
        assert!(
            code.contains("validation_panel::publish_compile_findings("),
            "export_compiled_now must publish the compile's findings to the T-655 panel; got:\n{code}"
        );
        assert!(
            code.contains("compiled_document_json_with_diagnostics()"),
            "export_compiled_now must take the bytes AND the findings from one compile; got:\n{code}"
        );
        // A `view!` here would be a second render surface for the same findings.
        assert!(
            !code.contains("view!"),
            "export_compiled_now must not render a panel of its own; got:\n{code}"
        );
        // …and the summary must not try to be the list: no per-finding message in the toast.
        assert!(
            !code.contains("f.message"),
            "the toast is a pointer, not the list; got:\n{code}"
        );
    }

    /* ─────────────────── T-698 — the clipboard exporters, pinned by value ───────────────────
     *
     * Deliberately NO source scanning. Resolution and composition are pure string functions, so they
     * are called with real inputs and their real output is asserted — a pin that cannot be satisfied
     * by a needle sitting in its own assertion.
     */

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// A document with one of each kind: a slot with a role and an asset, a slot with only a tag and
    /// NO asset (the "+ button" case), a vehicle, and an object.
    fn doc_fixtures() -> (String, String) {
        let slots = r#"{
            "s1": {"id":"s1","role":"SL","tag":"",
                   "assetId":"{8402}Prefabs/Characters/US/Character_US_GL.et",
                   "position":{"x":1250.0,"y":4800.0,"z":0,"rotation":90}},
            "s2": {"id":"s2","role":"","tag":"Overwatch","assetId":"",
                   "position":{"x":6400.0,"y":6400.0,"z":0,"rotation":0}}
        }"#;
        let small = r#"{
            "vehiclesById": {
                "v1": {"id":"v1","resourceName":"{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et",
                       "position":{"x":100.0,"y":250.0,"z":0,"rotation":0}}
            },
            "entitiesById": {
                "e1": {"id":"e1","alias":"prop:ammo_crate",
                       "resourceName":"{FA}Prefabs/Props/AmmoBox.et",
                       "position":{"x":12000.0,"y":0.0,"z":0,"rotation":0}}
            }
        }"#;
        (slots.to_string(), small.to_string())
    }

    /// **The pin the ticket turns on.**
    ///
    /// T-667 draws grid-reference labels on the map-pane edges. A mission maker reads an easting off
    /// the top edge and a northing off the left edge and says the pair out loud. This asserts that an
    /// entity standing exactly on one of those intersections exports precisely those two label
    /// strings, in that order, separated by one space — so the clipboard and the screen can never
    /// disagree. The labels are taken from `edge_eastings` / `edge_northings` themselves, not
    /// recomputed, so a change to the furniture's formatting fails HERE rather than shipping a
    /// clipboard convention nobody reconciled.
    #[test]
    fn the_exporter_grid_ref_is_the_map_furnitures_own_label_text() {
        use crate::eden_layout::{DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX};
        use crate::eden_toolbelt::{edge_eastings, edge_northings, GRID_STEP_M};
        use map_engine_core::camera::OrthoCamera;

        let (w, h) = (1600.0_f64, 900.0_f64);
        let mut cam = OrthoCamera::new(w, h, 6400.0, 6400.0, -2.0);
        cam.set_bounds(0.0, 0.0, 12_800.0, 12_800.0);
        let pane_right = w - DOCK_RIGHT_PX;

        let eastings = edge_eastings(&cam, DOCK_LEFT_PX, pane_right, STRIP_TOP_PX);
        let northings = edge_northings(&cam, DOCK_LEFT_PX, STRIP_TOP_PX, h);
        assert!(
            !eastings.is_empty() && !northings.is_empty(),
            "the fixture camera must actually show grid labels to compare against"
        );

        let mut checked = 0usize;
        for e in &eastings {
            // The world X the label sits on, snapped to its 1 km line.
            let wx =
                (cam.unproject_xy(e.pos_px, STRIP_TOP_PX)[0] / GRID_STEP_M).round() * GRID_STEP_M;
            for n in &northings {
                let wy = (cam.unproject_xy(DOCK_LEFT_PX, n.pos_px)[1] / GRID_STEP_M).round()
                    * GRID_STEP_M;
                assert_eq!(
                    super::format_grid_ref(wx, wy),
                    format!("{} {}", e.text, n.text),
                    "an entity on the intersection of the '{}' easting and the '{}' northing must \
                     export exactly what the map edges print",
                    e.text,
                    n.text
                );
                checked += 1;
            }
        }
        assert!(
            checked >= 4,
            "expected a grid of intersections, got {checked}"
        );

        // …and a position BETWEEN two labelled lines reads inside the band they bracket — the
        // six-figure read the furniture invites (1250 m sits between the "010" and "020" eastings,
        // and 4800 m between the "040" and "050" northings).
        assert_eq!(super::format_grid_ref(1250.0, 4800.0), "012 048");
    }

    #[test]
    fn resolve_selected_entities_reads_all_three_document_maps() {
        let (slots, small) = doc_fixtures();
        let out =
            super::resolve_selected_entities(&slots, &small, &ids(&["v1", "s1", "e1", "gone"]));
        assert_eq!(
            out.len(),
            3,
            "an id the document does not know must be DROPPED, never given a placeholder grid: \
             {out:?}"
        );
        // Selection order is preserved — the summary lists what the author picked, in that order.
        assert_eq!(out[0].kind, "vehicle");
        assert_eq!(
            out[0].classname,
            "{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et"
        );
        assert_eq!(out[1].kind, "slot");
        assert_eq!(out[1].label, "SL");
        assert!((out[1].x - 1250.0).abs() < 1e-9 && (out[1].y - 4800.0).abs() < 1e-9);
        assert_eq!(out[2].kind, "object");
        assert_eq!(out[2].label, "prop:ammo_crate");
        assert_eq!(out[2].classname, "{FA}Prefabs/Props/AmmoBox.et");

        // A slot with no role falls back to its tag rather than going nameless.
        let tagged = super::resolve_selected_entities(&slots, &small, &ids(&["s2"]));
        assert_eq!(tagged[0].label, "Overwatch");
        assert_eq!(
            tagged[0].classname, "",
            "the + button case carries no asset"
        );
    }

    #[test]
    fn a_single_selection_grid_export_is_the_bare_reference() {
        let (slots, small) = doc_fixtures();
        let one = super::resolve_selected_entities(&slots, &small, &ids(&["s1"]));
        assert_eq!(
            super::grid_position_text(&one),
            "012 048",
            "a one-entity copy must paste straight into a briefing line with nothing to strip"
        );

        // A multi-selection stays attributable rather than collapsing into one number.
        let many = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "v1"]));
        let text = super::grid_position_text(&many);
        assert_eq!(text.lines().count(), 2, "{text}");
        assert!(text.starts_with("012 048  SL"), "{text}");
        assert!(text.contains("001 002  UAZ469"), "{text}");
    }

    #[test]
    fn the_classname_export_keeps_duplicates_and_counts_what_it_left_out() {
        let (slots, small) = doc_fixtures();
        let sel = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "s1", "s2"]));
        let (text, skipped) = super::classnames_text(&sel);
        assert_eq!(
            skipped, 1,
            "the asset-less slot must be REPORTED, not exported as a blank line"
        );
        assert_eq!(
            text.lines().count(),
            2,
            "two of the same prefab is two entities — a silent dedupe changes the count: {text}"
        );
        assert!(
            !text.contains("\n\n") && !text.ends_with('\n'),
            "a blank line reads as a real, empty classname: {text:?}"
        );

        // A selection with nothing to say produces nothing — the caller turns this into a refusal.
        let none = super::resolve_selected_entities(&slots, &small, &ids(&["s2"]));
        assert_eq!(super::classnames_text(&none), (String::new(), 1));
    }

    #[test]
    fn the_selection_summary_names_counts_and_carries_the_same_grids() {
        let (slots, small) = doc_fixtures();
        let sel = super::resolve_selected_entities(&slots, &small, &ids(&["s1", "s2", "v1"]));
        let text = super::selection_summary_text(&sel);
        assert_eq!(
            text.lines().next().unwrap(),
            "3 entities selected — 2 slots, 1 vehicle",
            "{text}"
        );
        assert!(
            text.contains("- SL (slot) at 012 048 — {8402}"),
            "each row names what, where and of what: {text}"
        );
        assert!(
            text.contains("(no classname)"),
            "a missing classname must be spelled out, not left blank: {text}"
        );
        // The digest cannot disagree with the grid exporter about where anything stands.
        for e in &sel {
            assert!(text.contains(&super::format_grid_ref(e.x, e.y)), "{text}");
        }
        // Singular headline stays grammatical.
        let one = super::resolve_selected_entities(&slots, &small, &ids(&["v1"]));
        assert!(
            super::selection_summary_text(&one).starts_with("1 entity selected — 1 vehicle"),
            "{}",
            super::selection_summary_text(&one)
        );
    }

    #[test]
    fn every_exporter_is_empty_on_an_empty_selection() {
        assert_eq!(super::grid_position_text(&[]), "");
        assert_eq!(super::classnames_text(&[]), (String::new(), 0));
        assert_eq!(super::selection_summary_text(&[]), "");
    }

    #[test]
    fn the_prefab_leaf_never_invents_a_name() {
        assert_eq!(
            super::prefab_leaf("{ABCD}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et"),
            "UAZ469"
        );
        assert_eq!(super::prefab_leaf("Character_US_GL.et"), "Character_US_GL");
        assert_eq!(super::prefab_leaf("plain"), "plain");
        assert_eq!(super::prefab_leaf(""), "");
    }

    /// **T-773 — `write_clipboard` is the crate's one clipboard path, and it must stay awaited.**
    ///
    /// T-698 wrote this helper correctly but nothing pinned it, and meanwhile `server_intel`'s Copy
    /// button shipped the exact defect it was written against. T-773 repointed that button here, so
    /// three surfaces plus the Server Intel panel now inherit this body's contract — and an
    /// unpinned contract that four callers depend on is one refactor away from a silent lie.
    ///
    /// The behaviour itself cannot be exercised natively (there is no `navigator.clipboard` in a
    /// `cargo test` process), so this pins the shape through `class_r_scrub::live_code`, which cuts
    /// the test module first: the success toast must sit on the `Ok(_)` arm of the awaited future,
    /// and the failure arm must exist and must report. `Ok(_) => toasts.success(ok_message)` is a
    /// single needle on purpose — it fails if the await is removed, if the arm is reordered onto a
    /// bare call, or if success is toasted before the match.
    #[test]
    fn class_r_write_clipboard_toasts_only_on_the_resolve_arm() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        const SRC: &str = include_str!("mission_commands.rs");
        let production = live_code(SRC);
        let body = only_body(
            &production,
            "pub(crate) fn write_clipboard(text: String, ok_message: String, toasts: crate::toast::Toasts)",
        );

        assert!(
            body.contains("JsFuture::from(promise).await"),
            "the writeText promise must be AWAITED, never dropped; got:\n{body}"
        );
        assert!(
            body.contains("Ok(_) => toasts.success(ok_message)"),
            "success may only be reported on the resolve arm of the awaited promise; got:\n{body}"
        );
        assert!(
            body.contains("Err(e) => toasts.error("),
            "a rejected clipboard write must reach the operator, not the bin; got:\n{body}"
        );
        // `let _ = <anything>.write_text` is the fire-and-forget shape this ticket removed from the
        // repo. It must not come back in the helper every caller now trusts.
        assert!(
            !body.contains("let _ ="),
            "no discarded result inside the clipboard helper; got:\n{body}"
        );
    }
}
