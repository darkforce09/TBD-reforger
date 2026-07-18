//! Shared HTTP helpers for the map-asset host (full GET + Range).

/// Soft-fail byte GET (same-origin `/map-assets`).
pub async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.binary().await.ok()
}

pub async fn fetch_text(url: &str) -> Option<String> {
    let resp = gloo_net::http::Request::get(url).send().await.ok()?;
    if !(200..300).contains(&resp.status()) {
        return None;
    }
    resp.text().await.ok()
}

pub struct RangeBody {
    pub bytes: Vec<u8>,
    pub total: u64,
}

/// HTTP Range GET. Succeeds only on **206**; a 200 (server ignoring Range) is rejected so CI
/// never silently downloads the full 152_713_114 B sat bundle.
pub async fn fetch_range(url: &str, start: u64, end_inclusive: u64) -> Option<RangeBody> {
    let resp = gloo_net::http::Request::get(url)
        .header("Range", &format!("bytes={start}-{end_inclusive}"))
        .send()
        .await
        .ok()?;
    if resp.status() != 206 {
        return None;
    }
    let total = resp
        .headers()
        .get("content-range")
        .and_then(|cr| cr.split('/').nth(1)?.parse::<u64>().ok())
        .filter(|&t| t > 0)?;
    let bytes = resp.binary().await.ok()?;
    Some(RangeBody { bytes, total })
}
