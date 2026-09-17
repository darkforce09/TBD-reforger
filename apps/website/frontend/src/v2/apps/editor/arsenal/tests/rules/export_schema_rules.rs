use super::*;
use website_map_engine::data::store::operations::cargo_rules::WEAR_PICK_KEYS;

/// The pattern the shipped schema actually uses must be one this matcher can EVALUATE.
/// If it ever cannot, every wear key becomes a refusal — so this failing is the early warning
/// that the matcher has to grow, not a cosmetic nit.
#[test]
fn the_shipped_wear_key_pattern_is_evaluable_and_correct() {
    let schema: serde_json::Value = serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON).unwrap();
    let v2 = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["properties"]["loadoutVersion"]["const"] == "2")
        .expect("a v2 branch");
    let patterns = v2["properties"]["wear"]["patternProperties"]
        .as_object()
        .expect("wear is pattern-keyed");
    assert_eq!(patterns.len(), 1);
    let pattern = patterns.keys().next().unwrap();

    // Every canonical engine key the Arsenal writes.
    for k in WEAR_PICK_KEYS {
        assert_eq!(
            anchored_pattern_matches(pattern, k),
            Some(true),
            "canonical wear key `{k}` must satisfy the shipped pattern"
        );
    }
    // …and the shapes it is there to keep out.
    for bad in ["", "chest rig", "9lives", "_leading", "vest!", "a b"] {
        assert_eq!(
            anchored_pattern_matches(pattern, bad),
            Some(false),
            "`{bad}` must not satisfy the shipped pattern"
        );
    }
    // 64 chars is the ceiling (1 + {0,63}); 65 is not.
    let long = format!("a{}", "b".repeat(63));
    assert_eq!(anchored_pattern_matches(pattern, &long), Some(true));
    assert_eq!(
        anchored_pattern_matches(pattern, &format!("{long}c")),
        Some(false)
    );
}

/// A pattern this matcher cannot evaluate must answer `None` — "refuse", never "pass".
/// The whole schema gate rests on that: a constraint we cannot read is not a constraint we
/// may ignore.
#[test]
fn an_unsupported_pattern_refuses_rather_than_waving_through() {
    for unsupported in [
        "[a-z]+",       // unanchored
        "^(a|b)$",      // alternation
        "^a.c$",        // any-char
        r"^\d+$",       // escape class
        "^[a-z$",       // unterminated class
        "^[]$",         // empty class
        "^a{3,2}$",     // inverted range
        "^[a-z]{2,x}$", // unparseable quantifier
    ] {
        assert_eq!(
            anchored_pattern_matches(unsupported, "abc"),
            None,
            "`{unsupported}` must refuse, not guess"
        );
    }
    // The quantifiers it does implement, evaluated rather than refused.
    assert_eq!(anchored_pattern_matches("^[ab]*$", "abba"), Some(true));
    assert_eq!(anchored_pattern_matches("^[ab]+c$", "c"), Some(false));
    assert_eq!(anchored_pattern_matches("^[ab]?c$", "ac"), Some(true));
    assert_eq!(anchored_pattern_matches("^[a-c]{2}$", "ab"), Some(true));
    assert_eq!(anchored_pattern_matches("^[a-c]{2}$", "abc"), Some(false));
    assert_eq!(anchored_pattern_matches("^[^0-9]+$", "abc"), Some(true));
    assert_eq!(anchored_pattern_matches("^[^0-9]+$", "ab1"), Some(false));
    // Greedy-with-backtracking: the trailing literal must be reachable.
    assert_eq!(
        anchored_pattern_matches("^[a-z]{1,4}z$", "abcz"),
        Some(true)
    );
    // An absurd key is refused rather than evaluated (the MAX_PATTERN_INPUT bound).
    assert_eq!(
        anchored_pattern_matches("^[a-z]*$", &"a".repeat(MAX_PATTERN_INPUT + 1)),
        None
    );
}

/* ═════════ T-735 — the three fail-open forms, and the guard that could not see them ═════════ */

