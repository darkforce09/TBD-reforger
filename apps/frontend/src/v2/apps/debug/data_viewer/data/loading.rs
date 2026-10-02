//! Superseded reads are aborted and cannot overwrite a newer panel location.
use super::request_cache;
use crate::v2::core::api::client::public_reads::public_get;
use leptos::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

/// The reactive result of one viewer read: the decoded response, the last error message, and
/// whether a request is outstanding while there is no value to show.
#[derive(Clone, Copy)]
pub struct ReadState<T: Send + Sync + 'static> {
    pub value: RwSignal<Option<T>>,
    pub error: RwSignal<Option<String>>,
    pub loading: RwSignal<bool>,
}
/// Reads `url` each time it changes: clears the previous result, aborts the superseded request,
/// serves a cached response when one exists and fetches otherwise. An empty URL issues no
/// request.
pub fn use_read<T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static>(
    url: Memo<String>,
) -> ReadState<T> {
    read_with_refresh_policy(url, false)
}

/// Polls retain the last successful value and let any in-flight check finish.
pub fn use_polled_read<T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static>(
    url: Memo<String>,
) -> ReadState<T> {
    read_with_refresh_policy(url, true)
}

fn read_with_refresh_policy<T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static>(
    url: Memo<String>,
    retain_previous: bool,
) -> ReadState<T> {
    let value = RwSignal::new(None);
    let error = RwSignal::new(None);
    let loading = RwSignal::new(false);
    let controller = StoredValue::new_local(None::<web_sys::AbortController>);
    let pending = StoredValue::new(false);
    let sequence = RwSignal::new(0u64);
    Effect::new(move |_| {
        let url = url.get();
        if retain_previous && pending.get_value() {
            return;
        }
        sequence.update(|v| *v += 1);
        let ticket = sequence.get_untracked();
        controller.update_value(|old| {
            if let Some(old) = old.take() {
                old.abort();
            }
        });
        if !retain_previous {
            value.set(None);
            error.set(None);
        }
        loading.set(!url.is_empty() && value.get_untracked().is_none());
        if url.is_empty() {
            return;
        }
        if let Some(cached) =
            request_cache::get(&url).and_then(|s| serde_json::from_str::<T>(&s).ok())
        {
            value.set(Some(cached));
            loading.set(false);
            return;
        }
        let Ok(abort) = web_sys::AbortController::new() else {
            error.set(Some("Cannot create cancellable request".into()));
            loading.set(false);
            return;
        };
        controller.set_value(Some(abort.clone()));
        pending.set_value(true);
        leptos::task::spawn_local(async move {
            let result = public_get::<T>(&url, &abort.signal()).await;
            if abort.signal().aborted() || sequence.try_get_untracked() != Some(ticket) {
                return;
            }
            match result {
                Ok(result) => {
                    if let Ok(encoded) = serde_json::to_string(&result) {
                        request_cache::put(url, encoded);
                    }
                    value.set(Some(result));
                    error.set(None);
                }
                Err(message) => error.set(Some(message)),
            }
            pending.set_value(false);
            loading.set(false);
        });
    });
    on_cleanup(move || {
        controller.update_value(|old| {
            if let Some(old) = old.take() {
                old.abort();
            }
        })
    });
    ReadState {
        value,
        error,
        loading,
    }
}
