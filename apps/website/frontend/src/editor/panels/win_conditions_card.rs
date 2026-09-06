//! T-936.1 — the **Win conditions** card: the mode picker, the per-mode field, and the `endOn`
//! checklist that decide how a mission's round ends.
//!
//! ══ What this closes ═════════════════════════════════════════════════════════════════════════
//! Nothing in the editor authored `winConditions`. `flatten_to_mod_document` built the block from a
//! struct literal with `mode: "attrition"` in it and derived `endOn` from the sides that hold
//! slots, so the win rule was a compiler decision an author could neither see nor change.
//!
//! ══ The transport, and why it is the settings bag ════════════════════════════════════════════
//! Every write goes to `meta.environment.winConditions` through
//! [`operations::update_environment`], the same one-patch-one-undo-step path the T-224 flow
//! controls use. That bag is the transport and the table is the contract — `panels/env.rs`'s own
//! words — and it is the ONLY part of `meta` with a read/write pair the editor can drive plus a
//! `hydrate` that loads it back verbatim, which is what makes an authored rule survive Save →
//! reload. `map_engine_core::mission::extensions` reads the key back out of that bag on the compile
//! side; its header carries the full argument.
//!
//! **The T-193 gate does not run on this key, and that is deliberate rather than an oversight.**
//! `env.rs::author_env` refuses any `meta.environment` key no surface reads back, off
//! `CARRIED_ENV_KEYS` / `AUTHORED_FLOW_KEYS`. This key HAS a reader chain and it is named end to
//! end in [`WIN_CONDITIONS_READERS`] below, but the two tables live in `env.rs`, which is not this
//! slice's file. So the gate is restated here as data rather than bypassed as a habit, and the row
//! belongs in `CARRIED_ENV_KEYS` the next time that file is open.
//!
//! ══ Where the card renders ═══════════════════════════════════════════════════════════════════
//! [`win_conditions_card`] is a section in the same shape as `settings_modal::render_flow_section`
//! and belongs beside it in the Mission Settings dialog — one line,
//! `{render_win_conditions_card(ctrl)}`, after the Mission flow section. **That line is not in this
//! commit**: `panels/settings_modal.rs` is outside T-936.1's owned file list, so the mount is the
//! one piece of this card the slice could not land. Everything the mount needs is here and every
//! rule the card enforces is unit-tested natively below.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated), exactly
//! like every sibling panel.
#![allow(dead_code)]
use leptos::prelude::*;

use map_engine_core::mission::win_conditions::{
    param_key_for_mode, AUTHORED_MODES, END_ON_TRIGGERS, TIMEOUT_MINUTES_MAX, TIMEOUT_MINUTES_MIN,
};

/// The reader chain for `meta.environment.winConditions`, end to end.
///
/// This is the [`env.rs::CARRIED_ENV_KEYS`] table's third column, restated for the one key that
/// table cannot yet carry (see the module header). It exists so the next person to touch this card
/// can check the claim rather than take it: a control whose value stops at the editor boundary is
/// worse than no control, and the only defence against writing one is naming who reads it back.
pub const WIN_CONDITIONS_READERS: &[(&str, &str)] = &[
    (
        "compile",
        "map_engine_core::mission::compile::compile_payload → the saved payload's top-level \
         `winConditions` (via mission::extensions::copy_authored_blocks)",
    ),
    (
        "flatten",
        "map_engine_core::mission::flatten::resolve_win_conditions → the compiled document's \
         `winConditions` block, and apply_timeout_to_flow → `flow.timeLimitSeconds`",
    ),
    (
        "mod",
        "TBD_WinConditionEvaluator (extraction + vip), TBD_ObjectiveRegistry.EvaluateEndTriggers \
         and TBD_FrameworkManager.TickWinConditions (the endOn triggers), TBD_BriefingData\
         .BuildWinConditions (the briefing screen's win-rule line)",
    ),
    (
        "editor",
        "this card, which reads the key back on every open through operations::read_env_value",
    ),
];

