use super::*;

fn analyse(body: &str) -> Vec<String> {
    Analyzer::new("T.layout").run(body)
}

// ── THE REGRESSION THAT MADE THE FIRST CUT VACUOUS ──────────────────────────────────────

#[test]
fn guid_braces_do_not_desync_the_counter() {
    // Every Workbench GUID is a quoted `"{…}"`. Counting braces on the raw line makes this file
    // look balanced at the wrong depth; stripping quotes first makes it balanced at the right
    // one. The file is clean, so the correct answer is: no findings at all.
    let body = "\
OverlayWidgetClass \"{7BD1A70000000750}\" {
 Name \"Root\"
 {
  ImageWidgetClass \"{7BD1A70000000751}\" {
   Name \"Child\"
   Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {
    HorizontalAlign 3
   }
  }
 }
}
";
    assert_eq!(analyse(body), Vec::<String>::new());
}

#[test]
fn guid_desync_would_hide_a_c6() {
    // The pin above only means something if the UNSTRIPPED count is genuinely wrong — otherwise
    // a `strip_quoted` that did nothing would satisfy it. Assert the difference directly.
    let line = " Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {";
    assert_eq!(line.matches('{').count(), 2, "raw count is the bug");
    assert_eq!(line.matches('}').count(), 1);
    assert_eq!(strip_quoted(line).matches('{').count(), 1);
    assert_eq!(strip_quoted(line).matches('}').count(), 0);

    // And end-to-end: a container child whose slot has no HorizontalAlign must be caught. With
    // a desynced counter `owner_line[depth]` misses and C6 never fires — the T-181.51 defect.
    let body = "\
OverlayWidgetClass \"{7BD1A70000000750}\" {
 Name \"Root\"
 {
  ImageWidgetClass \"{7BD1A70000000751}\" {
   Name \"Child\"
   Slot OverlayWidgetSlot \"{7BD1A70000000752}\" {
    VerticalAlign 3
   }
  }
 }
}
";
    let got = analyse(body);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(
        got[0].starts_with("C6 T.layout:4 OverlayWidgetClass > ImageWidgetClass"),
        "{got:?}"
    );
}

// ── one test per arm ────────────────────────────────────────────────────────────────────

#[test]
fn c1_unbalanced_at_eof() {
    let got = analyse("FrameWidgetClass {\n Name \"R\"\n");
    assert_eq!(got, vec!["C1 T.layout: unbalanced braces (depth 1 at EOF)"]);
}

#[test]
fn c1_stray_close_also_reports_the_eof_line() {
    // awk `exit` runs END. Two findings from one defect is the script's behaviour, pinned here
    // so a "tidier" port cannot quietly drop the second line the acceptance diff expects.
    let got = analyse("FrameWidgetClass {\n}\n}\n");
    assert_eq!(
        got,
        vec![
            "C1 T.layout:3 closing brace with no opener",
            "C1 T.layout: unbalanced braces (depth -1 at EOF)",
        ]
    );
}

#[test]
fn c2_unattested_slot_class() {
    let body = "FrameWidgetClass {\n Slot BogusSlot {\n  HorizontalAlign 3\n }\n}\n";
    assert_eq!(
        analyse(body),
        vec!["C2 T.layout:2 unattested slot class BogusSlot"]
    );
}

#[test]
fn c3_geometry_mirror() {
    let body = "\
FrameWidgetClass {
 Slot FrameWidgetSlot {
  Anchor 0 0 1 1
  PositionX 5
  OffsetLeft 3
  SizeX 0
  OffsetRight 0
 }
}
";
    assert_eq!(
        analyse(body),
        vec![
            "C3 T.layout:2 PositionX 5 != OffsetLeft 3",
            "C3 T.layout:2 SizeX 0 but -(OffsetLeft 3 + OffsetRight 0) = -3",
        ]
    );
}

#[test]
fn c4_container_child_with_no_slot() {
    let body = "OverlayWidgetClass {\n {\n  FrameWidgetClass {\n  }\n }\n}\n";
    assert_eq!(
        analyse(body),
        vec![
            "C4 T.layout:3 OverlayWidgetClass > FrameWidgetClass has no Slot block — it will \
                 collapse to its desired size"
        ]
    );
}

