//! The crate's one clipboard write, which reports whether the copy landed.
//!
//! `navigator.clipboard.writeText` returns a promise that rejects on an insecure context, on an
//! unfocused document and on a denied permission. Dropping that promise and toasting success
//! anyway reports a copy that never happened: the operator pastes whatever was on the clipboard
//! before. [`write_clipboard`] awaits the promise, toasts success on its resolve arm only and
//! names the browser's reason on its reject arm. Every surface that copies text calls it; a
//! second clipboard path is a second vocabulary for "did the copy land", and one of the two ends
//! up wrong.

#[cfg(target_arch = "wasm32")]
use leptos::task::spawn_local;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};

#[cfg(target_arch = "wasm32")]
use crate::v2::core::ui::toast::Toasts;

/// Write `text` to the clipboard, then toast `ok_message` once the browser confirms the write,
/// or the browser's reason when it refuses.
#[cfg(target_arch = "wasm32")]
pub fn write_clipboard(text: String, ok_message: String, toasts: Toasts) {
    let clipboard = match clipboard_api() {
        Ok(c) => c,
        Err(why) => {
            toasts.error(format!("Could not copy — {why}."));
            return;
        }
    };
    let promise = clipboard.write_text(&text);
    spawn_local(async move {
        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(_) => toasts.success(ok_message),
            Err(e) => toasts.error(format!(
                "Could not copy to the clipboard — {}. Click the page and try again.",
                js_error_text(&e)
            )),
        }
    });
}

/// Resolve `navigator.clipboard`, refusing rather than throwing when the browser does not expose
/// it. The property is absent on an insecure origin (plain http on a non-localhost host), and
/// calling `writeText` on `undefined` would raise a JS exception straight through the wasm
/// boundary instead of producing a message the operator can act on.
#[cfg(target_arch = "wasm32")]
fn clipboard_api() -> Result<web_sys::Clipboard, String> {
    let win = web_sys::window().ok_or_else(|| "there is no browser window".to_string())?;
    let nav: JsValue = win.navigator().into();
    let raw = js_sys::Reflect::get(&nav, &JsValue::from_str("clipboard"))
        .map_err(|_| "this browser exposes no navigator.clipboard".to_string())?;
    if raw.is_undefined() || raw.is_null() {
        return Err(
            "the Clipboard API is unavailable here — it needs a secure context (https, or \
             localhost)"
                .to_string(),
        );
    }
    Ok(raw.unchecked_into::<web_sys::Clipboard>())
}

/// Best-effort human text for a rejected clipboard promise (a `DOMException` carries `message`).
#[cfg(target_arch = "wasm32")]
fn js_error_text(e: &JsValue) -> String {
    if let Some(s) = e.as_string() {
        return s;
    }
    if let Ok(m) = js_sys::Reflect::get(e, &JsValue::from_str("message")) {
        if let Some(s) = m.as_string() {
            return s;
        }
    }
    format!("{e:?}")
}

#[cfg(test)]
#[path = "tests/clipboard.rs"]
mod tests;
