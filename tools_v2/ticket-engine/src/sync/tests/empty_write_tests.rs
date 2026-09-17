use super::*;

#[test]
fn refuse_empty_write_reds_on_empty() {
    let err = refuse_empty_write("probe", true, "structurally empty").expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(msg.contains("refusing empty write (probe)"), "{msg}");
    assert!(msg.contains("structurally empty"), "{msg}");
}

#[test]
fn refuse_empty_write_ok_when_nonempty() {
    refuse_empty_write("probe", false, "unused").expect("non-empty must pass");
}

#[test]
fn marker_inner_vacuous_detects_bare_heading() {
    assert!(marker_inner_is_vacuous(""));
    assert!(marker_inner_is_vacuous(
        "### Recommended next work (auto-generated)\n\n"
    ));
    assert!(!marker_inner_is_vacuous(
        "### Recommended next work (auto-generated)\n\n- **T-090** — map (ready)\n"
    ));
    assert!(!marker_inner_is_vacuous(
        "**Latest shipped:** **T-535**\n\n**ACTIVE NOW:** **T-090** — Map\n"
    ));
}

#[test]
fn inject_next_refuses_empty_tickets_bare_heading() {
    let registry = json!({"tickets": []});
    let err = inject_next_block(&PathBuf::from("/tmp"), &registry)
        .expect_err("empty tickets must refuse ROADMAP collapse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") && msg.contains("ROADMAP"),
        "expected ROADMAP refuse, got: {msg}"
    );
}
