//! The folders of the Enfusion mod's three addons.
//!
//! **Role:** names each addon of the mod suite once: its folder name, which is also the name a
//! dedicated server's `addons/` folder links it under, and its folder relative to the checkout
//! root, under [`crate::workspace_folders::ENFUSION_MOD_DIR`].
//! **Position:** read by the mod commands (compile gate, world boot, playtest server, Workbench
//! bootstrap, mission test, gameplay policy generator), the website and staging deploys (rsync
//! exclusions, the instance addon link, the addon GUID read), the workstation setup (client addon
//! link, server profile) and the schema checks' mission validation, which reads the framework
//! addon's loader scripts.
//! **Signals & state:** none; constants.
//! **Invariants:** each addon folder is [`crate::workspace_folders::ENFUSION_MOD_DIR`], `/`, and
//! its folder name; each folder holds the addon's committed `addon.gproj`; the folder names never
//! change, because a game server and a Workbench session find an addon by them.

/// Folder name of the shipping game mod addon (`TBD_Framework`), in the mod folder and in a
/// dedicated server's `addons/` folder.
pub const FRAMEWORK_ADDON_FOLDER_NAME: &str = "tbd-framework";

/// Folder name of the Workbench export addon (`TBD_Export`), the project a Workbench session
/// opens; a game server never loads it.
pub const EXPORT_ADDON_FOLDER_NAME: &str = "tbd-export";

/// Folder name of the Enfusion MCP bridge addon (`TBD_EMCP`), which carries the Workbench Net API
/// handlers; a game server never loads it.
pub const MCP_BRIDGE_ADDON_FOLDER_NAME: &str = "tbd-emcp";

/// The shipping game mod addon's folder, relative to the checkout root.
pub const FRAMEWORK_ADDON_DIR: &str = "mod/tbd-framework";

/// The Workbench export addon's folder, relative to the checkout root.
pub const EXPORT_ADDON_DIR: &str = "mod/tbd-export";

/// The Enfusion MCP bridge addon's folder, relative to the checkout root.
pub const MCP_BRIDGE_ADDON_DIR: &str = "mod/tbd-emcp";
