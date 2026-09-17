use super::{
    aggregate_settings, fmt_setting_default, fmt_setting_value, mission_setting_pointer,
    values_agree, DiffState, SettingDefault, SettingOwner, SettingRow, ALL_SETTINGS_NOTE,
    MISSION_SCHEMA_JSON, MISSION_SETTING_POINTERS, NOT_A_SCHEMA_KEY, NO_DEFAULT_DECLARED,
    OWNER_UNRESOLVED_NOTE,
};
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_body};
use serde_json::json;

fn schema() -> serde_json::Value {
    serde_json::from_str(MISSION_SCHEMA_JSON).expect(
        "T-688: the embedded mission.schema.json must parse — it is the only source of \
                 every default this view reports",
    )
}

/// T-757 — zones + settings share ONE `include_str!` of mission.schema.json.
///
/// Perturbation this catches: restoring a second `include_str!` in this file, or dropping the
/// `crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA` alias so the view re-embeds. Needle path is assembled so
/// this test cannot become its own haystack; `live_code` drops cfg(test) so the pin cannot match
/// itself.
#[test]
fn zones_and_settings_share_one_mission_schema_embed() {
    // live_source keeps string literals (needed to see the include_str path); live_code would blank them.
    let zones = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/zones_panel.rs"
    )));
    let settings = live_source(include_str!("../settings_modal.rs"));
    let path = format!(
        "{}{}{}",
        "/../../../packages/tbd-schema/schema/", "mission", ".schema.json"
    );
    let embed = format!("\"{path}\"");
    assert_eq!(
        zones.matches(embed.as_str()).count(),
        1,
        "T-757: eden_zones must be the single include_str site among zones+settings"
    );
    assert!(
        zones.contains("pub(crate) const MISSION_SCHEMA"),
        "T-757: MISSION_SCHEMA must be pub(crate) so settings can share it"
    );
    assert_eq!(
        settings.matches(embed.as_str()).count(),
        0,
        "T-757: eden_settings must not re-embed mission.schema.json"
    );
    let shared = format!(
        "{}{}{}",
        "crate::v2::apps::editor::ui::inspector::zones_panel::", "MISSION", "_SCHEMA"
    );
    assert!(
        settings.contains(&shared),
        "T-757: eden_settings must read crate::v2::apps::editor::ui::inspector::zones_panel::MISSION_SCHEMA"
    );
    // Stale size lore (~40 KB vs ~91 KB) — drop rather than restate a drifting number.
    // live_source blanks comments; the ticket defect was comment lore, so read the zones file
    // raw (wave-135 F2). Restoring `~40 KB` in a doc-comment must RED.
    let zones_raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/zones_panel.rs"
    ));
    let stale = format!("{}{}", "~40 ", "KB");
    assert!(
        !zones_raw.contains(&stale),
        "T-757: eden_zones must not restate a drifted ~40 KB embed cost (comments count)"
    );
}

fn zone_rule_props() -> serde_json::Map<String, serde_json::Value> {
    schema()["$defs"]["zoneRules"]["properties"]
        .as_object()
        .expect("T-688: $defs/zoneRules/properties")
        .clone()
}

