//! The workspace layout model: crate categories, the judged set and the category edge matrix.
//!
//! **Role:** answers which members the layout laws judge, which class a category belongs to,
//! what a crate's declared targets are, and whether a dependency edge between two categories is
//! allowed (rule 5 of the crate-tier law).
//! **Position:** shared by [`super::crate_tiers`], [`super::crate_firewalls`],
//! [`super::crate_anatomy`] and [`super::strangler`]; reads parsed members only.
//! **Signals & state:** none; constants and pure functions.
//! **Invariants:** the judged set is every member that declares `[package.metadata.layout]` plus
//! every member under `crates/<category…>/<name>` or `tools/<category>/<name>`; the class of a
//! category is decided by its folder path alone, and an unknown category has no class (a
//! finding, never a silent default).

use crate::workspace_members::WorkspaceMember;

/// The root folder of the library crates.
pub const CRATES_ROOT: &str = "crates";
/// The root folder of the tooling crates.
pub const TOOLS_ROOT: &str = "tools";
/// The root folder of the crates the strangler rule retires.
pub const LEGACY_ROOT: &str = "legacy";
/// The root folder of the application crates.
pub const APPS_ROOT: &str = "apps";

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

/// The one tool crate that may depend on api crates: it seeds a staging database.
pub const STAGING_FIXTURES_PATH: &str = "tools/staging/staging_fixtures";

/// When true, a member outside the judged set fails the crate-tier law; until the close stage of
/// the workspace restructure it is listed in an informational line instead.
pub const UNJUDGED_MEMBERS_FAIL: bool = false;

/// The class of a category, which decides the edges its crates may declare.
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
    /// `crates/frontend/<layer>`.
    Frontend,
    /// `tools` and `tools/<category>`.
    Tools,
}

impl CategoryClass {
    /// True for the classes of [`ENGINE_CATEGORIES`].
    pub fn is_engine(self) -> bool {
        matches!(self, Self::Engine | Self::Graphics)
    }
}

/// The class of `category`, or `None` for a category the layout does not know.
pub fn category_class(category: &str) -> Option<CategoryClass> {
    if category == TOOLS_ROOT || category.starts_with("tools/") {
        return Some(CategoryClass::Tools);
    }
    if let Some(layer) = category.strip_prefix("crates/frontend/") {
        let known = ["foundation", "features", "pages", "workspaces"];
        return known.contains(&layer).then_some(CategoryClass::Frontend);
    }
    let class = match category {
        "crates/foundation" => CategoryClass::Foundation,
        "crates/contracts" => CategoryClass::Contracts,
        "crates/mission" => CategoryClass::Mission,
        "crates/ballistics" => CategoryClass::Ballistics,
        "crates/graphics" => CategoryClass::Graphics,
        "crates/map_rendering" | "crates/paper_doll" => CategoryClass::Rendering,
        "crates/mission_editing" => CategoryClass::MissionEditing,
        "crates/api" => CategoryClass::Api,
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

/// True when `path` sits under the root folder `root`.
pub fn is_under(path: &str, root: &str) -> bool {
    path.strip_prefix(root)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// One side of a dependency edge, as the category matrix reads it.
#[derive(Debug, Clone, Copy)]
pub struct EdgeEnd<'a> {
    /// The crate's folder, repository-relative.
    pub path: &'a str,
    /// The crate's category.
    pub category: &'a str,
    /// The crate's declared targets.
    pub targets: Option<TargetPlatforms>,
}

/// True when the category matrix lets `from` depend on `to`.
pub fn category_edge_allowed(from: EdgeEnd<'_>, to: EdgeEnd<'_>) -> bool {
    use CategoryClass::*;
    let (Some(from_class), Some(to_class)) =
        (category_class(from.category), category_class(to.category))
    else {
        return false;
    };
    match from_class {
        Foundation => to_class == Foundation,
        Contracts => matches!(to_class, Foundation | Contracts),
        Mission => matches!(to_class, Foundation | Mission) || to.category == "crates/geometry",
        Ballistics => matches!(to_class, Foundation | Ballistics),
        Graphics => matches!(to_class, Foundation | Graphics),
        Engine => matches!(to_class, Foundation | Contracts) || to_class.is_engine(),
        Rendering => {
            matches!(
                to_class,
                Foundation | Contracts | Mission | Ballistics | Rendering
            ) || to_class.is_engine()
        }
        MissionEditing => {
            matches!(to_class, Foundation | Mission | Ballistics | MissionEditing)
                || to_class.is_engine()
        }
        Api => matches!(
            to_class,
            Foundation | Contracts | Mission | Ballistics | Api
        ),
        Frontend => to_class != Api && to_class != Tools,
        Tools => {
            if to.targets == Some(TargetPlatforms::Wasm32) {
                return false;
            }
            matches!(
                to_class,
                Foundation | Contracts | Mission | Ballistics | Tools
            ) || (to_class.is_engine() && to.targets == Some(TargetPlatforms::Any))
                || (to_class == Api && from.path == STAGING_FIXTURES_PATH)
        }
    }
}

/// True when `cfg` is a platform expression that holds only on wasm32.
pub fn is_wasm32_only_cfg(cfg: &str) -> bool {
    let compact: String = cfg.chars().filter(|c| !c.is_whitespace()).collect();
    compact == "cfg(target_arch=\"wasm32\")"
        || compact == "wasm32-unknown-unknown"
        || compact == "cfg(all(target_arch=\"wasm32\",target_os=\"unknown\"))"
}
