//! The frontend-layering law: a lower frontend layer never imports a higher one.
//!
//! **Role:** judges the frontend in two modes. The in-crate mode maps every production and test
//! source of a crate that still holds several layers onto a layer — foundation below features
//! below pages and workspaces below the app shell — and, inside a folder whose children are
//! ordered, onto a sub-area, through a layer table the caller passes; it resolves every in-crate
//! module path each file names and reports every import edge that breaks the order: a lower layer
//! naming a higher one, pages naming workspaces or the reverse, one page area naming another, a
//! sub-area naming a sibling above its own tier (or a peer of its tier, unless the tier is one
//! mutual group), and a production file naming a test-only sub-area. The crate-edge mode
//! ([`crate_edges`]) judges the same orders over the dependency edges between frontend crates.
//! **Position:** `cargo xtask verify frontend-layering` prints [`check_frontend_layering`]; xtask
//! owns the layer tables, the sub-area orders and the crate orders, so a folder move or a crate
//! birth rewrites the configuration and not this law, which knows no folder or crate name.
//! **Signals & state:** none; reads the checkout.
//! **Invariants:** the law is hard at zero: every edge is a finding. An in-crate edge is one
//! (source file, target place) pair, reported once at the first line naming it, production and
//! test files counted apart. A source file no table row maps, and a child of an ordered folder
//! that sits in no tier and is not test-only, are findings; a missing crate source folder is
//! [`NotRun::TargetMissing`]; a configured row or ordered folder that no longer exists is a note.
//! A crate a crate order names that no workspace member carries is a finding.

pub mod crate_edges;

use std::collections::BTreeSet;
use std::path::Path;

use super::rust_module_references::{file_module, module_references};
use super::{LawOutcome, WorkspaceLawReport};
use crate::source_roots::is_test_file;
use crate_edges::{FrontendCrateEdges, crate_edge_scan};
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

/// One tier of a [`SubAreaOrder`]: the sub-areas that share a rank.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubAreaTier {
    /// The names of the tier's sub-areas.
    pub names: &'static [&'static str],
    /// True when the tier is one mutual group whose sub-areas may import each other; false when
    /// they are peers that never import each other.
    pub mutual: bool,
}

impl SubAreaTier {
    /// A tier of peers that never import each other.
    pub const fn peers(names: &'static [&'static str]) -> Self {
        Self {
            names,
            mutual: false,
        }
    }

    /// A tier that is one mutual group: its sub-areas may import each other.
    pub const fn group(names: &'static [&'static str]) -> Self {
        Self {
            names,
            mutual: true,
        }
    }
}

/// The strict order of the children (sub-areas) of one folder.
///
/// A sub-area is a child folder or file module of [`SubAreaOrder::parent`] in the in-crate mode,
/// and a crate (by package name) whose folder sits in [`SubAreaOrder::parent`] in the crate-edge
/// mode. A sub-area may import only sub-areas in a strictly lower tier, and the sub-areas of its
/// own tier only when that tier is a mutual group ([`SubAreaTier::group`]). A test-only sub-area
/// sits outside the tiers: it may import any sibling, and only test files (in the crate-edge
/// mode, dev-dependency edges), wherever they sit, may import it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubAreaOrder {
    /// The folder whose children are ordered: crate-relative in the in-crate mode
    /// (`src/foundation`), repository-relative in the crate-edge mode
    /// (`crates/frontend/foundation`).
    pub parent: &'static str,
    /// The tiers, lowest first.
    pub tiers: &'static [SubAreaTier],
    /// Sub-areas only test files (dev-dependency edges) may import.
    pub test_only: &'static [&'static str],
}

impl SubAreaOrder {
    /// The tier index of the sub-area `name`; `None` for a test-only or unlisted name.
    fn tier(&self, name: &str) -> Option<usize> {
        self.tiers
            .iter()
            .position(|tier| tier.names.contains(&name))
    }

    /// True when `name` is a test-only sub-area.
    fn is_test_only(&self, name: &str) -> bool {
        self.test_only.contains(&name)
    }

    /// True when the order lists `name`, in a tier or as test-only.
    fn lists(&self, name: &str) -> bool {
        self.tier(name).is_some() || self.is_test_only(name)
    }

    /// Every name the order lists, tiers first.
    fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tiers
            .iter()
            .flat_map(|tier| tier.names.iter().copied())
            .chain(self.test_only.iter().copied())
    }

    /// True when the sub-area `from` (`None` outside the order) naming the sibling `to` breaks
    /// the order; `test` marks a test file or a dev-dependency edge.
    fn breaks(&self, from: Option<&str>, to: &str, test: bool) -> bool {
        if from == Some(to) {
            return false;
        }
        if self.is_test_only(to) {
            return !test;
        }
        let Some(from) = from.filter(|from| !self.is_test_only(from)) else {
            return false;
        };
        match (self.tier(from), self.tier(to)) {
            (Some(from_tier), Some(to_tier)) => {
                to_tier > from_tier || (to_tier == from_tier && !self.tiers[from_tier].mutual)
            }
            // An unlisted sub-area is its own finding; its edges are judged once a tier lists it.
            _ => false,
        }
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

/// The in-crate layer table of one frontend crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendCrateLayers {
    /// The crate's folder, repository-relative.
    pub crate_path: &'static str,
    /// The rows; the longest matching path wins.
    pub rows: &'static [FrontendLayerRow],
    /// The folders whose children are ordered against each other.
    pub sub_area_orders: &'static [SubAreaOrder],
}