/// **THE pin.** The view's default for every `$defs/zoneRules` key must be exactly what the
/// schema declares there — read here a second time, independently, straight out of the committed
/// bytes.
///
/// This is the test the ticket asks for: it fails when the schema and the view disagree, which is
/// the only observable symptom a second source of truth ever has. A hand-written table of
/// defaults in Rust passes on the day it is written and goes red the first time the schema moves;
/// a typo goes red immediately. Perturbation this catches: replacing the schema read with a
/// literal for any single key.
#[test]
fn the_view_and_the_schema_agree_key_for_key() {
    let schema = schema();
    let props = zone_rule_props();
    assert!(
        props.len() >= 16,
        "T-688: $defs/zoneRules is the closed 16+ key vocabulary (T-241/T-685); found {}",
        props.len()
    );

    // A document authoring EVERY rule key on one zone. The authored values are deliberately
    // nonsense — what is under test is where the DEFAULT came from, not the value beside it.
    let mut rules = serde_json::Map::new();
    for key in props.keys() {
        rules.insert(key.clone(), json!("__authored_probe__"));
    }
    let doc = json!({
        "zonesById": {
            "zone-a": { "type": "objective_capture", "label": "Hilltop", "rules": rules }
        }
    });

    let rows = aggregate_settings(&doc);
    assert_eq!(
        rows.len(),
        props.len(),
        "T-688: every authored rule key must produce exactly one row — an aggregation that can \
         drop a setting is the defect this view exists to remove"
    );

    let mut with_default = 0usize;
    let mut without_default = 0usize;
    for (key, declared) in &props {
        let row = rows
            .iter()
            .find(|r| r.key == *key)
            .unwrap_or_else(|| panic!("T-688: no row for authored rule key `{key}`"));
        // One-hop `$ref`, exactly as the schema declares it (`targetAlias` → `$defs/alias`).
        let resolved = declared
            .get("$ref")
            .and_then(serde_json::Value::as_str)
            .and_then(|r| schema.pointer(r.trim_start_matches('#')))
            .unwrap_or(declared);
        let want = declared.get("default").or_else(|| resolved.get("default"));
        match (&row.default, want) {
            (SettingDefault::Schema { value, pointer }, Some(expected)) => {
                assert_eq!(
                    value, expected,
                    "T-688: the view and the schema DISAGREE about `{key}`'s default — the view \
                     says {value}, mission.schema.json says {expected}"
                );
                assert!(
                    pointer.ends_with(key.as_str()),
                    "T-688: `{key}`'s default must name the schema location it was read from, \
                     got {pointer:?}"
                );
                with_default += 1;
            }
            (SettingDefault::Declared { .. }, None) => without_default += 1,
            (got, want) => panic!(
                "T-688: `{key}` — the view reports {got:?} but mission.schema.json declares \
                 default={want:?}"
            ),
        }
    }
    assert!(
        with_default >= 11,
        "T-688: $defs/zoneRules declares defaults on at least 11 keys; the view found \
         {with_default}"
    );
    assert!(
        without_default > 0,
        "T-688: some rule keys declare no default (holdSeconds, points, …) and the view must \
         report them as such rather than inventing one"
    );
}

/// The honest half of the same rule for the MISSION-level keys: `mission.schema.json` declares a
/// `default` for NONE of them, so the view must say so.
///
/// **This is where a second source of truth would have been most tempting.** `eden_env` exposes
/// `FLOW_DEFAULT_TIMELIMIT_S = 5400` and friends — but since T-753 those are `mission::flatten`'s
/// own constants re-exported (`pub use`, eden_env.rs), not a mirrored copy: a COMPILER FALLBACK,
/// not a schema declaration. The distinction this test defends is unchanged; only the mechanism
/// moved. There is no longer a second copy that could drift.
/// Printing 5400 in a "schema default" column would be the view reporting a diff against a number
/// the schema never stated. Perturbation this catches: exactly that substitution.
#[test]
fn no_flow_constant_is_passed_off_as_a_schema_default() {
    let doc = json!({
        "meta": {
            "terrain": "everon",
            "environment": {
                "time": "06:00", "weather": "overcast",
                "briefingSeconds": 600, "safeStartSeconds": 300,
                "timeLimitSeconds": 900, "jip": "always"
            }
        }
    });
    let rows = aggregate_settings(&doc);
    for key in [
        "terrain",
        "time",
        "weather",
        "briefingSeconds",
        "safeStartSeconds",
        "timeLimitSeconds",
        "jip",
    ] {
        let row = rows
            .iter()
            .find(|r| r.key == key)
            .unwrap_or_else(|| panic!("T-688: no row for authored mission key `{key}`"));
        assert!(
            matches!(row.default, SettingDefault::Declared { .. }),
            "T-688: mission.schema.json declares NO default for `{key}` — the view must report \
             that, not substitute one. Got {:?}",
            row.default
        );
        assert_eq!(fmt_setting_default(&row.default), NO_DEFAULT_DECLARED);
        assert_eq!(row.diff_state(), DiffState::Unknown);
        assert_eq!(row.owner, SettingOwner::Mission);
    }

    // …and the constants themselves are named nowhere in the aggregation or its rendering.
    // T-755: also scan `from_schema_node` itself — the prior list stopped at five callers and
    // left the ONE value-carrying constructor free to substitute a FLOW_DEFAULT_*.
    let src = live_code(include_str!("../settings_modal.rs"));
    let banned = format!("FLOW{}", "_DEFAULT_");
    for f in [
        format!("fn aggregate{}", "_settings"),
        format!("fn schema{}", "_default"),
        format!("fn from{}", "_schema_node"),
        format!("fn render{}", "_all_settings_body"),
        format!("fn setting{}", "_row_view"),
        format!("fn fmt{}", "_setting_default"),
    ] {
        assert!(
            !only_body(&src, &f).contains(&banned),
            "T-688/T-755: `{banned}*` is a compiler fallback, not a schema default — it must not \
             reach `{f}`"
        );
    }
}

