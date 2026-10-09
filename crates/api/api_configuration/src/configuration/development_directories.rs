//! The development defaults of the API's directory settings, anchored on the checkout root.
//!
//! **Role:** names the four directory settings that have a development default inside the
//! checkout ([`UPLOAD`], [`EQUIPMENT_DATA`], [`MAP_ASSETS`], [`GLYPH_ASSETS`]) and resolves a
//! setting's value with [`resolve`]: a set value as written; an unset one, in development, the
//! default joined onto the checkout root; an unset one outside development, empty, for the
//! configuration's validation to report.
//! **Position:** inside the configuration module; `Config::load` resolves every directory setting
//! through it with the checkout-root walk of `repository_root` from the working directory, and
//! `Config::for_tests` with the walk from this crate's manifest folder.
//! **Signals & state:** none; the walk is the caller's closure, run once per unset setting in
//! development and never otherwise.
//! **Invariants:** a development default is an absolute path under the folder that holds the
//! repository root marker, so the API resolves the same directories from any working directory
//! inside the checkout; a walk that finds no root is a [`ConfigError::CheckoutRootNotFound`]
//! naming the setting, never a path relative to the working directory.

use std::path::PathBuf;

use super::ConfigError;

/// A directory setting whose development default lies inside the checkout.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CheckoutDirectory {
    /// The environment variable that names the directory.
    pub(crate) variable: &'static str,
    /// The development default, relative to the checkout root.
    pub(crate) checkout_relative: &'static str,
}

/// `UPLOAD_DIR`: the gitignored scratch folder the CMS thumbnail upload writes into.
pub(crate) const UPLOAD: CheckoutDirectory = CheckoutDirectory {
    variable: "UPLOAD_DIR",
    checkout_relative: "assets/scratch/api/uploads",
};

/// `EQUIPMENT_DATA_DIR`: the gitignored folder of the imported equipment datasets.
pub(crate) const EQUIPMENT_DATA: CheckoutDirectory = CheckoutDirectory {
    variable: "EQUIPMENT_DATA_DIR",
    checkout_relative: "assets/equipment",
};

/// `MAP_ASSETS_DIR`: the terrain tree served at `/map-assets`.
pub(crate) const MAP_ASSETS: CheckoutDirectory = CheckoutDirectory {
    variable: "MAP_ASSETS_DIR",
    checkout_relative: "assets/terrains",
};

/// `GLYPH_ASSETS_DIR`: the glyph tree served at `/map-assets/glyphs`.
pub(crate) const GLYPH_ASSETS: CheckoutDirectory = CheckoutDirectory {
    variable: "GLYPH_ASSETS_DIR",
    checkout_relative: "assets/glyphs",
};

/// The value of `directory`: `configured` when it is set; in development, an unset value becomes
/// the default joined onto the checkout root `checkout_root` finds; outside development an unset
/// value stays empty.
///
/// # Errors
///
/// [`ConfigError::CheckoutRootNotFound`] when the value is unset in development and
/// `checkout_root` finds no checkout root.
pub(crate) fn resolve(
    directory: CheckoutDirectory,
    configured: &str,
    development: bool,
    checkout_root: impl FnOnce() -> repository_root::Result<PathBuf>,
) -> Result<String, ConfigError> {
    if !configured.is_empty() || !development {
        return Ok(configured.to_string());
    }
    let root = checkout_root()
        .map_err(|walk| ConfigError::CheckoutRootNotFound(directory.variable, walk))?;
    Ok(root.join(directory.checkout_relative).display().to_string())
}
