//! Tests the win conditions card subject.

use super::*;
use serde_json::json;

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
    let with_minutes =
        with_param(Some(&next), "timeout", "timeoutMinutes", "45").expect("45 is in range");
    mission_model::objectives::win_conditions::validate(&with_minutes)
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
        let next = with_param(Some(&vip), "vip", "vipSlotId", blank).expect("blank is allowed");
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
    let next = with_param(None, "extraction", "extractionZoneId", "  z-lz  ").expect("ok");
    assert_eq!(next["extractionZoneId"], "z-lz");

    let next = with_param(None, "timeout", "timeoutMinutes", " 90 ").expect("ok");
    assert_eq!(
        next["timeoutMinutes"],
        json!(90),
        "a string would fail the schema"
    );
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

/// Every edit produces a block the compile's own validator accepts — the card cannot author a
/// rule the document will then refuse. Driven through the real functions in the order an author
/// clicks them.
#[test]
fn a_full_authoring_pass_produces_a_block_the_compile_accepts() {
    let block = with_mode(None, "vip");
    let block = with_param(Some(&block), "vip", "vipSlotId", "slot_sl").expect("id");
    let block = with_trigger(Some(&block), "vip", "faction_eliminated", true).expect("tick");
    let block = with_trigger(Some(&block), "vip", "time_limit", false).expect("untick");

    assert_eq!(
        block,
        json!({
            "mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "slot_sl"
        })
    );
    mission_model::objectives::win_conditions::validate(&block)
        .expect("the card must not author a block the compile refuses");
}

/// Clearing writes an explicit `null`, not an omitted key. `update_environment` MERGES, so an
/// omitted key would leave the previous rule in place and "None" would do nothing at all.
#[test]
fn clearing_writes_an_explicit_null_patch() {
    let cleared: serde_json::Value = serde_json::from_str(&env_patch(None)).expect("patch is JSON");
    assert_eq!(cleared, json!({"winConditions": null}));

    let block = default_block("attrition");
    let set: serde_json::Value =
        serde_json::from_str(&env_patch(Some(&block))).expect("patch is JSON");
    assert_eq!(set, json!({"winConditions": block}));
}