/// **A default value is built in exactly ONE place**, and that place reads it out of a schema
/// node. Any second construction site is a second source of truth by definition, so the count is
/// pinned rather than trusted.
///
/// Perturbation this catches: a `SettingDefault::Schema { … }` assembled from a hand-typed table
/// anywhere else in the file — including the path-spelled form that the old `Self::Schema {`
/// needle missed (wave-115 MINOR-2 / T-755).
#[test]
fn a_default_value_is_built_in_exactly_one_place() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let self_ctor = format!("Self::{} {{", "Schema");
    assert_eq!(
        src.matches(&self_ctor).count(),
        1,
        "T-688/T-755: the value-carrying default variant must be constructed exactly once (in \
         from_schema_node, out of a schema node). A second site is a second source of truth."
    );
    // Any `::Schema { value: … }` initialiser — path-spelled (`SettingDefault::`), alias-spelled
    // (`SD::`), or otherwise — is a second source of truth. Match-arm DESTRUCTURES bind fields
    // (`value,` / `pointer,`); constructions INITIALISE them (`value:`). The sole allowed site
    // is the `Self::Schema` above (wave-115 MINOR-2 / T-755; wave-134 F2 closes the alias gap).
    let schema_value_inits = src
        .match_indices("::Schema {")
        .filter(|&(i, _)| {
            let window = &src[i..src.len().min(i + 160)];
            window.contains("value:")
        })
        .count();
    assert_eq!(
        schema_value_inits,
        1,
        "T-688/T-755/wave-134: exactly one `::Schema {{ value: … }}` constructor (Self:: in              from_schema_node) — a path- or alias-spelled second site is a second source of truth"
    );
    // …and that one site reads the schema's own `default` key rather than deciding anything.
    let lit = live_source(include_str!("../settings_modal.rs"));
    let body = only_body(&lit, &format!("fn from{}", "_schema_node"));
    assert!(
        body.contains("\"default\""),
        "T-688: the one constructor must read the schema's `default` key"
    );
    // The reader that feeds it addresses the schema by JSON pointer — no key list of its own.
    let reader = only_body(&lit, &format!("fn schema{}", "_default"));
    assert!(
        reader.contains(".pointer("),
        "T-688: schema_default must locate a declaration by pointer in the embedded schema"
    );
}

/// Every schema location this view declares must actually resolve. A pointer that stopped
/// resolving would silently downgrade a real wire key to "editor-local", which reads as "nothing
/// to compare" — a lie by omission rather than by value.
#[test]
fn every_declared_pointer_resolves() {
    let schema = schema();
    for (key, pointer) in MISSION_SETTING_POINTERS {
        assert!(
            schema.pointer(pointer.trim_start_matches('#')).is_some(),
            "T-688: `{key}`'s declared location {pointer} no longer resolves in \
             mission.schema.json"
        );
        assert_eq!(mission_setting_pointer(key), Some(*pointer));
    }
    // The zone-rule pointers are formatted, so one representative proves the shape.
    assert!(schema
        .pointer("/$defs/zoneRules/properties/graceSeconds")
        .is_some());
}

