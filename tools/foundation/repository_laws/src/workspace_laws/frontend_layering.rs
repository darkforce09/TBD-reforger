//! The frontend-layering law: a lower frontend layer never imports a higher one.
//!
//! **Role:** maps every production and test source of a frontend crate onto a layer — foundation
//! below features below pages and workspaces below the app shell — and, inside a folder whose
//! children are ordered, onto a sub-area, through a layer table the caller passes; resolves every
//! in-crate module path each file names; and reports every import edge that breaks the order: a
//! lower layer naming a higher one, pages naming workspaces or the reverse, one page area naming
//! another, a sub-area naming a sibling sub-area at or above its own tier, and a production file
//! naming a test-only sub-area.
//! **Position:** `cargo xtask verify frontend-layering` prints [`check_frontend_layering`]; xtask
//! owns the layer table and the sub-area orders, so a folder move rewrites the table and not this
//! law, which knows no folder name of any crate.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** the law is hard at zero: every edge is a finding. An edge is one (source file,
//! target place) pair, reported once at the first line naming it, production and test files
//! counted apart. A source file no table row maps, and a child of an ordered folder that sits in
//! no tier and is not test-only, are findings; a missing crate folder is [`NotRun::TargetMissing`].

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
    /// Crate-relative folder (`src/foundation`) or file (`src/main.rs`) the row maps.
    pub path: &'static str,
    /// The layer of everything under the path.
    pub layer: FrontendLayer,
    /// True when the next folder under the path names an area (a page area, a workspace).
    pub has_areas: bool,
}

/// The strict order of the children (sub-areas) of one folder.
///
/// A sub-area is a child folder or file module of [`SubAreaOrder::parent`]. A file in one
/// sub-area may import only sibling sub-areas in a strictly lower tier; the sub-areas of one tier
/// are peers that never import each other. A test-only sub-area sits outside the tiers: it may
/// import any sibling, and only test files, wherever they sit in the crate, may import it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubAreaOrder {
    /// Crate-relative folder whose children are ordered, e.g. `src/foundation`.
    pub parent: &'static str,
    /// The tiers, lowest first; each lists the names of its peer sub-areas.
    pub tiers: &'static [&'static [&'static str]],
    /// Sub-areas only test files may import.
    pub test_only: &'static [&'static str],
}

impl SubAreaOrder {
    /// The tier index of the sub-area `name`; `None` for a test-only or unlisted name.
    fn tier(&self, name: &str) -> Option<usize> {
        self.tiers.iter().position(|tier| tier.contains(&name))
    }

    /// True when `name` is a test-only sub-area.
    fn is_test_only(&self, name: &str) -> bool {
        self.test_only.contains(&name)
    }

    /// The sub-area the crate-relative `path` (a file, or a module path spelled as a folder path)
    /// sits in; `None` outside the parent and for the parent's own module.
    fn sub_area_of<'p>(&self, path: &'p str) -> Option<&'p str> {
        let rest = path.strip_prefix(self.parent)?.strip_prefix('/')?;
        let name = rest
            .split('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".rs");
        (!name.is_empty() && name != "mod").then_some(name)
    }
}

/// The layer table of one frontend crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendCrateLayers {
    /// The crate's folder, repository-relative.
    pub crate_path: &'static str,
    /// The rows; the longest matching path wins.
    pub rows: &'static [FrontendLayerRow],
    /// The folders whose children are ordered against each other.
    pub sub_area_orders: &'static [SubAreaOrder],
}

/// One import edge that breaks the layer or sub-area order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeringEdge {
    /// The importing file, repository-relative.
    pub file: String,
    /// The first line naming the target.
    pub line_no: usize,
    /// True when the importing file is a test file.
    pub test: bool,
    /// The importing file's place, e.g. `pages/mission_hub` or `foundation/transport`.
    pub from: String,
    /// The imported place, e.g. `workspaces/editor` or `foundation/auth`.
    pub to: String,
}

/// What one scan of a frontend crate found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayeringScan {
    /// Every import edge that breaks the order.
    pub edges: Vec<LayeringEdge>,
    /// Source files no row of the layer table maps, repository-relative.
    pub unmapped: Vec<String>,
    /// Children of an ordered folder that sit in no tier and are not test-only, as
    /// repository-relative module paths (`apps/web/src/foundation/stray`).
    pub unordered: BTreeSet<String>,
}

/// The layering report over `crates`: any edge fails the law.
pub fn check_frontend_layering(
    repo_root: &Path,
    crates: &[FrontendCrateLayers],
) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "frontend-layering",
        frontend_layering_outcome(repo_root, crates),
    )
}

