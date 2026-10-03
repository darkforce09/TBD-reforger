//! The guards on the offline pack wait.

use super::*;

#[test]
fn mortar_offline_a_ready_pack_reports_its_optional_icon_font_as_complete_or_missing() {
    assert_eq!(optional_files_report(Some("complete")).unwrap(), "complete");
    assert_eq!(optional_files_report(Some("missing")).unwrap(), "missing");
}

#[test]
fn mortar_offline_a_ready_pack_without_a_valid_optional_report_fails() {
    let absent = optional_files_report(None).unwrap_err().to_string();
    assert!(absent.contains("without data-offline-optional"), "{absent}");
    let unknown = optional_files_report(Some("partial"))
        .unwrap_err()
        .to_string();
    assert!(unknown.contains("\"partial\""), "{unknown}");
    assert!(optional_files_report(Some("")).is_err());
}
