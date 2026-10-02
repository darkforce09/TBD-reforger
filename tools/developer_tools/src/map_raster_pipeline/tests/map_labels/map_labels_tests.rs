//! The label exporters refuse an empty label set before touching the output file, so a failed
//! extraction cannot silently overwrite a good export with nothing.

use super::super::refuse_empty_write;

#[test]
fn height_labels_refuse_empty_contract() {
    // Perturbation contract: empty label set must refuse before any write.
    let err = refuse_empty_write(
        "export-height-labels",
        true,
        "zero labels — refusing empty overwrite of height-labels.json",
    )
    .expect_err("must refuse");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write (export-height-labels)"),
        "{msg}"
    );
    assert!(msg.contains("zero labels"), "{msg}");
}
