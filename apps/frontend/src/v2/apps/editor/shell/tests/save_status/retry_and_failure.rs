//! Save Status tests tests.

use super::*;
use crate::v2::core::test_support::class_r_scrub::{live_code, live_source, only_item};

fn reset() {
    STATUS.with(|s| *s.borrow_mut() = SaveStatus::Saved);
    LAST_EPISODE.with(|e| *e.borrow_mut() = None);
    LAST_TOAST.with(|t| *t.borrow_mut() = None);
    STATUS_SIG.with(|s| *s.borrow_mut() = None);
    RETRY.with(|r| *r.borrow_mut() = None);
}

fn persist_text() -> String {
    [
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist/record_store.rs"
        )),
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/v2/apps/editor/shell/persist/save_scheduler.rs"
        )),
    ]
    .join("\n")
}

fn persist_live() -> String {
    live_code(&persist_text())
}

fn persist_source() -> String {
    live_source(&persist_text())
}

fn run_save_err_arm() -> String {
    let live = persist_live();
    let item = only_item(&live, "async fn run_save(").to_string();
    let at = item
        .find("if let Err(e) = save_state_as")
        .unwrap_or_else(|| panic!("run_save must call save_state_as; item={item}"));
    item[at..].to_string()
}

/// RED on the pre-T-937.4 persist: `save_state_as` Err is `console.warn` then `return`. No
/// [`SaveStatus`], no toast, no chip — forcing that Err is not observable in editor state.
#[test]
fn run_save_err_arm_reports_into_save_status() {
    let arm = run_save_err_arm();
    assert!(
        arm.contains("report_failed") && arm.contains("format_save_error"),
        "forcing save_state_as Err currently shows no observable state — persist must report \
             every Err into SaveStatus (quota named via format_save_error). arm={arm}"
    );
    assert!(
        !arm.contains("console::warn_1") || arm.contains("report_failed"),
        "console.warn alone is not an error surface"
    );
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
fn unreadable_lockout_offers_retry() {
    reset();
    report_unreadable(UNREADABLE_RETRY_LIMIT);
    assert_eq!(
        chip_label(&status()).as_deref(),
        Some("Local backup unreadable")
    );
    let src = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/save_status.rs"
    )));
    let prod = src.split("#[cfg(test)]").next().expect("test module");
    assert!(
        prod.contains("Retry") && prod.contains("invoke_retry"),
        "locked Unreadable chip must offer Retry"
    );
    let live = persist_live();
    let hide = only_item(&live, "pub fn register_flush_on_hide(").to_string();
    assert!(
        hide.contains("set_retry_handler"),
        "persist must register the chip Retry handler. hide={hide}"
    );
}

#[test]
fn idle_debounce_is_at_most_one_second() {
    assert!(
        IDLE_DEBOUNCE_MS <= 1_000,
        "idle debounce must be ≤ 1 s, got {IDLE_DEBOUNCE_MS}"
    );
    let live = persist_live();
    let debounce = only_item(&live, "pub const fn debounce_ms(").to_string();
    assert!(
        debounce.contains("DEBOUNCE_MS"),
        "debounce_ms must return the T-937.4 delay. debounce={debounce}"
    );
    let src = persist_source();
    assert!(
        src.contains("IDLE_DEBOUNCE_MS"),
        "persist.rs must arm the idle writer from save_status::IDLE_DEBOUNCE_MS"
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

#[test]
fn note_unreadable_retries_three_times_before_lockout() {
    let live = persist_live();
    let item = only_item(&live, "async fn note_unreadable(").to_string();
    assert!(
        item.contains("read_raw") && item.contains("UNREADABLE_RETRY_LIMIT"),
        "note_unreadable must retry the read UNREADABLE_RETRY_LIMIT times. item={item}"
    );
    assert!(
        item.contains("report_unreadable"),
        "each failed retry / lockout must reach SaveStatus::Unreadable. item={item}"
    );
    assert!(
        item.contains("sleep_ms") || item.contains("set_timeout"),
        "retries must back off, not spin. item={item}"
    );
}

#[test]
fn hidden_flushes_immediately_pagehide_stays_fire_and_forget() {
    let live = persist_live();
    let src = persist_source();
    let hide = only_item(&live, "pub fn register_flush_on_hide(").to_string();
    let hide_src = only_item(&src, "pub fn register_flush_on_hide(").to_string();
    assert!(
        hide_src.contains("visibilitychange")
            && hide.contains("hidden")
            && hide.contains("flush_state"),
        "visibilitychange-hidden must flush immediately. hide={hide} hide_src={hide_src}"
    );
    assert!(
        hide_src.contains("pagehide") && hide.contains("spawn_local"),
        "pagehide fire-and-forget must stay. hide={hide} hide_src={hide_src}"
    );
    let run = only_item(&live, "async fn run_save(").to_string();
    let flush = only_item(&live, "pub async fn flush_state(").to_string();
    assert!(
        run.contains("SAVE_IN_FLIGHT") || flush.contains("SAVE_IN_FLIGHT"),
        "hidden flush and pagehide must share an in-flight guard"
    );
}
