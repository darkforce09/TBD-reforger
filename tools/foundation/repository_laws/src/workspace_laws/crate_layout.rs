//! The workspace layout model: crate categories, the judged set and declared targets.
//!
//! **Role:** answers which members the layout laws judge, which class a category belongs to and
//! what a crate's declared targets are.
//! **Position:** shared by `super::crate_firewalls` and [`super::crate_anatomy`]; reads parsed
//! members only.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** the judged set is every member that declares `[package.metadata.layout]` plus
//! every member under `crates/<category…>/<name>` or `tools/<category>/<name>`; the class of a
//! category is decided by its folder path alone, and an unknown category has no class, never a
//! silent default.

use crate::workspace_members::WorkspaceMember;

/// The root folder of the tooling crates.
pub const TOOLS_ROOT: &str = "tools";

/// The engine categories: CPU map engine crates and the graphics crates.
pub const ENGINE_CATEGORIES: &[&str] = &[
    "crates/geometry",
    "crates/world_formats",
    "crates/terrain",
    "crates/world_objects",
    "crates/line_of_sight",
    "crates/map_overlay",
    "crates/streaming",
    "crates/graphics",
];

/// The mission editing category: the headless editing layer of the Mission Creator.
pub const MISSION_EDITING_CATEGORY: &str = "crates/mission_editing";

/// The one tool crate that may depend on api crates and sqlx: it seeds a staging database.
pub const STAGING_FIXTURES_PATH: &str = "tools/staging/staging_fixtures";

/// The class of a category, which decides the external crates its crates may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryClass {
    /// `crates/foundation`.
    Foundation,
    /// `crates/contracts`.
    Contracts,
    /// `crates/mission`.
    Mission,
    /// `crates/ballistics`.
    Ballistics,
    /// `crates/graphics`.
    Graphics,
    /// Every other engine category of [`ENGINE_CATEGORIES`].
    Engine,
    /// `crates/map_rendering` and `crates/paper_doll`.
    Rendering,
    /// `crates/mission_editing`.
    MissionEditing,
    /// `crates/api`.
    Api,
    /// `crates/fleet`: the game server host agent.
    Fleet,
    /// `crates/frontend/<layer>`, the layers of [`FRONTEND_LAYER_FOLDERS`].
    Frontend,
    /// `tools` and `tools/<category>`.
    Tools,
}

/// The layer folders under `crates/frontend/`: the frontend categories, lowest layer first, the
/// shell (the single-page app and the offline service worker) last.
pub const FRONTEND_LAYER_FOLDERS: &[&str] =
    &["foundation", "features", "pages", "workspaces", "shell"];

/// The class of `category`, or `None` for a category the layout does not know.
pub fn category_class(category: &str) -> Option<CategoryClass> {
    if category == TOOLS_ROOT || category.starts_with("tools/") {
        return Some(CategoryClass::Tools);
    }
    if let Some(layer) = category.strip_prefix("crates/frontend/") {
        return FRONTEND_LAYER_FOLDERS
            .contains(&layer)
            .then_some(CategoryClass::Frontend);
    }
    let class = match category {
        "crates/foundation" => CategoryClass::Foundation,
        "crates/contracts" => CategoryClass::Contracts,
        "crates/mission" => CategoryClass::Mission,
        "crates/ballistics" => CategoryClass::Ballistics,
        "crates/graphics" => CategoryClass::Graphics,
        "crates/map_rendering" | "crates/paper_doll" => CategoryClass::Rendering,
        MISSION_EDITING_CATEGORY => CategoryClass::MissionEditing,
        "crates/api" => CategoryClass::Api,
        "crates/fleet" => CategoryClass::Fleet,
        other if ENGINE_CATEGORIES.contains(&other) => CategoryClass::Engine,
        _ => return None,
    };
    Some(class)
}

/// The platforms a crate builds for, as `[package.metadata.layout] targets` declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPlatforms {
    /// `any`: native and wasm32.
    Any,
    /// `wasm32`: wasm32 only.
    Wasm32,
}

/// The declared targets of `member`, when it declares a known value.
pub fn declared_targets(member: &WorkspaceMember) -> Option<TargetPlatforms> {
    match member.manifest.layout.as_ref()?.targets.as_deref()? {
        "any" => Some(TargetPlatforms::Any),
        "wasm32" => Some(TargetPlatforms::Wasm32),
        _ => None,
    }
}

/// The category `member` declares, or its folder's category when it declares none.
pub fn effective_category(member: &WorkspaceMember) -> String {
    member
        .manifest
        .layout
        .as_ref()
        .and_then(|layout| layout.category.clone())
        .unwrap_or_else(|| member.parent_folder().to_string())
}

/// True when the layout laws judge `member`.
pub fn is_judged(member: &WorkspaceMember) -> bool {
    member.manifest.layout.is_some() || sits_in_layout_folder(&member.path)
}

/// True for `crates/<category…>/<name>` and `tools/<category>/<name>`.
pub fn sits_in_layout_folder(path: &str) -> bool {
    let depth = path.split('/').count();
    (path.starts_with("crates/") && depth >= 3) || (path.starts_with("tools/") && depth == 3)
}
