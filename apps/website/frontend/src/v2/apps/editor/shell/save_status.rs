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
    static TOASTS: RefCell<Option<crate::v2::core::ui::toast::Toasts>> = const { RefCell::new(None) };
}

/// Current status. Source of truth is the cell so persist timers (no reactive owner) can write it.
#[must_use]
pub fn status() -> SaveStatus {
    STATUS.with(|s| s.borrow().clone())
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

/// Report that the draft is safely on disk.
pub fn report_saved() {
    report(SaveStatus::Saved);
}

/// Report that a save is in flight.
pub fn report_saving() {
    report(SaveStatus::Saving);
}

/// Report that a save was refused, carrying the reason the chip and toast will name.
pub fn report_failed(reason: impl Into<String>) {
    report(SaveStatus::Failed(reason.into()));
}

/// Report that the local backup could not be read back, carrying how many retries have run so
/// the chip can show the attempt against its limit.
pub fn report_unreadable(retries: u8) {
    report(SaveStatus::Unreadable(retries));
}

/// Persist registers the lockout Retry action (re-read the unreadable keys).
pub fn set_retry_handler(handler: impl Fn() + 'static) {
    RETRY.with(|r| *r.borrow_mut() = Some(Box::new(handler)));
}

/// Run the registered Retry action, if one is registered. A retry with no handler is a no-op
/// rather than a panic: the chip can outlive the code that armed it.
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
    if let Some(t) = use_context::<crate::v2::core::ui::toast::Toasts>() {
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
    if let Some(t) = use_context::<crate::v2::core::ui::toast::Toasts>() {
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

// `last_toast` is test-only and MUST stay below every production item: the Class-R probe in
// `unreadable_lockout_offers_retry` splits this file at the FIRST `#[cfg(test)]` and asserts over
// what precedes it. An earlier test-gated item truncates that haystack and the probe silently
// stops reading the code it exists to check — the T-937.1 failure of wave 252, one wave on.
/// Last Failed toast payload this page/test lifetime, if any. Native tests read this because
/// persist's wasm `save_state_as` cannot run on the host.
#[cfg(test)]
#[must_use]
pub fn last_toast() -> Option<String> {
    LAST_TOAST.with(|t| t.borrow().clone())
}

#[cfg(test)]
#[path = "tests/save_status/retry_and_failure.rs"]
mod tests;
