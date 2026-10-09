//! The save status: a quota failure is named as quota, a failure episode toasts once, and a
//! save in flight is visible until it lands.

use super::*;

fn reset() {
    STATUS.with(|s| *s.borrow_mut() = SaveStatus::Saved);
    LAST_EPISODE.with(|e| *e.borrow_mut() = None);
    LAST_TOAST.with(|t| *t.borrow_mut() = None);
    STATUS_SIG.with(|s| *s.borrow_mut() = None);
    RETRY.with(|r| *r.borrow_mut() = None);
}

#[test]
fn quota_failure_is_named_quota() {
    let named = format_save_error("DomException(QuotaExceededError)");
    assert!(
        named.contains("quota"),
        "quota failure must be named as quota, got {named}"
    );
    let other = format_save_error("failed to add a value");
    assert!(
        !other.to_ascii_lowercase().contains("quota"),
        "non-quota errors must not be labelled quota, got {other}"
    );
}

#[test]
fn failed_status_toasts_once_per_episode() {
    reset();
    report_failed("quota: browser storage is full — the draft was not saved");
    assert_eq!(
        status(),
        SaveStatus::Failed("quota: browser storage is full — the draft was not saved".into())
    );
    assert_eq!(
        last_toast().as_deref(),
        Some("quota: browser storage is full — the draft was not saved")
    );
    let label = chip_label(&status()).expect("Failed chip must be visible");
    assert!(
        label.contains("quota"),
        "Failed chip must name the reason, got {label}"
    );
    LAST_TOAST.with(|t| *t.borrow_mut() = None);
    report_failed("quota: browser storage is full — the draft was not saved");
    assert_eq!(last_toast(), None, "one toast per failure episode");
    report_saved();
    report_failed("quota: browser storage is full — the draft was not saved");
    assert!(
        last_toast().is_some(),
        "a new episode after Saved must toast again"
    );
}

#[test]
fn saving_is_observable() {
    reset();
    report_saving();
    assert_eq!(chip_label(&status()).as_deref(), Some("Saving…"));
    set_retry_handler(|| {});
    invoke_retry();
    report_saved();
    assert_eq!(chip_label(&status()), None);
}
