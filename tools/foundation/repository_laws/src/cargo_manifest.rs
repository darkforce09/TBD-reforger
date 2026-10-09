//! A `Cargo.toml` reader for the dependency, feature and workspace laws.
//!
//! **Role:** turns a Cargo manifest into the parts the laws read: the package name and the
//! package keys it inherits from the workspace, every dependency edge (in every dependency table,
//! including target-specific, renamed and `workspace = true` ones), every declared feature, the
//! `[package.metadata.layout]` declaration, the `[lints]` source, the library and binary targets,
//! and a root manifest's `[workspace]` member list.
//! **Position:** used by [`super::workspace_members`] and the workspace laws in
//! [`super::workspace_laws`]; reads one file.
//! **Signals & state:** none; pure functions over manifest text.
//! **Invariants:** it reads the TOML subset Cargo manifests are written in — section headers,
//! `key = value` lines, dotted keys, inline tables and arrays that may span lines, `#` comments —
//! without a TOML dependency, so `repository_laws` needs no TOML parser. A renamed
//! dependency (`alias = { package = "real" }`) is reported under its real package name, every
//! spelling of one dependency (inline table, dotted keys, `[dependencies.<name>]` sub-table) is one
//! edge, a `[workspace.dependencies]` entry is never an edge, and a `#` comment never produces one.

use std::path::Path;

use verification_core::verdict::NotRun;

mod toml_subset;

use toml_subset::{
    balanced, first_string, inherits_from_workspace, inline_field, is_true, string_items, unquoted,
    without_comment,
};

/// Which build a dependency edge feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    /// `[dependencies]`: the crate's own build.
    Normal,
    /// `[dev-dependencies]`: tests, examples and benches only.
    Development,
    /// `[build-dependencies]`: the build script.
    Build,
}

/// One dependency edge of a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    /// The table the edge sits in, as its header spells it: `dependencies`, `dev-dependencies`,
    /// `build-dependencies` or `target.'cfg(…)'.dependencies`.
    pub table: String,
    /// The build the table feeds.
    pub kind: DependencyKind,
    /// The platform expression of a `[target.<expression>.…]` table, unquoted; `None` for a
    /// table every platform reads.
    pub target_cfg: Option<String>,
    /// The key the manifest declares the edge under.
    pub key: String,
    /// The package the edge resolves to: the `package` field when present, the key otherwise.
    pub package: String,
    /// The `path` field, when present.
    pub path: Option<String>,
    /// The features the edge enables.
    pub features: Vec<String>,
    /// True when the edge takes its source from `[workspace.dependencies]`
    /// (`{ workspace = true }`, `name.workspace = true` or a sub-table `workspace = true`).
    pub from_workspace: bool,
    /// 1-based line of the key.
    pub line_no: usize,
}

impl DependencyEdge {
    /// True for an edge in a `dev-dependencies` table, target-specific or not.
    pub fn is_dev_dependency(&self) -> bool {
        self.kind == DependencyKind::Development
    }
}

/// One `[features]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureDeclaration {
    /// The feature's name.
    pub name: String,
    /// What it enables: other features, `dep:` names and `crate/feature` pairs.
    pub enables: Vec<String>,
    /// 1-based line of the declaration.
    pub line_no: usize,
}

/// One `[package]` key other than `name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageField {
    /// The key, without a `.workspace` suffix: `edition`, `rust-version`, `license`, …
    pub key: String,
    /// True when the value comes from `[workspace.package]` (`key.workspace = true` or
    /// `key = { workspace = true }`).
    pub inherited: bool,
    /// The value as written, unquoted.
    pub value: String,
    /// 1-based line of the key.
    pub line_no: usize,
}

/// Where the crate's lint levels come from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LintsSource {
    /// The manifest has no `[lints]` table.
    #[default]
    Absent,
    /// `[lints] workspace = true`.
    Workspace,
    /// A `[lints]` or `[lints.<tool>]` table with the crate's own levels.
    Local,
}

/// The `[package.metadata.layout]` table: where the crate sits in the workspace layout.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutDeclaration {
    /// `category`: the folder path between the repository root and the crate.
    pub category: Option<String>,
    /// `tier`, as written: a whole number when well formed.
    pub tier: Option<String>,
    /// `targets`: `any` or `wasm32`.
    pub targets: Option<String>,
    /// 1-based line of the table header.
    pub line_no: usize,
}

impl LayoutDeclaration {
    /// The declared tier as a number, when it is a whole number.
    pub fn tier_number(&self) -> Option<u32> {
        self.tier.as_deref().and_then(|tier| tier.parse().ok())
    }
}

/// A `[lib]` or `[[bin]]` target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildTarget {
    /// The `name` field, when present.
    pub name: Option<String>,
    /// The `path` field, when present.
    pub path: Option<String>,
    /// 1-based line of the table header.
    pub line_no: usize,
}

