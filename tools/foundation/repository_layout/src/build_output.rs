//! The build output folder and the purpose subfolders the tools build into.
//!
//! **Role:** the name of the one gitignored folder at a checkout root that holds all build
//! output, the toolchain environment folder inside it (one per glibc a machine builds with), the
//! name of each tool's purpose subfolder inside that, the formula that joins them, and the
//! root-level folder names no tool writes any more.
//! **Position:** `xtask` pins the shared cargo target folder and builds its `mk` recipes, its wave
//! gate steps, its MCP daemon and its database selftest into these subfolders, and its reclaim
//! sweeps delete the retired names; `developer_tools` writes its mesh dumps under the folder.
//! **Signals & state:** none; constants and pure path joins.
//! **Invariants:** all build output lives under one `target/` folder, split by
//! [`ToolchainEnvironment`] so binaries linked against two glibcs never share a folder. Each
//! purpose subfolder is its
//! own `CARGO_TARGET_DIR` (or trunk dist folder, or compose project) and so holds its own cargo
//! lock, and no subfolder name is an entry cargo writes inside a target directory (profile
//! folders, `build`, `doc`, `package`, `tmp`, target triples, its bookkeeping files), so nesting
//! shares no file with the shared cache. Every name is one plain path component.

use std::path::{Path, PathBuf};

/// The one gitignored folder, relative to a checkout root, that holds all build output. Under the
/// primary checkout it is also the shared cargo cache every worktree builds into.
pub const BUILD_OUTPUT_FOLDER: &str = "target";

/// Where a build runs on a development machine: on the host, or inside the development container,
/// whose older glibc must never share a cargo target folder with the host's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolchainEnvironment {
    /// Cargo run on the host (the `hcargo` wrapper from inside the container).
    Host,
    /// Cargo run inside the development container.
    Container,
}

impl ToolchainEnvironment {
    /// The environment of a process: [`Self::Container`] when `in_container`.
    pub fn from_container_flag(in_container: bool) -> Self {
        if in_container {
            Self::Container
        } else {
            Self::Host
        }
    }

    /// The environment's folder name under [`BUILD_OUTPUT_FOLDER`].
    pub fn folder_name(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Container => "container",
        }
    }
}

/// `<checkout_root>/target/<environment>`: the shared cargo target folder of one toolchain
/// environment, which every worktree builds into.
pub fn toolchain_build_folder(checkout_root: &Path, environment: ToolchainEnvironment) -> PathBuf {
    checkout_root
        .join(BUILD_OUTPUT_FOLDER)
        .join(environment.folder_name())
}

/// The development API's private `CARGO_TARGET_DIR` (`cargo xtask mk rust-api`), under the
/// current checkout rather than the primary one.
pub const DEV_API_SUBFOLDER: &str = "dev-api";
/// The wave gate's trunk `CARGO_TARGET_DIR`; `TBD_GATE_TRUNK_TARGET` overrides it.
pub const GATE_TRUNK_SUBFOLDER: &str = "gate-trunk";
/// The wave gate's trunk dist folder; `TBD_GATE_TRUNK_DIST` overrides it.
pub const GATE_FRONTEND_DIST_SUBFOLDER: &str = "gate-dist-frontend";
/// The wave gate's `cargo check` and clippy `CARGO_TARGET_DIR`; `TBD_GATE_CHECK_TARGET`
/// overrides it.
pub const GATE_CHECK_SUBFOLDER: &str = "gate-check";
/// The wave gate's schema step `CARGO_TARGET_DIR`; `TBD_GATE_SCHEMA_TARGET` overrides it.
pub const GATE_SCHEMA_SUBFOLDER: &str = "gate-schema";
/// The wave gate's `api` test `CARGO_TARGET_DIR`.
pub const GATE_API_SUBFOLDER: &str = "gate-api";
/// The wave gate's `frontend` test `CARGO_TARGET_DIR`.
pub const GATE_FRONTEND_SUBFOLDER: &str = "gate-frontend";
/// The wave gate's `xtask` and `developer_tools` test `CARGO_TARGET_DIR`.
pub const GATE_TOOLS_SUBFOLDER: &str = "gate-tools";
/// A slice gate's private frontend test `CARGO_TARGET_DIR` is this prefix followed by the slice id.
pub const GATE_SLICE_FRONTEND_PREFIX: &str = "gate-slice-frontend-";
/// The continuous-integration scratch `CARGO_TARGET_DIR` that `mk reclaim-target-ci` deletes.
pub const CONTINUOUS_INTEGRATION_SUBFOLDER: &str = "ci";
/// The `mcpd` broker's private `CARGO_TARGET_DIR` (`cargo xtask mcp daemon start`), so a wave
/// gate never rewrites the running daemon's binary; `MCPD_CARGO_TARGET_DIR` overrides it.
pub const MCP_DAEMON_SUBFOLDER: &str = "dev-mcpd";
/// The throwaway compose project of `cargo xtask db selftest`'s compose-parity arm.
pub const DATABASE_SELFTEST_SUBFOLDER: &str = "db-selftest";
/// The prefix every wave-gate subfolder shares; `platform wave reclaim --gate-dirs` sweeps the
/// subfolders that carry it.
pub const GATE_SUBFOLDER_PREFIX: &str = "gate-";
/// The wave driver's run lane: the one extra target directory inside the shared cache, named for
/// its owner, the main checkout.
pub const RUN_TARGET_SUBDIR: &str = "run-main";

/// Every fixed purpose subfolder name under [`BUILD_OUTPUT_FOLDER`], for the proof that none
/// collides with an entry cargo writes there itself.
pub const PURPOSE_SUBFOLDERS: &[&str] = &[
    DEV_API_SUBFOLDER,
    GATE_TRUNK_SUBFOLDER,
    GATE_FRONTEND_DIST_SUBFOLDER,
    GATE_CHECK_SUBFOLDER,
    GATE_SCHEMA_SUBFOLDER,
    GATE_API_SUBFOLDER,
    GATE_FRONTEND_SUBFOLDER,
    GATE_TOOLS_SUBFOLDER,
    CONTINUOUS_INTEGRATION_SUBFOLDER,
    MCP_DAEMON_SUBFOLDER,
    DATABASE_SELFTEST_SUBFOLDER,
];

/// `<checkout_root>/target/<environment>/<subfolder>`: the one formula every tool names its build
/// output with.
pub fn build_output_subfolder(
    checkout_root: &Path,
    environment: ToolchainEnvironment,
    subfolder: &str,
) -> PathBuf {
    toolchain_build_folder(checkout_root, environment).join(subfolder)
}

/// Root-level folder names beside [`BUILD_OUTPUT_FOLDER`] that no tool writes; a machine that ran
/// earlier tooling can still hold them, and the reclaim commands delete them.
pub const RETIRED_ROOT_LEVEL_FOLDERS: &[&str] = &[
    "target-dev-api",
    "target-ci",
    "target-dev-mcpd",
    "target-mk-db-selftest",
];
/// Prefixes of the retired root-level gate folders (cargo target folders and trunk dist folders).
pub const RETIRED_ROOT_LEVEL_FOLDER_PREFIXES: &[&str] = &["target-gate-", "dist-gate-"];

/// Is `name` (a folder at a checkout root) one of the retired root-level build folders?
pub fn is_retired_root_level_build_folder(name: &str) -> bool {
    RETIRED_ROOT_LEVEL_FOLDERS.contains(&name)
        || RETIRED_ROOT_LEVEL_FOLDER_PREFIXES
            .iter()
            .any(|prefix| name.starts_with(prefix))
}
