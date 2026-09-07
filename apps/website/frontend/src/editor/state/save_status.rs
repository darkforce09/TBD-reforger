//! T-937.4 — observable persist status: chip + toast, not `console.warn`.
//!
//! `persist.rs` is wasm32-only and used to swallow every `save_state_as` `Err` into
//! `web_sys::console::warn_1`, so a quota failure (or any other IndexedDB write error) left the
//! author with no chip, no toast, and no signal. This module is the native-testable surface that
//! persist reports into: [`SaveStatus`], a status chip, and one toast per Failed episode.

use std::cell::RefCell;

use leptos::prelude::*;

/// Idle debounce the persist writer must arm with. T-937.4: at most 1 s (was 5 s).
pub const IDLE_DEBOUNCE_MS: i32 = 1_000;

/// `note_unreadable` retries the read this many times with backoff before lockout.
pub const UNREADABLE_RETRY_LIMIT: u8 = 3;

/// Autosave status the chip renders and tests assert.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SaveStatus {
    Saved,
    Saving,
    Failed(String),
    Unreadable(u8),
}

thread_local! {
    static STATUS: RefCell<SaveStatus> = const { RefCell::new(SaveStatus::Saved) };
    static STATUS_SIG: RefCell<Option<RwSignal<SaveStatus>>> = const { RefCell::new(None) };
    static LAST_EPISODE: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_TOAST: RefCell<Option<String>> = const { RefCell::new(None) };
    static RETRY: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
    #[cfg(target_arch = "wasm32")]
    static CHIP_MOUNTED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    #[cfg(target_arch = "wasm32")]
    static TOASTS: RefCell<Option<crate::core::toast::Toasts>> = const { RefCell::new(None) };
}

/// Current status. Source of truth is the cell so persist timers (no reactive owner) can write it.
#[must_use]
pub fn status() -> SaveStatus {
    STATUS.with(|s| s.borrow().clone())
}

/// Last Failed toast payload this page/test lifetime, if any. Native tests read this because
/// persist's wasm `save_state_as` cannot run on the host.
#[cfg(test)]
#[must_use]
pub fn last_toast() -> Option<String> {
    LAST_TOAST.with(|t| t.borrow().clone())
}

/// Chip copy, or `None` when the chip should hide (Saved — the draft-recency chip already covers
/// a successful flush).
#[must_use]
pub fn chip_label(status: &SaveStatus) -> Option<String> {
    match status {
        SaveStatus::Saved => None,
        SaveStatus::Saving => Some("Saving…".to_string()),
        SaveStatus::Failed(reason) => Some(format!("Save failed: {reason}")),
        SaveStatus::Unreadable(n) if *n >= UNREADABLE_RETRY_LIMIT => {
            Some("Local backup unreadable".to_string())
        }
        SaveStatus::Unreadable(n) => Some(format!(
            "Retrying local backup ({n}/{UNREADABLE_RETRY_LIMIT})"
        )),
    }
}

/// Name a persist error. Quota failures are named as quota regardless of the surrounding wrapper
/// text (`QuotaExceededError`, `NS_ERROR_DOM_QUOTA_REACHED`, idb `DomException` debug, …).
#[must_use]
pub fn format_save_error(raw: &str) -> String {
    if raw.to_ascii_lowercase().contains("quota") {
        "quota: browser storage is full — the draft was not saved".to_string()
    } else {
        format!("save failed: {raw}")
    }
}

pub fn report_saved() {
    report(SaveStatus::Saved);
}

pub fn report_saving() {
    report(SaveStatus::Saving);
}

pub fn report_failed(reason: impl Into<String>) {
    report(SaveStatus::Failed(reason.into()));
}

pub fn report_unreadable(retries: u8) {
    report(SaveStatus::Unreadable(retries));
}

/// Persist registers the lockout Retry action (re-read the unreadable keys).
pub fn set_retry_handler(handler: impl Fn() + 'static) {
    RETRY.with(|r| *r.borrow_mut() = Some(Box::new(handler)));
}

pub fn invoke_retry() {
    RETRY.with(|r| {
        if let Some(h) = r.borrow().as_ref() {
            h();
        }
    });
}

/// Capture the shell toast context (if a reactive owner is live) and mount the chip overlay.
#[cfg(target_arch = "wasm32")]
pub fn bind_runtime() {
    if let Some(t) = use_context::<crate::core::toast::Toasts>() {
        TOASTS.with(|c| *c.borrow_mut() = Some(t));
    }
    ensure_chip_mounted();
}