/// A root manifest's `[workspace]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceDeclaration {
    /// `members`: folder paths and globs, as written.
    pub members: Vec<String>,
    /// `exclude`: folder paths, as written.
    pub exclude: Vec<String>,
    /// 1-based line of the table header.
    pub line_no: usize,
}

/// The parts of a manifest the laws read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CargoManifest {
    /// `[package] name`, when the manifest declares a package.
    pub package_name: Option<String>,
    /// Every `[package]` key other than `name`, in manifest order.
    pub package_fields: Vec<PackageField>,
    /// Every dependency edge, in manifest order.
    pub dependencies: Vec<DependencyEdge>,
    /// Every `[features]` entry, in manifest order.
    pub features: Vec<FeatureDeclaration>,
    /// `[package.metadata.layout]`, when declared.
    pub layout: Option<LayoutDeclaration>,
    /// Where the lint levels come from.
    pub lints: LintsSource,
    /// The `[lib]` table, when declared.
    pub library: Option<BuildTarget>,
    /// Every `[[bin]]` table, in manifest order.
    pub binaries: Vec<BuildTarget>,
    /// The `[workspace]` table of a root manifest.
    pub workspace: Option<WorkspaceDeclaration>,
}

impl CargoManifest {
    /// The declaration of `feature`, when the manifest declares it.
    pub fn feature(&self, feature: &str) -> Option<&FeatureDeclaration> {
        self.features
            .iter()
            .find(|declared| declared.name == feature)
    }

    /// The `[package]` key `key`, when the manifest declares it.
    pub fn package_field(&self, key: &str) -> Option<&PackageField> {
        self.package_fields.iter().find(|field| field.key == key)
    }
}