/// **The walk is over the DOCUMENT.** A key this file has never heard of still gets a row — as
/// `NotInSchema`, never skipped. That is what stops a future `author_env` key, or a rule the
/// schema later drops, from vanishing out of "every setting in this mission".
///
/// Perturbation this catches: iterating a key table instead of the document.
#[test]
fn the_aggregation_walks_the_document_and_omits_nothing() {
    let doc = json!({
        "meta": { "environment": {
            "showGrid": true,
            "hillshadeOpacity": 0.4,
            "aKeyNobodyHasWrittenYet": 7
        } },
        "zonesById": { "z1": { "type": "boundary", "rules": { "notARuleAnyMore": 3 } } }
    });
    let rows = aggregate_settings(&doc);
    let keys: Vec<&str> = rows.iter().map(|r| r.key.as_str()).collect();
    for key in [
        "showGrid",
        "hillshadeOpacity",
        "aKeyNobodyHasWrittenYet",
        "notARuleAnyMore",
    ] {
        assert!(
            keys.contains(&key),
            "T-688: `{key}` is authored in the document and must appear; got {keys:?}"
        );
        let row = rows.iter().find(|r| r.key == key).expect("row");
        assert_eq!(
            row.default,
            SettingDefault::NotInSchema,
            "T-688: `{key}` is not declared in mission.schema.json — say so, do not guess"
        );
        assert_eq!(fmt_setting_default(&row.default), NOT_A_SCHEMA_KEY);
    }
}

/// The diff-from-default filter keeps everything it cannot PROVE is unchanged. Hiding a row whose
/// schema declares no default would be the view asserting a fact nobody has — the same defect as
/// an invented default, told by omission.
#[test]
fn the_diff_filter_keeps_what_it_cannot_prove() {
    let doc = json!({
        "meta": { "environment": { "timeLimitSeconds": 900 } },
        "zonesById": { "z1": { "type": "objective_capture", "rules": {
            // `penalty`'s schema default is "warn"; `contestable`'s is true.
            "penalty": "warn",
            "contestable": false
        } } }
    });
    let rows = aggregate_settings(&doc);
    let by = |k: &str| rows.iter().find(|r| r.key == k).expect("row").clone();

    assert_eq!(by("penalty").diff_state(), DiffState::Matches);
    assert_eq!(by("contestable").diff_state(), DiffState::Differs);
    assert_eq!(by("timeLimitSeconds").diff_state(), DiffState::Unknown);

    let kept: Vec<String> = rows
        .iter()
        .filter(|r| r.survives_diff_filter())
        .map(|r| r.key.clone())
        .collect();
    assert!(kept.contains(&"contestable".to_string()));
    assert!(
        kept.contains(&"timeLimitSeconds".to_string()),
        "T-688: a key with no declared default cannot be shown to be at its default, so the \
         filter must keep it"
    );
    assert!(
        !kept.contains(&"penalty".to_string()),
        "T-688: a row provably at its schema default is what the filter hides"
    );
}

/// A zone rule's owner is the ZONE, named and addressable. The `subject_id` is the document id a
/// click routes on; the label is what the row shows (name, else type, else id — never faceless).
#[test]
fn zone_rules_are_owned_by_their_zone() {
    let doc = json!({ "zonesById": {
        "z-named": { "type": "objective_capture", "label": "Hilltop",
                     "rules": { "captureSeconds": 90 } },
        "z-plain": { "type": "boundary", "rules": { "graceSeconds": 45 } }
    } });
    let rows = aggregate_settings(&doc);
    let named = rows
        .iter()
        .find(|r| r.key == "captureSeconds")
        .expect("row");
    assert_eq!(named.owner.subject_id(), Some("z-named"));
    assert!(named.owner.label().contains("Hilltop"));
    assert!(named.owner.label().contains("Zone"));

    let plain = rows.iter().find(|r| r.key == "graceSeconds").expect("row");
    assert_eq!(plain.owner.subject_id(), Some("z-plain"));
    assert!(
        plain.owner.label().contains("boundary"),
        "T-688: an unlabelled zone falls back to its type, got {:?}",
        plain.owner.label()
    );
    // A mission-level row names no entity, so it is not a click-through target.
    let doc = json!({ "meta": { "terrain": "everon" } });
    assert_eq!(aggregate_settings(&doc)[0].owner.subject_id(), None);
}

