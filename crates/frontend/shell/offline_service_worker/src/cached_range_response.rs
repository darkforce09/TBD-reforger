//! `Range` requests answered from a cached full body.
//!
//! **Role:** turns a cached `200` response and the request's `Range` header into the full
//! response, a `206 Partial Content` slice or a `416 Range Not Satisfiable`, by the plan of
//! [`crate::range_slicing::plan_range_response`].
//! **Position:** called by `fetch_handling` for map assets found in the cache; the renderer's
//! `Range` reads of the satellite mosaic land here offline.
//! **Signals & state:** none; builds new `Response` objects.
//! **Invariants:** the slice comes from `Blob.slice` over the cached body, so the body is never
//! copied into memory whole; the `206` keeps every cached header and replaces only
//! `Content-Range` and `Content-Length`.

use crate::range_slicing::{
    RangeResponsePlan, STATUS_PARTIAL_CONTENT, STATUS_RANGE_NOT_SATISFIABLE, plan_range_response,
    unsatisfied_content_range,
};
use wasm_bindgen::JsValue;
use web_sys::{Blob, Headers, Response, ResponseInit};

use crate::worker_scope::settle;

/// Answers the `Range` header `range` from the cached full response `cached`.
pub(crate) async fn answer_from_cached_body(
    cached: Response,
    range: &str,
) -> Result<Response, JsValue> {
    let headers = Headers::new_with_headers(&cached.headers())?;
    let body: Blob = settle(cached.blob()?).await?;
    // `Blob.size` is an exact integer below 2^53, the largest body a browser holds.
    let total_len = body.size() as u64;
    match plan_range_response(Some(range), total_len) {
        RangeResponsePlan::FullBody => {
            let init = ResponseInit::new();
            init.set_status(cached.status());
            init.set_status_text(&cached.status_text());
            init.set_headers_headers(&headers);
            Response::new_with_opt_blob_and_init(Some(&body), &init)
        }
        RangeResponsePlan::Partial(slice) => {
            let part =
                body.slice_with_f64_and_f64(slice.start as f64, slice.end_exclusive() as f64)?;
            headers.set("content-range", &slice.content_range())?;
            headers.set("content-length", &slice.len().to_string())?;
            let init = ResponseInit::new();
            init.set_status(STATUS_PARTIAL_CONTENT);
            init.set_status_text("Partial Content");
            init.set_headers_headers(&headers);
            Response::new_with_opt_blob_and_init(Some(&part), &init)
        }
        RangeResponsePlan::Unsatisfiable { total_len } => {
            headers.set("content-range", &unsatisfied_content_range(total_len))?;
            headers.set("content-length", "0")?;
            let init = ResponseInit::new();
            init.set_status(STATUS_RANGE_NOT_SATISFIABLE);
            init.set_status_text("Range Not Satisfiable");
            init.set_headers_headers(&headers);
            Response::new_with_opt_blob_and_init(None, &init)
        }
    }
}