/// Read and parse the manifest at `path`; a missing or unreadable manifest is [`NotRun`].
pub fn read_manifest(path: &Path) -> Result<CargoManifest, NotRun> {
    if !path.is_file() {
        return Err(NotRun::TargetMissing(path.to_path_buf()));
    }
    let text = std::fs::read_to_string(path).map_err(|source| NotRun::Unreadable {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(parse_manifest(&text))
}

/// Parse manifest text.
pub fn parse_manifest(text: &str) -> CargoManifest {
    let mut manifest = CargoManifest::default();
    let mut section = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    while index < lines.len() {
        let line_no = index + 1;
        let mut entry = without_comment(lines[index]).trim().to_string();
        index += 1;
        if entry.is_empty() {
            continue;
        }
        if entry.starts_with('[') {
            section = open_section(&mut manifest, &entry, line_no);
            continue;
        }
        while !balanced(&entry) && index < lines.len() {
            entry.push(' ');
            entry.push_str(without_comment(lines[index]).trim());
            index += 1;
        }
        if let Some((key, value)) = entry.split_once('=') {
            apply_entry(
                &mut manifest,
                &section,
                &unquoted(key.trim()),
                value.trim(),
                line_no,
            );
        }
    }
    manifest
}

/// Record what a section header opens and return the section's name.
fn open_section(manifest: &mut CargoManifest, header: &str, line_no: usize) -> String {
    let array_of_tables = header.starts_with("[[");
    let section = header
        .trim_matches(|c| c == '[' || c == ']')
        .trim()
        .to_string();
    let target = BuildTarget {
        line_no,
        ..BuildTarget::default()
    };
    match section.as_str() {
        "lib" => manifest.library = Some(target),
        "bin" if array_of_tables => manifest.binaries.push(target),
        "lints" => manifest.lints = LintsSource::Local,
        "package.metadata.layout" => {
            manifest.layout = Some(LayoutDeclaration {
                line_no,
                ..LayoutDeclaration::default()
            })
        }
        "workspace" => {
            manifest.workspace = Some(WorkspaceDeclaration {
                line_no,
                ..WorkspaceDeclaration::default()
            })
        }
        other if other.starts_with("lints.") => manifest.lints = LintsSource::Local,
        other => {
            if let Some((table, key)) = dependency_subtable(other) {
                manifest.dependencies.push(new_edge(&table, key, line_no));
            }
        }
    }
    section
}

/// Record one `key = value` entry of `section`.
fn apply_entry(
    manifest: &mut CargoManifest,
    section: &str,
    key: &str,
    value: &str,
    line_no: usize,
) {
    match section {
        "package" => apply_package_entry(manifest, key, value, line_no),
        "features" => manifest.features.push(FeatureDeclaration {
            name: key.to_string(),
            enables: string_items(value),
            line_no,
        }),
        "lints" if key == "workspace" && is_true(value) => manifest.lints = LintsSource::Workspace,
        "package.metadata.layout" => {
            if let Some(layout) = manifest.layout.as_mut() {
                match key {
                    "category" => layout.category = Some(unquoted(value)),
                    "tier" => layout.tier = Some(unquoted(value)),
                    "targets" => layout.targets = Some(unquoted(value)),
                    _ => {}
                }
            }
        }
        "lib" => apply_target_entry(manifest.library.as_mut(), key, value),
        "bin" => apply_target_entry(manifest.binaries.last_mut(), key, value),
        "workspace" => {
            if let Some(workspace) = manifest.workspace.as_mut() {
                match key {
                    "members" => workspace.members = string_items(value),
                    "exclude" => workspace.exclude = string_items(value),
                    _ => {}
                }
            }
        }
        table if is_dependency_table(table) => {
            apply_dependency_line(manifest, table, key, value, line_no)
        }
        _ => {
            if dependency_subtable(section).is_some()
                && let Some(edge) = manifest.dependencies.last_mut()
            {
                apply_dependency_field(edge, key, value);
            }
        }
    }
}

/// Record a `[package]` key: the name, or a field and whether it is inherited.
fn apply_package_entry(manifest: &mut CargoManifest, key: &str, value: &str, line_no: usize) {
    let (base, suffix) = match key.split_once('.') {
        Some((base, suffix)) => (base, Some(suffix)),
        None => (key, None),
    };
    if base == "name" && suffix.is_none() {
        manifest.package_name = Some(unquoted(value));
        return;
    }
    let inherited =
        (suffix == Some("workspace") && is_true(value)) || inherits_from_workspace(value);
    manifest.package_fields.push(PackageField {
        key: base.to_string(),
        inherited,
        value: unquoted(value),
        line_no,
    });
}

/// Record a `name` or `path` field of a `[lib]` or `[[bin]]` table.
fn apply_target_entry(target: Option<&mut BuildTarget>, key: &str, value: &str) {
    if let Some(target) = target {
        match key {
            "name" => target.name = Some(unquoted(value)),
            "path" => target.path = Some(unquoted(value)),
            _ => {}
        }
    }
}

/// Record a line of a dependency table: a whole edge (`name = …`) or one field of an edge
/// spelled with dotted keys (`name.workspace = true`, `name.features = […]`).
fn apply_dependency_line(
    manifest: &mut CargoManifest,
    table: &str,
    key: &str,
    value: &str,
    line_no: usize,
) {
    let Some((name, field)) = key.split_once('.') else {
        manifest
            .dependencies
            .push(edge_from_value(table, unquoted(key), value, line_no));
        return;
    };
    let name = unquoted(name);
    let existing = manifest
        .dependencies
        .iter()
        .position(|edge| edge.table == table && edge.key == name);
    let at = existing.unwrap_or_else(|| {
        manifest
            .dependencies
            .push(new_edge(table, name.clone(), line_no));
        manifest.dependencies.len() - 1
    });
    apply_dependency_field(&mut manifest.dependencies[at], field, value);
}

/// Record one field of a dependency edge.
fn apply_dependency_field(edge: &mut DependencyEdge, field: &str, value: &str) {
    match field {
        "package" => edge.package = unquoted(value),
        "path" => edge.path = Some(unquoted(value)),
        "features" => edge.features = string_items(value),
        "workspace" => edge.from_workspace = is_true(value),
        _ => {}
    }
}

/// True for a section header naming a crate's dependency table; `[workspace.dependencies]`
/// declares versions for members and is not one.
fn is_dependency_table(section: &str) -> bool {
    !section.starts_with("workspace")
        && ["dependencies", "dev-dependencies", "build-dependencies"]
            .iter()
            .any(|table| section == *table || section.ends_with(&format!(".{table}")))
}

/// `(table, key)` for a `[dependencies.<key>]`-style sub-table header.
fn dependency_subtable(section: &str) -> Option<(String, String)> {
    let (table, key) = section.rsplit_once('.')?;
    is_dependency_table(table).then(|| (table.to_string(), unquoted(key)))
}

/// An edge with no fields yet, classified by its table.
fn new_edge(table: &str, key: String, line_no: usize) -> DependencyEdge {
    let kind = if table.ends_with("dev-dependencies") {
        DependencyKind::Development
    } else if table.ends_with("build-dependencies") {
        DependencyKind::Build
    } else {
        DependencyKind::Normal
    };
    let target_cfg = table.strip_prefix("target.").map(|rest| {
        let expression = rest
            .rsplit_once('.')
            .map_or(rest, |(expression, _)| expression);
        unquoted(expression)
    });
    DependencyEdge {
        table: table.to_string(),
        kind,
        target_cfg,
        package: key.clone(),
        key,
        path: None,
        features: Vec::new(),
        from_workspace: false,
        line_no,
    }
}

/// The edge a `key = value` line declares, reading `package`, `path`, `features` and
/// `workspace` out of an inline table.
fn edge_from_value(table: &str, key: String, value: &str, line_no: usize) -> DependencyEdge {
    let mut edge = new_edge(table, key, line_no);
    if let Some(package) = inline_field(value, "package") {
        edge.package = first_string(&package);
    }
    edge.path = inline_field(value, "path").map(|rest| first_string(&rest));
    edge.features = inline_field(value, "features")
        .map(|rest| string_items(&rest))
        .unwrap_or_default();
    edge.from_workspace = inherits_from_workspace(value);
    edge
}

#[cfg(test)]
#[path = "tests/cargo_manifest.rs"]
mod tests;