/// A document the SHIPPED schema accepts. Every T-735 trap below starts from this, so a
/// refusal can only be the schema edit's doing and never the document's.
fn a_valid_v1_document() -> serde_json::Value {
    serde_json::json!({
        "loadoutVersion": "1",
        "modpackId": "mp",
        "gear": {"primary": null, "uniform": null, "vest": null, "helmet": null},
    })
}

/// The shipped schema with one edit applied — "one `$defs` edit away" is the exact distance
/// the T-735 verifier measured between today's clean file and a suite reporting green over
/// documents the importer never examined. These tests walk that distance on purpose.
fn shipped_schema_with(edit: impl FnOnce(&mut serde_json::Value)) -> serde_json::Value {
    let mut schema: serde_json::Value = serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON)
        .expect("the shipped loadout-export schema must parse");
    edit(&mut schema);
    schema
}

/// Walk every SCHEMA POSITION in `schema` and report `(path, keyword)` for each keyword the
/// checker does not implement, plus how many positions were visited.
///
/// **Structural, never keyword-sniffing — that distinction is the T-735 defect.** The pin this
/// replaces decided "is this node a schema?" by asking whether it carried at least one
/// SUPPORTED keyword, which is circular: a node carrying ONLY unsupported keywords —
/// `{"maxItems": 1}`, precisely the shape the guard exists to catch — answered "not a schema"
/// and was skipped whole. This walk knows where subschemas LIVE instead: the values of
/// `properties`/`patternProperties`/`$defs`, the entries of `oneOf`, and `items` /
/// `additionalProperties` when they hold a subschema. What it finds there is a schema
/// regardless of what it contains, so an unrecognised node is reported rather than excused.
fn unsupported_keywords_in(schema: &serde_json::Value) -> (usize, Vec<String>) {
    fn walk(node: &serde_json::Value, at: &str, seen: &mut usize, found: &mut Vec<String>) {
        let Some(map) = node.as_object() else {
            found.push(format!("{at}: not a schema object"));
            return;
        };
        *seen += 1;
        for k in map.keys() {
            if !SUPPORTED_SCHEMA_KEYWORDS.contains(&k.as_str()) {
                found.push(format!("{at}: {k}"));
            }
        }
        for named in ["properties", "patternProperties", "$defs"] {
            let children = map.get(named).and_then(serde_json::Value::as_object);
            for (name, child) in children.into_iter().flatten() {
                walk(child, &format!("{at}/{named}/{name}"), seen, found);
            }
        }
        let branches = map.get("oneOf").and_then(serde_json::Value::as_array);
        for (i, b) in branches.into_iter().flatten().enumerate() {
            walk(b, &format!("{at}/oneOf/{i}"), seen, found);
        }
        for single in ["items", "additionalProperties"] {
            match map.get(single) {
                Some(child) if child.is_object() => {
                    walk(child, &format!("{at}/{single}"), seen, found)
                }
                _ => {}
            }
        }
    }
    let mut seen = 0usize;
    let mut found = Vec::new();
    walk(schema, "#", &mut seen, &mut found);
    (seen, found)
}

/// The keyword guard: if the schema grows an assertion this checker does not implement, the
/// importer must REFUSE, not silently accept a document it only partly examined.
#[test]
fn an_unimplemented_keyword_is_a_refusal_not_a_shrug() {
    // Sanity: the shipped schema uses nothing outside the supported set, so a real document
    // passes. (If this half fails, the other half is what tells you why.)
    assert!(validate_against_loadout_export_schema(&a_valid_v1_document()).is_ok());

    // Every keyword the shipped file actually uses is declared supported — a `$defs` entry
    // gaining `maxItems` tomorrow must go red here rather than pass unchecked.
    let schema: serde_json::Value = serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON).unwrap();
    let (seen, found) = unsupported_keywords_in(&schema);
    assert!(
        found.is_empty(),
        "the shipped schema uses keywords the importer does not implement: {found:?}"
    );
    assert!(
        seen > 10,
        "the walk must actually have reached the schema nodes ({seen})"
    );
    // And the production audit — the one that runs on a real import — agrees.
    assert!(validate_against_schema(&schema, &a_valid_v1_document()).is_ok());
}