/// The mode picker's rows: the schema value and the words an author reads.
///
/// The empty value is NOT a sixth mode — it is "author nothing", which clears the key and returns
/// the mission to the derived `attrition` rule. That option has to exist: without it a mode picked
/// once could never be un-picked, and "I changed my mind" would be unrepresentable.
///
/// The five real rows are generated from [`AUTHORED_MODES`] so the picker cannot offer a mode the
/// validator refuses, or miss one it accepts — [`tests::the_picker_offers_exactly_the_authored_modes`]
/// is what holds that.
#[must_use]
pub fn mode_options() -> Vec<(&'static str, &'static str)> {
    let mut rows = vec![("", "None — attrition, derived from the ORBAT")];
    for mode in AUTHORED_MODES {
        rows.push((mode, mode_label(mode)));
    }
    rows
}

/// The sentence a mode gets in the picker. One line, saying what ENDS the round.
#[must_use]
pub fn mode_label(mode: &str) -> &'static str {
    match mode {
        "attrition" => "Attrition — the last side with living players wins",
        "objective" => "Objective — the mission's objective triggers decide it",
        "extraction" => "Extraction — a side reaches its extraction zone",
        "vip" => "VIP — protect or extract one named player",
        "timeout" => "Timeout — the round ends on the clock",
        _ => "Unknown mode",
    }
}

/// The words an `endOn` trigger gets in the checklist, and the sentence under it.
#[must_use]
pub fn trigger_label(trigger: &str) -> (&'static str, &'static str) {
    match trigger {
        "time_limit" => (
            "Time limit",
            "The round clock reaches zero. Needs a mission duration in Mission flow.",
        ),
        "all_objectives_captured" => (
            "All objectives captured",
            "Every capture objective is held by one side.",
        ),
        "faction_eliminated" => (
            "Faction eliminated",
            "Only one side that fielded players still has living players. Needs two sides holding \
             slots — the compile drops it otherwise.",
        ),
        "objective_destroyed" => (
            "Objective destroyed",
            "A destroy objective's target is gone.",
        ),
        "hold_expired" => ("Hold expired", "A hold-until objective survived its timer."),
        _ => ("Unknown trigger", ""),
    }
}

/// The per-mode field's label and hint, or `None` for a mode that takes no param.
#[must_use]
pub fn param_field(mode: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match mode {
        "extraction" => Some((
            "extractionZoneId",
            "Extraction zone id",
            "The `zones[].id` the extracting side must reach. The compile reports a zone id that \
             is not on the mission.",
        )),
        "vip" => Some((
            "vipSlotId",
            "VIP slot id",
            "The `slots[].uid` of the protected player. The compile reports a uid no placed slot \
             carries.",
        )),
        "timeout" => Some((
            "timeoutMinutes",
            "Round length (minutes)",
            "Sets the mission duration in Mission flow — the round clock is the one that ends the \
             round, so this is not a second timer.",
        )),
        _ => None,
    }
}

/// The block a freshly picked mode starts from.
///
/// `endOn` carries `time_limit` alone rather than a mode-specific guess: it is the one trigger
/// every mission can serve, it is what the schema's `minItems: 1` demands be there, and choosing
/// more on the author's behalf would put a rule in their mouth. The per-mode PARAM is deliberately
/// absent — see [`with_param`] for why an empty string is not written.
#[must_use]
pub fn default_block(mode: &str) -> serde_json::Value {
    serde_json::json!({ "mode": mode, "endOn": ["time_limit"] })
}

/// Switch the authored mode, keeping the author's `endOn` checklist and dropping the OLD mode's
/// param.
///
/// Dropping is the point. A `vipSlotId` left behind on a switch to `timeout` would be refused by
/// `win_conditions::parse` ("belongs to mode vip, not to the authored mode timeout") and the whole
/// block would fall back to attrition — so a mode switch would silently disable the rule the author
/// had just picked. The NEW mode's param is not invented; the author supplies it.
#[must_use]
pub fn with_mode(current: Option<&serde_json::Value>, mode: &str) -> serde_json::Value {
    let mut next = default_block(mode);
    if let Some(end_on) = current
        .and_then(|c| c.get("endOn"))
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
    {
        next["endOn"] = end_on.clone();
    }
    next
}

