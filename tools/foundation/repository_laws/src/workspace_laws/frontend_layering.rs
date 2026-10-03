//! The frontend-layering law: a lower frontend layer never imports a higher one.
//!
//! **Role:** maps every production and test source of a frontend crate onto a layer — foundation
//! below features below pages and workspaces below the app shell — through a layer table the
//! caller passes, resolves every in-crate module path each file names, and counts the import
//! edges that break the order: a lower layer naming a higher one, pages naming workspaces or the
//! reverse, and one page area naming another.
//! **Position:** `cargo xtask verify frontend-layering` prints [`check_frontend_layering`]; xtask
//! owns the layer table, so a folder move rewrites the table and not this law.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** an edge is one (source file, target layer and area) pair, counted once however
//! many lines name it, and production and test edges are counted apart. The law is a ratchet:
//! more edges than [`FRONTEND_LAYERING_CEILING`] fail, fewer pass with a note to lower it. A
//! source file no table row maps is a finding, and a missing crate folder is
//! [`NotRun::TargetMissing`].

use std::collections::BTreeSet;
use std::path::Path;

use super::rust_module_references::{file_module, module_references};
use super::{LawOutcome, WorkspaceLawReport};
use crate::source_roots::is_test_file;
use verification_core::scan;
use verification_core::verdict::NotRun;

/// A frontend layer, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrontendLayer {
    /// Shared foundations every page and workspace builds on.
    Foundation,
    /// Features shared by several pages.
    Features,
    /// Platform pages, one area per navigation section.
    Pages,
    /// Standalone workspaces such as the Mission Creator.
    Workspaces,
    /// The app shell: entry point, route table and frame.
    Shell,
}

impl FrontendLayer {
    /// The layer's rank; a file may import only layers at or below its own rank.
    pub fn rank(self) -> u8 {
        match self {
            Self::Foundation => 0,
            Self::Features => 1,
            Self::Pages | Self::Workspaces => 2,
            Self::Shell => 3,
        }
    }

    /// The layer's name as a report prints it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Foundation => "foundation",
            Self::Features => "features",
            Self::Pages => "pages",
            Self::Workspaces => "workspaces",
            Self::Shell => "shell",
        }
    }
}

/// One row of a frontend crate's layer table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendLayerRow {
    /// Crate-relative folder (`src/v2/core`) or file (`src/router.rs`) the row maps.
    pub path: &'static str,
    /// The layer of everything under the path.
    pub layer: FrontendLayer,
    /// True when the next folder under the path names an area (a page area, a workspace).
    pub has_areas: bool,
}

/// The layer table of one frontend crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendCrateLayers {
    /// The crate's folder, repository-relative.
    pub crate_path: &'static str,
    /// The rows; the longest matching path wins.
    pub rows: &'static [FrontendLayerRow],
}

/// The most layering edges the tree may hold, production and test apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayeringCeiling {
    /// Edges from production files.
    pub production: usize,
    /// Edges from test files.
    pub test: usize,
}

/// The ratchet ceiling: the edges the tree holds today. Lower it as edges are removed; it reaches
/// zero when the frontend splits into layer crates. The production edges are the foundation's
/// imports of the Mission Creator (`core/ui` select, slider and search box; `core/auth/store.rs`)
/// and of the route table (`core/auth/route_guard.rs`), the mission hub's imports of the Mission
/// Creator (`library/dossier_upload*.rs`, `review_workspace/page.rs`), and administration pages'
/// imports of the mission hub's review record; the test edges are two route-table checks and the
/// review workspace's test.
pub const FRONTEND_LAYERING_CEILING: LayeringCeiling = LayeringCeiling {
    production: 12,
    test: 3,
};

/// One import edge that breaks the layer order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeringEdge {
    /// The importing file, repository-relative.
    pub file: String,
    /// The first line naming the target.
    pub line_no: usize,
    /// True when the importing file is a test file.
    pub test: bool,
    /// The importing file's place, e.g. `pages/mission_hub`.
    pub from: String,
    /// The imported place, e.g. `workspaces/editor`.
    pub to: String,
}

/// The layering report over `crates`, judged against `ceiling`.
pub fn check_frontend_layering(
    repo_root: &Path,
    crates: &[FrontendCrateLayers],
    ceiling: LayeringCeiling,
) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "frontend-layering",
        frontend_layering_outcome(repo_root, crates, ceiling),
    )
}

