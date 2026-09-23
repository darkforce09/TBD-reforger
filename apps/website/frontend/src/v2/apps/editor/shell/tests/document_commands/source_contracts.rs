//! Document Commands tests tests.

#[test]
fn class_r_source_forbids_value_pretty_on_compiled_export() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/document_commands/imp/compilation.rs"
    ));
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
    let prose = SRC.split("#[cfg(test)]").next().expect("tests marker");
    assert!(
        prose.contains("401") && prose.contains("expired session"),
        "ROW_META docs must name 401 / expired session"
    );
}

/// T-746 / wave 131 F2 — `set_row_meta` must retain `detail.game_mode` inside `HydratedRow`
/// (not a hollow `game_mode: String::new()` that still greps as `game_mode`). HOST cannot call
/// the wasm-only `set_row_meta`; the Class-R body pin is the load-bearing gate. Needles are
/// split so this assertion line cannot satisfy itself.
#[test]
fn t746_row_hydrate_keeps_game_mode_beside_meta() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/document_commands.rs"
    )));
    let set = only_body(&src, "pub fn set_row_meta");
    let row_hydrate = format!("{}{}", "ROW_", "HYDRATE");
    let hydrated = format!("{}{}", "Hydrated", "Row {");
    let from_detail = format!("{}{}", "game_mode: detail.", "game_mode");
    assert!(
        set.contains(&row_hydrate),
        "T-746: set_row_meta must write ROW_HYDRATE"
    );
    assert!(
        set.contains(&hydrated),
        "T-746: set_row_meta must construct HydratedRow"
    );
    assert!(
        set.contains(&from_detail),
        "T-746: HydratedRow.game_mode must come from detail.game_mode (not String::new())"
    );
    for getter in [
        "pub(crate) fn hydrated_row",
        "pub(crate) fn row_max_players",
        "pub(crate) fn note_hydrated_game_mode",
    ] {
        assert!(src.contains(getter), "T-746: missing getter/note {getter}");
    }
}

/// **T-601 — calibration for the export pin above.**
///
/// The needle it cannot do without is `compiled_export_text(&doc)`. Every wrapper in the
/// battery must stop satisfying it, or a `to_string_pretty` download could ship while this
/// pin reported the compact wire path was live.
#[test]
fn the_export_pin_rejects_every_dead_code_wrapper() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
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
        let forged = format!("pub fn compiled_document_json() {{\n    {body}\n}}\n#[cfg(test)]\n");
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
        let caught =
            std::panic::catch_unwind(|| only_body(&scrubbed, "pub fn compiled_document_json()"))
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

#[test]
fn class_r_merge_mission_now_runs_the_after_local_edit_tail() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/document_commands/imp/mission_merge.rs"
    ));
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

use website_map_engine::data::scenario::validate::Finding;
use website_map_engine::data::scenario::validate::Primitive;
use website_map_engine::data::scenario::validate::Severity;

/// A finding shaped like the ones a compile emits.
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

#[test]
fn compile_findings_reach_the_validation_panel() {
    use crate::v2::apps::editor::ui::inspector::validation_panel::{
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
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/document_commands/imp/exports.rs"
    ));
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
