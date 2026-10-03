//! The pinned `enfusion-mcp` npm package.
//!
//! **Role:** the folder holding the npm manifest, lockfile and node version that pin the
//! `enfusion-mcp` server this repository runs, and the server module `npm ci` installs there.
//! **Position:** `xtask mod dev-bootstrap` runs `npm ci` in the folder; the `developer_tools`
//! entry-point resolver and the `xtask mcp` commands start the installed module.
//! **Signals & state:** none; constants and pure path joins.
//! **Invariants:** the entry point lies inside the package folder's `node_modules`; the package
//! folder sits outside every crate root, so no crate-scoped file walk reaches the installed tree.

use std::path::{Path, PathBuf};

/// Directory holding the npm manifest, lockfile and node version that pin the `enfusion-mcp`
/// server this repository runs. It sits outside every crate root so that the installed
/// dependency tree beside it is never walked by a crate-scoped file scan.
pub const ENFUSION_MCP_NODE_PACKAGE_DIR: &str = "tools/enfusion_mcp_node_package";

/// The `enfusion-mcp` server module installed by `npm ci` in that package directory, the one
/// spelling of that path in the workspace: the entry-point resolver derives every caller's runner
/// command from it, and the process pattern that identifies a running server.
pub const ENFUSION_MCP_ENTRYPOINT: &str =
    "tools/enfusion_mcp_node_package/node_modules/enfusion-mcp/dist/index.js";

/// Absolute path of the pinned `enfusion-mcp` server module inside a checkout.
pub fn enfusion_mcp_entrypoint(root: &Path) -> PathBuf {
    root.join(ENFUSION_MCP_ENTRYPOINT)
}

/// Absolute path of the npm package directory inside a checkout, where
/// `cargo xtask mod dev-bootstrap` runs `npm ci`.
pub fn enfusion_mcp_node_package_dir(root: &Path) -> PathBuf {
    root.join(ENFUSION_MCP_NODE_PACKAGE_DIR)
}

#[cfg(test)]
#[path = "tests/enfusion_mcp_node_package_tests.rs"]
mod tests;
