use super::refuse_empty_write;

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