/// **T-735 form 4 — the blind guard.** The pin above is only worth its lines if it can SEE the
/// node shape it exists to catch. The pre-T-735 `is_schema` heuristic could not: it inspected
/// only nodes already carrying a supported keyword, so a `$defs` entry of nothing but
/// unimplemented assertions was skipped by the very check written to catch it.
#[test]
fn the_guard_walk_sees_a_node_carrying_only_unsupported_keywords() {
    let trap = serde_json::json!({"maxItems": 1});
    assert!(
        !trap
            .as_object()
            .unwrap()
            .keys()
            .any(|k| SUPPORTED_SCHEMA_KEYWORDS.contains(&k.as_str())),
        "the trap must carry NO supported keyword — that is exactly the shape the old \
         keyword-sniffing heuristic declared 'not a schema' and skipped"
    );
    let schema = shipped_schema_with(|s| s["$defs"]["trap"] = trap);

    let (_, found) = unsupported_keywords_in(&schema);
    assert_eq!(
        found,
        vec!["#/$defs/trap: maxItems".to_string()],
        "the guard walk must name the node it used to skip"
    );
    // …and the importer refuses documents against that schema, even though NOTHING $refs the
    // trap: a rule set this build cannot fully read is not one it may report success against.
    let faults = validate_against_schema(&schema, &a_valid_v1_document())
        .expect_err("an unreachable-but-unreadable $defs node must refuse, not be waved through");
    assert!(
        faults.iter().any(|f| f.contains("`maxItems`")),
        "the refusal must name the keyword: {faults:?}"
    );
}

/// **T-735 form 1 — `oneOf` discarded the refusals of non-passing branches.** `passing == 1`
/// returned early and threw a losing branch's findings away wholesale. A branch's FAULTS are
/// rightly discarded (the document did not claim that branch); its REFUSALS are not — a
/// refusal is this build admitting it cannot read the rules, and that is true whichever branch
/// the document took. Before T-735 an unimplemented keyword behind a `$ref` in a losing branch
/// never surfaced and the document was ACCEPTED.
#[test]
fn a_losing_one_of_branch_cannot_hide_an_unimplemented_keyword() {
    let doc = a_valid_v1_document();
    // Control: unedited, this document is valid, so every refusal below is the edit's doing.
    let shipped: serde_json::Value = serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON).unwrap();
    assert!(validate_against_schema(&shipped, &doc).is_ok());

    // (a) The trap sits on a key the document HAS, so the losing v2 branch's own walk reaches
    //     it and records the refusal — which `check_schema_one_of` then had to carry out.
    let reachable = shipped_schema_with(|s| {
        s["$defs"]["trap"] = serde_json::json!({"maxItems": 1});
        s["oneOf"][1]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/trap"});
    });
    let faults = validate_against_schema(&reachable, &doc)
        .expect_err("a refusal from the losing branch must survive branch selection");
    assert!(
        faults.iter().any(|f| f.contains("`maxItems`")),
        "the refusal must name the keyword it could not implement: {faults:?}"
    );

    // (b) The same trap on a key the document does NOT have. No document walk can ever reach
    //     this position, so only a document-INDEPENDENT audit of the schema can refuse it —
    //     which is why the fix is an audit and not just a fix to the branch bookkeeping.
    let unreachable = shipped_schema_with(|s| {
        s["$defs"]["trap"] = serde_json::json!({"maxItems": 1});
        s["oneOf"][1]["properties"]["cargo"] = serde_json::json!({"$ref": "#/$defs/trap"});
    });
    assert!(
        validate_against_schema(&unreachable, &doc).is_err(),
        "a keyword this build cannot implement must refuse even where no document reaches it"
    );

    // (c) The branch bookkeeping itself, pinned with the audit BYPASSED. Without this, (a) and
    //     (b) would both stay green if `check_schema_one_of` went back to discarding refusals,
    //     because the audit alone would carry them. A second line of defence is worth pinning
    //     precisely where the first line is the thing that failed.
    let mut out = SchemaFaults::default();
    check_schema_node(&reachable, &reachable, &doc, "", &mut out);
    assert!(
        out.faults.is_empty(),
        "this document is valid v1; only the refusal should stand: {:?}",
        out.faults
    );
    assert!(
        out.refusals.iter().any(|r| r.contains("`maxItems`")),
        "branch selection dropped the losing branch's refusal again: {:?}",
        out.refusals
    );
}

