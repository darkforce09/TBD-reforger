//! The map engine's whole-crate UI-framework ban.
//!
//! **Role:** checks that `map_engine` depends on no UI framework and imports none, in any
//! module — rule 5 keeps the browser out of `editing/`, this keeps a component framework out of
//! the whole crate.
//! **Position:** a sibling of the seven reported rules in [`super`]; it is not part of the
//! `verify engine-layers` report, and the `engineering_laws` test binary of `api` asserts
//! on it. Reads the manifest through [`crate::cargo_manifest`].
//! **Signals & state:** none; pure functions over the checkout.
//! **Invariants:** the manifest arm reads every dependency table under the real package name, so
//! a renamed or target-specific edge is caught; the source arm matches the two shapes that are an
//! import — a `<framework>::` path and `extern crate` — so prose naming a framework, such as the
//! `cargo xtask mk leptos` command, is not a finding. An empty source walk is reported as a count
//! of zero for the caller to refuse. `web-sys` and `wasm-bindgen` are browser bindings, not UI
//! frameworks: the crate's renderer and streaming host use them on purpose.

use std::path::Path;

use super::rules::MAP_CRATE_REL;
use super::scanning::{is_source, rel};
use crate::cargo_manifest::read_manifest;
use verification_core::{NotRun, Pattern, scan};

/// A UI framework's package name, with any `-`/`_` suffixed companion crate (`leptos_router`,
/// `dioxus-web`).
pub const UI_FRAMEWORK_PACKAGE_RE: &str =
    r"^(leptos|yew|dioxus|sycamore|egui|eframe|iced|slint|tauri|relm4|gtk4)([-_][a-z0-9_-]+)?$";

/// A source line that imports a UI framework: a `<framework>::` path or an `extern crate`.
pub const UI_FRAMEWORK_IMPORT_RE: &str = r"\b(leptos|yew|dioxus|sycamore|egui|eframe|iced|slint|tauri|relm4|gtk4)(_[a-z0-9_]+)?\s*::|\bextern\s+crate\s+(leptos|yew|dioxus|sycamore|egui|eframe|iced|slint|tauri|relm4|gtk4)\b";

/// Everything one run of the UI-framework ban found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiFrameworkScan {
    /// How many `.rs` files under `legacy/map_engine/src` the source arm read.
    pub source_files: usize,
    /// Every dependency edge on, and every source import of, a UI framework.
    pub findings: Vec<String>,
}

/// Scan the map engine's manifest and sources under `repo_root` for a UI framework.
pub fn map_engine_ui_framework_findings(repo_root: &Path) -> Result<UiFrameworkScan, NotRun> {
    let package = compiled(UI_FRAMEWORK_PACKAGE_RE)?;
    let import = compiled(UI_FRAMEWORK_IMPORT_RE)?;
    let manifest_rel = format!("{MAP_CRATE_REL}/Cargo.toml");
    let manifest = read_manifest(&repo_root.join(&manifest_rel))?;
    let mut findings: Vec<String> = manifest
        .dependencies
        .iter()
        .filter(|edge| package.is_match(&edge.package))
        .map(|edge| {
            format!(
                "{manifest_rel}:{}: [{}] depends on the UI framework {}",
                edge.line_no, edge.table, edge.package
            )
        })
        .collect();

    let root = repo_root.to_path_buf();
    let sources = scan::walk_files(&[&repo_root.join(MAP_CRATE_REL).join("src")], move |p| {
        is_source(&root, p) && p.extension().is_some_and(|e| e == "rs")
    })?;
    for hit in scan::matching_lines(&import, &sources)? {
        findings.push(format!("{} — imports a UI framework", rel(repo_root, &hit)));
    }
    Ok(UiFrameworkScan {
        source_files: sources.len(),
        findings,
    })
}

/// Compile one of this module's patterns; a pattern that does not compile is a check that
/// cannot run.
fn compiled(pattern: &str) -> Result<Pattern, NotRun> {
    Pattern::regex(pattern).map_err(|error| NotRun::ToolError {
        tool: "regex".into(),
        status: 2,
        stderr: error.to_string(),
    })
}

#[cfg(test)]
#[path = "tests/ui_framework_ban.rs"]
mod tests;
