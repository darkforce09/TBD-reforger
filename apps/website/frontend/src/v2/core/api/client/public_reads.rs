//! Anonymous GET requests never read or mutate authentication state.
use serde::de::DeserializeOwned;

thread_local! { static ACTIVE:std::cell::Cell<usize>=const{std::cell::Cell::new(0)}; }
struct RequestSlot;
impl Drop for RequestSlot {
    fn drop(&mut self) {
        ACTIVE.with(|n| n.set(n.get().saturating_sub(1)));
    }
}
async fn acquire(signal: &web_sys::AbortSignal) -> Result<RequestSlot, String> {
    loop {
        if signal.aborted() {
            return Err("Request cancelled".into());
        }
        if ACTIVE.with(|n| {
            if n.get() < 4 {
                n.set(n.get() + 1);
                true
            } else {
                false
            }
        }) {
            return Ok(RequestSlot);
        }
        gloo_timers::future::TimeoutFuture::new(25).await;
    }
}

pub async fn public_get<T: DeserializeOwned>(
    path: &str,
    signal: &web_sys::AbortSignal,
) -> Result<T, String> {
    if !path.starts_with('/') || path.starts_with("//") {
        return Err("Invalid public API path".into());
    }
    let _slot = acquire(signal).await?;
    for attempt in 0..3 {
        if signal.aborted() {
            return Err("Request cancelled".into());
        }
        let response = gloo_net::http::Request::get(&format!("{}{path}", super::API_BASE))
            .credentials(web_sys::RequestCredentials::Omit)
            .abort_signal(Some(signal))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if response.status() == 429 && attempt < 2 {
            let wait = response
                .headers()
                .get("Retry-After")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(2)
                .min(60);
            gloo_timers::future::TimeoutFuture::new(wait * 1000).await;
            continue;
        }
        if !response.ok() {
            let body = response.json::<serde_json::Value>().await.ok();
            return Err(body
                .as_ref()
                .and_then(super::error_body_message)
                .unwrap_or_else(|| format!("Request failed ({})", response.status())));
        }
        return response.json().await.map_err(|e| e.to_string());
    }
    Err("Rate limited; try again shortly".into())
}
