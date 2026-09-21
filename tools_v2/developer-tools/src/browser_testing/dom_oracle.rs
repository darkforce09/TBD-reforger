//! The browser-oracle verify and accept gate.
//!
//! Captures the normalized DOM — through the serializer `fixture_injection` injects — plus a PNG
//! for every leaf route, and diffs both against the frozen goldens under
//! `tools_v2/developer-tools/fixtures/dom_oracle/oracle-freeze/`. `verify` is the regression gate;
//! `accept` re-sources ONE route's golden from the current `apps/website/frontend/dist` with a
//! recorded note. There is no whole-tree re-freeze: the goldens are not regenerable from any dist
//! this repository still builds, so a bulk overwrite would destroy the oracle it exists to check.
//!
//! Readiness = the injected clock freeze plus fixture-intercepted fetches, then a stability loop:
//! serialize until two consecutive captures are byte-identical. Viewport pinned 1440×900.
//! Exit 0 = all routes green; 1 = any diff/missing; 3 = driver error (mapped in the bin).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::browser_testing::cdp::{self, Browser};
use crate::browser_testing::fixture_injection::{DOM_SERIALIZER_SRC, FREEZE_SRC};
use crate::browser_testing::server::{ServeConfig, repo_root, start_server};

// The committed seed golden ids (memory/fixtures): mission / event / event-mission.
const MISSION: &str = "512d8658-7025-4a70-94e9-a1b44a7aa155";
const EVENT: &str = "c71a4d1a-a616-4b88-ba7a-fccbc5ca26b7";
const EM: &str = "89b1b731-37a8-4926-901a-3c7ff7de5eb3";

pub struct Route {
    pub slug: &'static str,
    pub path: String,
    pub authed: bool,
}

/// Floor on accept-mode DOM size (`js_len` / manifest `bytes`). Committed goldens are
/// ≥ ~3.4 KB (`callback.dom.json`); the SPA-failure sentinel the serializer returns is the
/// 4-char literal `"null"`. Floor sits well below any real page and well above that stub.
pub const MIN_ACCEPT_DOM_JS_LEN: usize = 256;

const SETTLE: &str = "(async()=>{await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));return true})()";

pub struct Capture {
    pub dom: String,
    pub png: Vec<u8>,
}

pub struct VSuiteArgs {
    pub mode: String,
    pub leptos_dir: PathBuf,
    pub only: String,
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
