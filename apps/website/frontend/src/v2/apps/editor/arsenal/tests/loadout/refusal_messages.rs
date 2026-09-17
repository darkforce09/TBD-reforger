use super::serialization_and_export_tests::{attachment_feed, picks};
use super::*;

/// **T-737 — a refusal has to say which row.**
///
/// The defect is not "the message is wrong"; every message here is true. The defect is that
/// two *different* rows produce the *same* true sentence, so the list the author is shown
/// cannot be acted on. Every test below therefore uses **two** stranded rows — one row cannot
/// observe this defect at all, and a test written with one would have stayed green through it.
mod t737 {
    use super::*;

    /// A ready feed carrying arbitrary typed edges. `attachment_feed` only speaks
    /// `attachment_on_weapon`; the two rows this defect is about (`optic`, `magazine`) are
    /// `RowSource::Edge` rows on two *other* edge types, so they need their own feed.
    fn typed_feed(edges: &[(&str, &str, &str)]) -> CompatFeed {
        let rows: Vec<crate::v2::core::api::dto::RegistryCompatEdge> = edges
            .iter()
            .enumerate()
            .map(
                |(i, (from, to, ty))| crate::v2::core::api::dto::RegistryCompatEdge {
                    id: i.to_string(),
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

    /// `mod t699`'s buffer helpers, re-declared rather than borrowed: they are private to that
    /// module, and a sibling reaching into it would not compile.
    fn buf(source: &str, json: Option<&str>) -> BufferedLoadout {
        BufferedLoadout {
            source_id: source.to_string(),
            loadout_json: json.map(str::to_string),
        }
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    /// One weapon swap, two stranded rows: an ACOG and a STANAG that this catalog knows —
    /// on the *other* rifle. Both edge rows are refused, and refused for the same reason.
    fn two_stranded_rows() -> (String, CompatFeed) {
        let raw = picks_to_export(
            &picks(&[
                ("primary", "res://rifle_m16"),
                ("optic", "res://acog"),
                ("magazine", "res://mag_stanag"),
            ]),
            &[],
            "mp",
        );
        let feed = typed_feed(&[
            ("res://rifle_ak", "res://acog", "optic_on_weapon"),
            ("res://rifle_ak", "res://mag_stanag", "mag_in_weapon"),
        ]);
        (raw, feed)
    }

    /// **The claim in the ticket title.** Two stranded rows must render as two lines the
    /// author can tell apart, each naming its own row — while the reason each carries survives
    /// intact underneath.
    ///
    /// RED (the shipped defect): render the list with `.map(|e| e.message)` again — i.e. make
    /// `refusal_line` return `e.message.clone()` unconditionally → "two stranded rows must not
    /// print the same line".
    #[test]
    fn two_stranded_rows_render_as_two_distinguishable_refusals() {
        let (raw, feed) = two_stranded_rows();
        let refusals = try_import(&raw, &[], &feed).expect_err("a stranded loadout is refused");
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

    /// The same two rows down the **Apply** door, which shares the refusal contract and shared
    /// the defect. `buffer_refusals` names which copied loadout is bad; `refusal_line` names
    /// which row inside it — and both are needed, because one buffered loadout can strand two
    /// rows at once.
    #[test]
    fn apply_refusals_name_the_row_as_well_as_the_source() {
        let (raw, feed) = two_stranded_rows();
        let refusals = plan_apply(&ids(&["t1"]), &[buf("s1", Some(&raw))], 7, &[], &feed)
            .expect_err("…nor applicable");
        assert_eq!(refusals.len(), 2, "{refusals:?}");
        assert_eq!(
            refusals[0].message, refusals[1].message,
            "the source prefix alone cannot tell two rows of ONE loadout apart"
        );

        let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
        assert_ne!(lines[0], lines[1], "{lines:?}");
        for line in &lines {
            assert!(
                line.contains("Buffered loadout from s1"),
                "which copied loadout is still the first question: {line}"
            );
        }
        assert!(lines[0].starts_with("Optic — "), "{lines:?}");
        assert!(lines[1].starts_with("Magazine — "), "{lines:?}");
    }

    /// Schema and parse faults are left exactly as they were: their messages already carry the
    /// JSON pointer, which is a better address than any row label, and there is no row to name.
    /// The exemption is `rules::row` answering `None` — not a hard-coded key comparison.
    #[test]
    fn document_faults_keep_their_own_address() {
        for raw in ["{ not json", r#"{"loadoutVersion":"2"}"#] {
            let refusals = try_import(raw, &[], &CompatFeed::default())
                .expect_err("a malformed document is refused");
            assert!(!refusals.is_empty());
            for e in &refusals {
                assert_eq!(e.key, IMPORT_DOC_KEY, "no row is to blame: {e:?}");
                assert_eq!(
                    refusal_line(e),
                    e.message,
                    "a document fault must not grow a row prefix it cannot justify"
                );
            }
        }
    }

    /// **The T-686 asymmetry is not this ticket's to change, and this pins that it did not.**
    /// Export refuses on capacity ONLY, so a loadout with a stranded optic can still be
    /// downloaded; the import gate additionally refuses compat, so the same bytes cannot come
    /// back in. That is intended — the import gate's job is to not let an outside document put
    /// the editor into a state the author did not author — and it is precisely the case where
    /// naming the row matters most, so the naming is asserted on the very bytes that prove it.
    #[test]
    fn the_export_import_asymmetry_still_holds() {
        let (raw, feed) = two_stranded_rows();
        let doc_picks = loadout_to_picks(Some(&raw));
        assert!(
            try_export(&doc_picks, &[], &[], "mp").is_ok(),
            "export refuses on capacity only — a stranded optic must still download"
        );
        let refusals = try_import(&raw, &[], &feed)
            .expect_err("…and the import gate must still refuse those same bytes on compat");
        let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
        assert_ne!(
            lines[0], lines[1],
            "the exportable-but-not-importable case is where naming the row matters most"
        );
    }

    /// **The same defect one level down — the case `refusal_line` structurally cannot reach.**
    ///
    /// Two stranded ROWS differ in their `key`, so prefixing the row label separates them. Two
    /// stranded ATTACHMENTS hang off the SAME weapon row: same key, therefore same prefix, and
    /// the reason was identical too — so an author who swapped one rifle was handed
    /// "Primary — Attachment not compatible with the selected Primary" **twice** and learned
    /// what was wrong but not which of their two attachments to pull. The only place left to
    /// carry the difference is the message, so the message names the attachment.
    ///
    /// One attachment cannot observe this, exactly as one row could not observe T-737's.
    ///
    /// RED (the shipped defect): drop `` `{rn}` `` from both message arms in
    /// `attachment_errors` → "two stranded attachments must not print the same line".
    #[test]
    fn two_stranded_attachments_on_one_row_render_as_two_distinguishable_refusals() {
        // Both attachments are known to this catalog — on the OTHER rifle. One swap strands
        // both at once, which is the whole point: it is a single authoring mistake.
        let feed = attachment_feed(&[
            ("res://handguard", "res://rifle_ak"),
            ("res://supp", "res://rifle_ak"),
        ]);
        let mut p = picks(&[("primary", "res://rifle_m16")]);
        p.insert(
            attachments_key("primary"),
            pack_attachments(&["res://handguard".into(), "res://supp".into()]),
        );

        let errs = attachment_errors(&p, &feed);
        assert_eq!(errs.len(), 2, "both attachments are stranded: {errs:?}");
        // The premise, as a fact about the data: `refusal_line` has nothing to work with here.
        // One row, one key — the difference cannot live where T-737 put it.
        assert_eq!(
            [errs[0].key, errs[1].key],
            ["primary", "primary"],
            "{errs:?}"
        );

        let lines: Vec<String> = errs.iter().map(refusal_line).collect();
        assert_ne!(
            lines[0], lines[1],
            "two stranded attachments must not print the same line: {lines:?}"
        );
        // …and each line names *its own* attachment, not merely some attachment.
        assert!(
            lines[0].contains("res://handguard") && !lines[0].contains("res://supp"),
            "{lines:?}"
        );
        assert!(
            lines[1].contains("res://supp") && !lines[1].contains("res://handguard"),
            "{lines:?}"
        );
        // Naming the item is not paid for with the row prefix T-737 added.
        for line in &lines {
            assert!(line.starts_with("Primary — "), "{lines:?}");
        }

        // The hostless arm carries the same burden: no Primary at all, two attachments still
        // stranded, still two lines an author can tell apart.
        p.remove("primary");
        let hostless: Vec<String> = attachment_errors(&p, &feed)
            .iter()
            .map(refusal_line)
            .collect();
        assert_eq!(hostless.len(), 2, "{hostless:?}");
        assert_ne!(hostless[0], hostless[1], "{hostless:?}");
        assert!(hostless[0].contains("res://handguard"), "{hostless:?}");
        assert!(hostless[1].contains("res://supp"), "{hostless:?}");
    }
}
