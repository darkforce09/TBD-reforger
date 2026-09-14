//! Published content: the modpacks a deployment requires.
//!
//! **Role:** the modpack listing and the single-modpack payload the content screens read.
//! **Position:** deserialised straight from the backend's JSON and handed to the pages that
//! render it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** a modpack's mod list is carried opaquely — the shape belongs to the launcher that
//! consumes it, and the app only passes it through.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One modpack: what it is called, and the mods it pins.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Modpack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub total_size_bytes: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub workshop_url: String,
    pub is_current: bool,
    pub created_at: String,
}

/// The modpack payload a single-modpack endpoint returns.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ModpackDto {
    #[serde(flatten)]
    pub modpack: Modpack,
    pub mods: Vec<Value>,
}
