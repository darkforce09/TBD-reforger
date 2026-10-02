//! The scheme guard on every link the platform stores or renders.
//!
//! **Role:** answers whether a string names an absolute `http` or `https` URL that a browser
//! follows rather than executes ([`is_http_url`]), and holds the table of inputs and verdicts
//! that pins the answer (`cases`, compiled for tests and the `test_fixtures` feature).
//! **Position:** foundation tier, depending on `url` only. The API calls the guard before it
//! stores a URL; the single-page app calls it before it renders one as a link or an image.
//! **Signals & state:** none; one pure function and constant data.
//! **Invariants:** both boundaries call this one predicate, so what the API accepts and what the
//! page renders cannot drift apart; every entry of the case table holds for it.

mod http_url;
pub mod prelude;

#[cfg(any(test, feature = "test_fixtures"))]
pub mod cases;

pub use http_url::is_http_url;