/// **T-735 form 2 — `additionalProperties` was read only as `== false`.** It is a SUBSCHEMA
/// keyword; `false` is merely the subschema that nothing satisfies. Reading it as a boolean
/// flag made `{"additionalProperties": {"type": "string"}}` assert nothing at all, so
/// `{"x": 123}` came back `Ok`.
#[test]
fn schema_form_additional_properties_is_checked_not_a_silent_no_op() {
    let schema = shipped_schema_with(|s| {
        s["oneOf"][0]["additionalProperties"] = serde_json::json!({"type": "string"});
    });
    let mut doc = a_valid_v1_document();

    doc["extra"] = serde_json::json!(123);
    let faults = validate_against_schema(&schema, &doc)
        .expect_err("a schema-form additionalProperties must be APPLIED, not skipped");
    assert!(
        faults
            .iter()
            .any(|f| f.contains("/extra") && f.contains("expected string")),
        "the fault must name the key and the rule it broke: {faults:?}"
    );

    // Satisfied, the same subschema is silence — a refusal here would be as wrong as a shrug.
    doc["extra"] = serde_json::json!("fine");
    assert!(
        validate_against_schema(&schema, &doc).is_ok(),
        "a value the subschema accepts must pass"
    );

    // The boolean forms keep their meanings: `true` admits anything, `false` still closes.
    let open = shipped_schema_with(|s| {
        s["oneOf"][0]["additionalProperties"] = serde_json::json!(true);
    });
    doc["extra"] = serde_json::json!(123);
    assert!(validate_against_schema(&open, &doc).is_ok());
    let closed: serde_json::Value = serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON).unwrap();
    let shut = validate_against_schema(&closed, &doc)
        .expect_err("additionalProperties: false still closes the object");
    assert!(
        shut.iter()
            .any(|f| f.contains("additionalProperties is false")),
        "{shut:?}"
    );
}

/// **T-735 form 3 — tuple-form `items` was dropped on an `as_object()` miss.** The walk handed
/// the array to `check_schema_node`, which recorded NOTHING and returned, so `[123]` validated
/// against `[{"type": "string"}]`. This checker implements only the single-subschema form, so
/// the tuple form must REFUSE — the fail-closed contract, not a silently skipped check.
#[test]
fn tuple_form_items_is_refused_not_dropped() {
    let schema = shipped_schema_with(|s| {
        s["oneOf"][0]["properties"]["tags"] =
            serde_json::json!({"type": "array", "items": [{"type": "string"}]});
    });
    let mut doc = a_valid_v1_document();

    doc["tags"] = serde_json::json!([123]);
    let faults = validate_against_schema(&schema, &doc)
        .expect_err("tuple-form `items` must refuse rather than validate nothing");
    assert!(
        faults.iter().any(|f| f.contains("`items`")),
        "the refusal must name the keyword it dropped: {faults:?}"
    );

    // Document-independent: an EMPTY array reaches no element at all, so a fix that only fires
    // per-element would leave the hole open for exactly the document that exercises it least.
    doc["tags"] = serde_json::json!([]);
    assert!(
        validate_against_schema(&schema, &doc).is_err(),
        "the refusal must not depend on the document happening to have an element"
    );

    // The form this checker DOES implement still works, and still catches the bad element.
    let single = shipped_schema_with(|s| {
        s["oneOf"][0]["properties"]["tags"] =
            serde_json::json!({"type": "array", "items": {"type": "string"}});
    });
    doc["tags"] = serde_json::json!([123]);
    let elem = validate_against_schema(&single, &doc)
        .expect_err("the implemented `items` form must still assert");
    assert!(
        elem.iter().any(|f| f.contains("expected string")),
        "{elem:?}"
    );
    doc["tags"] = serde_json::json!(["ok"]);
    assert!(validate_against_schema(&single, &doc).is_ok());
}

/* ═══════ wave 128 — the fourth fail-open form: `$ref` CHAINS, and the audit that blessed them ═══════ */

