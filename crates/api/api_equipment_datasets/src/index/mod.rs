//! Rebuildable SQLite navigation indexes; native values remain in source files.
mod resource_writer;
pub mod writer;

/// The navigation index layout version: a generation's index lives under
/// `indexes/<id>/<INDEX_VERSION>/` and loads only when its seal names this version.
pub const INDEX_VERSION: &str = "1";