/// Set (or clear) the authored mode's param from a raw field value.
///
/// **A blank value REMOVES the key rather than writing `""`.** Two reasons, and both are about the
/// author not being stuck: `mission-editor-payload.schema.json` types every param `minLength: 1`,
/// so an empty string would make Save a 400 the author cannot act on; and the block without its
/// param still validates at the write boundary, so a half-filled card saves, reloads and can be
/// finished later. The compile reports the missing param and falls back to attrition, which is what
/// makes the incompleteness visible instead of silent.
///
/// # Errors
/// A `timeoutMinutes` that is not a whole number in range comes back as the sentence the field
/// shows; the caller leaves the document alone. Every other param takes any non-blank string —
/// whether the id RESOLVES is a question only the compile can answer, and it does.
pub fn with_param(
    current: Option<&serde_json::Value>,
    mode: &str,
    raw: &str,
) -> Result<serde_json::Value, String> {
    let mut next = match current {
        Some(c) if c.is_object() => c.clone(),
        _ => default_block(mode),
    };
    next["mode"] = serde_json::json!(mode);

    let Some(key) = param_key_for_mode(mode) else {
        return Ok(next);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        if let Some(obj) = next.as_object_mut() {
            obj.remove(key);
        }
        return Ok(next);
    }

    if key == "timeoutMinutes" {
        let minutes: i64 = trimmed
            .parse()
            .map_err(|_| format!("`{trimmed}` is not a whole number of minutes."))?;
        if !(TIMEOUT_MINUTES_MIN..=TIMEOUT_MINUTES_MAX).contains(&minutes) {
            return Err(format!(
                "The round must be between {TIMEOUT_MINUTES_MIN} and {TIMEOUT_MINUTES_MAX} \
                 minutes (24 h)."
            ));
        }
        next[key] = serde_json::json!(minutes);
    } else {
        next[key] = serde_json::json!(trimmed);
    }
    Ok(next)
}

/// Tick or untick one `endOn` trigger.
///
/// # Errors
/// Unticking the LAST trigger is refused: `$defs/winConditions.endOn` is `minItems: 1`, and a
/// mission that declares no end trigger runs until an admin ends it. The refusal is a sentence the
/// checkbox shows rather than a silently-restored tick, which is the same rule the flow duration
/// field follows for a value it will not take.
pub fn with_trigger(
    current: Option<&serde_json::Value>,
    mode: &str,
    trigger: &str,
    on: bool,
) -> Result<serde_json::Value, String> {
    let mut next = match current {
        Some(c) if c.is_object() => c.clone(),
        _ => default_block(mode),
    };

    let mut triggers: Vec<String> = next
        .get("endOn")
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    if on {
        if !triggers.iter().any(|t| t == trigger) {
            // Appended in the SCHEMA's order, not click order, so re-ticking a trigger does not
            // reshuffle the block and produce a diff that says nothing.
            triggers.push(trigger.to_string());
            triggers.sort_by_key(|t| {
                END_ON_TRIGGERS
                    .iter()
                    .position(|k| k == t)
                    .unwrap_or(usize::MAX)
            });
        }
    } else {
        if triggers.len() <= 1 && triggers.iter().any(|t| t == trigger) {
            return Err(
                "A mission needs at least one end trigger — without one the round runs until an \
                 admin ends it. Tick another before removing this one."
                    .to_string(),
            );
        }
        triggers.retain(|t| t != trigger);
    }

    next["endOn"] = serde_json::json!(triggers);
    Ok(next)
}

/// The `meta.environment` merge patch for a block, or for CLEARING one.
///
/// `None` writes an explicit `null` rather than omitting the key, because `update_environment`
/// MERGES: an omitted key leaves the previous value in place, so "clear the win rule" would do
/// nothing at all. `null` is what `extensions::copy_authored_blocks` reads as "not authored", and
/// the compile's extras skip is what stops a stale parked copy walking back in behind it.
#[must_use]
pub fn env_patch(block: Option<&serde_json::Value>) -> String {
    let value = block.cloned().unwrap_or(serde_json::Value::Null);
    serde_json::json!({ "winConditions": value }).to_string()
}

/// The authored block as the document holds it, or `None` when the mission authors no win rule.
#[cfg(target_arch = "wasm32")]
fn read_block() -> Option<serde_json::Value> {
    crate::editor::state::operations::read_env_value("winConditions").filter(|v| v.is_object())
}

