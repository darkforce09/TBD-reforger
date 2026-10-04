//! The crate-edge mode of the frontend-layering law: the dependency edges between frontend crates.
//!
//! **Role:** places every frontend crate — each workspace member under the frontend crates root,
//! by the layer folder it sits in, and the app crate as the shell — and judges every dependency
//! edge (normal, dev and build) from one frontend crate to another: the layer order of
//! [`super::breaks_order`] (page crates are areas, so two page crates never depend on each other),
//! the crate orders ([`SubAreaOrder`] over package names, a test-only crate reached only through
//! dev-dependencies), and the independence of two orders of one layer folder, whose crates never
//! depend on each other. A frontend crate outside every layer folder, a crate in a layer folder
//! that has orders but sits in none of them, and a crate an order names sitting in another layer
//! folder are findings.
//! **Position:** a child of [`super`]; [`super::frontend_layering_outcome`] runs it after the
//! in-crate mode over the members [`read_workspace_members`] reads. xtask passes the
//! configuration ([`FrontendCrateEdges`]); the mode knows no crate or folder name.
//! **Signals & state:** none; reads the checkout's manifests.
//! **Invariants:** an edge is one (manifest, target crate, dev or not) triple, reported at the
//! first line declaring it; a crate an order names that no member carries is listed in
//! [`CrateEdgeScan::absent`], and [`super::frontend_layering_outcome`] reports it as a finding;
//! an app crate that is no member is [`NotRun::TargetMissing`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{FrontendLayer, LayeringEdge, Place, SubAreaOrder, breaks_order};
use crate::workspace_members::{WorkspaceMember, read_workspace_members};
use verification_core::verdict::NotRun;

/// One layer folder of the frontend crates: every crate directly inside it has its layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendLayerFolder {
    /// The folder, repository-relative (`crates/frontend/foundation`).
    pub path: &'static str,
    /// The layer of every crate in the folder.
    pub layer: FrontendLayer,
}

/// The configuration of the crate-edge mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrontendCrateEdges {
    /// The app crate's folder, repository-relative; the app is the shell layer.
    pub shell_crate: &'static str,
    /// The folder every frontend library crate sits under (`crates/frontend`).
    pub crates_root: &'static str,
    /// The layer folders under [`FrontendCrateEdges::crates_root`].
    pub layer_folders: &'static [FrontendLayerFolder],
    /// The crate orders; each [`SubAreaOrder::parent`] is a layer folder and its names are
    /// package names. Two orders of one layer folder are independent.
    pub crate_orders: &'static [SubAreaOrder],
}

/// What one crate-edge scan found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrateEdgeScan {
    /// Every frontend crate placed, the app included, by repository-relative path.
    pub crates: Vec<String>,
    /// Every dependency edge that breaks an order; `file` is the depending crate's manifest.
    pub edges: Vec<LayeringEdge>,
    /// Frontend crates in no layer folder, in no order of a layer folder that has orders, or in
    /// another layer folder than their order's: one finding line each.
    pub misplaced: Vec<String>,
    /// Package names an order lists that no workspace member carries: one finding line each.
    pub absent: Vec<String>,
}

/// The crate-edge scan of the checkout at `repo_root` under `config`.
pub fn crate_edge_scan(
    repo_root: &Path,
    config: &FrontendCrateEdges,
) -> Result<CrateEdgeScan, NotRun> {
    let members = read_workspace_members(repo_root)?;
    let shell = members
        .iter()
        .find(|member| member.path == config.shell_crate)
        .ok_or_else(|| NotRun::TargetMissing(repo_root.join(config.shell_crate)))?;
    let mut scan = CrateEdgeScan::default();
    let mut places: BTreeMap<&str, (&WorkspaceMember, Place)> = BTreeMap::new();
    places.insert(
        &shell.package_name,
        (shell, place(FrontendLayer::Shell, None)),
    );
    let root = format!("{}/", config.crates_root);
    for member in members
        .iter()
        .filter(|member| member.path.starts_with(&root))
    {
        let folder = member.parent_folder();
        let Some(layer_folder) = config.layer_folders.iter().find(|f| f.path == folder) else {
            scan.misplaced.push(format!(
                "{} sits in no frontend layer folder ({})",
                member.path,
                folder_list(config)
            ));
            continue;
        };
        let orders_here = || config.crate_orders.iter().filter(|o| o.parent == folder);
        if orders_here().next().is_some()
            && !orders_here().any(|order| order.lists(&member.package_name))
        {
            scan.misplaced.push(format!(
                "{} sits in no crate order of {folder}",
                member.path
            ));
        }
        let area = Some(member.package_name.clone());
        places.insert(
            &member.package_name,
            (member, place(layer_folder.layer, area)),
        );
    }
    for order in config.crate_orders {
        for name in order.names() {
            match members.iter().find(|member| member.package_name == name) {
                None => scan.absent.push(name.to_string()),
                Some(member) if member.parent_folder() != order.parent => {
                    scan.misplaced.push(format!(
                        "{} is named in the crate order of {} but sits in {}",
                        member.path,
                        order.parent,
                        member.parent_folder()
                    ));
                }
                Some(_) => {}
            }
        }
    }
    for (from_name, (member, from)) in &places {
        let mut seen = BTreeSet::new();
        for edge in &member.manifest.dependencies {
            let to_name = edge.package.as_str();
            let Some((_, to)) = places.get(to_name) else {
                continue;
            };
            let dev = edge.is_dev_dependency();
            if to_name != *from_name
                && breaks_crate_order(config, (from_name, from), (to_name, to), dev)
                && seen.insert((to_name, dev))
            {
                scan.edges.push(LayeringEdge {
                    file: format!("{}/Cargo.toml", member.path),
                    line_no: edge.line_no,
                    test: dev,
                    from: from.label(),
                    to: to.label(),
                });
            }
        }
    }
    scan.crates = places
        .values()
        .map(|(member, _)| member.path.clone())
        .collect();
    scan.crates.sort();
    scan.absent.sort();
    scan.absent.dedup();
    Ok(scan)
}

/// True when the crate `from` depending on the crate `to` (through a dev-dependency when `dev`)
/// breaks the layer order, a crate order, or the independence of two orders of one layer folder.
fn breaks_crate_order(
    config: &FrontendCrateEdges,
    (from_name, from): (&str, &Place),
    (to_name, to): (&str, &Place),
    dev: bool,
) -> bool {
    if breaks_order(from, to) {
        return true;
    }
    let breaks_an_order = config
        .crate_orders
        .iter()
        .filter(|order| order.lists(to_name))
        .any(|order| order.breaks(order.lists(from_name).then_some(from_name), to_name, dev));
    let order_of = |name: &str| config.crate_orders.iter().find(|order| order.lists(name));
    let separate_orders = match (order_of(from_name), order_of(to_name)) {
        (Some(from_order), Some(to_order)) => {
            from_order.parent == to_order.parent && from_order != to_order
        }
        _ => false,
    };
    breaks_an_order || separate_orders
}

/// The place of a crate in `layer`, its package name the area.
fn place(layer: FrontendLayer, area: Option<String>) -> Place {
    Place {
        layer,
        area,
        sub_area: None,
    }
}

/// The configured layer folders, comma-separated.
fn folder_list(config: &FrontendCrateEdges) -> String {
    config
        .layer_folders
        .iter()
        .map(|folder| folder.path)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "../tests/frontend_layering_crate_edges.rs"]
mod tests;
