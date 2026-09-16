//! Arsenal tab — the **Smart Forge** (ArsenalTab.tsx + arsenalRules.ts + SoldierSilhouette.tsx
//! port, T-159.27 → T-167). A doc-backed loadout editor: the 14 loadout rows (incl. the compat
//! `edge` rows optic/magazine keyed off the picked weapon), the **attachment set** each weapon
//! accepts (T-197), a clickable **SVG paper-doll**, an honest **weight** readout, and per-row
//! **compat validation** — persisted on the slot via `editor_ops::set_loadout` (one undo step per
//! pick) as the canonical `SlotLoadoutV2` shape (the same `picksToLoadout` output the mod equip
//! reads), so a pick round-trips through Save/Export.
//!
//! The domain decisions (rows, compat graph, option building, validation, doll regions, weight)
//! live in [`crate::editor::arsenal::arsenal_rules`] (pure, native-tested). This module is the UI + the persisted
//! serialization ([`picks_to_loadout`] / [`loadout_to_picks`]: optic/magazine ride `weapons[0]` as
//! sticky sub-fields; attachments ride their own weapon's `attachments[]`).
//!
//! # Persistence — there is no Save button here, and that is the design (T-503)
//!
//! Every pick and every cargo edit calls [`crate::editor::state::operations::set_loadout`] the moment it happens.
//! Nothing stages. T-503 asked whether that is a bug — whether the Arsenal should grow an explicit
//! Save with a dirty indicator and a discard path — and the answer from the rest of the SPA is no,
//! twice over:
//!
//! * **Every other mission-document editor commits on the spot.** `editor_ops.rs` funnels **56**
//!   call sites into `mission_history::after_local_edit`, out of **59** SPA-wide. The other three
//!   are single direct calls in `mission_hydrate.rs`, `mission_commands.rs` and `mission_editor.rs`,
//!   none of them an editor commit point, so the argument below is unaffected.
//!
//!   **Re-measured 2026-08-08 (T-779). The previous figures — 26 here, 28 SPA-wide, "the other
//!   two" — were all wrong**, stale since the 2026-07-31 measurement, and the file list was wrong
//!   too: it named `mission_editor.rs` and `mission_hydrate.rs` and missed `mission_commands.rs`.
//!   HOW TO RE-DERIVE, because a grep over this file counts this prose and the source pins below:
//!   remove the cfg-test modules by BRACE MATCHING — splitting at the first such attribute
//!   truncates every file carrying more than one test module, which silently reports
//!   `mission_editor.rs` as zero — then strip comments, blank string literals, count
//!   `after_local_edit()` and subtract the definition in `mission_history.rs`.
//!   (This paragraph deliberately does not spell that attribute out: `arsenal_production_src()`
//!   splits on it, so writing it here truncates the production source and hides the cites below
//!   from `t739::arsenal_cites_live_set_loadout_lines`. That pin caught exactly that mistake being
//!   made in this paragraph.) Line cites are otherwise omitted on purpose: five drifted during the
//!   127–141 remediation run, and a file+symbol survives edits that a number does not.
//!   The Arsenal's `set_loadout`
//!   (`editor_ops.rs:46`) is one of them. Its own siblings in this very modal are the clearest
//!   case: Transform X/Y/Z/rotation (`attributes.rs:265`) and Identity role/tag/stance
//!   (`attributes.rs:335`) commit on blur/Enter with no Save of their own — `attributes.rs:7` states
//!   the contract in as many words ("rebind + persist + one undo step per commit"). Same for the
//!   outliner, the ORBAT manager (`orbat_manager.rs:1301`) and the top-strip title
//!   (`eden_chrome.rs:1057`). A Save button in the Arsenal would make it the only editor in the
//!   application with a second commit point, and would break the one-undo-step-per-pick contract
//!   the module header above is built on.
//! * **The editor already has exactly one commit point, and it is not per-panel.**
//!   `after_local_edit` sets `HistoryCtx::dirty` (`mission_history.rs:62`), a debounced IDB persist
//!   keeps the work across a reload, `register_unload_guard` (T-189) refuses to let the tab close
//!   over it, and **Save Version** publishes it to the server and clears the flag. Undo is Ctrl+Z,
//!   not a per-panel discard button.
//!
//! What *was* wrong is that the author could not tell any of this from inside the Arsenal. The one
//! platform-wide "your work is not saved yet" signal is the `•` next to the mission title
//! (`eden_chrome.rs:1066`) — and this tab renders under a full-viewport `bg-black/50
//! backdrop-blur-sm` scrim (`attributes.rs:88`) that dims and blurs precisely that indicator while
//! the Arsenal is open. So the fix is not a Save button; it is saying it here, in the panel, next
//! to the verdict badge — see the `data-arsenal-persist` line at the bottom of [`ArsenalTab`]. The
//! wiring is pinned by `tests::t503`, so a future slice that quietly introduces staging goes red.
#![allow(dead_code)]

// T-934.6 — arsenal nest children (flat siblings before the move).
// T-167 — Smart-Arsenal domain core (arsenalRules.ts + arsenalDollModel.ts port; pure/native-tested).
pub mod arsenal_rules;
// T-159.22 — flat registry rows → the Factions palette tree (the T-068.3 `buildCatalogTree` port).
// Pure data, no web-sys: ungated so its unit tests run on the native `cargo test` shell.
pub mod asset_catalog;
// T-172 B10 — the 3D arsenal doll mount (DollEngine, wasm-only like the map engine host).
#[cfg(target_arch = "wasm32")]
pub mod arsenal_doll;
// T-934.8 — the pure loadout core (serialization, export/import gates, buffer verbs,
// receipts). Re-exported below so every `crate::editor::arsenal::X` path keeps working.
pub mod loadout;
// T-934.8 — the Arsenal view panels (cargo editor, doll host, compat/attachments,
// paper-doll); `ArsenalTab` below is the only caller.
mod panels;

// The whole public loadout surface re-exports, used-or-not: `crate::editor::arsenal::X` is the
// documented path (operations/cargo.rs cites `arsenal::buffer_draw` / `arsenal::stripped_loadout`
// by that name), and a bin crate lints a re-export nothing consumes yet as unused.
#[allow(unused_imports)]
pub use loadout::{
    apply_receipt, buffer_draw, buffer_refusals, commit_one_write, commit_writes, copy_receipt,
    loadout_to_picks, picks_to_export, picks_to_loadout, plan_apply, plan_remove, refusal_line,
    remove_receipt, stripped_loadout, try_export, try_import, BufferedLoadout, ImportedLoadout,
    LoadoutWrite,
};

use loadout::{
    attachments_of, export_modpack_id, import_summary, kit_default_items, loadout_faults,
    slot_asset_id,
};
use panels::{cargo_panel, compat_panel, doll_view};
use std::collections::HashMap;

use leptos::prelude::*;

use crate::editor::arsenal::arsenal_rules::{
    self as rules, format_loadout_weight, index_by_name, loadout_weight, row_options, CompatFeed,
};
use crate::v2::core::api::dto::RegistryItem;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";

/// T-503 — the persistence contract as the **author** reads it, not as the module doc reads it.
///
/// The Arsenal has no Save button (see the module header for why the rest of the SPA says it should
/// not), and until this line existed nothing in the panel said so: an author who made a pick and
/// closed the modal had no way to tell whether the pick had been kept. This is the answer, and it
/// is unconditional because the behaviour is.
const PERSIST_ALWAYS: &str = "Every pick and cargo edit here is written to the mission document the moment you make it — the Arsenal has no Save button by design, and Ctrl+Z undoes one pick.";

/// The half of the persistence line that reads the live `mission_history` dirty flag: the mission
/// itself has nothing waiting for the server. Paired with [`PERSIST_UNSAVED`].
const PERSIST_CLEAN: &str = "The mission has no unsaved changes.";

/// The dirty half: the doc holds work no server version carries yet. This is the same state the top
/// strip's `•` reports — which this modal's backdrop is busy blurring, hence the repeat here.
const PERSIST_UNSAVED: &str =
    "The mission has unsaved changes — Save Version publishes them to the server.";

/// T-779 — the third state, and the only one that reports a FAILURE. The document refused the write
/// because the entity this Arsenal was opened over is no longer in the mission; the picks on screen
/// are now local to this modal and nothing else. It overrides both states above, because "the
/// mission has no unsaved changes" is technically true and completely misleading here — the author's
/// last pick did not become a change at all.
const PERSIST_REFUSED: &str = "That last pick did NOT reach the mission document — this entity is no longer in the mission (deleted, or undone away while the Arsenal was open). Close this panel and re-open the Arsenal on a live entity; nothing you pick here now will be kept.";

