//! tbd-tools — async/heavy Rust tooling (T-165 Node eradication program).
//!
//! Modules land slice by slice (see `docs/platform/t165_node_eradication.md`):
//! - T-165.5: `cdp` (Chrome DevTools Protocol client over tokio-tungstenite), `serve`
//!   (static SPA server: COOP/COEP + fallback + apiProxy + mapAssetsDir), `inject`
//!   (the verbatim browser-injected FREEZE/DOM-serializer JS — NEVER re-implemented
//!   natively; byte-parity of the frozen V goldens depends on injecting the identical
//!   strings), `diff_node` (the structural DOM tree diff).
//! - T-165.7: the MCP broker (`mcpd` bin).
//! - T-165.8/.9: `world` + `map` pipeline modules.

pub mod cdp;
pub mod density;
pub mod forest;
pub mod geometry;
pub mod inject;
pub mod serve;
pub mod smokes;
pub mod sroutes;
pub mod vsuite;

pub const PROGRAM: &str = "T-165";
