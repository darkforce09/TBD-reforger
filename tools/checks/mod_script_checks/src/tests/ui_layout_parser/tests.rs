use super::*;

fn analyse(body: &str) -> Vec<String> {
    Analyzer::new("T.layout").run(body)
}

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
fn c1_unbalanced_at_eof() {
    let got = analyse("FrameWidgetClass {\n Name \"R\"\n");
    assert_eq!(got, vec!["C1 T.layout: unbalanced braces (depth 1 at EOF)"]);
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
