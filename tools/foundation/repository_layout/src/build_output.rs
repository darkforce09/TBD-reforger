//! The build output folder.
//!
//! **Role:** the name of the one gitignored folder at a checkout root that holds all build
//! output and the tools' scratch subfolders.
//! **Position:** `xtask` pins the shared cargo target folder and its purpose subfolders under
//! it; `developer_tools` writes its mesh dumps there. Each tool names its own subfolders.
//! **Signals & state:** none; a constant.
//! **Invariants:** a single relative folder name, never a path with separators.

/// The one gitignored folder, relative to a checkout root, that holds all build output.
pub const BUILD_OUTPUT_FOLDER: &str = "target";
