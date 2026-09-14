//! The hero image: picking a file, uploading it, and putting its address on the draft.
//!
//! **Role:** the upload route, the file picker and the multipart send behind the editor's hero
//! control, and the conversion of the returned address into an absolute one.
//! **Position:** behind one control in the editor form.
//! **Signals & state:** writes `thumbnail_url` on the draft being edited; nothing else.
//! **Invariants:** the address stored on the draft must be absolute, because the publish endpoint
//! refuses a relative one. The picker is created, clicked and left to fire on its own, so its
//! listener is kept alive past the call that made it.
#![allow(dead_code)]

use super::doc::Doc;
use crate::v2::core::auth::AuthStore;
use leptos::prelude::*;

/// The multipart upload route; the file is sent under the field name `file`.
pub(super) fn cms_uploads_path() -> &'static str {
    "/cms/uploads"
}

/// An uploaded file's address as an absolute one, so the publish payload is accepted.
///
/// An address that is already absolute passes through unchanged.
#[cfg(target_arch = "wasm32")]
pub(super) fn absolute_cms_upload_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default();
    if origin.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        format!("{origin}{trimmed}")
    } else {
        format!("{origin}/{trimmed}")
    }
}

/// Open a file picker, upload what is chosen, and store its address on the draft and its row.
pub(super) fn pick_and_upload_hero(
    store: AuthStore,
    doc_id: StoredValue<String>,
    docs: RwSignal<Vec<Doc>>,
    thumbnail_url: RwSignal<String>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (store, doc_id, docs, thumbnail_url);
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let toasts = crate::v2::core::ui::toast::use_toasts();
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            toasts.error("Hero image upload failed — no document");
            return;
        };
        let Ok(input) = document
            .create_element("input")
            .map_err(|_| ())
            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().map_err(|_| ()))
        else {
            toasts.error("Hero image upload failed — could not open file picker");
            return;
        };
        input.set_type("file");
        input.set_accept("image/jpeg,image/png,image/webp,.jpg,.jpeg,.png,.webp");

        let input_for_cb = input.clone();
        let on_change = Closure::once(move |_ev: web_sys::Event| {
            let Some(file) = input_for_cb.files().and_then(|list| list.item(0)) else {
                return;
            };
            leptos::task::spawn_local(async move {
                match crate::v2::core::api::client::api_upload_file::<serde_json::Value>(
                    store,
                    cms_uploads_path(),
                    file,
                )
                .await
                {
                    Ok(resp) => {
                        let raw = resp
                            .get("url")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        if raw.is_empty() {
                            toasts.error("Upload returned no url");
                            return;
                        }
                        let url = absolute_cms_upload_url(&raw);
                        thumbnail_url.set(url.clone());
                        let id = doc_id.get_value();
                        docs.update(|list| {
                            if let Some(doc) = list.iter_mut().find(|x| x.id == id) {
                                doc.thumbnail_url = url;
                            }
                        });
                        toasts.success("Hero image uploaded");
                    }
                    Err(e) => {
                        toasts.error(crate::v2::core::api::client::api_error_message(
                            &e,
                            "Hero upload failed",
                        ));
                    }
                }
            });
        });
        let _ =
            input.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
        // Keep the one-shot listener alive past this stack frame (picker is fire-and-forget).
        on_change.forget();
        input.click();
    }
}