/// Does the live mission document hold work the server has not seen?
///
/// `mission_history` is `cfg(target_arch = "wasm32")` (it drives the hosted doc), so the native view
/// shell answers `false`: there is no editor mounted there and therefore nothing unsaved. The read
/// itself is `try_get_untracked`, so the persistence line below tracks a local commit counter to
/// re-run — the modal scrim means an Arsenal commit is the only edit that can happen while this is
/// on screen.
fn mission_has_unsaved_work() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::editor::state::history::is_dirty()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

/// The Smart Arsenal tab — mounted in the Attributes modal (T-159.26 seam). `registry` is the flat
/// catalog; `compat` the edge feed (both fetched once by the editor); `slot_id` + `loadout_json`
/// come from the modal's re-read.
#[component]
pub fn ArsenalTab(
    slot_id: String,
    /// The slot's current `loadout` JSON (from `editor_ops::read_loadout`).
    loadout_json: Option<String>,
    /// The flat registry gear rows, `None` while loading.
    registry: RwSignal<Option<Vec<RegistryItem>>>,
    /// The compat edge feed (optic/magazine rows + validation).
    compat: RwSignal<CompatFeed>,
) -> impl IntoView {
    // T-068.15.2 — open-time cargo seed for pre-existing slots (place/apply already
    // seed at their own hooks): only fires when the loadout has no `cargo` key and
    // the character has `character_default_cargo` defaults; returns the seeded JSON
    // so this render uses it without a re-read.
    #[cfg(target_arch = "wasm32")]
    let loadout_json = crate::editor::state::operations::seed_slot_cargo(&slot_id).or(loadout_json);
    // T-504 — the slot's character prefab, read once: it cannot change while the modal is open, and
    // it keys the kit-default evidence the undeliverable-cargo rule needs.
    let asset_id = StoredValue::new(slot_asset_id(&slot_id));
    let id = StoredValue::new(slot_id);
    // Reactive picks so the doll, weight, validation, and dependent edge rows all re-render live.
    let picks = RwSignal::new(loadout_to_picks(loadout_json.as_deref()));
    // Cargo rows + whether the loadout carries the `cargo` key (the "user state" marker —
    // absent means a later seed may still fire, so persists stay key-less until touched).
    let (cargo0, cargo_present0) = rules::cargo_from_loadout(loadout_json.as_deref());
    let cargo = RwSignal::new(cargo0);
    let cargo_present = RwSignal::new(cargo_present0);
    // The rail/doll active region (highlighted row + hotspot). Default to the primary weapon.
    let active_key = RwSignal::new("primary".to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, cargo_present);

    // T-503 — commits made in this tab, purely so the persistence line below can re-run: the dirty
    // flag it reads is `try_get_untracked` and therefore not reactive on its own.
    let commits = RwSignal::new(0u32);
    // T-779 — did the DOCUMENT refuse the last write? `set_loadout` returns `false` when the slot
    // id this modal was opened with is no longer in the mission (deleted, or undone away while the
    // Arsenal sat open over it). Before T-779 that case dirtied the mission anyway, so the line
    // below at least went yellow for the wrong reason; now the tail is correctly gated and the
    // panel would otherwise render its green "no unsaved changes" verdict over a pick that never
    // landed. That is the wave-129 rule exactly — never report success over something that did not
    // happen — so the refusal gets its own state rather than silence.
    let persist_refused = RwSignal::new(false);
    // Persist the current picks + cargo as the canonical V2 loadout (one undo step). wasm-only.
    //
    // T-503 — this is THE commit, and it runs on every mutation with nothing staged in between.
    // That is deliberate and matches every other mission-document editor in the SPA; the module
    // header sets out the evidence, and `tests::t503` pins the wiring.
    let persist = move |map: &HashMap<String, String>, items: &[RegistryItem]| {
        #[cfg(target_arch = "wasm32")]
        {
            let names: HashMap<String, String> = items
                .iter()
                .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                .collect();
            let rows = cargo.get_untracked();
            let rows = cargo_present.get_untracked().then_some(rows.as_slice());
            let took = crate::editor::state::operations::set_loadout(
                &id.get_value(),
                picks_to_loadout(map, &names, rows),
            );
            // Set on every persist, not only on a refusal: a later pick that DOES land must clear
            // the warning, or the panel starts lying in the other direction.
            persist_refused.set(!took);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (map, items);
        // A re-render tick, not a success claim — the verdict itself is read from the document and
        // from `persist_refused` below, never from this counter.
        commits.update(|n| *n = n.wrapping_add(1));
    };
    // Cargo edits mark the key present, then persist through the same path.
    let persist_cargo = move |items: &[RegistryItem]| {
        cargo_present.set(true);
        persist(&picks.get_untracked(), items);
    };

    // T-686 — the import outcome, said in the panel next to the button that produced it. Two
    // signals and not one `Result` because they render differently and never both: a receipt is a
    // quiet line, a refusal is a list the author has to read.
    let import_status = RwSignal::new(String::new());
    let import_refusals = RwSignal::new(Vec::<String>::new());

    // T-699 — the loadout buffer's outcome, kept in its own pair of signals for the same reason the
    // import's is: a receipt is a quiet line and a refusal is a list, they render differently, and
    // an Apply refusal must not be mistaken for something the import did.
    let buffer_status = RwSignal::new(String::new());
    let buffer_refusals = RwSignal::new(Vec::<String>::new());
    // The buffer itself lives in an `editor_ops` thread_local (it outlives this modal — you copy in
    // one Arsenal and apply from another), so nothing about it is reactive. This counter is what the
    // Apply affordance re-reads it on.
    let buffer_epoch = RwSignal::new(0u32);

    // T-172 B10 — full screen-04 Smart Forge layout (operator-confirmed scope): region icon
    // rail · filtered item list · 3D doll (DollEngine; SVG paper-doll only as the create-error
    // fallback, the T-154 contract) · compat panel · COMPAT/VALID badges · Download loadout JSON.
    // Data flow unchanged: picks/active_key drive everything; persist writes SlotLoadoutV2.
    let doll_unavailable = RwSignal::new(false);
    let filter = RwSignal::new(String::new());
    // Switching regions clears the list filter (each region gets a fresh search).
    Effect::new(move |prev: Option<String>| {
        let k = active_key.get();
        if prev.as_deref().is_some_and(|p| p != k) {
            filter.set(String::new());
        }
        k
    });
    view! {
        <div class="flex flex-col gap-2">
            {move || match registry.get() {
                None => view! {
                    <p class="text-label-sm normal-case text-outline">"Loading catalog…"</p>
                }.into_any(),
                Some(items) => {
                    let names: HashMap<String, String> = items
                        .iter()
                        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                        .collect();
                    let items = StoredValue::new(items);
                    let names = StoredValue::new(names);
                    let pick_item = move |key: String, value: String| {
                        picks.update(|m| {
                            if value.is_empty() { m.remove(key.as_str()); }
                            else { m.insert(key.clone(), value.clone()); }
                        });
                        persist(&picks.get_untracked(), &items.get_value());
                    };
                    // T-686 — apply an ACCEPTED import. **This is the one-undo-step contract.**
                    //
                    // The three `set`s are signal writes and commit nothing; the single `persist`
                    // that follows is the only document mutation, and `persist` is one
                    // `editor_ops::set_loadout` is **at most one** `mission_history::after_local_edit`
                    // (`editor_ops.rs:62`) is at most one undo step. So Ctrl+Z after an import
                    // restores the whole loadout the author had before it — not the last wear row
                    // of it. "At most" since T-779: the tail is gated on the document having taken
                    // the write, so an import applied over an entity that is no longer in the
                    // mission mints no step at all rather than an empty one.
                    // No new atomic-batch API was needed: the Arsenal's existing commit already
                    // takes the entire `SlotLoadoutV2` document in one call, which is exactly the
                    // shape an import wants. `tests::t686::the_import_applies_in_one_commit` pins it.
                    let apply_import = move |doc: ImportedLoadout, items: &[RegistryItem]| {
                        picks.set(doc.picks);
                        cargo.set(doc.cargo);
                        cargo_present.set(doc.cargo_present);
                        persist(&picks.get_untracked(), items);
                    };
                    // T-686 — the file picker. Same off-DOM programmatic idiom as the mission
                    // upload (`missions.rs:1875`) and the CMS hero upload (`content.rs:632`): a
                    // one-shot `<input type=file>` that never sits in the DOM, so there is no dead
                    // control in a panel most authors will never import into.
                    //
                    // Read → parse → validate all happen before a single signal is written; the
                    // apply above is the only writer, and it only ever sees an `Ok`.
                    let import_loadout = move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::closure::Closure;
                            use wasm_bindgen::JsCast;

                            let picker = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.create_element("input").ok())
                                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok());
                            let Some(input) = picker else {
                                import_status.set(String::new());
                                import_refusals
                                    .set(vec!["Could not open the file picker.".to_string()]);
                                return;
                            };
                            input.set_type("file");
                            input.set_accept("application/json,.json");

                            let input_for_cb = input.clone();
                            let on_change = Closure::once(move |_ev: web_sys::Event| {
                                let Some(file) =
                                    input_for_cb.files().and_then(|list| list.item(0))
                                else {
                                    return;
                                };
                                let name = file.name();
                                import_refusals.set(Vec::new());
                                import_status.set(format!("Reading {name}…"));
                                leptos::task::spawn_local(async move {
                                    // `Blob::text()` is a Promise — the browser reads off disk on
                                    // its own thread and the tab stays interactive through it.
                                    let text = match wasm_bindgen_futures::JsFuture::from(
                                        file.text(),
                                    )
                                    .await
                                    {
                                        Ok(v) => v.as_string().unwrap_or_default(),
                                        Err(_) => {
                                            import_status.set(String::new());
                                            import_refusals.set(vec![format!(
                                                "Could not read {name}."
                                            )]);
                                            return;
                                        }
                                    };
                                    let its = items.get_value();
                                    match try_import(&text, &its, &compat.get_untracked()) {
                                        Ok(doc) => {
                                            let line = import_summary(
                                                &name,
                                                &doc,
                                                &export_modpack_id(&its),
                                            );
                                            apply_import(doc, &its);
                                            import_refusals.set(Vec::new());
                                            import_status.set(line);
                                        }
                                        Err(refusals) => {
                                            // The refusal contract, said first and said plainly:
                                            // a document that does not validate applies NOTHING.
                                            // T-737 — through `refusal_line`, so each reason
                                            // arrives with the row it is about: two rows stranded
                                            // by one weapon swap give the identical reason and
                                            // are otherwise indistinguishable.
                                            import_status.set(String::new());
                                            import_refusals.set(
                                                std::iter::once(format!(
                                                    "{name} was not applied — this loadout is unchanged.",
                                                ))
                                                .chain(refusals.iter().map(refusal_line))
                                                .collect(),
                                            );
                                        }
                                    }
                                });
                            });
                            let _ = input.add_event_listener_with_callback(
                                "change",
                                on_change.as_ref().unchecked_ref(),
                            );
                            // One-shot listener outlives this frame — the picker is
                            // fire-and-forget (the `content.rs` contract).
                            on_change.forget();
                            input.click();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // No DOM, no file picker, and no hosted document to import into.
                            let _ = (apply_import, import_status, import_refusals, items);
                        }
                    };
                    // T-699 — Apply and Remove Everything write the whole SELECTION, and this modal
                    // is open over one member of it, so this panel's signals are stale the instant
                    // they land. Mirror the doc back into them. Deliberately NOT through `persist`:
                    // these are signal writes only, and persisting here would open an N+1th
                    // transaction that re-commits what was just committed — one extra Ctrl+Z press
                    // standing between the author and the state they had.
                    let resync_open_slot = move || {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let lo = crate::editor::state::operations::read_loadout(&id.get_value());
                            picks.set(loadout_to_picks(lo.as_deref()));
                            let (rows, present) = rules::cargo_from_loadout(lo.as_deref());
                            cargo.set(rows);
                            cargo_present.set(present);
                        }
                    };
                    // T-699 Copy (3DEN-LOAD-001) — buffers EVERY selected entity's loadout.
                    let copy_loadouts = move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let n = crate::editor::state::operations::copy_loadouts_from_selection();
                            buffer_refusals.set(Vec::new());
                            buffer_status.set(if n == 0 {
                                "Nothing to copy — select the soldiers to copy from first. The buffer is unchanged.".to_string()
                            } else {
                                copy_receipt(&crate::editor::state::operations::loadout_buffer())
                            });
                            buffer_epoch.update(|e| *e = e.wrapping_add(1));
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = (buffer_status, buffer_refusals, buffer_epoch);
                    };
                    // T-699 Apply (3DEN-LOAD-002) — one buffered loadout per selected entity, drawn
                    // at random. The gate is `plan_apply` (T-686's, over the whole buffer); a
                    // refusal writes nothing, and the receipt states the real undo cost (T-732).
                    let apply_loadouts = move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let its = items.get_value();
                            let buffered = crate::editor::state::operations::loadout_buffer_len();
                            match crate::editor::state::operations::apply_loadout_buffer_to_selection(
                                &its,
                                &compat.get_untracked(),
                            ) {
                                Ok((0, _)) => {
                                    buffer_refusals.set(Vec::new());
                                    buffer_status.set(
                                        "Nothing was applied — copy at least one loadout, then select the entities to write it to.".to_string(),
                                    );
                                }
                                Ok((planned, commits)) => {
                                    resync_open_slot();
                                    buffer_refusals.set(Vec::new());
                                    buffer_status
                                        .set(apply_receipt(planned, buffered, commits));
                                }
                                Err(refusals) => {
                                    // Same refusal contract as the import, said the same way: a
                                    // buffer that does not validate applies NOTHING. T-737 — and
                                    // rendered the same way too, because the identical-reason
                                    // problem is identical here: `buffer_refusals` names WHICH
                                    // copied loadout is bad, `refusal_line` names which row in it.
                                    buffer_status.set(String::new());
                                    buffer_refusals.set(
                                        std::iter::once(
                                            "Nothing was applied — every selected loadout is unchanged.".to_string(),
                                        )
                                        .chain(refusals.iter().map(refusal_line))
                                        .collect(),
                                    );
                                }
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = (buffer_status, buffer_refusals, resync_open_slot, items, compat);
                    };
                    // T-699 Remove Everything (3DEN-LOAD-010) — the one strip verb. The nine
                    // per-category variants are `maybe` upstream and deliberately absent.
                    let strip_loadouts = move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let (planned, commits) =
                                crate::editor::state::operations::remove_all_loadouts_from_selection();
                            resync_open_slot();
                            buffer_refusals.set(Vec::new());
                            buffer_status.set(if planned == 0 {
                                "Nothing to strip — select one or more soldiers first.".to_string()
                            } else {
                                remove_receipt(planned, commits)
                            });
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = (buffer_status, buffer_refusals, resync_open_slot);
                    };
                    view! {
                        // Top badges: compat status (left) + live weight (right).
                        <div class="flex items-center justify-between">
                            {move || {
                                let s = compat.get().status;
                                let (cls, label) = match s {
                                    rules::CompatStatus::Ready => (
                                        "rounded border border-success/40 bg-success/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-success",
                                        "Compat active",
                                    ),
                                    rules::CompatStatus::Loading => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-on-surface-variant",
                                        "Compat loading…",
                                    ),
                                    rules::CompatStatus::Unavailable => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-outline",
                                        "Compat unavailable",
                                    ),
                                };
                                view! { <span class=cls data-compat-badge>{label}</span> }
                            }}
                            <div class="flex items-center gap-3">
                                // T-068.15.2 — per-container capacity readout (registry-only:
                                // max kg + grid W×H; absent values simply don't render).
                                {move || {
                                    let key = active_key.get();
                                    if !rules::CAPACITY_KEYS.contains(&key.as_str()) {
                                        return ().into_any();
                                    }
                                    let rn = picks.with(|m| m.get(key.as_str()).cloned()).filter(|v| !v.is_empty());
                                    let Some(rn) = rn else { return ().into_any() };
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let Some(it) = idx.get(rn.as_str()) else { return ().into_any() };
                                    let mut parts: Vec<String> = Vec::new();
                                    if let Some(kg) = it.max_weight_kg {
                                        parts.push(format!("max {kg} kg"));
                                    }
                                    if let (Some(w), Some(h)) = (it.cargo_grid_w, it.cargo_grid_h) {
                                        parts.push(format!("{w}\u{00d7}{h} grid"));
                                    }
                                    if parts.is_empty() {
                                        return ().into_any();
                                    }
                                    view! {
                                        <span
                                            data-capacity-badge
                                            class="rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-on-surface-variant"
                                        >
                                            {parts.join(" · ")}
                                        </span>
                                    }.into_any()
                                }}
                                {move || {
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let map = picks.get();
                                    let mut w = loadout_weight(&map, &idx);
                                    // T-197 — attachments hang off a weapon, not off a row, so
                                    // `loadout_weight` (which walks LOADOUT_ROWS) cannot see them.
                                    // A suppressor is 0.68 kg of real carried mass; omitting it
                                    // would make an "honest weight" readout quietly dishonest.
                                    // Scoped to weapons that are actually picked — that is exactly
                                    // the set `picks_to_loadout` persists attachments for.
                                    for &(key, _, _) in rules::WEAPON_SLOTS {
                                        if map.get(key).is_none_or(String::is_empty) {
                                            continue;
                                        }
                                        for rn in attachments_of(&map, key) {
                                            w.item_count += 1;
                                            match idx.get(rn.as_str()).and_then(|it| it.weight_kg) {
                                                Some(kg) => w.known_kg += kg,
                                                None => w.unknown_count += 1,
                                            }
                                        }
                                    }
                                    let w = format_loadout_weight(&w);
                                    view! {
                                        <p class="font-mono text-label-sm tabular-nums normal-case text-on-surface-variant">{w}</p>
                                    }
                                }}
                            </div>
                        </div>
                        <div class="grid h-[52vh] min-h-0 grid-cols-[44px_230px_minmax(0,1fr)_230px] gap-3">
                            // Region icon rail (14, RAIL order).
                            <div class="custom-scrollbar flex flex-col items-center gap-1 overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 py-1.5">
                                {rules::RAIL_REGIONS.iter().map(|r| {
                                    let key = r.key;
                                    view! {
                                        <button
                                            type="button"
                                            data-arsenal-rail=key
                                            aria-label=region_title(key)
                                            title=region_title(key)
                                            class=move || {
                                                let active = active_key.get() == key;
                                                let equipped = picks.with(|m| m.get(key).is_some_and(|v| !v.is_empty()));
                                                if active {
                                                    "flex size-8 items-center justify-center rounded-md bg-primary/25 text-primary"
                                                } else if equipped {
                                                    "flex size-8 items-center justify-center rounded-md text-primary/80 transition-colors hover:bg-white/10"
                                                } else {
                                                    "flex size-8 items-center justify-center rounded-md text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                                }
                                            }
                                            on:click=move |_| active_key.set(key.to_string())
                                        >
                                            <span class="material-symbols-outlined text-[18px]">{region_icon(key)}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            // Item list for the active region (filter + None + grouped options).
                            <div class="custom-scrollbar flex min-h-0 flex-col overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2">
                                {move || {
                                    let feed = compat.get();
                                    let map = picks.get();
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let key = active_key.get();
                                    let Some(row) = rules::LOADOUT_ROWS.iter().find(|r| r.key == key) else {
                                        return view! { <p class="text-label-sm text-outline">"—"</p> }.into_any();
                                    };
                                    let current = map.get(row.key).cloned().unwrap_or_default();
                                    let opts = row_options(row, &current, &map, &its, &idx, feed.ready_graph());
                                    let q = filter.get().trim().to_lowercase();
                                    let opts: Vec<_> = opts
                                        .into_iter()
                                        .filter(|o| q.is_empty() || o.label.to_lowercase().contains(&q))
                                        .collect();
                                    let count = opts.len();
                                    // Group by registry category (screen 04's WEAPONS/… headers).
                                    let mut groups: Vec<(String, Vec<rules::RowOption>)> = Vec::new();
                                    for o in opts {
                                        let cat = idx
                                            .get(o.value.as_str())
                                            .map(|it| it.category.to_uppercase())
                                            .unwrap_or_else(|| "OTHER".to_string());
                                        match groups.last_mut() {
                                            Some((c, list)) if *c == cat => list.push(o),
                                            _ => groups.push((cat, vec![o])),
                                        }
                                    }
                                    // T-197 — attachment faults are keyed on the WEAPON row, so
                                    // they surface on the row whose pick the author must change.
                                    // T-240 — over-capacity cargo joins them, keyed on the garment
                                    // row backing the container.
                                    // T-504 — and cargo with nowhere known to go, keyed on the
                                    // container's own wear row, which is the pick that fixes it.
                                    let kit = kit_default_items(&feed, asset_id.get_value().as_deref());
                                    let err = loadout_faults(&map, &cargo.get(), &feed, &idx, kit.as_ref())
                                        .into_iter()
                                        .find(|e| e.key == row.key)
                                        .map(|e| e.message);
                                    let row_key = row.key;
                                    let none_cls = if current.is_empty() {
                                        "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                    } else {
                                        "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                    };
                                    view! {
                                        <div class="flex items-center justify-between px-1 pb-1">
                                            <span class="text-label-sm font-semibold uppercase tracking-wider text-on-surface">{row.label}</span>
                                            <span class="font-mono text-label-sm text-outline">{count}</span>
                                        </div>
                                        <input
                                            type="search"
                                            aria-label=format!("Filter {}", row.label)
                                            placeholder=format!("Filter {}…", row.label.to_lowercase())
                                            prop:value=move || filter.get()
                                            on:input=move |ev| filter.set(event_target_value(&ev))
                                            class="mb-1.5 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none placeholder:text-outline focus:border-primary/60"
                                        />
                                        <button
                                            type="button"
                                            class=none_cls
                                            on:click=move |_| pick_item(row_key.to_string(), String::new())
                                        >
                                            <span>"— None —"</span>
                                            {current.is_empty().then(|| view! { <MaterialCheck /> })}
                                        </button>
                                        {groups.into_iter().map(|(cat, list)| view! {
                                            <p class="mt-1.5 px-1 font-mono text-[10px] tracking-widest text-outline uppercase">{cat}</p>
                                            {list.into_iter().map(|o| {
                                                let is_current = o.value == current;
                                                let cls = if is_current {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                                } else if o.incompatible {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-error transition-colors hover:bg-white/10"
                                                } else {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                                };
                                                let value = o.value.clone();
                                                let data_value = o.value.clone();
                                                view! {
                                                    <button
                                                        type="button"
                                                        data-value=data_value
                                                        class=cls
                                                        on:click=move |_| pick_item(row_key.to_string(), value.clone())
                                                    >
                                                        <span class="truncate normal-case">{o.label.clone()}</span>
                                                        {is_current.then(|| view! { <MaterialCheck /> })}
                                                    </button>
                                                }
                                            }).collect_view()}
                                        }).collect_view()}
                                        {err.map(|m| view! {
                                            <p class="mt-1.5 px-1 text-label-sm normal-case text-error">{m}</p>
                                        })}
                                    }
                                        .into_any()
                                }}
                            </div>
                            // Center: the 3D doll (SVG paper-doll on create failure) + caption.
                            <div class="relative flex min-h-0 flex-col overflow-hidden rounded-lg bg-[#858fa1]">
                                <div class="relative min-h-0 flex-1">
                                    {move || doll_view(picks, active_key, names, doll_unavailable)}
                                </div>
                                <p class="pointer-events-none absolute inset-x-0 bottom-1 text-center font-mono text-label-sm text-surface-container-lowest">
                                    {move || {
                                        let key = active_key.get();
                                        let label = rules::LOADOUT_ROWS.iter().find(|r| r.key == key).map_or("", |r| r.label);
                                        let name = picks.with(|m| m.get(key.as_str()).cloned()).filter(|v| !v.is_empty())
                                            .map(|rn| names.with_value(|n| n.get(&rn).cloned().unwrap_or(rn)))
                                            .unwrap_or_else(|| "empty".to_string());
                                        format!("{label} — {name}")
                                    }}
                                </p>
                            </div>
                            // Compat panel: the active item + its dependent edge slots.
                            <div class="custom-scrollbar flex min-h-0 flex-col overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2.5">
                                {move || compat_panel(picks, active_key, compat, names, items, pick_item)}
                            </div>
                        </div>
                        // T-068.15.2 — container cargo editor (SlotLoadoutV2.cargo[]; seeded from
                        // character_default_cargo; warn-only weight/volume budget).
                        <div
                            data-cargo-editor
                            class="custom-scrollbar max-h-[22vh] overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2.5"
                        >
                            {move || cargo_panel(cargo, picks, items, names, persist_cargo)}
                        </div>
                        // Bottom: validation verdict + loadout download.
                        <div class="flex items-center justify-between gap-2">
                            {move || {
                                let feed = compat.get();
                                let map = picks.get();
                                let its = items.get_value();
                                // T-197 — a stranded attachment is a real loadout fault; the
                                // verdict badge counts it alongside the edge-row faults.
                                // T-240 — and over-capacity cargo alongside both.
                                // T-504 — and cargo the kit has nowhere to put, so the badge stops
                                // saying "Loadout valid" over rows nothing was going to deliver.
                                let kit = kit_default_items(&feed, asset_id.get_value().as_deref());
                                let errs = loadout_faults(&map, &cargo.get(), &feed, &index_by_name(&its), kit.as_ref());
                                if errs.is_empty() {
                                    view! {
                                        <span
                                            data-loadout-valid
                                            class="rounded border border-success/40 bg-success/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-success"
                                        >
                                            "Loadout valid"
                                        </span>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <span
                                            data-loadout-valid
                                            class="rounded border border-error-alert/40 bg-error/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-error-alert"
                                        >
                                            {format!("{} issue(s)", errs.len())}
                                        </span>
                                    }
                                        .into_any()
                                }
                            }}
                            // T-240 — the export refusal, said out loud next to the button that
                            // stopped working. The per-container reason (with its estimate
                            // caveat) is on the garment row and on this control's tooltip.
                            <div class="flex min-w-0 items-center gap-2">
                                {move || {
                                    let its = items.get_value();
                                    let refusals = rules::cargo_capacity_errors(
                                        &picks.get(), &cargo.get(), &index_by_name(&its),
                                    );
                                    if refusals.is_empty() {
                                        return ().into_any();
                                    }
                                    let n = refusals.len();
                                    let why = refusals
                                        .iter()
                                        .map(|e| e.message.as_str())
                                        .collect::<Vec<_>>()
                                        .join("\n\n");
                                    view! {
                                        <span
                                            data-export-blocked=n.to_string()
                                            title=why
                                            class="truncate text-label-sm normal-case text-error-alert"
                                        >
                                            {format!(
                                                "Export blocked — {n} container(s) over the catalogued capacity",
                                            )}
                                        </span>
                                    }
                                        .into_any()
                                }}
                                // T-686 — the other half of the round-trip. Never disabled: the
                                // gate is `try_import`, and an author with a bad file needs to be
                                // told WHY, which requires letting them pick it.
                                <button
                                    type="button"
                                    data-loadout-import
                                    class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                    on:click=import_loadout
                                >
                                    <span class="material-symbols-outlined text-[16px]">"upload"</span>
                                    "Import loadout JSON"
                                </button>
                                <button
                                    type="button"
                                    prop:disabled=move || {
                                        let its = items.get_value();
                                        !rules::cargo_capacity_errors(
                                            &picks.get(), &cargo.get(), &index_by_name(&its),
                                        )
                                            .is_empty()
                                    }
                                    class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:border-outline-variant/20 disabled:text-outline disabled:hover:bg-transparent"
                                    on:click=move |_| {
                                        // T-199 — the FILE contract, not the doc field. `picks_to_export`
                                        // writes `loadout-export.schema.json` v2; the old call wrote the
                                        // editor's `SlotLoadoutV2` dict, which fails both `oneOf` branches
                                        // and which the mod reader refuses. An empty Arsenal still exports:
                                        // a bare-soldier document is valid and says so (all-null wear, no
                                        // weapons), where the old "clear the field" `None` had to be papered
                                        // over with a hand-written literal that was itself non-conforming.
                                        //
                                        // T-240 — through `try_export`, not `picks_to_export`. The
                                        // `disabled` attribute is the affordance; THIS is the gate. A
                                        // refusal produces no bytes, so there is nothing to download.
                                        #[cfg(target_arch = "wasm32")]
                                        if let Ok(json) = try_export(
                                            &picks.get_untracked(),
                                            &cargo.get_untracked(),
                                            &items.get_value(),
                                            &export_modpack_id(&items.get_value()),
                                        ) {
                                            let _ = crate::editor::state::commands_hotkeys::download_json("loadout-export.json", &json);
                                        }
                                    }
                                >
                                    <span class="material-symbols-outlined text-[16px]">"download"</span>
                                    "Download loadout JSON"
                                </button>
                            </div>
                        </div>
                        // T-699 — the loadout buffer. Three verbs over the live SELECTION, not over
                        // the slot this modal was opened on: Copy buffers every selected entity,
                        // Apply writes one buffered loadout to each selected entity (drawn at
                        // random), Remove Everything strips them. A buffer, NOT inheritance —
                        // T-687's parent/child kits were cancelled, and nothing here stores a link
                        // back to a source (see `arsenal::BufferedLoadout`).
                        <div class="flex min-w-0 flex-wrap items-center gap-2 rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2">
                            <span class="shrink-0 font-mono text-label-sm uppercase tracking-wider text-on-surface-variant">
                                "Loadout buffer"
                            </span>
                            <button
                                type="button"
                                data-loadout-copy
                                title="Buffer the loadout of every selected entity."
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                on:click=copy_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"content_copy"</span>
                                "Copy"
                            </button>
                            <button
                                type="button"
                                data-loadout-apply
                                title="Write one buffered loadout to each selected entity, picked at random when several are buffered."
                                prop:disabled=move || {
                                    buffer_epoch.track();
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        crate::editor::state::operations::loadout_buffer_len() == 0
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        true
                                    }
                                }
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:border-outline-variant/20 disabled:text-outline disabled:hover:bg-transparent"
                                on:click=apply_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"casino"</span>
                                "Apply"
                            </button>
                            <button
                                type="button"
                                data-loadout-strip
                                title="Clear every wear row, weapon and cargo row on the selection. Cargo stays cleared."
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                on:click=strip_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"delete_sweep"</span>
                                "Remove Everything"
                            </button>
                            <span
                                data-loadout-buffered
                                class="truncate font-mono text-label-sm tabular-nums normal-case text-outline"
                            >
                                {move || {
                                    buffer_epoch.track();
                                    #[cfg(target_arch = "wasm32")]
                                    let n = crate::editor::state::operations::loadout_buffer_len();
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let n = 0usize;
                                    format!("{n} buffered")
                                }}
                            </span>
                            // T-771's Attributes banner already owns both scopes (one-entity
                            // picks/cargo vs whole-selection Copy/Apply/Remove Everything). Local
                            // reminder only — kept short so the two disclosures do not compete.
                            <span class="basis-full text-label-sm normal-case text-outline">
                                "Buffer verbs: whole selection."
                            </span>
                        </div>
                        // T-699 — the buffer's outcome. A refusal lists EVERY reason and applied
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
                        // T-686 — the import outcome. A refusal lists EVERY reason and applied
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
                        // T-503 — the persistence contract, said in the panel. The platform's one
                        // "not saved yet" signal is the `•` beside the mission title, and this tab
                        // renders under a full-viewport blur scrim that dims exactly that. So the
                        // Arsenal repeats it here rather than leaving the author to guess whether a
                        // pick stuck. `data-arsenal-persist` carries the state for the gate harness.
                        // T-779 — `refused` is checked FIRST and wins outright. The dirty flag is
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
                            // refusal would be the contradiction the T-779 fix exists to remove.
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
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Small check glyph for the current pick row.
#[component]
fn MaterialCheck() -> impl IntoView {
    view! { <span class="material-symbols-outlined shrink-0 text-[16px]">"check"</span> }
}

/// Rail tooltip title per region.
fn region_title(key: &str) -> &'static str {
    rules::LOADOUT_ROWS
        .iter()
        .find(|r| r.key == key)
        .map_or("", |r| r.label)
}

/// Rail icon per region (Material Symbols approximations of the screen-04 glyphs).
fn region_icon(key: &str) -> &'static str {
    match key {
        "primary" => "swords",
        "optic" => "filter_center_focus",
        "magazine" => "dataset",
        "launcher" => "rocket_launch",
        "handgun" => "front_hand",
        "throwable" => "bomb",
        "headCover" => "sports_motorsports",
        "jacket" => "apparel",
        "vest" => "shield",
        "armoredVest" => "security",
        "backpack" => "backpack",
        "handwear" => "waving_hand",
        "pants" => "accessibility",
        _ => "footprint", // boots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ═══════════ T-503 — the Arsenal commits on the spot, and now says so ═══════════ */

    /// arsenal.rs with everything **unreachable** removed, so a source pin cannot be greened by a
    /// needle that no running build can reach.
    ///
    /// T-601 moved the machinery to [`crate::v2::core::test_support::class_r_scrub`], which every Class-R pin in this crate
    /// now shares. The behaviour it replaced was literal matching: `#[cfg(any())]` was a **string**
    /// compare and the constant-false conditions were a **seven-entry whitelist**, so
    /// `#[cfg( any() )]`, `if 1 > 2`, `if std::hint::black_box(false)` and `while false` all walked
    /// straight past it (measured, wave 77 F3). The replacement parses the `cfg` predicate and
    /// constant-folds the condition, so spelling and whitespace stop being the defence.
    fn live_production_src() -> String {
        // T-934.8 — the Arsenal production surface spans three files. Every pin below is
        // scoped through `only_body`, and concatenating all three keeps the whole-surface
        // claims whole-surface. Each file is scrubbed SEPARATELY: `cut_test_module` truncates
        // a haystack at its first cfg-test attribute, so scrubbing a concatenation would stop
        // examining everything after the first file's test tail.
        [
            include_str!("mod.rs"),
            include_str!("loadout.rs"),
            include_str!("panels.rs"),
        ]
        .into_iter()
        .map(crate::v2::core::test_support::class_r_scrub::live_code)
        .collect()
    }

    use crate::v2::core::test_support::class_r_scrub::only_body as fn_body;

    /// T-503 Class-R: the panel must state the persistence contract, because the platform's only
    /// unsaved indicator (the top-strip `•`) sits behind this modal's blur scrim.
    ///
    /// RED (removed): delete the `data-arsenal-persist` block from the view → "the Arsenal must
    /// carry a data-arsenal-persist line".
    /// RED (decoy): re-add it inside `if true == false { … }` → same failure.
    #[test]
    fn the_panel_states_the_persistence_contract() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub fn ArsenalTab(");
        assert!(
            tab.contains("data-arsenal-persist"),
            "the Arsenal must carry a data-arsenal-persist line the author can read"
        );
        for needle in [
            "PERSIST_ALWAYS",
            "PERSIST_CLEAN",
            "PERSIST_UNSAVED",
            "mission_has_unsaved_work()",
        ] {
            assert!(
                tab.contains(needle),
                "the persistence line must render {needle} on a live path"
            );
        }
        // The verdict badge and the per-row line both read `loadout_faults`, which is where the
        // T-504 warning lands — if either stops, the warning stops being visible.
        assert!(
            tab.matches("loadout_faults(").count() >= 2,
            "both the per-row line and the verdict badge must read loadout_faults"
        );

        // The shipped copy has to answer the question the author actually has ("did that stick?")
        // without claiming the mission is on the server, which is a different promise.
        assert!(
            PERSIST_ALWAYS.contains("no Save button"),
            "{PERSIST_ALWAYS}"
        );
        assert!(PERSIST_ALWAYS.contains("Ctrl+Z"), "{PERSIST_ALWAYS}");
        assert!(
            PERSIST_UNSAVED.contains("Save Version"),
            "{PERSIST_UNSAVED}"
        );
        assert!(
            PERSIST_CLEAN.contains("no unsaved changes"),
            "{PERSIST_CLEAN}"
        );
        assert!(!mission_has_unsaved_work(), "native shell hosts no editor");
    }

    /* ═══════════ T-686 — the import half of the round-trip ═══════════ */
    //
    // T-934.8 — the pure import-gate tests moved beside the code they pin
    // (`loadout.rs::tests::t686`); the ArsenalTab wiring pin stays beside the view.

    mod t686 {
        use super::*;

        /// T-686 / T-736 Class-R: the import must reach the live document through EXACTLY ONE
        /// commit, so Ctrl+Z restores the whole pre-import loadout rather than the last field of
        /// it. The body is pinned by SYMBOL (`let apply_import =`) through `live_code` +
        /// `only_body`: three whole-document signal writes + one `persist`, and **nothing else**.
        ///
        /// A spelling blacklist of `for` / `while` / `.iter()` is NOT enough (wave-112 MINOR-1):
        /// `.into_iter().map(|p| persist1(p)).count()`, a bare `loop {}`, or a recursive helper
        /// all keep one textual `persist(` and dodge those four needles. The leftover-token check
        /// below is the wide negative — any extra alphanumeric residue is the N-step class.
        ///
        /// RED (N steps / `for`): per-pick `persist` loop → leftover tokens (and/or persist count).
        /// RED (N steps / `into_iter`): `doc.picks.into_iter().map(|p| persist1(p)).count()` →
        /// leftover `into_iter` / `map` / `count` (the old blacklist missed this — `.into_iter()`
        /// does not contain `.iter()`).
        /// RED (N steps / `loop`): bare `loop { persist(...); break; }` → leftover `loop`/`break`.
        /// RED (ungated): call `apply_import` outside the `Ok(doc)` arm → the `try_import` pin.
        /// RED (decoy, `#[cfg(any())]`): park the picker in a dead item → same failure.
        #[test]
        fn the_import_applies_in_one_commit() {
            let live = live_production_src();
            let tab = fn_body(&live, "pub fn ArsenalTab(");
            assert!(
                tab.contains("try_import("),
                "the import must be gated on a live path"
            );
            assert!(
                tab.contains("apply_import(doc, &its)"),
                "only an accepted document may be applied"
            );
            assert!(
                tab.contains("data-loadout-import"),
                "the panel must carry an import control the author can reach"
            );

            // Locate by SYMBOL — never a whole-file needle hunt that can match the test module.
            let apply = fn_body(&live, "let apply_import =");
            let commits = apply.matches("persist(").count();
            assert_eq!(
                commits, 1,
                "an import is ONE undo step: apply_import must commit exactly once, found {commits}"
            );
            // Positive shape: the whole document lands through three signal writes, then the one
            // shared commit. Exact args so a per-field `picks.set(k, v)` walk cannot green this.
            const WHOLE_DOC: &[&str] = &[
                "picks.set(doc.picks)",
                "cargo.set(doc.cargo)",
                "cargo_present.set(doc.cargo_present)",
                "persist(&picks.get_untracked(), items)",
            ];
            let mut rest = apply.to_string();
            for needle in WHOLE_DOC {
                assert!(
                    rest.contains(needle),
                    "the apply must replace the whole loadout in one commit — missing `{needle}`"
                );
                rest = rest.replacen(needle, "", 1);
            }
            // Wide negative (T-736): after the four known live statements are removed, no
            // alphanumeric residue may remain. That is the class the spelling blacklist missed —
            // `into_iter` / `map` / `loop` / a recursive helper name all leave tokens here.
            let leftover: String = rest
                .chars()
                .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            assert!(
                leftover.is_empty(),
                "an import is ONE undo step: apply_import must be three whole-document signal \
                 writes + one persist and nothing else — leftover tokens {leftover:?} betray an \
                 N-step walk (for / into_iter / loop / recursion / …)"
            );
            assert!(
                !apply.contains("set_loadout"),
                "the apply must go through the same `persist` every other pick uses"
            );
        }
    }

    /* ═════════ T-699 — the loadout buffer: Copy · Apply (random) · Remove Everything ═════════ */
    //
    // T-934.8 — the pure planner/receipt tests moved beside the code they pin
    // (`loadout.rs::tests::t699`); the ops and panel wiring pins stay here.

    mod t699 {
        use super::*;

        /// `editor_ops.rs` with everything unreachable removed. Note it carries **no test module at
        /// all**, so the T-759 hazard that makes an `include_str!` pin match its own fixtures cannot
        /// arise here — asserted below rather than assumed, because the day somebody adds one is the
        /// day this pin needs re-reading.
        fn live_ops_src() -> String {
            // T-934.7 — the ops module was split; the exclusion pins below are whole-module
            // claims, so the haystack concatenates every submodule.
            crate::v2::core::test_support::class_r_scrub::live_code(
                &[
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/slot_attributes.rs"
                    ),
                    include_str!("../state/operations/cargo.rs"),
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/composition_library.rs"
                    ),
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../map-engine/src/data/store/operations/compositions.rs"
                    )),
                    crate::v2::core::test_support::editor_operations::CONTEXT,
                    crate::v2::core::test_support::editor_operations::ENTITY,
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/selection_transform.rs"
                    ),
                ]
                .concat(),
            )
        }

        /// Class-R: the three verbs must reach the live document through the gated, counted path —
        /// and the excluded nine must not have crept in.
        ///
        /// RED (ungated): call `plan_remove` from the apply verb instead of `plan_apply` → "Apply
        /// must be gated on plan_apply".
        /// RED (fake atomicity): move `after_local_edit()` inside the write loop, or add a second
        /// one → the one-tail assertion.
        /// RED (scope creep): add `remove_nvgs_from_selection` → the exclusion assertion names it.
        #[test]
        fn the_ops_layer_wires_the_three_verbs_and_only_the_three() {
            let ops = live_ops_src();
            assert!(
                !ops.contains("#[cfg(test)]"),
                "editor_ops.rs grew a test module — this pin's SRC would now match its own \
                 fixtures (T-759). Truncate SRC at the test module before trusting it again."
            );

            let copy = fn_body(&ops, "pub fn copy_loadouts_from_selection(");
            assert!(
                copy.contains("cargo::buffer_loadouts_from_selection("),
                "Copy must reach the buffer through the one buffering verb; body: {copy}"
            );
            let domain_cargo = crate::v2::core::test_support::class_r_scrub::live_code(
                include_str!("../../../../map-engine/src/data/store/operations/cargo.rs"),
            );
            let domain_copy = fn_body(&domain_cargo, "pub fn buffer_loadouts_from_selection(");
            assert!(
                domain_copy.contains("LOADOUT_BUFFER")
                    && domain_copy.contains("copy_loadouts_from_selection("),
                "Copy must buffer every SELECTED slot; body: {domain_copy}"
            );
            assert!(
                fn_body(&domain_cargo, "pub fn copy_loadouts_from_selection(")
                    .contains("BufferedLoadout"),
                "Copy must buffer bytes, not source ids — an id would be inheritance (T-687)"
            );

            let apply = fn_body(&ops, "pub fn apply_loadout_buffer_to_selection(");
            assert!(
                apply.contains("plan_apply("),
                "Apply must be gated on plan_apply — the T-686 rule pass"
            );
            assert!(
                !apply.contains("update_slot_loadout"),
                "Apply must not write the document behind the shared committer's back"
            );
            assert!(
                fn_body(&ops, "pub fn remove_all_loadouts_from_selection(")
                    .contains("plan_remove("),
                "Remove Everything must go through the same planner"
            );

            // The one place a loadout write reaches the document, and the undo arithmetic that
            // makes it honest: N transactions, ONE shared post-change tail (which is NOT an undo
            // boundary — see T-732).
            let commit = fn_body(&ops, "fn commit_loadout_writes(");
            assert!(commit.contains("cargo::commit_loadout_writes(core, writes)"));
            let domain = crate::v2::core::test_support::class_r_scrub::live_code(include_str!(
                "../../../../map-engine/src/data/store/operations/cargo.rs"
            ));
            let domain_commit = fn_body(&domain, "pub fn commit_loadout_writes(");
            assert_eq!(
                domain_commit.matches("update_slot_loadout(").count(),
                1,
                "exactly one write call site: {commit}"
            );
            assert_eq!(
                commit.matches("after_local_edit(").count(),
                1,
                "exactly one shared tail, fired after the writes — not per write"
            );
            assert!(
                domain_commit.contains("commit_writes("),
                "the write loop is `arsenal::commit_writes`, so the count is testable natively"
            );

            // The nine per-category strip variants are `maybe` upstream and deliberately excluded.
            for excluded in [
                "remove_nvgs",
                "remove_vests",
                "remove_goggles",
                "remove_headgear",
                "remove_weapons",
                "remove_backpack",
            ] {
                assert!(
                    !ops.contains(excluded),
                    "`{excluded}` is one of the nine excluded per-category strip verbs (marked \
                     `maybe`); T-699 ships Remove Everything and nothing narrower"
                );
            }
        }

        /// Class-R: the panel must carry all three controls and route each to its verb, and the
        /// Apply must resync this modal's signals WITHOUT a second commit — an extra `persist` there
        /// would put one more Ctrl+Z press between the author and the state they had.
        #[test]
        fn the_panel_carries_the_three_verbs_and_resyncs_without_recommitting() {
            let live = live_production_src();
            let tab = fn_body(&live, "pub fn ArsenalTab(");
            for needle in [
                "data-loadout-copy",
                "data-loadout-apply",
                "data-loadout-strip",
            ] {
                assert!(tab.contains(needle), "the panel must carry {needle}");
            }
            for wiring in [
                "on:click=copy_loadouts",
                "on:click=apply_loadouts",
                "on:click=strip_loadouts",
            ] {
                assert!(
                    tab.contains(wiring),
                    "an unwired control is not a verb: {wiring}"
                );
            }
            let resync = fn_body(&live, "let resync_open_slot =");
            assert!(
                resync.contains("read_loadout("),
                "the resync must re-read the live document"
            );
            assert!(
                !resync.contains("persist("),
                "the resync is signal writes only — a persist here is an extra undo step"
            );
        }
    }

    /// T-737 — the rendering wiring half; the refusal-construction tests moved beside the
    /// code they pin (`loadout.rs::tests::t737`).
    mod t737 {
        use super::*;

        /// Class-R: the fix has to be in the panel, not merely available to it. Both refusal lists
        /// — the import's and the Apply's — must render through `refusal_line`.
        ///
        /// RED: revert either list to `.map(|e| e.message)` → the count drops to 1.
        #[test]
        fn both_refusal_lists_render_through_refusal_line() {
            let live = live_production_src();
            let tab = fn_body(&live, "pub fn ArsenalTab(");
            assert_eq!(
                tab.matches("refusal_line").count(),
                2,
                "both refusal lists must name the row; found: {}",
                tab.matches("refusal_line").count()
            );
        }
    }

    /* ═══════════ T-739 — inverted suppress-on-multi claim cannot return ═══════════ */

    /// Class-R for the wave-112 NIT that became T-739: gap_analysis asserted multi-selection
    /// **suppresses** the Attributes modal, and a T-648 comment in `editor_ops` still said the
    /// same after T-649 inverted the guard. Pins are semantic (no false phrase) plus live line
    /// cites for `set_loadout` / its `after_local_edit` tail — hardcoding a stale number goes red
    /// the moment either cite drifts again.
    ///
    /// RED (false comment returns): restore `(it suppresses on a multi-selection)` in
    /// `rotate_selection_to_face`'s doc → "editor_ops must not re-claim suppress-on-multi".
    /// RED (gap falsehood returns): restore `multi-selection **suppresses**` in gap_analysis →
    /// "gap_analysis must not re-claim suppress-on-multi".
    /// RED (stale arsenal cite): change either `editor_ops.rs:NNNN` cite away from the live
    /// `pub fn set_loadout` / its tail line → "arsenal must cite the live set_loadout line".
    mod t739 {
        /// Production `editor_ops` source (comments kept — the defect lives in a doc
        /// comment). T-934.7 — the module was split; the suppress-on-multi absences are
        /// whole-module claims, so this concatenates every submodule.
        fn ops_src() -> String {
            [
                include_str!(
                    "../../../../map-engine/src/editing/hosted_commands/slot_attributes.rs"
                ),
                include_str!("../state/operations/cargo.rs"),
                include_str!(
                    "../../../../map-engine/src/editing/hosted_commands/composition_library.rs"
                ),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../map-engine/src/data/store/operations/compositions.rs"
                )),
                crate::v2::core::test_support::editor_operations::CONTEXT,
                crate::v2::core::test_support::editor_operations::ENTITY,
                include_str!(
                    "../../../../map-engine/src/editing/hosted_commands/selection_transform.rs"
                ),
            ]
            .concat()
        }

        /// `operations/cargo.rs` alone — the file `set_loadout` now lives in, so the computed
        /// line numbers below are REAL lines of that file (the `editor_ops.rs:` cite prefix is
        /// the historical name the arsenal docs kept across the T-934.6/.7 moves).
        fn cargo_src() -> &'static str {
            include_str!("../state/operations/cargo.rs")
        }

        fn arsenal_production_src() -> String {
            // Keep comments (the cites live there). Truncate each file at its first
            // `#[cfg(test)]` so the pin modules' own RED prose cannot green or red the
            // production-cite asserts. T-934.8 — the absence claims below are whole-surface
            // claims, so every arsenal production half concatenates.
            [
                include_str!("mod.rs"),
                include_str!("loadout.rs"),
                include_str!("panels.rs"),
            ]
            .into_iter()
            .map(|full| full.split("#[cfg(test)]").next().unwrap_or(full))
            .collect()
        }

        fn gap_src() -> &'static str {
            include_str!(
                "../../../../../../docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md"
            )
        }

        fn live_set_loadout_lines(ops: &str) -> (usize, usize) {
            let lines: Vec<&str> = ops.lines().collect();
            let set_idx = lines
                .iter()
                .position(|l| l.starts_with("pub fn set_loadout"))
                .expect("pub fn set_loadout must exist");
            let next_pub = lines[set_idx + 1..]
                .iter()
                .position(|l| l.starts_with("pub fn "))
                .map(|i| set_idx + 1 + i)
                .expect("a following pub fn after set_loadout");
            let tail_idx = lines[set_idx..next_pub]
                .iter()
                .position(|l| l.contains("mission_history::after_local_edit()"))
                .map(|i| set_idx + i)
                .expect("set_loadout must fire after_local_edit");
            (set_idx + 1, tail_idx + 1)
        }

        #[test]
        fn editor_ops_must_not_reclaim_suppress_on_multi() {
            let ops = ops_src();
            assert!(
                !ops.contains("it suppresses on a multi-selection"),
                "T-739: editor_ops must not re-claim suppress-on-multi after T-649 inverted it"
            );
            assert!(
                !ops.contains("suppresses on a multi-selection"),
                "T-739: editor_ops must not re-claim suppress-on-multi after T-649 inverted it"
            );
            // Positive: the shared opener still documents the inversion.
            assert!(
                ops.contains("sel.len() > 1 && sel.contains(&id)")
                    && ops.contains("A multi-selection now OPENS the modal"),
                "T-739: open_attrs_modal must keep the T-649 inversion prose"
            );
        }

        #[test]
        fn gap_analysis_must_not_reclaim_suppress_on_multi() {
            let gap = gap_src();
            assert!(
                !gap.contains("multi-selection **suppresses**"),
                "T-739: gap_analysis ATTR-OPEN-001 must not re-claim suppress-on-multi"
            );
            assert!(
                !gap.contains("multi-select suppression"),
                "T-739: gap_analysis must not re-claim multi-select suppression"
            );
            assert!(
                gap.contains("a multi-selection now OPENS multi-edit") && gap.contains("T-649 ✅"),
                "T-739: gap_analysis must state the T-649 open-on-multi truth"
            );
        }

        #[test]
        fn arsenal_cites_live_set_loadout_lines() {
            let ops = cargo_src();
            // Production only — the pin module itself names the old numbers in RED prose, and
            // include_str!(arsenal.rs) would otherwise false-fail on its own commentary.
            let arsenal = arsenal_production_src();
            let (set_line, tail_line) = live_set_loadout_lines(ops);
            let set_cite = format!("editor_ops.rs:{set_line}");
            let tail_cite = format!("editor_ops.rs:{tail_line}");
            assert!(
                arsenal.contains(&set_cite),
                "T-739: arsenal module docs must cite live set_loadout at {set_cite}"
            );
            assert!(
                arsenal.contains(&tail_cite),
                "T-739: arsenal import undo note must cite live after_local_edit at {tail_cite}"
            );
            // Stale numbers from the wave-112 filing must stay gone from production source.
            assert!(
                !arsenal.contains("editor_ops.rs:777"),
                "T-739: arsenal production source must not keep drifted cite editor_ops.rs:777"
            );
            assert!(
                !arsenal.contains("editor_ops.rs:1611"),
                "T-739: arsenal production source must not keep drifted cite editor_ops.rs:1611"
            );
        }
    }

    /* ═══════════ T-779 — the single write path must not fake its acknowledgement ═══════════ */

    /// T-770 gave `MissionDocCore::update_slot_loadout` a `bool` and taught the BATCH path
    /// ([`commit_writes`]) to count it. The frontend half never landed: `editor_ops::set_loadout`
    /// called the mutator as a statement and hardcoded `true` for `did`, so the history tail fired
    /// whenever `OPS_CTX` and the document merely existed. A pick against a slot id the mission no
    /// longer held dirtied the mission and minted an undo step over a document that had not
    /// changed, and no receipt on that path could tell a write from a no-op.
    ///
    /// Two pins, because the defect has two halves and one test cannot see both:
    ///
    /// * **Behaviour** — [`commit_one_write`] is driven with a refusing sink, which is the exact
    ///   production shape for an unknown id. This is the only half that can be *run*: `editor_ops`
    ///   is `cfg(target_arch = "wasm32")` from its first line, `MissionDocCore` is behind the
    ///   wasm-only `doc` feature, and neither is reachable from a native test.
    /// * **Wiring** — the live, scrubbed `editor_ops.rs` must actually route through that gate and
    ///   must not carry the discarded-ack statement anywhere. The negative runs over the WHOLE
    ///   live module and is never scoped to an item, so moving the offending statement elsewhere
    ///   cannot green it.
    ///
    /// RED (restore the hardcoded `true`): put `core.update_slot_loadout(id, loadout_json);` back
    /// as a statement with `true` under it → "T-779: editor_ops must not discard the
    /// update_slot_loadout acknowledgement".
    /// RED (ungate the tail): fire the tail unconditionally inside `commit_one_write` →
    /// "T-779: a refused write must mint no history tail".
    /// RED (swallow the answer): drop `-> bool` from `set_loadout` → "T-779: set_loadout must
    /// return the document's answer".
    /// RED (bypass the gate): call `after_local_edit` directly from `set_loadout` again →
    /// "T-779: set_loadout must gate its tail through arsenal::commit_one_write".
    mod t779 {
        use super::*;

        /// `editor_ops.rs` with comments and unreachable constructs removed, and string/char
        /// literals blanked — the needles below are calls and shapes, never copy, so a decoy
        /// parked in a literal must not match. It carries no test module of its own (asserted).
        fn live_ops() -> String {
            // T-934.7 — whole-module negatives (the discarded-ack statement must appear NOWHERE
            // in the ops surface), so the haystack concatenates every submodule.
            crate::v2::core::test_support::class_r_scrub::live_code(
                &[
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/slot_attributes.rs"
                    ),
                    include_str!("../state/operations/cargo.rs"),
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/composition_library.rs"
                    ),
                    include_str!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../map-engine/src/data/store/operations/compositions.rs"
                    )),
                    crate::v2::core::test_support::editor_operations::CONTEXT,
                    crate::v2::core::test_support::editor_operations::ENTITY,
                    include_str!(
                        "../../../../map-engine/src/editing/hosted_commands/selection_transform.rs"
                    ),
                ]
                .concat(),
            )
        }

        /// The live wiring: `set_loadout` returns the document's answer and gates the tail on it.
        #[test]
        fn set_loadout_returns_the_documents_answer_instead_of_a_hardcoded_true() {
            let ops = live_ops();
            assert!(
                !ops.contains("#[cfg(test)]"),
                "editor_ops.rs grew a test module — this pin's SRC would now match its own \
                 fixtures (T-759). Truncate SRC at the test module before trusting it again."
            );

            // NEGATIVE — deliberately unscoped. The defect is a statement that discards the
            // mutator's `bool`; scoping this to `set_loadout` would let the same shape reappear in
            // any sibling and stay green.
            assert!(
                !ops.contains("core.update_slot_loadout(id, loadout_json);"),
                "T-779: editor_ops must not discard the update_slot_loadout acknowledgement — \
                 that `bool` is what T-770 added and what tells a write from a no-op"
            );

            let item = crate::v2::core::test_support::class_r_scrub::only_item(
                &ops,
                "pub fn set_loadout(",
            );
            assert!(
                item.contains("-> bool"),
                "T-779: set_loadout must return the document's answer so the caller can surface a \
                 refusal: {item}"
            );
            assert!(
                item.contains("commit_one_write("),
                "T-779: set_loadout must gate its tail through arsenal::commit_one_write, the one \
                 seam where the refusal→no-tail arithmetic can be driven natively: {item}"
            );
            assert_eq!(
                item.matches("after_local_edit(").count(),
                1,
                "T-779: exactly one tail, and it must sit inside the gate: {item}"
            );
            assert_eq!(
                item.matches("update_slot_loadout(").count(),
                1,
                "T-779: exactly one write call site in set_loadout: {item}"
            );

            // The seed siblings carried the same shape and were fixed in the same pass.
            assert!(
                !ops.contains("core.update_slot_loadout(id, Some(json));"),
                "T-779: seed_cargo_in_core must return the sink's answer, not a hardcoded true"
            );
            assert!(
                !ops.contains("core.update_slot_loadout(id, Some(json.clone()));"),
                "T-779: seed_slot_cargo's Option must carry the sink's answer — its own tail is \
                 gated on that Option being Some"
            );
        }

        /// The refusal has to reach the OPERATOR. Gating the tail correctly means a refused pick no
        /// longer dirties the mission — so the persistence line, left alone, would answer the
        /// author's "did that stick?" with a green "The mission has no unsaved changes" over a pick
        /// that never landed. That is the wave-129 rule violated by the fix itself, so the panel
        /// grows a third state that overrides both of the others.
        #[test]
        fn a_refused_pick_is_visible_in_the_panel_not_silent() {
            let live = live_production_src();
            let tab = fn_body(&live, "pub fn ArsenalTab(");
            assert!(
                tab.contains("persist_refused"),
                "T-779: the panel must hold the refusal state, or a refused pick is silent"
            );
            assert!(
                tab.contains("PERSIST_REFUSED"),
                "T-779: the persistence line must be able to render the refusal copy"
            );
            // The refusal must be checked BEFORE the dirty flag: `mission_has_unsaved_work()` stays
            // accurate during a refusal and would otherwise get to answer the wrong question.
            let refused_at = tab.find("persist_refused.get()").expect(
                "T-779: the persistence line must READ the refusal state, not just hold it",
            );
            let unsaved_at = tab
                .find("mission_has_unsaved_work()")
                .expect("the dirty read must still be there");
            assert!(
                refused_at < unsaved_at,
                "T-779: the refusal must be decided before the dirty flag — a mission with no \
                 unsaved work is a true statement and a misleading answer when the last pick was \
                 refused"
            );

            // The commit must CAPTURE the answer rather than call and forget. Checked structurally
            // (is the call bound to something?) and not by matching one formatting of one line.
            let call_at = tab
                .find("crate::editor::state::operations::set_loadout(")
                .expect("T-779: the Arsenal must still reach set_loadout on a live path");
            let before = &tab[..call_at];
            assert!(
                before.trim_end().ends_with('='),
                "T-779: the Arsenal must not call set_loadout as a bare statement — the return is \
                 the only thing that can tell the author the write was refused. Preceding text: {}",
                &before[before.len().saturating_sub(80)..]
            );
            assert!(
                tab.contains("persist_refused.set("),
                "T-779: the captured answer must reach the panel state, or it is captured and \
                 thrown away"
            );

            // The shipped copy has to say what happened and what to do, without claiming the
            // mission is broken. Constants, so this reads the live strings.
            assert!(
                PERSIST_REFUSED.contains("did NOT reach"),
                "{PERSIST_REFUSED}"
            );
            assert!(
                PERSIST_REFUSED.contains("no longer in the mission"),
                "the refusal must name the CAUSE, or the author cannot act on it: \
                 {PERSIST_REFUSED}"
            );
            assert!(
                !PERSIST_REFUSED.contains("no unsaved changes"),
                "the refusal must not repeat the clean verdict: {PERSIST_REFUSED}"
            );
        }
    }
}
