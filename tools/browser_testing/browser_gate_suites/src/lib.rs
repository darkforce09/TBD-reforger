//! The headless browser gates of the single-page app.
//!
//! **Role:** the static server the gates load the built app from ([`server`]), the DOM oracle
//! that freezes and verifies every route's rendered DOM ([`dom_oracle`]), the route drift check
//! ([`route_drift`]), the Mission Creator smokes and the render check ([`editor_smoke_tests`]),
//! the data viewer gate ([`equipment_data_viewer`]), the fire-mission solver's browser against
//! native agreement ([`ballistics_agreement`]), the mortar calculator's offline pack
//! ([`mortar_offline`]), the screen capture harness ([`screen_capture`]), the gate doctor
//! ([`diagnostics`]), the session tokens, fixture scripts and repository locations they share,
//! and the `gate` and `capture` command lines ([`command_lines`]).
//! **Position:** tier 5 of `tools/browser_testing`, over `chrome_devtools_protocol` (every
//! browser), `repository_layout` (the checkout and its shared locations), `process_runner`,
//! `content_digest` and the ballistics crates (`ballistics_model`, `fire_mission_planning`,
//! `ballistics_agreement_cases`) the two ballistics gates solve with. The `gate` and `capture`
//! binaries of `developer_tools` call [`command_lines`]; `cargo xtask mk leptos-gates` runs them.
//! **Signals & state:** each gate owns its tokio tasks, its browser and its server for one run;
//! the server shares one read-only configuration across its connections.
//! **Invariants:** a gate's verdict is its returned exit code (0 green, 1 gate fail); an error
//! means the gate could not run; the committed fixtures under `fixtures/dom_oracle/` change only
//! through `gate v-suite accept`.

pub mod ballistics_agreement;
pub mod command_lines;
pub mod diagnostics;
pub mod dom_oracle;
pub mod editor_smoke_tests;
pub mod equipment_data_viewer;
mod error;
pub mod fixture_injection;
pub mod gate_layout;
pub mod mortar_offline;
pub mod prelude;
pub mod route_drift;
pub mod screen_capture;
pub mod server;
pub mod session_tokens;

pub use error::{Error, Result};
