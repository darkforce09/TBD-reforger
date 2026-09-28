//! A `Cargo.toml` reader for the dependency and feature laws.
//!
//! **Role:** turns a Cargo manifest into its package name, every dependency edge (in every
//! dependency table, including target-specific and renamed ones) and every declared feature.
//! **Position:** used by [`super::crate_dependencies`] and
//! [`super::engine_layers::map_engine_ui_framework_findings`]; reads one file.
//! **Signals & state:** none; pure functions over manifest text.
//! **Invariants:** it reads the TOML subset Cargo manifests are written in — section headers,
//! `key = value` lines, inline tables and arrays that may span lines, `#` comments — without a
//! TOML dependency, so `verification-core` keeps its two dependencies. A renamed dependency
//! (`alias = { package = "real" }`) is reported under its real package name, and a `#` comment
//! never produces an edge.

use std::path::Path;

use crate::verdict::NotRun;

/// One dependency edge of a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEdge {
    /// The table the edge sits in, as its header spells it: `dependencies`, `dev-dependencies`,
    /// `build-dependencies` or `target.'cfg(…)'.dependencies`.
    pub table: String,
    /// The key the manifest declares the edge under.
    pub key: String,
    /// The package the edge resolves to: the `package` field when present, the key otherwise.
    pub package: String,
    /// The `path` field, when present.
    pub path: Option<String>,
    /// The features the edge enables.
    pub features: Vec<String>,
    /// 1-based line of the key.
    pub line_no: usize,
}

impl DependencyEdge {
    /// True for an edge in a `dev-dependencies` table, target-specific or not.
    pub fn is_dev_dependency(&self) -> bool {
        self.table == "dev-dependencies" || self.table.ends_with(".dev-dependencies")
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

/// The parts of a manifest the laws read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CargoManifest {
    /// `[package] name`, when the manifest declares a package.
    pub package_name: Option<String>,
    /// Every dependency edge, in manifest order.
    pub dependencies: Vec<DependencyEdge>,
    /// Every `[features]` entry, in manifest order.
    pub features: Vec<FeatureDeclaration>,
}

impl CargoManifest {
    /// The declaration of `feature`, when the manifest declares it.
    pub fn feature(&self, feature: &str) -> Option<&FeatureDeclaration> {
        self.features
            .iter()
            .find(|declared| declared.name == feature)
    }
}

/// Read and parse the manifest at `path`; a missing or unreadable manifest is [`NotRun`].
pub fn read_manifest(path: &Path) -> Result<CargoManifest, NotRun> {
    if !path.exists() {
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
            section = entry
                .trim_matches(|c| c == '[' || c == ']')
                .trim()
                .to_string();
            if let Some((table, key)) = dependency_subtable(&section) {
                manifest.dependencies.push(DependencyEdge {
                    table,
                    package: key.clone(),
                    key,
                    path: None,
                    features: Vec::new(),
                    line_no,
                });
            }
            continue;
        }
        while !balanced(&entry) && index < lines.len() {
            entry.push(' ');
            entry.push_str(without_comment(lines[index]).trim());
            index += 1;
        }
        let Some((key, value)) = entry.split_once('=') else {
            continue;
        };
        let key = unquoted(key.trim());
        let value = value.trim();
        if section == "package" && key == "name" {
            manifest.package_name = Some(unquoted(value));
        } else if section == "features" {
            manifest.features.push(FeatureDeclaration {
                name: key,
                enables: string_items(value),
                line_no,
            });
        } else if is_dependency_table(&section) {
            let key = key.split('.').next().unwrap_or_default().to_string();
            manifest
                .dependencies
                .push(edge_from_value(&section, key, value, line_no));
        } else if dependency_subtable(&section).is_some()
            && let Some(edge) = manifest.dependencies.last_mut()
        {
            match key.as_str() {
                "package" => edge.package = unquoted(value),
                "path" => edge.path = Some(unquoted(value)),
                "features" => edge.features = string_items(value),
                _ => {}
            }
        }
    }
    manifest
}

/// True for a section header naming a dependency table.
fn is_dependency_table(section: &str) -> bool {
    ["dependencies", "dev-dependencies", "build-dependencies"]
        .iter()
        .any(|table| section == *table || section.ends_with(&format!(".{table}")))
}

/// `(table, key)` for a `[dependencies.<key>]`-style sub-table header.
fn dependency_subtable(section: &str) -> Option<(String, String)> {
    let (table, key) = section.rsplit_once('.')?;
    is_dependency_table(table).then(|| (table.to_string(), unquoted(key)))
}

/// The edge a `key = value` line declares, reading `package`, `path` and `features` out of an
/// inline table.
fn edge_from_value(table: &str, key: String, value: &str, line_no: usize) -> DependencyEdge {
    let field = |name: &str| -> Option<String> {
        let at = inline_field(value, name)?;
        let rest = value[at..].trim_start().strip_prefix('=')?.trim_start();
        Some(rest.to_string())
    };
    let package = field("package")
        .map(|rest| first_string(&rest))
        .unwrap_or_else(|| key.clone());
    DependencyEdge {
        table: table.to_string(),
        path: field("path").map(|rest| first_string(&rest)),
        features: field("features")
            .map(|rest| string_items(&rest))
            .unwrap_or_default(),
        package,
        key,
        line_no,
    }
}

/// Byte offset just past the field name `name` inside an inline table, when the name stands as
/// a whole key (preceded by `{`, `,` or whitespace and followed by `=`).
fn inline_field(value: &str, name: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(found) = value[from..].find(name) {
        let start = from + found;
        let end = start + name.len();
        let before = value[..start].chars().next_back();
        let after = value[end..].trim_start();
        if matches!(before, Some('{' | ',' | ' ' | '\t')) && after.starts_with('=') {
            return Some(end);
        }
        from = end;
    }
    None
}

/// The first `"…"` string in `text`, or `text` itself without quotes.
fn first_string(text: &str) -> String {
    text.split('"')
        .nth(1)
        .map(str::to_string)
        .unwrap_or_else(|| unquoted(text))
}

/// Every `"…"` string in `text`, in order — the items of an array or a single string value.
/// The first array closes the list, so `features = ["a"], optional = true` yields `a` only.
fn string_items(text: &str) -> Vec<String> {
    let text = match (text.find('['), text.find(']')) {
        (Some(open), Some(close)) if open < close => &text[open..close],
        _ => text,
    };
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// `text` without surrounding quotes.
fn unquoted(text: &str) -> String {
    text.trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

/// `line` up to the first `#` that is not inside a string.
fn without_comment(line: &str) -> &str {
    let mut quoted = false;
    for (offset, character) in line.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..offset],
            _ => {}
        }
    }
    line
}

/// True when every `{` and `[` of `entry` outside strings has closed.
fn balanced(entry: &str) -> bool {
    let mut depth = 0i32;
    let mut quoted = false;
    for character in entry.chars() {
        match character {
            '"' => quoted = !quoted,
            '{' | '[' if !quoted => depth += 1,
            '}' | ']' if !quoted => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

#[cfg(test)]
#[path = "tests/cargo_manifest.rs"]
mod tests;
