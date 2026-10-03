//! The id the `Fetch` domain gives a request it paused.
//!
//! **Role:** [`InterceptedRequestId`], the typed `requestId` of a `Fetch.requestPaused` event
//! that [`crate::Page::fulfill_raw`], [`crate::Page::fulfill_json`] and
//! [`crate::Page::continue_request`] answer.
//! **Position:** read by the gates from each paused-request event and handed back to the page
//! that paused it.
//! **Signals & state:** none; a value type.
//! **Invariants:** serialises as the bare string Chromium sent, so the protocol messages carry
//! exactly the bytes they did as a plain string.

newtype_ids::string_id! {
    /// The `requestId` Chromium gives a request paused by the `Fetch` domain; valid only on the
    /// page that paused it, until that request is fulfilled or continued.
    pub struct InterceptedRequestId;
}
