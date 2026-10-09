//! The HTTP(S) scheme guard at each writer of a stored link: a non-HTTP value is refused with
//! 400 and the stored row is left exactly as it was.
//!
//! The `http_url_guard` crate's unit tests prove the predicate over its case table; these prove
//! the wiring, one writer per module.

mod announcement_thumbnail;
mod event_banner;
mod mission_thumbnail;
mod telemetry_replay_link;