fn report(next: SaveStatus) {
    let prev = status();
    STATUS.with(|s| *s.borrow_mut() = next.clone());
    STATUS_SIG.with(|sig| {
        if let Some(sig) = *sig.borrow() {
            sig.set(next.clone());
        }
    });
    if let SaveStatus::Failed(reason) = &next {
        let is_new_episode = LAST_EPISODE.with(|e| {
            let mut e = e.borrow_mut();
            if e.as_deref() == Some(reason.as_str()) {
                false
            } else {
                *e = Some(reason.clone());
                true
            }
        });
        if is_new_episode {
            toast_failed(reason);
        }
    } else if matches!(prev, SaveStatus::Failed(_)) {
        LAST_EPISODE.with(|e| *e.borrow_mut() = None);
    }
    ensure_chip_mounted();
}

fn toast_failed(reason: &str) {
    LAST_TOAST.with(|t| *t.borrow_mut() = Some(reason.to_string()));
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(toasts) = TOASTS.with(|t| *t.borrow()) {
            toasts.error(format!("Save failed: {reason}"));
            return;
        }
        inject_fallback_toast(reason);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_chip_mounted() {}

#[cfg(target_arch = "wasm32")]
fn ensure_chip_mounted() {
    use wasm_bindgen::JsCast;
    if CHIP_MOUNTED.with(std::cell::Cell::get) {
        return;
    }
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if document.get_element_by_id("tbd-save-status-host").is_some() {
        CHIP_MOUNTED.with(|c| c.set(true));
        return;
    }
    let Some(body) = document.body() else {
        return;
    };
    let Ok(host) = document.create_element("div") else {
        return;
    };
    host.set_id("tbd-save-status-host");
    let _ = body.append_child(&host);
    let Ok(html) = host.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };
    let handle = leptos::mount::mount_to(html, || view! { <SaveStatusChip /> });
    handle.forget();
    CHIP_MOUNTED.with(|c| c.set(true));
}

#[cfg(target_arch = "wasm32")]
fn inject_fallback_toast(reason: &str) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(body) = document.body() else {
        return;
    };
    let Ok(node) = document.create_element("div") else {
        return;
    };
    node.set_attribute("role", "status").ok();
    node.set_attribute("data-testid", "persist-save-toast").ok();
    node.set_class_name(
        "fixed right-4 top-4 z-[100] glass flex w-80 items-start gap-2 rounded-lg border \
         border-error-alert/40 px-4 py-3 text-sm text-error-alert shadow-lg",
    );
    node.set_text_content(Some(&format!("Save failed: {reason}")));
    let _ = body.append_child(&node);
    if let Some(win) = web_sys::window() {
        let el = node.clone();
        let cb = Closure::once_into_js(move || {
            el.remove();
        });
        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            4_000,
        );
    }
}

/// Status chip. Self-mounted on wasm from [`bind_runtime`]/[`report`]; also mountable in a parent
/// view. Offers Retry when unreadable retries have locked out.
#[component]
pub fn SaveStatusChip() -> impl IntoView {
    let sig = RwSignal::new(status());
    STATUS_SIG.with(|s| *s.borrow_mut() = Some(sig));
    #[cfg(target_arch = "wasm32")]
    if let Some(t) = use_context::<crate::core::toast::Toasts>() {
        TOASTS.with(|c| *c.borrow_mut() = Some(t));
    }

    view! {
        {move || {
            let st = sig.get();
            let label = chip_label(&st)?;
            let show_retry = matches!(st, SaveStatus::Unreadable(n) if n >= UNREADABLE_RETRY_LIMIT);
            Some(view! {
                <div
                    class="glass fixed bottom-4 right-4 z-[90] flex items-center gap-2 rounded-lg border border-outline-variant/40 px-3 py-2 text-sm text-on-surface shadow-lg"
                    data-testid="save-status-chip"
                    role="status"
                    aria-live="polite"
                >
                    <span>{label}</span>
                    {show_retry.then(|| {
                        view! {
                            <button
                                type="button"
                                class="rounded border border-outline-variant/40 px-2 py-0.5 text-xs"
                                data-testid="save-status-retry"
                                on:click=move |_| invoke_retry()
                            >
                                "Retry"
                            </button>
                        }
                    })}
                </div>
            })
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::arsenal::class_r_scrub::{live_code, live_source, only_item};

    fn reset() {
        STATUS.with(|s| *s.borrow_mut() = SaveStatus::Saved);
        LAST_EPISODE.with(|e| *e.borrow_mut() = None);
        LAST_TOAST.with(|t| *t.borrow_mut() = None);
        STATUS_SIG.with(|s| *s.borrow_mut() = None);
        RETRY.with(|r| *r.borrow_mut() = None);
    }

    fn persist_live() -> String {
        live_code(include_str!("persist.rs"))
    }

    fn persist_source() -> String {
        live_source(include_str!("persist.rs"))
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
        let src = live_source(include_str!("save_status.rs"));
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
}