/// The hard verdict over the layering edges of `crates`: one finding per edge.
pub fn frontend_layering_outcome(
    repo_root: &Path,
    crates: &[FrontendCrateLayers],
) -> Result<LawOutcome, NotRun> {
    let mut outcome = LawOutcome::default();
    let mut edges = Vec::new();
    for layers in crates {
        let scan = layering_edges(repo_root, layers)?;
        edges.extend(scan.edges);
        outcome.findings.extend(
            scan.unmapped
                .into_iter()
                .map(|file| format!("{file} sits under no row of the layer table")),
        );
        outcome.findings.extend(
            scan.unordered
                .into_iter()
                .map(|sub_area| format!("{sub_area} sits in no tier of its folder's order")),
        );
    }
    let production = edges.iter().filter(|edge| !edge.test).count();
    let test = edges.len() - production;
    outcome.summary =
        format!("{production} production and {test} test layering edge(s); the law allows none");
    outcome.findings.extend(edges.iter().map(|edge| {
        let kind = if edge.test { "test" } else { "production" };
        format!(
            "{kind} edge {}:{}: {} imports {}",
            edge.file, edge.line_no, edge.from, edge.to
        )
    }));
    Ok(outcome)
}

/// Every layering edge of one crate, the files no row maps and the unordered sub-areas.
pub fn layering_edges(
    repo_root: &Path,
    layers: &FrontendCrateLayers,
) -> Result<LayeringScan, NotRun> {
    let crate_root = repo_root.join(layers.crate_path);
    let source = crate_root.join("src");
    let files = scan::walk_files(&[source.as_path()], scan::with_extension(&["rs"]))?;
    let mut scan = LayeringScan::default();
    for file in files {
        let crate_rel = file
            .strip_prefix(&crate_root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let repo_rel = format!("{}/{crate_rel}", layers.crate_path);
        let Some(from) = place_of(layers, &crate_rel, false) else {
            scan.unmapped.push(repo_rel);
            continue;
        };
        for order in layers.sub_area_orders {
            if let Some(name) = order.sub_area_of(&crate_rel)
                && order.tier(name).is_none()
                && !order.is_test_only(name)
            {
                let parent = order.parent;
                scan.unordered
                    .insert(format!("{}/{parent}/{name}", layers.crate_path));
            }
        }
        let text = std::fs::read_to_string(&file).map_err(|source| NotRun::Unreadable {
            path: file.clone(),
            source,
        })?;
        let test = is_test_file(&repo_rel);
        let mut seen = BTreeSet::new();
        for reference in module_references(&text, &file_module(&crate_rel)) {
            let target = format!("src/{}", reference.segments.join("/"));
            let Some(to) = place_of(layers, &target, true) else {
                continue;
            };
            let breaks = breaks_order(&from, &to)
                || layers
                    .sub_area_orders
                    .iter()
                    .any(|order| breaks_sub_area_order(order, &crate_rel, &target, test));
            if breaks && seen.insert(to.label()) {
                scan.edges.push(LayeringEdge {
                    file: repo_rel.clone(),
                    line_no: reference.line_no,
                    test,
                    from: from.label(),
                    to: to.label(),
                });
            }
        }
    }
    Ok(scan)
}

/// A layer and, for a row with areas, the area; inside an ordered folder, the sub-area.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Place {
    layer: FrontendLayer,
    area: Option<String>,
    sub_area: Option<String>,
}

impl Place {
    fn label(&self) -> String {
        [
            Some(self.layer.name()),
            self.area.as_deref(),
            self.sub_area.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("/")
    }
}

/// The place of the crate-relative `path`: a source file, or (`module_path`) a module path
/// spelled as a folder path, where a file row `src/main.rs` maps the module `src/main`.
fn place_of(layers: &FrontendCrateLayers, path: &str, module_path: bool) -> Option<Place> {
    let mut best: Option<(&FrontendLayerRow, &str, usize)> = None;
    for row in layers.rows {
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
    let sub_area = layers
        .sub_area_orders
        .iter()
        .find_map(|order| order.sub_area_of(path))
        .map(str::to_string);
    Some(Place {
        layer: row.layer,
        area,
        sub_area,
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

/// True when the file at crate-relative `file` (a test file when `test`) naming the module path
/// `target` breaks `order`: a production file outside a test-only sub-area naming it, or a
/// sub-area naming a sibling whose tier is not strictly lower than its own.
fn breaks_sub_area_order(order: &SubAreaOrder, file: &str, target: &str, test: bool) -> bool {
    let Some(to) = order.sub_area_of(target) else {
        return false;
    };
    let from = order.sub_area_of(file);
    if from == Some(to) {
        return false;
    }
    if order.is_test_only(to) {
        return !test;
    }
    let Some(from) = from.filter(|from| !order.is_test_only(from)) else {
        return false;
    };
    match (order.tier(from), order.tier(to)) {
        (Some(from_tier), Some(to_tier)) => to_tier >= from_tier,
        // An unlisted sub-area is its own finding; its edges are judged once a tier lists it.
        _ => false,
    }
}

#[cfg(test)]
#[path = "tests/frontend_layering.rs"]
mod tests;
