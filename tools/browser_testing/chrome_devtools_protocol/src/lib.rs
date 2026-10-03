//! The Chrome DevTools Protocol client the headless browser gates drive Chromium with.
//!
//! **Role:** finds a Chromium build ([`find_chromium`]), starts it headless in its own process
//! group with the gate's GPU flags and font cache ([`launch`], [`launch_with_gpu`]), opens pages
//! over one WebSocket each ([`new_page`]) and speaks the protocol on them ([`Page`]): calls,
//! events, navigation, evaluation, input, screenshots and request interception.
//! **Position:** tier 1 of `tools/browser_testing`, over `newtype_ids`; the suites of
//! `browser_gate_suites` drive every browser through it.
//! **Signals & state:** a [`Browser`] owns the Chromium process group, its profile folder and the
//! tail of its output; a [`Page`] owns its socket reader task, the pending calls and the event
//! waiters; the gate font cache folder is decided once per process ([`resolved_font_cache`]).
//! **Invariants:** both output pipes of Chromium are drained from spawn; the viewport and the init
//! scripts are applied before a page's first navigation; every launch gets its own profile
//! folder; shutting a browser down signals its whole process group.

mod browser_session;
mod chromium_discovery;
mod error;
mod gate_font_cache;
mod intercepted_request;
pub mod prelude;

pub use browser_session::{
    Browser, GpuBackend, Page, VIEWPORT, launch, launch_with_gpu, new_page, sleep_ms, wait_http,
};
pub use chromium_discovery::{find_chromium, is_headless_shell};
pub use error::{Error, Result};
pub use gate_font_cache::{
    CacheOrigin, GATE_FONT_CACHE_ENV, gate_font_cache_dir, resolved_font_cache,
};
pub use intercepted_request::InterceptedRequestId;
