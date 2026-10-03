//! The browser-oracle verify and accept gate.
//!
//! Captures the normalized DOM — through the serializer `fixture_injection` injects — plus a PNG
//! for every leaf route, and diffs both against the frozen goldens under
//! `tools/browser_testing/browser_gate_suites/fixtures/dom_oracle/oracle-freeze/`. `verify` is the regression gate;
//! `accept` re-sources ONE route's golden from the current `apps/frontend/dist` with a
//! recorded note. There is no whole-tree re-freeze: the goldens are not regenerable from any dist
//! this repository still builds, so a bulk overwrite would destroy the oracle it exists to check.
//!
//! Readiness = the injected clock freeze plus fixture-intercepted fetches, then a stability loop:
//! serialize until two consecutive captures are byte-identical. Viewport pinned 1440×900.
//! Exit 0 = all routes green; 1 = any diff/missing; 3 = driver error (mapped in the bin).
//!
//! **Role:** the DOM oracle's types (`Route`, `Capture`, `VSuiteArgs`), the committed seed ids and
//! the accept size floor; re-exports its entries.
//! **Position:** `gate v-suite verify|accept`; its children hold the routes, the request router and
//! the run modes.
//! **Signals & state:** none at this level.
//! **Invariants:** an accepted DOM is valid JSON above the size floor; verify compares the
//! normalised DOM byte for byte.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::{Result, ResultExt, refusal};
use serde_json::{Value, json};

use crate::fixture_injection::{DOM_SERIALIZER_SRC, FREEZE_SRC};
use crate::server::{ServeConfig, start_server};
use ::repository_layout::find_repository_root;
use chrome_devtools_protocol::{self as cdp, Browser};

// The committed seed golden ids (memory/fixtures): mission / event / event-mission.
const MISSION: &str = "512d8658-7025-4a70-94e9-a1b44a7aa155";
const EVENT: &str = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";
const EM: &str = "89b1b731-37a8-4926-901a-3c7ff7de5eb3";

/// One route of the DOM oracle: its golden's slug, the path the browser opens, and whether the
/// page loads with the admin session seeded.
pub struct Route {
    /// The golden's file stem under `fixtures/dom_oracle/oracle-freeze/`.
    pub slug: &'static str,
    /// The app path the browser navigates to.
    pub path: String,
    /// Whether the admin session is seeded before the page boots.
    pub authed: bool,
}

/// Floor on accept-mode DOM size (`js_len` / manifest `bytes`). Committed goldens are
/// ≥ ~3.4 KB (`callback.dom.json`); the SPA-failure sentinel the serializer returns is the
/// 4-char literal `"null"`. Floor sits well below any real page and well above that stub.
pub const MIN_ACCEPT_DOM_JS_LEN: usize = 256;

const SETTLE: &str = "(async()=>{await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()";

/// One route's capture: its serialised DOM and its screenshot.
pub struct Capture {
    /// The normalised DOM as `window.__domOracleSerialize` returned it.
    pub dom: String,
    /// The page's PNG screenshot.
    pub png: Vec<u8>,
}

/// The arguments of `gate v-suite`.
pub struct VSuiteArgs {
    /// `verify` (compare against the goldens) or `accept` (replace them).
    pub mode: String,
    /// The single-page app folder whose `dist/` the gate serves.
    pub leptos_dir: PathBuf,
    /// One route slug to run alone; empty runs every route.
    pub only: String,
    /// Why a golden is accepted, recorded in the manifest.
    pub note: String,
}

#[cfg(test)]
#[path = "tests/dom_oracle/tests.rs"]
mod tests;

#[path = "dom_oracle/routes.rs"]
mod routes;
pub use routes::MissingFixture;
pub use routes::capture_route;
pub use routes::diff_node;
pub use routes::js_len;
pub use routes::routes;
pub use routes::run;
pub(crate) use routes::seed_script;
use routes::sha_hex;
pub use routes::validate_accept_dom;

#[path = "dom_oracle/run_modes.rs"]
mod run_modes;
use run_modes::run_modes;