#[test]
fn multiple_c6_findings_come_out_in_ascending_line_order() {
    // THE ONE DEVIATION FROM THE SCRIPT, pinned so it stays a decision. mawk 1.3.4 prints these
    // two in hash order (29 before 14 on the real TBD_ListRow.layout); a BTreeMap prints them
    // in line order. See the module-level "MEASURED DEVIATION" section for why line order wins.
    let body = "\
OverlayWidgetClass {
 {
  ImageWidgetClass {
   Slot OverlayWidgetSlot {
    VerticalAlign 3
   }
  }
  TextWidgetClass {
   Slot OverlayWidgetSlot {
    VerticalAlign 3
   }
  }
 }
}
";
    let got = analyse(body);
    assert_eq!(got.len(), 2, "{got:?}");
    assert!(got[0].starts_with("C6 T.layout:3 "), "{got:?}");
    assert!(got[1].starts_with("C6 T.layout:8 "), "{got:?}");
}

#[test]
fn a_non_container_parent_needs_no_slot() {
    // FrameWidgetClass anchors its children, so C4 must not fire under one.
    let body = "FrameWidgetClass {\n {\n  ImageWidgetClass {\n  }\n }\n}\n";
    assert_eq!(analyse(body), Vec::<String>::new());
}

// ── the preserved latent bug ────────────────────────────────────────────────────────────

#[test]
fn stale_geometry_leaks_across_slots() {
    // `flush_frame` returns before `delete val` when have_frame is false, so an OffsetLeft
    // written on a ButtonWidgetSlot survives into the NEXT FrameWidgetSlot. Reported, not
    // fixed — changing it changes what the gate prints on a broken tree.
    let body = "\
ButtonWidgetClass {
 Slot ButtonWidgetSlot {
  OffsetLeft 99
 }
 {
  ImageWidgetClass {
   Slot FrameWidgetSlot {
    HorizontalAlign 3
    PositionX 0
   }
  }
 }
}
";
    let got = analyse(body);
    assert!(
        got.iter()
            .any(|l| l.contains("PositionX 0 != OffsetLeft 99")),
        "the leak is load-bearing for byte-compatibility: {got:?}"
    );
}

// ── awk primitives ──────────────────────────────────────────────────────────────────────

#[test]
fn awk_number_conversion_matches_v_plus_zero() {
    assert_eq!(awk_to_number("240"), 240.0);
    assert_eq!(awk_to_number("-480"), -480.0);
    assert_eq!(awk_to_number("0.5"), 0.5);
    assert_eq!(awk_to_number("Fill"), 0.0);
    assert_eq!(awk_to_number("3 0 0 0"), 3.0);
    assert_eq!(awk_to_number(""), 0.0);
}

#[test]
fn awk_string_conversion_uses_percent_d_for_integers() {
    assert_eq!(awk_to_string(240.0), "240");
    assert_eq!(awk_to_string(-0.0), "0");
    assert_eq!(awk_to_string(-3.0), "-3");
    assert_eq!(awk_to_string(0.5), "0.5");
}

#[test]
fn records_keep_a_trailing_carriage_return() {
    // `str::lines()` would eat the `\r` and change what a C3 message prints.
    assert_eq!(awk_records("a\r\nb\n"), vec!["a\r", "b"]);
    assert_eq!(awk_records(""), Vec::<&str>::new());
    assert_eq!(awk_records("a"), vec!["a"]);
}

#[test]
fn widget_decl_predicate_matches_the_bash_regex() {
    assert!(is_widget_decl("ButtonWidgetClass {"));
    assert!(is_widget_decl("  SizeLayoutWidgetClass \"{7BD}\" {"));
    assert!(is_widget_decl("\tOverlayWidgetClass{"));
    assert!(!is_widget_decl("WidgetClass {"), "needs a prefix");
    assert!(!is_widget_decl(" TBD_ListBoxRow \"{7BD}\" {"));
    assert!(!is_widget_decl(" components {"));
}

#[test]
fn geometry_keyword_needs_a_separator() {
    assert!(starts_with_keyword(" PositionX 5", "PositionX"));
    assert!(!starts_with_keyword(" PositionXY 5", "PositionX"));
}