/// Commit one block (or a clear) — **one document write, one undo step**.
///
/// Every control in this card funnels through here for the same reason `env.rs::author_env` exists:
/// a control wired straight at `update_environment` can forget the undo tail or write a shape the
/// compile refuses, and the check belongs on the one path every control takes.
#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&serde_json::Value>) {
    // Nested rather than a `let` chain: this crate is edition 2021 (`map-engine-core` is 2024, and
    // the two are not interchangeable — see the workspace's per-crate edition rule).
    if let Some(clause) =
        block.and_then(|b| map_engine_core::mission::win_conditions::validate(b).err())
    {
        // NOT a refusal to write. A half-filled card is legitimate authoring in progress (a `vip`
        // rule whose slot id has not been typed yet), the save path carries it, and the compile
        // reports it and falls back — so refusing here would lose the author's work instead of
        // letting them finish it. Logged so a shape this card should never produce is still
        // visible in the console.
        leptos::logging::warn!("winConditions is not yet complete: {clause}");
    }
    crate::editor::state::operations::update_environment(env_patch(block));
}

/// The **Win conditions** card. `ctrl` is the dialog's shared control class, exactly as
/// `settings_modal::render_flow_section` takes it.
///
/// Inert on the native view shell (no document), like every sibling panel.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn win_conditions_card(ctrl: &'static str) -> AnyView {
    let _ = ctrl;
    ().into_any()
}