/// **`$ref -> $ref -> assertions` validated NOTHING, and the T-735 audit passed it clean.**
///
/// [`schema_deref`] resolves exactly one hop and no caller re-derefs, so the document walk
/// replaced the node with a target that was itself nothing but a pointer and then checked no
/// assertion at all — `{"modpackId": 123}` came back `Ok` against `{"type": "string"}` at the
/// far end. The audit made that WORSE rather than catching it: the audit did follow the chain,
/// reached the assertions, found every keyword supported, and reported the schema fully
/// readable — the guard vouching for rules the walk never read, which is the exact shape T-735
/// exists to close.
///
/// Refusal, not chain-following, is the fix: "this importer follows one hop" is honest and
/// fails CLOSED, where a chain-follower would put cycle detection inside the one code path
/// whose entire job is not to lie. Unreachable through the shipped file today — every `$ref` in
/// it is single-hop — and one ordinary `$defs` refactor away, with every other pin still green.
#[test]
fn a_ref_chain_is_refused_rather_than_checking_nothing() {
    let chained = shipped_schema_with(|s| {
        s["$defs"]["tail"] = serde_json::json!({"type": "string", "minLength": 2});
        s["$defs"]["hop"] = serde_json::json!({"$ref": "#/$defs/tail"});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/hop"});
    });

    // The document the chain used to ACCEPT: an integer where the far end says string.
    let mut bad = a_valid_v1_document();
    bad["modpackId"] = serde_json::json!(123);
    let faults = validate_against_schema(&chained, &bad)
        .expect_err("a $ref chain must refuse, not check nothing and answer Ok");
    assert!(
        faults
            .iter()
            .any(|f| f.contains("#/$defs/hop") && f.contains("itself a $ref")),
        "the refusal must name the pointer whose assertions it dropped: {faults:?}"
    );

    // Document-INDEPENDENT, like every other refusal T-735 added: a chain this build cannot
    // follow is refused for the documents that would have passed too. A refusal that waited for
    // a document to break the far-end rule would leave the hole open for every document that
    // does not — which is how the first three forms survived a green suite.
    assert!(
        validate_against_schema(&chained, &a_valid_v1_document()).is_err(),
        "the chain refusal must not wait for a document that happens to break the far end"
    );

    // CONTROL — the single hop this importer DOES implement still asserts, both keywords, and
    // still passes a good document. Without this the assertions above stay green if `$ref`
    // support is simply amputated, which is fail-closed by uselessness rather than by contract.
    let single = shipped_schema_with(|s| {
        s["$defs"]["tail"] = serde_json::json!({"type": "string", "minLength": 2});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/tail"});
    });
    let typed = validate_against_schema(&single, &bad)
        .expect_err("the implemented single-hop $ref must still assert `type`");
    assert!(
        typed.iter().any(|f| f.contains("expected string")),
        "a single hop must yield the document FAULT, not a refusal: {typed:?}"
    );
    let mut short = a_valid_v1_document();
    short["modpackId"] = serde_json::json!("m");
    assert!(
        validate_against_schema(&single, &short).is_err(),
        "the far-end `minLength` must be live too, or the control proves only half a hop"
    );
    assert!(validate_against_schema(&single, &a_valid_v1_document()).is_ok());
}

