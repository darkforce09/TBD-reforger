//! Tests the win conditions card subject.

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

/// The fields a mode shows are exactly the params it may carry — its required one plus its
/// optional ones, in that order. A field the validator refuses would author a block the compile
/// throws away; a missing field would make a legal param unreachable.
#[test]
fn the_param_fields_match_the_params_each_mode_takes() {
    for mode in AUTHORED_MODES {
        let shown: Vec<&str> = param_fields(mode).into_iter().map(|(k, _, _)| k).collect();
        let mut expected: Vec<&str> = Vec::new();
        if let Some(key) = param_key_for_mode(mode) {
            expected.push(key);
        }
        expected.extend(optional_param_keys_for_mode(mode).iter().copied());
        assert_eq!(shown, expected, "{mode}");

        for (key, label, hint) in param_fields(mode) {
            assert!(!key.is_empty(), "{mode} shows a field with no key");
            assert_ne!(label, "Unknown field", "{mode}: `{key}` has no words");
            assert!(!hint.is_empty(), "{mode}: `{key}` has no sentence");
        }
    }
    // vip is the mode with two fields — the required VIP and the optional zone that makes
    // "the VIP got out" observable.
    assert_eq!(
        param_fields("vip")
            .into_iter()
            .map(|(k, _, _)| k)
            .collect::<Vec<_>>(),
        ["vipSlotId", "extractionZoneId"]
    );
}

/// A field a mode may not carry is refused rather than written. `win_conditions::parse` refuses
/// the WHOLE block over a stray param, so writing one would silently disable the rule the
/// author is editing.
#[test]
fn a_field_that_is_not_the_modes_own_is_refused() {
    let err = with_param(None, "timeout", "vipSlotId", "s1").expect_err("timeout has no VIP");
    assert!(err.contains("vipSlotId"), "{err}");
    assert!(err.contains("timeout"), "{err}");
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
    let with_minutes =
        with_param(Some(&next), "timeout", "timeoutMinutes", "45").expect("45 is in range");
    website_map_engine::data::scenario::win_conditions::validate(&with_minutes)
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

/// The field's refusals, and the sentence each shows. The card's range check reads the SAME
/// constants the compile does, so the two cannot disagree about what is authorable.
#[test]
fn a_timeout_outside_the_range_is_refused_with_a_sentence() {
    for bad in [TIMEOUT_MINUTES_MIN - 1, 0, TIMEOUT_MINUTES_MAX + 1, 100_000] {
        let err = with_param(None, "timeout", "timeoutMinutes", &bad.to_string())
            .expect_err("out of range must be refused");
        assert!(err.contains(&TIMEOUT_MINUTES_MAX.to_string()), "{err}");
    }
    for word in ["ninety", "45.5", "-"] {
        let err =
            with_param(None, "timeout", "timeoutMinutes", word).expect_err("not a whole number");
        assert!(err.contains("whole number"), "{err}");
    }
    for good in [TIMEOUT_MINUTES_MIN, 90, TIMEOUT_MINUTES_MAX] {
        with_param(None, "timeout", "timeoutMinutes", &good.to_string()).expect("in range");
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
    let next = with_trigger(Some(&base), "vip", "hold_expired", false).expect("no-op is allowed");
    assert_eq!(next["endOn"], json!(["time_limit"]));
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
    website_map_engine::data::scenario::win_conditions::validate(&block)
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
