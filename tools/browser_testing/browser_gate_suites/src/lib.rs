//! The headless browser gates of the single-page app.
//!
//! **Role:** the static server the gates load the built app from ([`server`]), the DOM oracle
//! that freezes and verifies every route's rendered DOM ([`dom_oracle`]), the route drift check
//! ([`route_drift`]), the Mission Creator smokes and the render check ([`editor_smoke_tests`]),
//! the data viewer gate ([`equipment_data_viewer`]), the screen capture harness
//! ([`screen_capture`]), the gate doctor ([`diagnostics`]), and the session tokens, fixture
//! scripts and repository locations they share.
//! **Position:** tier 2 of `tools/browser_testing`, over `chrome_devtools_protocol` (every
//! browser), `repository_layout` (the checkout and its shared locations), `process_runner` and
//! `content_digest`. The `gate` and `capture` command lines of `developer_tools` call it;
//! `cargo xtask mk leptos-gates` runs those binaries.
//! **Signals & state:** each gate owns its tokio tasks, its browser and its server for one run;
//! the server shares one read-only configuration across its connections.
//! **Invariants:** a gate's verdict is its returned exit code (0 green, 1 gate fail); an error
//! means the gate could not run; the committed fixtures under `fixtures/dom_oracle/` change only
//! through `gate v-suite accept`.

pub mod diagnostics;
pub mod dom_oracle;
pub mod editor_smoke_tests;
pub mod equipment_data_viewer;
mod error;
pub mod fixture_injection;
pub mod gate_layout;
pub mod prelude;
pub mod route_drift;
pub mod screen_capture;
pub mod server;
pub mod session_tokens;

pub use error::{Error, Result};