/// **A `$ref` CYCLE is the same hole with the far end removed** — one hop of deref lands back on
/// a bare `$ref`, so the walk checked nothing, and the audit walked the loop on its `visited`
/// bookkeeping alone and called it supported. The one-hop rule closes both shapes at once:
/// a cycle is just a chain that comes back round.
#[test]
fn a_ref_cycle_is_refused_by_the_same_one_hop_rule() {
    let mut bad = a_valid_v1_document();
    bad["modpackId"] = serde_json::json!(123);

    // The tightest knot there is: a `$def` that is nothing but a pointer at itself.
    let direct = shipped_schema_with(|s| {
        s["$defs"]["knot"] = serde_json::json!({"$ref": "#/$defs/knot"});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/knot"});
    });
    let faults = validate_against_schema(&direct, &bad)
        .expect_err("a $ref cycle must refuse, not accept a document it never examined");
    assert!(
        faults
            .iter()
            .any(|f| f.contains("#/$defs/knot") && f.contains("itself a $ref")),
        "the refusal must name the cycle's pointer: {faults:?}"
    );
    assert!(
        validate_against_schema(&direct, &a_valid_v1_document()).is_err(),
        "a cycle is unreadable for every document, including the ones that would have passed"
    );

    // Two-step: a -> b -> a. Nothing at either end is ever an assertion.
    let pair = shipped_schema_with(|s| {
        s["$defs"]["ping"] = serde_json::json!({"$ref": "#/$defs/pong"});
        s["$defs"]["pong"] = serde_json::json!({"$ref": "#/$defs/ping"});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/ping"});
    });
    let two = validate_against_schema(&pair, &bad)
        .expect_err("a two-step $ref cycle must refuse as loudly as a one-step one");
    assert!(
        two.iter()
            .any(|f| f.contains("#/$defs/pong") && f.contains("itself a $ref")),
        "the refusal must name the hop it would not follow: {two:?}"
    );

    // PRECISION — the rule bans `$ref`-at-a-`$ref`, NOT recursion. A `$def` that reaches itself
    // through a subschema still resolves in one hop at every position, is terminated by
    // `visited`, and must keep validating. A fix that banned self-reference outright would pass
    // every assertion above and refuse a schema this importer can read perfectly well.
    let recursive = shipped_schema_with(|s| {
        s["$defs"]["rec"] = serde_json::json!({"type": "array", "items": {"$ref": "#/$defs/rec"}});
    });
    assert!(
        validate_against_schema(&recursive, &a_valid_v1_document()).is_ok(),
        "recursion through a subschema is one hop at every step — it must still be supported"
    );
}

/// An RFC 6901 ESCAPED pointer segment must refuse, not resolve to the wrong node.
///
/// `~1` spells `/` and `~0` spells `~`, so `#/$defs/a~1b` names the key `a/b`. `schema_deref`
/// splits on `/` and looks segments up verbatim, so it would ask for a key literally called
/// `a~1b`. The wave-128 re-verifier proved that is not just a miss: with BOTH keys present the
/// pointer silently resolved to the wrong schema and a document the real schema rejects came
/// back `Ok(())`. This is the same failure the `$ref`-chain rule closes — reading something
/// other than what the author wrote, quietly — so it refuses by the same one-hop logic.
#[test]
fn an_escaped_pointer_segment_is_refused_rather_than_resolved_to_the_wrong_node() {
    // The decoy is the whole point: `a/b` is the pointer's true target and carries an assertion
    // the document fails; the literal `a~1b` is what the un-unescaped lookup actually finds.
    let escaped = shipped_schema_with(|s| {
        s["$defs"]["a/b"] = serde_json::json!({"type": "string", "minLength": 9999});
        s["$defs"]["a~1b"] = serde_json::json!({});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/a~1b"});
    });

    let faults = validate_against_schema(&escaped, &a_valid_v1_document()).expect_err(
        "an escaped pointer segment must refuse — resolving it verbatim reads a schema the \
         author did not write",
    );
    assert!(
        faults.iter().any(|f| f.contains("modpackId")),
        "the refusal must name the position whose $ref it would not follow: {faults:?}"
    );

    // Document-independent: the refusal is about the schema, so the document that would have
    // been wrongly ACCEPTED must be refused too, not quietly passed.
    assert!(
        validate_against_schema(&escaped, &{
            let mut d = a_valid_v1_document();
            d["modpackId"] = serde_json::json!("x");
            d
        })
        .is_err(),
        "the escaped pointer is unreadable for every document, including the one the wrong \
         node would have accepted"
    );

    // PRECISION — an ordinary unescaped pointer next to it still resolves. The rule bans `~`
    // in a segment, not `$defs` keys or pointers in general.
    let plain = shipped_schema_with(|s| {
        s["$defs"]["plain"] = serde_json::json!({"type": "string"});
        s["oneOf"][0]["properties"]["modpackId"] = serde_json::json!({"$ref": "#/$defs/plain"});
    });
    assert!(
        validate_against_schema(&plain, &a_valid_v1_document()).is_ok(),
        "a normal single-hop pointer must be unaffected by the escaped-segment rule"
    );
}
