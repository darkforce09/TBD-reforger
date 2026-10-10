//! The legacy ticket data folder.
//!
//! **Role:** the repository-relative path of the folder that still holds the file-based ticket
//! records (`T-<id>.toml`, the wave lock, receipts and estimates) awaiting `ttm import` into the
//! central ticket manager.
//! **Position:** no tool reads or writes ticket data there any more (the `ttm` command line owns
//! tickets and waves); the documentation link check and the relocation tool name it only to leave
//! its files alone.
//! **Signals & state:** none; one constant.
//! **Invariants:** the path is relative, uses `/` separators and has no trailing slash.

/// The folder of file-based ticket records kept as data until they are imported into the central
/// ticket manager; nothing in the workspace parses them.
pub const LEGACY_TICKETS_DIR: &str = ".ai/tickets";