/// The ratchet verdict over the layering edges of `crates`.
pub fn frontend_layering_outcome(
    repo_root: &Path,
    crates: &[FrontendCrateLayers],
    ceiling: LayeringCeiling,
) -> Result<LawOutcome, NotRun> {
    let mut outcome = LawOutcome::default();
    let mut edges = Vec::new();
    for layers in crates {
        let (crate_edges, unmapped) = layering_edges(repo_root, layers)?;
        edges.extend(crate_edges);
        outcome.findings.extend(
            unmapped
                .into_iter()
                .map(|file| format!("{file} sits under no row of the layer table")),
        );
    }
    let production = edges.iter().filter(|edge| !edge.test).count();
    let test = edges.len() - production;
    outcome.summary = format!(
        "{production} production and {test} test edge(s) against a ceiling of {} and {}",
        ceiling.production, ceiling.test
    );
    for edge in &edges {
        let kind = if edge.test { "test" } else { "production" };
        outcome.notes.push(format!(
            "{kind} edge {}:{}: {} imports {}",
            edge.file, edge.line_no, edge.from, edge.to
        ));
    }
    for (label, count, limit) in [
        ("production", production, ceiling.production),
        ("test", test, ceiling.test),
    ] {
        if count > limit {
            outcome.findings.push(format!(
                "{count} {label} layering edge(s) exceed the ceiling of {limit}; remove the new \
                 import instead of raising the ceiling"
            ));
        } else if count < limit {
            outcome.notes.push(format!(
                "{count} {label} layering edge(s) are under the ceiling of {limit}; lower \
                 FRONTEND_LAYERING_CEILING.{label} to {count}"
            ));
        }
    }
    Ok(outcome)
}

/// Every layering edge of one crate, and the files no row maps.
pub fn layering_edges(
    repo_root: &Path,
    layers: &FrontendCrateLayers,
) -> Result<(Vec<LayeringEdge>, Vec<String>), NotRun> {
    let crate_root = repo_root.join(layers.crate_path);
    let source = crate_root.join("src");
    let files = scan::walk_files(&[source.as_path()], scan::with_extension(&["rs"]))?;
    let mut edges = Vec::new();
    let mut unmapped = Vec::new();
    for file in files {
        let crate_rel = file
            .strip_prefix(&crate_root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let repo_rel = format!("{}/{crate_rel}", layers.crate_path);
        let Some(from) = place_of(layers.rows, &crate_rel, false) else {
            unmapped.push(repo_rel);
            continue;
        };
        let text = std::fs::read_to_string(&file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        let mut seen = BTreeSet::new();
        for reference in module_references(&text, &file_module(&crate_rel)) {
            let target = format!("src/{}", reference.segments.join("/"));
            let Some(to) = place_of(layers.rows, &target, true) else {
                continue;
            };
            if breaks_order(&from, &to) && seen.insert(to.label()) {
                edges.push(LayeringEdge {
                    file: repo_rel.clone(),
                    line_no: reference.line_no,
                    test: is_test_file(&repo_rel),
                    from: from.label(),
                    to: to.label(),
                });
            }
        }
    }
    Ok((edges, unmapped))
}

/// A layer and, for a row with areas, the area.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    layer: FrontendLayer,
    area: Option<String>,
}

impl Place {
    fn label(&self) -> String {
        match &self.area {
            Some(area) => format!("{}/{area}", self.layer.name()),
            None => self.layer.name().to_string(),
        }
    }
}

/// The place of the crate-relative `path`: a source file, or (`module_path`) a module path
/// spelled as a folder path, where a file row `src/router.rs` maps the module `src/router`.
fn place_of(rows: &[FrontendLayerRow], path: &str, module_path: bool) -> Option<Place> {
    let mut best: Option<(&FrontendLayerRow, &str, usize)> = None;
    for row in rows {
        let row_path = if module_path {
            row.path.trim_end_matches("/mod.rs").trim_end_matches(".rs")
        } else {
            row.path
        };
        let rest = if path == row_path {
            Some("")
        } else if module_path && row.path.ends_with("mod.rs") {
            // A folder's own module file maps that module alone, never the modules under it.
            None
        } else {
            path.strip_prefix(row_path)
                .and_then(|rest| rest.strip_prefix('/'))
        };
        if let Some(rest) = rest
            && best.is_none_or(|(_, _, chosen)| row_path.len() > chosen)
        {
            best = Some((row, rest, row_path.len()));
        }
    }
    let (row, rest, _) = best?;
    let area = row
        .has_areas
        .then(|| {
            rest.split('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches(".rs")
        })
        .filter(|area| !area.is_empty() && *area != "mod")
        .map(str::to_string);
    Some(Place {
        layer: row.layer,
        area,
    })
}

/// True when an import from `from` to `to` breaks the layer order.
fn breaks_order(from: &Place, to: &Place) -> bool {
    use FrontendLayer::{Pages, Workspaces};
    if to.layer.rank() > from.layer.rank() {
        return true;
    }
    match (from.layer, to.layer) {
        (Pages, Workspaces) | (Workspaces, Pages) => true,
        (Pages, Pages) => from.area.is_some() && to.area.is_some() && from.area != to.area,
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/frontend_layering.rs"]
mod tests;
