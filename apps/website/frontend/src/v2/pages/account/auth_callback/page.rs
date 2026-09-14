//! The page the sign-in redirect lands on: read the fragment, mint the session, move on.
//!
//! **Role:** completes the sign-in round trip. Reads the token fragment the backend redirected
//! with, fetches the profile, hands the result to the session store, and navigates onward.
//! **Position:** the redirect target of the sign-in flow. It renders only a status line — nothing
//! here is a destination a person navigates to on purpose.
//! **Signals & state:** writes the session store; keeps a local error signal for the failure copy.
//! Clears the fragment from the address bar as soon as it has been read.
//! **Invariants:** the fragment carries credentials, so it is scrubbed from history rather than
//! merely navigated away from — a back button must not be able to replay it. The parse runs once
//! at mount; a second run would find the fragment already gone.

use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::v2::core::auth::persist;
use crate::v2::core::auth::AuthStore;
#[cfg(target_arch = "wasm32")]
use crate::v2::core::auth::{RefreshResponse, Session};

/// The message shown for each failure code the backend can redirect with.
fn auth_error_copy(code: &str) -> &'static str {
    match code {
        "missing_code" => "Discord did not return an authorization code. Please try again.",
        "invalid_state" => "The sign-in request expired or was tampered with. Please try again.",
        "discord_unreachable" => "Could not reach Discord. Please try again in a moment.",
        "banned" => "This account is banned from the platform.",
        "oauth_unconfigured" => {
            "Discord sign-in is not configured on this server. Contact an administrator."
        }
        "no_session" => "No sign-in details were found. Please start from the login page.",
        _ => "Something went wrong completing sign-in. Please try again.",
    }
}

/// Read the token fragment, or the error code the backend redirected with instead.
///
/// Returns the pair and whether a game account is linked, or the message to show.
#[cfg(target_arch = "wasm32")]
fn parse_callback_hash() -> Result<(RefreshResponse, bool), String> {
    let win = web_sys::window().ok_or_else(|| auth_error_copy("no_session").to_string())?;
    let hash = win.location().hash().unwrap_or_default();
    let params: Vec<(String, String)> = hash
        .trim_start_matches('#')
        .split('&')
        .filter_map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = it.next()?;
            let v = it.next().unwrap_or("");
            Some((
                js_sys::decode_uri_component(k)
                    .ok()
                    .map(|s| s.into())
                    .unwrap_or_else(|| k.to_string()),
                js_sys::decode_uri_component(v)
                    .ok()
                    .map(|s| s.into())
                    .unwrap_or_else(|| v.to_string()),
            ))
        })
        .collect();
    let get = |key: &str| {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    if let Some(code) = get("error") {
        return Err(auth_error_copy(&code).to_string());
    }
    let access = get("access_token").filter(|s| !s.is_empty());
    let refresh = get("refresh_token").filter(|s| !s.is_empty());
    let expires = get("expires_at").filter(|s| !s.is_empty());
    match (access, refresh, expires) {
        (Some(access_token), Some(refresh_token), Some(expires_at)) => Ok((
            RefreshResponse {
                access_token,
                refresh_token,
                expires_at,
            },
            get("arma_linked").as_deref() == Some("true"),
        )),
        _ => Err(auth_error_copy("no_session").to_string()),
    }
}

/// Remove the fragment from the address bar without adding a history entry.
///
/// Replacing the entry rather than pushing one is the point: the fragment carries credentials, and
/// a back button must not be able to return to them.
#[cfg(target_arch = "wasm32")]
fn scrub_callback_hash() {
    if let Some(win) = web_sys::window() {
        let path = win
            .location()
            .pathname()
            .unwrap_or_else(|_| "/auth/callback".into());
        let _ = win.history().ok().and_then(|h| {
            h.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path))
                .ok()
        });
    }
}

/// The sign-in landing page.
///
/// Renders a single status line — "Completing sign in…", or the failure copy. On mount it reads the
/// fragment, scrubs it from the address bar, fetches the profile with the new access token, hands
/// the session to the store, persists it, and navigates to the destination the sign-in started
/// from. Mounted by the router at the redirect path; it has no parent slot and no props.
#[component]
pub fn AuthCallbackPage() -> impl IntoView {
    let store = expect_context::<AuthStore>();
    let error = RwSignal::new(Option::<String>::None);
    let busy = RwSignal::new(true);

    #[cfg(target_arch = "wasm32")]
    {
        match parse_callback_hash() {
            Err(msg) => {
                scrub_callback_hash();
                error.set(Some(msg));
                busy.set(false);
            }
            Ok((tokens, arma_fallback)) => {
                scrub_callback_hash();
                store.set_tokens(tokens.clone());
                persist(&store.persist_state());
                leptos::task::spawn_local(async move {
                    match crate::v2::core::api::client::api_get::<
                        crate::v2::core::api::dto::MeResponse,
                    >(store, "/me")
                    .await
                    {
                        Ok(me) => {
                            store.set_session(Session {
                                access_token: store
                                    .access_token
                                    .get_untracked()
                                    .unwrap_or_default(),
                                refresh_token: store
                                    .refresh_token
                                    .get_untracked()
                                    .unwrap_or_default(),
                                expires_at: store.expires_at.get_untracked().unwrap_or_default(),
                                user: me.user,
                                arma_linked: me.arma_linked || arma_fallback,
                            });
                            persist(&store.persist_state());
                            if let Some(win) = web_sys::window() {
                                let _ = win.location().set_href("/");
                            }
                        }
                        Err(_) => {
                            // Keep the minted tokens: a reload can still bootstrap from them.
                            error.set(Some(auth_error_copy("server_error").to_string()));
                            busy.set(false);
                        }
                    }
                });
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = store;
        busy.set(false);
        error.set(Some(auth_error_copy("no_session").to_string()));
    }

    view! {
        <div class="flex min-h-screen items-center justify-center bg-background p-6">
            <div class="max-w-md rounded-xl border border-border-subtle bg-surface-container p-8 text-center">
                <Show when=move || error.get().is_some()>
                    <h1 class="text-xl font-semibold text-error">"Sign-in failed"</h1>
                    <p class="mt-2 text-sm text-on-surface-variant">
                        {move || error.get().unwrap_or_default()}
                    </p>
                    <a href="/login" class="mt-4 inline-block text-primary hover:underline">
                        "Back to login"
                    </a>
                </Show>
                <Show when=move || busy.get() && error.get().is_none()>
                    <h1 class="text-xl font-semibold text-on-surface">"Completing sign-in…"</h1>
                    <p class="mt-2 text-sm text-on-surface-variant">
                        "Establishing your session."
                    </p>
                </Show>
            </div>
        </div>
    }
}