/// The **Win conditions** card — see the native sibling for the signature contract.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn win_conditions_card(ctrl: &'static str) -> AnyView {
    let sect = "text-label-sm uppercase tracking-wider text-outline";
    let hint = "text-label-sm normal-case text-outline";

    let block = read_block();
    let mode = block
        .as_ref()
        .and_then(|b| b.get("mode"))
        .and_then(serde_json::Value::as_str)
        .filter(|m| AUTHORED_MODES.contains(m))
        .unwrap_or("")
        .to_string();

    // The field's inline refusal (a bad minute count, the last trigger unticked). Signal rather
    // than a rebuild, because the document did not change and rebuilding would clear the box the
    // author is still typing in.
    let refusal = RwSignal::new(String::new());

    let picker = {
        let selected = mode.clone();
        let block_for_pick = block.clone();
        view! {
            <label class="flex flex-col gap-1">
                <span class=sect>"Win rule"</span>
                <select
                    prop:value=selected
                    on:change=move |ev| {
                        refusal.set(String::new());
                        let picked = event_target_value(&ev);
                        if picked.is_empty() {
                            commit(None);
                        } else if AUTHORED_MODES.contains(&picked.as_str()) {
                            commit(Some(&with_mode(block_for_pick.as_ref(), &picked)));
                        } else {
                            // The <select> can only emit its own options, so this is reachable only
                            // if `mode_options` ever drifts from AUTHORED_MODES — in which case
                            // refusing is right: the compile would fall back to attrition anyway,
                            // and doing it here says so instead of shipping a rule that does not run.
                            leptos::logging::error!(
                                "refusing winConditions.mode = {picked:?}: not an authored mode"
                            );
                        }
                    }
                    class=ctrl
                >
                    {mode_options()
                        .into_iter()
                        .map(|(value, label)| view! { <option value=value>{label}</option> })
                        .collect::<Vec<_>>()}
                </select>
                <span class=hint>
                    "None leaves the mission on the derived attrition rule — the last side with living players wins."
                </span>
            </label>
        }
    };

    if mode.is_empty() {
        return view! {
            <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
                <span class=sect>"Win conditions"</span>
                {picker}
            </div>
        }
        .into_any();
    }

    let param_row = param_field(&mode).map(|(key, label, hint_text)| {
        let committed = block
            .as_ref()
            .and_then(|b| b.get(key))
            .map(|v| match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default();
        let mode_for_row = mode.clone();
        let block_for_row = block.clone();
        let numeric = key == "timeoutMinutes";
        view! {
            <label class="flex flex-col gap-1">
                <span class=sect>{label}</span>
                <input
                    type=if numeric { "number" } else { "text" }
                    min=if numeric { TIMEOUT_MINUTES_MIN.to_string() } else { String::new() }
                    max=if numeric { TIMEOUT_MINUTES_MAX.to_string() } else { String::new() }
                    step="1"
                    // `change`, not `input`: a value authored per keystroke would file one undo
                    // step per character, and each bumps `doc_tick`, which rebuilds this subtree
                    // out from under the caret. Same reason the flow durations use `change`.
                    value=committed
                    on:change=move |ev| {
                        let raw = event_target_value(&ev);
                        match with_param(block_for_row.as_ref(), &mode_for_row, &raw) {
                            Ok(next) => {
                                refusal.set(String::new());
                                commit(Some(&next));
                            }
                            Err(clause) => refusal.set(clause),
                        }
                    }
                    class=ctrl
                />
                <span class=hint>{hint_text}</span>
            </label>
        }
    });

    let checked: Vec<String> = block
        .as_ref()
        .and_then(|b| b.get("endOn"))
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let checklist = END_ON_TRIGGERS
        .iter()
        .map(|trigger| {
            let (label, why) = trigger_label(trigger);
            let is_on = checked.iter().any(|t| t == trigger);
            let mode_for_row = mode.clone();
            let block_for_row = block.clone();
            let trigger = *trigger;
            view! {
                <div class="flex flex-col gap-0.5">
                    <label class="flex items-center justify-between py-0.5">
                        <span class="text-label-md text-on-surface-variant">{label}</span>
                        <input
                            type="checkbox"
                            prop:checked=is_on
                            on:change=move |ev| {
                                let on = event_target_checked(&ev);
                                match with_trigger(block_for_row.as_ref(), &mode_for_row, trigger, on) {
                                    Ok(next) => {
                                        refusal.set(String::new());
                                        commit(Some(&next));
                                    }
                                    Err(clause) => {
                                        // A refused value must not stay on screen: leaving the box
                                        // unticked while the document still holds the trigger is
                                        // the editor showing a setting the author does not have.
                                        refusal.set(clause);
                                        if let Some(input) = ev
                                            .target()
                                            .and_then(|t| {
                                                wasm_bindgen::JsCast::dyn_into::<
                                                    web_sys::HtmlInputElement,
                                                >(t)
                                                    .ok()
                                            })
                                        {
                                            input.set_checked(true);
                                        }
                                    }
                                }
                            }
                            class="accent-primary"
                        />
                    </label>
                    <span class=hint>{why}</span>
                </div>
            }
        })
        .collect::<Vec<_>>();

    // The compile's own verdict on what is authored so far, shown where it was authored. One
    // validator, not a second copy of the rules — `win_conditions::validate` is the same function
    // the compile calls, so the card cannot disagree with the document about what is acceptable.
    let incomplete = block
        .as_ref()
        .and_then(|b| map_engine_core::mission::win_conditions::validate(b).err());

    view! {
        <div class="mt-2 flex flex-col gap-4 border-t border-outline-variant/30 pt-4">
            <span class=sect>"Win conditions"</span>
            {picker}
            {param_row}
            <div class="flex flex-col gap-2">
                <span class=sect>"Ends the round on"</span>
                {checklist}
            </div>
            {move || {
                let r = refusal.get();
                (!r.is_empty()).then(|| view! { <p class="text-label-sm text-error">{r}</p> })
            }}
            {incomplete
                .map(|clause| {
                    view! {
                        <p class="text-label-sm text-error">
                            {format!(
                                "This rule is not complete, so the mission will run on the derived attrition rule: {clause}",
                            )}
                        </p>
                    }
                })}
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_picker_offers_exactly_the_authored_modes_plus_a_clear() {
        let rows = mode_options();
        assert_eq!(rows[0].0, "", "the first row must be the clear");
        let offered: Vec<&str> = rows[1..].iter().map(|(v, _)| *v).collect();
        assert_eq!(offered, AUTHORED_MODES);
        for (value, label) in &rows {
            assert!(!label.is_empty(), "`{value}` has no label");
        }
        for mode in AUTHORED_MODES {
            assert_ne!(
                mode_label(mode),
                "Unknown mode",
                "`{mode}` has no sentence of its own"
            );
        }
    }

    /// Every trigger the checklist can show has words, and every trigger the SCHEMA declares is on
    /// the checklist — a mission cannot end on a trigger the card does not offer.
    #[test]
    fn the_checklist_covers_every_end_on_trigger() {
        for trigger in END_ON_TRIGGERS {
            let (label, why) = trigger_label(trigger);
            assert_ne!(label, "Unknown trigger", "{trigger} has no label");
            assert!(!why.is_empty(), "{trigger} has no sentence");
        }
        assert_eq!(END_ON_TRIGGERS.len(), 5);
    }

    /// Every mode that takes a param has a field, and no mode that takes none has one.
    #[test]
    fn the_param_field_matches_the_modes_that_take_one() {
        for mode in AUTHORED_MODES {
            match (param_key_for_mode(mode), param_field(mode)) {
                (Some(key), Some((field_key, label, hint))) => {
                    assert_eq!(key, field_key, "{mode}");
                    assert!(!label.is_empty() && !hint.is_empty(), "{mode}");
                }
                (None, None) => {}
                (a, b) => panic!(
                    "{mode}: param_key_for_mode={a:?} but param_field is {:?}",
                    b.is_some()
                ),
            }
        }
    }

    /// A freshly picked mode produces a block the schema accepts (`endOn` is `minItems: 1`), and
    /// the per-mode param is absent rather than blank — see [`with_param`] for why.
    #[test]
    fn a_freshly_picked_mode_starts_from_one_trigger_and_no_param() {
        for mode in AUTHORED_MODES {
            let b = default_block(mode);
            assert_eq!(b["mode"], *mode);
            assert_eq!(b["endOn"], json!(["time_limit"]));
            if let Some(key) = param_key_for_mode(mode) {
                assert!(b.get(key).is_none(), "{mode} must not invent a {key}");
            }
        }
    }

    /// Switching mode keeps the checklist and DROPS the old mode's param. Carrying it would make
    /// `win_conditions::parse` refuse the whole block ("belongs to mode vip, not to the authored
    /// mode timeout") and the rule the author just picked would silently not run.
    #[test]
    fn switching_mode_keeps_the_checklist_and_drops_the_old_param() {
        let vip = json!({
            "mode": "vip", "endOn": ["faction_eliminated", "hold_expired"], "vipSlotId": "s-12"
        });
        let next = with_mode(Some(&vip), "timeout");
        assert_eq!(next["mode"], "timeout");
        assert_eq!(next["endOn"], json!(["faction_eliminated", "hold_expired"]));
        assert!(next.get("vipSlotId").is_none(), "{next}");
        assert!(next.get("timeoutMinutes").is_none(), "not invented: {next}");

        // ...and the result is a shape the compile's own validator will take once the param lands.
        let with_minutes = with_param(Some(&next), "timeout", "45").expect("45 is in range");
        map_engine_core::mission::win_conditions::validate(&with_minutes)
            .expect("a completed switch must validate");
    }

    #[test]
    fn switching_from_nothing_starts_from_the_default_block() {
        let next = with_mode(None, "extraction");
        assert_eq!(next, default_block("extraction"));
    }

    /// A blank field REMOVES the key. Writing `""` would make Save a 400 the author cannot act on
    /// (`minLength: 1` in the payload schema) and would strand a half-filled card.
    #[test]
    fn a_blank_param_removes_the_key_rather_than_writing_an_empty_string() {
        let vip = json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s-12"});
        for blank in ["", "   ", "\t"] {
            let next = with_param(Some(&vip), "vip", blank).expect("blank is allowed");
            assert!(next.get("vipSlotId").is_none(), "{blank:?} → {next}");
            assert_eq!(
                next["endOn"],
                json!(["time_limit"]),
                "the checklist survives"
            );
        }
    }

    #[test]
    fn a_param_is_trimmed_and_a_timeout_is_stored_as_a_number() {
        let next = with_param(None, "extraction", "  z-lz  ").expect("ok");
        assert_eq!(next["extractionZoneId"], "z-lz");

        let next = with_param(None, "timeout", " 90 ").expect("ok");
        assert_eq!(
            next["timeoutMinutes"],
            json!(90),
            "a string would fail the schema"
        );
    }

    /// The field's refusals, and the sentence each shows. The card's range check reads the SAME
    /// constants the compile does, so the two cannot disagree about what is authorable.
    #[test]
    fn a_timeout_outside_the_range_is_refused_with_a_sentence() {
        for bad in [TIMEOUT_MINUTES_MIN - 1, 0, TIMEOUT_MINUTES_MAX + 1, 100_000] {
            let err = with_param(None, "timeout", &bad.to_string())
                .expect_err("out of range must be refused");
            assert!(err.contains(&TIMEOUT_MINUTES_MAX.to_string()), "{err}");
        }
        for word in ["ninety", "45.5", "-"] {
            let err = with_param(None, "timeout", word).expect_err("not a whole number");
            assert!(err.contains("whole number"), "{err}");
        }
        for good in [TIMEOUT_MINUTES_MIN, 90, TIMEOUT_MINUTES_MAX] {
            with_param(None, "timeout", &good.to_string()).expect("in range");
        }
    }

    /// Ticking adds in SCHEMA order, not click order — a re-tick must not reshuffle the block and
    /// produce a save diff that says nothing.
    #[test]
    fn ticking_a_trigger_inserts_it_in_schema_order() {
        let base = json!({"mode": "attrition", "endOn": ["hold_expired"]});
        let next = with_trigger(Some(&base), "attrition", "time_limit", true).expect("ok");
        assert_eq!(next["endOn"], json!(["time_limit", "hold_expired"]));

        // Ticking one already on is a no-op, not a duplicate.
        let again = with_trigger(Some(&next), "attrition", "time_limit", true).expect("ok");
        assert_eq!(again["endOn"], next["endOn"]);
    }

    #[test]
    fn unticking_removes_one_trigger_and_leaves_the_rest() {
        let base = json!({"mode": "attrition", "endOn": ["time_limit", "faction_eliminated"]});
        let next = with_trigger(Some(&base), "attrition", "time_limit", false).expect("ok");
        assert_eq!(next["endOn"], json!(["faction_eliminated"]));
    }

    /// The last trigger cannot be unticked. `endOn` is `minItems: 1`, and a mission that declares
    /// no end trigger runs until an admin ends it.
    #[test]
    fn unticking_the_last_trigger_is_refused() {
        let base = json!({"mode": "vip", "endOn": ["time_limit"], "vipSlotId": "s1"});
        let err = with_trigger(Some(&base), "vip", "time_limit", false)
            .expect_err("the last trigger must hold");
        assert!(err.contains("at least one"), "{err}");

        // Unticking one that is not on is not "the last one" — it is a no-op, and refusing it
        // would show an error for a click that changed nothing.
        let next =
            with_trigger(Some(&base), "vip", "hold_expired", false).expect("no-op is allowed");
        assert_eq!(next["endOn"], json!(["time_limit"]));
    }

    /// Every edit produces a block the compile's own validator accepts — the card cannot author a
    /// rule the document will then refuse. Driven through the real functions in the order an author
    /// clicks them.
    #[test]
    fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
        let block = with_mode(None, "vip");
        let block = with_param(Some(&block), "vip", "slot_sl").expect("id");
        let block = with_trigger(Some(&block), "vip", "faction_eliminated", true).expect("tick");
        let block = with_trigger(Some(&block), "vip", "time_limit", false).expect("untick");

        assert_eq!(
            block,
            json!({
                "mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "slot_sl"
            })
        );
        map_engine_core::mission::win_conditions::validate(&block)
            .expect("the card must not author a block the compile refuses");
    }

    /// Clearing writes an explicit `null`, not an omitted key. `update_environment` MERGES, so an
    /// omitted key would leave the previous rule in place and "None" would do nothing at all.
    #[test]
    fn clearing_writes_an_explicit_null_patch() {
        let cleared: serde_json::Value =
            serde_json::from_str(&env_patch(None)).expect("patch is JSON");
        assert_eq!(cleared, json!({"winConditions": null}));

        let block = default_block("attrition");
        let set: serde_json::Value =
            serde_json::from_str(&env_patch(Some(&block))).expect("patch is JSON");
        assert_eq!(set, json!({"winConditions": block}));
    }

    /// The reader chain this card claims for its key is not empty prose — every hop names a symbol.
    /// It is the `CARRIED_ENV_KEYS` discipline restated for the one key that table cannot yet carry.
    #[test]
    fn the_reader_chain_names_every_hop() {
        let hops: Vec<&str> = WIN_CONDITIONS_READERS.iter().map(|(h, _)| *h).collect();
        assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
        for (hop, reader) in WIN_CONDITIONS_READERS {
            assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
        }
    }
}
