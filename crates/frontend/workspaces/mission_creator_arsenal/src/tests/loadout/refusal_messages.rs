//! Import refusals: two stranded rows render as two distinguishable lines, and export
//! refuses on capacity only while import also refuses on compat.

use super::serialization_and_export_tests::picks;
use super::*;

/// A ready feed carrying arbitrary typed edges. `attachment_feed` only speaks
/// `attachment_on_weapon`; the two rows this defect is about (`optic`, `magazine`) are
/// `RowSource::Edge` rows on two *other* edge types, so they need their own feed.
fn typed_feed(edges: &[(&str, &str, &str)]) -> CompatFeed {
    let rows: Vec<frontend_api_dtos::RegistryCompatEdge> = edges
        .iter()
        .enumerate()
        .map(
            |(i, (from, to, ty))| frontend_api_dtos::RegistryCompatEdge {
                id: i.to_string().into(),
                modpack_id: "m".into(),
                from_node: (*from).into(),
                to_node: (*to).into(),
                edge_type: (*ty).into(),
                evidence: String::new(),
                qty: 1,
                created_at: String::new(),
                updated_at: String::new(),
            },
        )
        .collect();
    CompatFeed {
        status: rules::CompatStatus::Ready,
        graph: rules::CompatGraph::from_edges(&rows),
    }
}

/// One weapon swap, two stranded rows: an ACOG and a STANAG that this catalog knows on the
/// other rifle. Both edge rows are refused, and refused for the same reason.
fn two_stranded_rows() -> (String, CompatFeed) {
    let raw = picks_to_export(
        &picks(&[
            ("primary", "res://rifle_m16"),
            ("optic", "res://acog"),
            ("magazine", "res://mag_stanag"),
        ]),
        &[],
        &ModpackId::from("mp"),
    );
    let feed = typed_feed(&[
        ("res://rifle_ak", "res://acog", "optic_on_weapon"),
        ("res://rifle_ak", "res://mag_stanag", "mag_in_weapon"),
    ]);
    (raw, feed)
}

#[test]
fn two_stranded_rows_render_as_two_distinguishable_refusals() {
    let (raw, feed) = two_stranded_rows();
    let refusals = try_import(&raw, &[], &feed)
        .expect_err("a stranded loadout is refused")
        .into_refusals();
    assert_eq!(refusals.len(), 2, "two rows are stranded: {refusals:?}");

    // The premise, stated as a fact about the data rather than assumed: the two REASONS
    // are byte-identical. Everything that distinguishes the rows lives in `key`, which is
    // exactly what the old rendering threw away.
    assert_eq!(
        refusals[0].message, refusals[1].message,
        "the premise of this test — the reason alone cannot tell the rows apart"
    );
    assert_eq!(
        [refusals[0].key, refusals[1].key],
        ["optic", "magazine"],
        "…and the key is where the difference is: {refusals:?}"
    );

    let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
    assert_ne!(
        lines[0], lines[1],
        "two stranded rows must not print the same line: {lines:?}"
    );
    assert!(lines[0].starts_with("Optic — "), "{lines:?}");
    assert!(lines[1].starts_with("Magazine — "), "{lines:?}");
    for (line, e) in lines.iter().zip(&refusals) {
        assert!(
            line.ends_with(&e.message),
            "naming the row must not cost the reason: {line}"
        );
    }
}

#[test]
fn the_export_import_asymmetry_still_holds() {
    let (raw, feed) = two_stranded_rows();
    let doc_picks = loadout_to_picks(Some(&raw));
    assert!(
        try_export(&doc_picks, &[], &[], &ModpackId::from("mp")).is_ok(),
        "export refuses on capacity only — a stranded optic must still download"
    );
    let refusals = try_import(&raw, &[], &feed)
        .expect_err("…and the import gate must still refuse those same bytes on compat")
        .into_refusals();
    let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
    assert_ne!(
        lines[0], lines[1],
        "the exportable-but-not-importable case is where naming the row matters most"
    );
}