/// `120` (schema, integer) and `120.0` (authored through a number control) are the same setting.
/// Derived `Value` equality would call them different and paint an untouched row "changed" —
/// which is exactly the false diff this whole view must not produce.
#[test]
fn numeric_defaults_compare_across_int_and_float() {
    assert!(values_agree(&json!(120), &json!(120.0)));
    assert!(values_agree(&json!(0), &json!(-0.0)));
    assert!(!values_agree(&json!(120), &json!(121)));
    assert!(values_agree(&json!("warn"), &json!("warn")));
    assert!(!values_agree(&json!("warn"), &json!("kill")));
    assert!(!values_agree(&json!(true), &json!(1)));

    let doc = json!({ "zonesById": { "z1": { "type": "objective_capture",
        "rules": { "captureSeconds": 120.0 } } } });
    assert_eq!(
        aggregate_settings(&doc)[0].diff_state(),
        DiffState::Matches,
        "T-688: an authored 120.0 against a schema default of 120 is not a change"
    );
}

/// **Constraint (1) — READ-ONLY.** The aggregated view must not become a second editing surface:
/// duplicating every attribute control in the programme is the failure mode the ticket names.
///
/// Perturbation this catches: wiring any cell to `author_env`, an `editor_ops` mutator, or
/// dropping an `<input>`/`<select>`/`<textarea>` into a row "just for the numbers".
#[test]
fn the_aggregated_view_is_not_a_second_editing_surface() {
    let src = live_source(include_str!("../settings_modal.rs"));
    let editing_needles = [
        format!("author{}", "_env"),
        format!("update{}", "_environment"),
        format!("set{}", "_zone_rule"),
        format!("attrs{}", "_update"),
        "<input".to_string(),
        "<select".to_string(),
        "<textarea".to_string(),
        "contenteditable".to_string(),
    ];
    for f in [
        format!("fn render{}", "_all_settings_body"),
        format!("fn setting{}", "_row_view"),
        format!("fn render{}", "_all_settings_pointer"),
    ] {
        let body = only_body(&src, &f);
        for needle in &editing_needles {
            assert!(
                !body.contains(needle.as_str()),
                "T-688: `{needle}` must not appear in `{f}` — the aggregated view displays, it \
                 does not edit"
            );
        }
    }
}

/// **Constraint (2) — rows click through to the owning entity, through the SHIPPED router.**
///
/// wog.md 14.6: a findings list that only prints is worse than one that selects. T-655 already
/// ships that path (`validation_panel::route_select_by_subject_id`, registered from
/// `mission_editor.rs`), so this view reuses it rather than standing up a second selection
/// mechanism. Perturbation this catches: a bespoke selection path, or a row that names an owner
/// and does nothing with it.
#[test]
fn rows_click_through_the_shipped_t655_router() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let body = only_body(&src, &format!("fn setting{}", "_row_view"));
    assert!(
        body.contains(&format!("route{}", "_select_by_subject_id")),
        "T-688: a row click must route through T-655's registered click-to-select router"
    );
    assert!(
        body.contains(&format!("validation{}", "_panel")),
        "T-688: the router is the validation panel's — reuse it, do not fork it"
    );
    assert!(
        !body.contains(&format!("register{}", "_select_by_id")),
        "T-688: this view must not REGISTER a router — that would replace T-655's"
    );
    // The row asks the owner for its id rather than re-deriving one, so a mission-level row
    // (which has no entity) cannot be given a click that clears the selection.
    assert!(
        body.contains(&format!("subject{}", "_id")),
        "T-688: the click id must come from the row's owner"
    );
}

