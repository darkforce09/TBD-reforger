//! Role: win conditions.
//! Position: `mission/compiler/flatten` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{DiagnosticAcc, HashSet, ModFlow, ModWinConditions, render_authored_str};

/// ══ What the authored path still refuses to carry ════════════════════════════════════════════ Two gates survive the author's choice, and both are about a document the game server would REJECT rather than about second-guessing the rule:.
pub(super) fn resolve_win_conditions(
    authored: Option<&crate::data::scenario::win_conditions::AuthoredWinConditions>,
    derived_end_on: Vec<String>,
    sides_holding_slots: usize,
    zone_ids: &HashSet<&str>,
    slot_uids: &HashSet<&str>,
    diagnostics: &mut DiagnosticAcc,
) -> ModWinConditions {
    let Some(a) = authored else {
        return ModWinConditions {
            mode: "attrition".to_string(),
            end_on: derived_end_on,
            params: crate::data::scenario::win_conditions::WinConditionParams::default(),
        };
    };

    let mut end_on: Vec<String> = Vec::with_capacity(a.end_on.len());
    for trigger in &a.end_on {
        if trigger == "faction_eliminated" && sides_holding_slots < 2 {
            diagnostics.win_conditions(
                format!(
                    "`winConditions.endOn` declares `faction_eliminated` and only \
                     {sides_holding_slots} faction(s) hold slots, so the compile drops that \
                     trigger. `TBD_MissionValidator` REFUSES a document that declares it with no \
                     second side to eliminate — the server would park in LOADING with nothing you \
                     could change from the editor."
                ),
                "winConditions",
            );
            continue;
        }
        end_on.push(trigger.clone());
    }

    if end_on.is_empty() {
        end_on.push(crate::data::scenario::win_conditions::FALLBACK_TRIGGER.to_string());
        diagnostics.win_conditions(
            format!(
                "`winConditions.endOn` has nothing left after the checks above, and the schema \
                 requires at least one trigger — the compile falls back to `{}`. A mission that \
                 declares no end trigger runs until an admin ends it.",
                crate::data::scenario::win_conditions::FALLBACK_TRIGGER
            ),
            "winConditions",
        );
    }

    if a.mode == "timeout" && !end_on.iter().any(|t| t == "time_limit") {
        end_on.push("time_limit".to_string());
        diagnostics.win_conditions(
            "`winConditions.mode` is `timeout` and `endOn` did not declare `time_limit`, so the \
             compile adds it. The round clock only arms when that trigger is declared, so without \
             it the authored timeout could never end the round."
                .to_string(),
            "winConditions",
        );
    }

    if let Some(zone_id) = a.params.extraction_zone_id.as_deref()
        && !zone_ids.contains(zone_id)
    {
        diagnostics.win_conditions(
            format!(
                "`winConditions.extractionZoneId` is {} and no zone with that id reached the \
                 compiled document, so the extraction can never be reached and the round can never \
                 be won on it. The value is carried unchanged — `TBD_WinConditionEvaluator` \
                 resolves it through `TBD_ZoneRegistry` and refuses to evaluate rather than \
                 choosing a zone for you.",
                render_authored_str(zone_id)
            ),
            zone_id,
        );
    }
    if let Some(slot_uid) = a.params.vip_slot_id.as_deref()
        && !slot_uids.contains(slot_uid)
    {
        diagnostics.win_conditions(
            format!(
                "`winConditions.vipSlotId` is {} and no slot with that uid reached the compiled \
                 document, so there is no VIP to protect or extract. The value is carried \
                 unchanged — `TBD_WinConditionEvaluator` refuses to evaluate rather than promoting \
                 some other player to VIP.",
                render_authored_str(slot_uid)
            ),
            slot_uid,
        );
    }

    ModWinConditions {
        mode: a.mode.clone(),
        end_on,
        params: a.params.clone(),
    }
}

/// Apply timeout to flow using the supplied domain data.
pub(super) fn apply_timeout_to_flow(
    flow: &mut ModFlow,
    win: &ModWinConditions,
    flow_time_limit_authored: bool,
    diagnostics: &mut DiagnosticAcc,
) {
    let Some(minutes) = win.params.timeout_minutes else {
        return;
    };
    let seconds = minutes * 60;
    if flow.time_limit_seconds == seconds {
        return;
    }

    if !flow_time_limit_authored {
        flow.time_limit_seconds = seconds;
        return;
    }
    diagnostics.win_conditions(
        format!(
            "`winConditions.mode` is `timeout` with `timeoutMinutes` {minutes} ({seconds} s), and \
             `flow.timeLimitSeconds` was {} — the compile emits {seconds}. The win rule is the \
             more specific statement, and the round clock reads `flow.timeLimitSeconds`, so the \
             two cannot both stand.",
            flow.time_limit_seconds
        ),
        "winConditions",
    );
    flow.time_limit_seconds = seconds;
}
