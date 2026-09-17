//! the **Win conditions** card: the mode picker, the per-mode field, and the `endOn`
//! checklist that decide how a mission's round ends.
//!
//! ══ What this closes ═════════════════════════════════════════════════════════════════════════
//! Nothing in the editor authored `winConditions`. `flatten_to_mod_document` built the block from a
//! struct literal with `mode: "attrition"` in it and derived `endOn` from the sides that hold
//! slots, so the win rule was a compiler decision an author could neither see nor change.
//!
//! ══ The transport, and why it is the settings bag ════════════════════════════════════════════
//! Every write goes to `meta.environment.winConditions` through
//! [`operations::update_environment`], the same one-patch-one-undo-step path the  flow
//! controls use. That bag is the transport and the table is the contract — `ui/inspector/env.rs`'s own
//! words — and it is the ONLY part of `meta` with a read/write pair the editor can drive plus a
//! `hydrate` that loads it back verbatim, which is what makes an authored rule survive Save →
//! reload. `map_engine_core::mission::extensions` reads the key back out of that bag on the compile
//! side; its header carries the full argument.
//!
//! **The  gate does not run on this key, and that is deliberate rather than an oversight.**
//! `env.rs::author_env` refuses any `meta.environment` key no surface reads back, off
//! `CARRIED_ENV_KEYS` / `AUTHORED_FLOW_KEYS`. This key HAS a reader chain and it is named end to
//! end in [`WIN_CONDITIONS_READERS`] below, but the two tables live in `env.rs`, which is not this
//! slice's file. So the gate is restated here as data rather than bypassed as a habit, and the row
//! belongs in `CARRIED_ENV_KEYS` the next time that file is open.
//!
//! ══ Where the card renders ═══════════════════════════════════════════════════════════════════
//! [`win_conditions_card`] is a section in the same shape as `settings_modal::render_flow_section`
//! and renders beside it in the Mission Settings dialog. It is MOUNTED, at
//! `settings_modal.rs`'s `{win_conditions_card(ctrl)}` immediately after the Mission flow section.
//!
//!  could not land that line itself — `ui/modals/settings_modal.rs` was outside its owned file
//! list — so the mount went in with the  bookkeeping instead. A card that is built,
//! registered and tested but never mounted is a mechanism that cannot fire, which is why it landed
//! in the same wave rather than as a follow-up.
//!
//! Pure Rust + JSON; the doc-driving bodies are wasm-only (`operations` is wasm32-gated), exactly
//! like every sibling panel.
#![allow(dead_code)]
use leptos::prelude::*;

use website_map_engine::data::scenario::win_conditions::optional_param_keys_for_mode;
use website_map_engine::data::scenario::win_conditions::param_key_for_mode;
use website_map_engine::data::scenario::win_conditions::AUTHORED_MODES;
use website_map_engine::data::scenario::win_conditions::END_ON_TRIGGERS;
use website_map_engine::data::scenario::win_conditions::TIMEOUT_MINUTES_MAX;
use website_map_engine::data::scenario::win_conditions::TIMEOUT_MINUTES_MIN;

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

/// Every field a mode shows, in the order the card lays them out: `(key, label, hint)`.
///
/// A mode's REQUIRED param leads; the optional ones follow, off
/// [`optional_param_keys_for_mode`] so the card cannot offer a field the validator refuses or miss
/// one it would accept. That is the whole reason this reads two tables instead of hard-coding
/// three rows — [`tests::the_param_fields_match_the_params_each_mode_takes`] holds it.
#[must_use]
pub fn param_fields(mode: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut rows = Vec::new();
    if let Some(key) = param_key_for_mode(mode) {
        rows.push(field_for(key, true));
    }
    for key in optional_param_keys_for_mode(mode) {
        rows.push(field_for(key, false));
    }
    rows
}

/// One param key's label and hint. `required` picks the wording; the key is the same either way.
fn field_for(key: &str, required: bool) -> (&'static str, &'static str, &'static str) {
    match (key, required) {
        ("extractionZoneId", true) => (
            "extractionZoneId",
            "Extraction zone id",
            "The `zones[].id` the extracting side must reach. The compile reports a zone id that \
             is not on the mission.",
        ),
        ("extractionZoneId", false) => (
            "extractionZoneId",
            "Extraction zone id (optional)",
            "Where the VIP has to reach to win. Leave it empty for a protect-only rule — then only \
             the VIP's death ends the round.",
        ),
        ("vipSlotId", _) => (
            "vipSlotId",
            "VIP slot id",
            "The `slots[].uid` of the protected player. The compile reports a uid no placed slot \
             carries.",
        ),
        ("timeoutMinutes", _) => (
            "timeoutMinutes",
            "Round length (minutes)",
            "Sets the mission duration in Mission flow — the round clock is the one that ends the \
             round, so this is not a second timer.",
        ),
        _ => (
            "",
            "Unknown field",
            "This param has no field — add one rather than leaving the author unable to set it.",
        ),
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
    key: &str,
    raw: &str,
) -> Result<serde_json::Value, String> {
    let mut next = match current {
        Some(c) if c.is_object() => c.clone(),
        _ => default_block(mode),
    };
    next["mode"] = serde_json::json!(mode);

    if param_key_for_mode(mode) != Some(key) && !optional_param_keys_for_mode(mode).contains(&key) {
        return Err(format!("`{key}` is not a field of the `{mode}` rule."));
    }

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
    crate::v2::apps::editor::bridge::host_state::editor_context::read_env_value("winConditions")
        .filter(|v| v.is_object())
}

/// Commit one block (or a clear) — **one document write, one undo step**.
///
/// Every control in this card funnels through here for the same reason `env.rs::author_env` exists:
/// a control wired straight at `update_environment` can forget the undo tail or write a shape the
/// compile refuses, and the check belongs on the one path every control takes.
#[cfg(target_arch = "wasm32")]
fn commit(block: Option<&serde_json::Value>) {
    if let Some(clause) =
        block.and_then(|b| website_map_engine::data::scenario::win_conditions::validate(b).err())
    {
        leptos::logging::warn!("winConditions is not yet complete: {clause}");
    }
    crate::v2::apps::editor::bridge::host_state::editor_context::update_environment(env_patch(
        block,
    ));
}

/// The **Win conditions** card. `ctrl` is the dialog's shared control class, exactly as
/// `settings_modal::render_flow_section` takes it.
///
/// Inert on the native view shell (no document), like every sibling panel.
mod view;
pub use view::win_conditions_card;

#[cfg(test)]
#[path = "tests/win_conditions_card/mode_authoring.rs"]
mod tests;