/// Copy pins. The header must say the defaults are the SCHEMA's (an author reading a diff has to
/// know what it was measured against), and the unresolved-owner note must name where a zone IS
/// selected — a dead click that explains itself is recoverable, a silent one is not.
#[test]
fn the_copy_says_what_the_numbers_mean() {
    let note = ALL_SETTINGS_NOTE.to_lowercase();
    assert!(
        note.contains("mission.schema.json"),
        "T-688: the header must name the schema as the source of the default column"
    );
    assert!(
        note.contains("read-only"),
        "T-688: the header must say the view does not edit"
    );
    let unresolved = OWNER_UNRESOLVED_NOTE.to_lowercase();
    assert!(
        unresolved.contains("zones panel"),
        "T-688: when the router declines, the note must say where the zone IS selected"
    );
}

/// Presentation only: a string loses its quotes, an empty string is visible as empty rather than
/// as a blank cell, everything else is shown as the document writes it.
#[test]
fn values_render_without_reshaping_them() {
    assert_eq!(fmt_setting_value(&json!("overcast")), "overcast");
    assert_eq!(fmt_setting_value(&json!("")), "(empty)");
    assert_eq!(fmt_setting_value(&json!(5400)), "5400");
    assert_eq!(fmt_setting_value(&json!(true)), "true");
    assert_eq!(fmt_setting_value(&json!(0.4)), "0.4");
}

/// Row order is deterministic — the pointer table's order, then unlisted mission keys sorted,
/// then zones by id and rules by key. An author who scrolls to a row must find it in the same
/// place next time, and a test can only pin what does not shuffle.
#[test]
fn row_order_is_stable() {
    let doc = json!({
        "meta": { "terrain": "everon", "environment": {
            "jip": "always", "time": "06:00", "showGrid": true, "showHillshade": false
        } },
        "zonesById": {
            "z-b": { "type": "boundary", "rules": { "penalty": "kill", "graceSeconds": 10 } },
            "z-a": { "type": "spawn", "rules": { "warnEverySeconds": 2 } }
        }
    });
    let rows: Vec<(String, Option<String>)> = aggregate_settings(&doc)
        .into_iter()
        .map(|r| (r.key, r.owner.subject_id().map(ToString::to_string)))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("terrain".into(), None),
            ("time".into(), None),
            ("jip".into(), None),
            ("showGrid".into(), None),
            ("showHillshade".into(), None),
            ("warnEverySeconds".into(), Some("z-a".into())),
            ("graceSeconds".into(), Some("z-b".into())),
            ("penalty".into(), Some("z-b".into())),
        ]
    );
}

/// The dialog is reachable: Mission Settings grows the pointer row that opens it, and the dialog
/// is mounted as a sibling so it outlives that dialog being closed (the T-691 idiom).
#[test]
fn the_view_is_reachable_from_mission_settings() {
    let src = live_code(include_str!("../settings_modal.rs"));
    let dialog = only_body(&src, "fn MissionSettingsDialog");
    assert!(
        dialog.contains(&format!("render{}", "_all_settings_pointer")),
        "T-688: Mission Settings must carry the pointer row that opens the aggregated view"
    );
    assert!(
        dialog.contains(&format!("All{}", "SettingsDialog")),
        "T-688: the aggregated view must be mounted as a sibling of Mission Settings"
    );
    let pointer = only_body(&src, &format!("fn render{}", "_all_settings_pointer"));
    assert!(
        pointer.contains(&format!("open{}", "_all_settings")),
        "T-688: the pointer row must open the aggregated view"
    );
}

/// A `SettingRow` carries all four columns the ticket names — key, owning entity, authored value,
/// schema default — so no column can be quietly dropped from the type the view renders.
#[test]
fn a_row_carries_all_four_columns() {
    let doc = json!({ "zonesById": { "z1": { "type": "objective_capture", "label": "Hill",
        "rules": { "captureSeconds": 240 } } } });
    let row: SettingRow = aggregate_settings(&doc).remove(0);
    assert_eq!(row.key, "captureSeconds");
    assert_eq!(row.owner.label(), "Zone — Hill");
    assert_eq!(fmt_setting_value(&row.value), "240");
    assert_eq!(fmt_setting_default(&row.default), "120");
    assert_eq!(row.diff_state(), DiffState::Differs);
}