/// The whole configuration of the law: both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendLayering {
    /// The in-crate mode: the layer table of every crate whose modules are judged.
    pub in_crate: &'static [FrontendCrateLayers],
    /// The crate-edge mode: the layer folders and crate orders of the frontend crates.
    pub crate_edges: FrontendCrateEdges,
}

/// One import or dependency edge that breaks the layer or sub-area order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayeringEdge {
    /// The importing file (a source file, or a crate's `Cargo.toml`), repository-relative.
    pub file: String,
    /// The first line naming the target.
    pub line_no: usize,
    /// True when the importing file is a test file, or the edge a dev-dependency.
    pub test: bool,
    /// The importing place, e.g. `pages/mission_hub`, `foundation/transport` or (a crate)
    /// `foundation/frontend_ui`.
    pub from: String,
    /// The imported place, e.g. `workspaces/editor` or `foundation/auth`.
    pub to: String,
}

/// What one in-crate scan of a frontend crate found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayeringScan {
    /// Every import edge that breaks the order.
    pub edges: Vec<LayeringEdge>,
    /// Source files no row of the layer table maps, repository-relative.
    pub unmapped: Vec<String>,
    /// Children of an ordered folder that sit in no tier and are not test-only, as
    /// repository-relative module paths (`apps/web/src/foundation/stray`).
    pub unordered: BTreeSet<String>,
    /// Configured rows and ordered folders that no longer exist, repository-relative.
    pub absent_paths: Vec<String>,
}

/// The layering report over both modes of `layering`: any edge fails the law.
pub fn check_frontend_layering(
    repo_root: &Path,
    layering: &FrontendLayering,
) -> WorkspaceLawReport {
    WorkspaceLawReport::from_outcome(
        "frontend-layering",
        frontend_layering_outcome(repo_root, layering),
    )
}

/// The hard verdict over the layering edges of both modes: one finding per edge.
pub fn frontend_layering_outcome(
    repo_root: &Path,
    layering: &FrontendLayering,
) -> Result<LawOutcome, NotRun> {
    let mut outcome = LawOutcome::default();
    let mut edges = Vec::new();
    for layers in layering.in_crate {
        let scan = layering_edges(repo_root, layers)?;
        edges.extend(scan.edges);
        outcome
            .notes
            .extend(scan.absent_paths.into_iter().map(|path| {
                format!("{path} is configured in the layer table but does not exist; drop its row")
            }));
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
    let crates = crate_edge_scan(repo_root, &layering.crate_edges)?;
    outcome.findings.extend(crates.absent.iter().map(|name| {
        format!("{name} is named in a frontend crate order but is no workspace member")
    }));
    outcome.findings.extend(crates.misplaced.iter().cloned());
    let production = edges.iter().filter(|edge| !edge.test).count();
    let normal = crates.edges.iter().filter(|edge| !edge.test).count();
    outcome.summary = format!(
        "{production} production and {} test layering edge(s) between modules; {normal} normal \
         and {} dev edge(s) between {} frontend crate(s); the law allows none",
        edges.len() - production,
        crates.edges.len() - normal,
        crates.crates.len()
    );
    outcome.findings.extend(edges.iter().map(|edge| {
        let kind = if edge.test { "test" } else { "production" };
        format!(
            "{kind} edge {}:{}: {} imports {}",
            edge.file, edge.line_no, edge.from, edge.to
        )
    }));
    outcome.findings.extend(crates.edges.iter().map(|edge| {
        let kind = if edge.test { "dev" } else { "normal" };
        format!(
            "{kind} crate edge {}:{}: {} depends on {}",
            edge.file, edge.line_no, edge.from, edge.to
        )
    }));
    Ok(outcome)
}

/// Every in-crate layering edge of one crate, the files no row maps, the unordered sub-areas
/// and the configured paths that no longer exist.
pub fn layering_edges(
    repo_root: &Path,
    layers: &FrontendCrateLayers,
) -> Result<LayeringScan, NotRun> {
    let crate_root = repo_root.join(layers.crate_path);
    let source = crate_root.join("src");
    let files = scan::walk_files(&[source.as_path()], scan::with_extension(&["rs"]))?;
    let mut scan = LayeringScan::default();
    let configured = layers
        .rows
        .iter()
        .map(|row| row.path)
        .chain(layers.sub_area_orders.iter().map(|order| order.parent));
    for path in configured {
        if !crate_root.join(path).exists() {
            scan.absent_paths
                .push(format!("{}/{path}", layers.crate_path));
        }
    }
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
                && !order.lists(name)
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

/// A layer and, for a row with areas (or a crate), the area; inside an ordered folder, the
/// sub-area.
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

/// True when an import from `from` to `to` breaks the layer order: a lower layer naming a
/// higher one, pages and workspaces naming each other, or one page area naming another.
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
/// `target` breaks `order`: a production file naming a test-only sub-area, or a sub-area naming
/// a sibling the order puts out of its reach ([`SubAreaOrder::breaks`]).
fn breaks_sub_area_order(order: &SubAreaOrder, file: &str, target: &str, test: bool) -> bool {
    order
        .sub_area_of(target)
        .is_some_and(|to| order.breaks(order.sub_area_of(file), to, test))
}

#[cfg(test)]
#[path = "tests/frontend_layering.rs"]
mod tests;
