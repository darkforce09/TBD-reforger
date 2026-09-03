//! `cargo xtask map instances-verify --instances <slug>.instances.json --recon
//! <slug>_children.json` — the T-090.11.3 socket-transform check: every architectural instance
//! the offline pipeline placed from an XOB socket (`source: xobSocket`) is matched to the
//! Workbench recon dump of the live entity hierarchy and must agree within [`POS_TOL_M`] /
//! [`YAW_TOL_DEG`].
//!
//! Matching: the recon (as shipped in the addon at the time of the first dump) records class,
//! components, bounds size, world yaw and `relPos` (world-axis offset from the building origin)
//! but an empty `resource` for prefab-nested children, so children are bucketed into a
//! [`Group`] from class + components (window sets are `Building`, door sets are depth-1
//! `GenericEntity`, leaves carry `DoorComponent`, panes carry
//! `SCR_DestructionMultiPhaseComponent`, entries are `StaticModelEntity`) and paired with the
//! instances of the same group by greedy nearest position in the building's local frame.
//! Instances that descend from a furniture instance are skipped: the world places the furniture
//! composition as a sibling entity, not under the building, so the recon cannot see them.
//!
//! Frames: the building's `rootAngles` turn `relPos` / world yaw into the local frame the
//! instances use. The rotation handedness is not assumed: both yaw signs are tried and the one
//! with the smaller total position error is reported (the T-090.11.3 handedness pin).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use map_engine_core::building_compound::{
    InstanceKind, InstanceRecord, InstancesFile, PlacementSource,
};
use map_engine_core::geometry::rigid::Rigid;
use serde::Deserialize;

pub const POS_TOL_M: f64 = 0.02;
pub const YAW_TOL_DEG: f64 = 1.0;
/// Candidate pairs farther apart than this are never matched (keeps a missing entity from
/// stealing a neighbour's partner).
const MATCH_CAP_M: f64 = 1.5;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconFile {
    pub slug: String,
    pub root_angles: [f64; 3],
    pub children: Vec<ReconChild>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReconChild {
    pub depth: u32,
    #[serde(default)]
    pub name: String,
    pub class: String,
    #[serde(default)]
    pub resource: String,
    pub rel_pos: [f64; 3],
    pub yaw_deg: f64,
    #[serde(default)]
    pub size: [f64; 3],
    #[serde(default)]
    pub components: Vec<String>,
    // ── T-090.11.3 enrichment (present once the recon plugin is compiled with ExtrasJson) ──
    /// The `Hierarchy` component's `PivotID` — the socket the child hangs on.
    #[serde(default)]
    pub pivot_id: String,
    /// Origin in the PARENT entity's frame (`parent.CoordToLocal`).
    #[serde(default)]
    pub local_pos: Option<[f64; 3]>,
    /// World `[pitch, yaw, roll]`.
    #[serde(default)]
    pub angles_deg: Option<[f64; 3]>,
    /// `DoorComponent` params on a leaf.
    #[serde(default)]
    pub door: Option<ReconDoor>,
}

/// The recon's `door` object (a `DoorComponent`'s hinge params).
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReconDoor {
    #[serde(default)]
    pub angle_range: f64,
    #[serde(default)]
    pub closed_angle: f64,
    #[serde(default)]
    pub initial_angle: f64,
}

/// Architectural bucket shared by recon children and instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    WindowFrame,
    DoorFrame,
    DoorLeaf,
    Glass,
    Prop,
    Other,
}

impl ReconChild {
    pub fn group(&self) -> Group {
        let has = |c: &str| self.components.iter().any(|k| k == c);
        if has("DoorComponent") {
            Group::DoorLeaf
        } else if self.class == "Building" {
            Group::WindowFrame
        } else if self.class == "StaticModelEntity" {
            Group::Prop
        } else if self.depth == 1 {
            Group::DoorFrame
        } else if has("SCR_DestructionMultiPhaseComponent") || self.size[2].abs() < 1e-3 {
            Group::Glass
        } else {
            Group::Other
        }
    }
}

fn instance_group(kind: InstanceKind) -> Group {
    match kind {
        InstanceKind::WindowFrame => Group::WindowFrame,
        InstanceKind::DoorFrame => Group::DoorFrame,
        InstanceKind::DoorLeaf => Group::DoorLeaf,
        InstanceKind::Glass => Group::Glass,
        InstanceKind::Prop => Group::Prop,
        _ => Group::Other,
    }
}

/// One instance ↔ recon child pairing.
#[derive(Debug, Clone)]
pub struct Match {
    pub instance: String,
    pub child_index: usize,
    pub group: Group,
    pub pos_err_m: f64,
    pub yaw_err_deg: f64,
}

#[derive(Debug, Default)]
pub struct Report {
    /// Yaw sign used for the world → local rotation (`+1` = `Rigid::from_enfusion` as is).
    pub yaw_sign: f64,
    pub matches: Vec<Match>,
    pub unmatched: Vec<String>,
    /// Recon children no instance claimed (index, group).
    pub extra: Vec<(usize, Group)>,
    /// Instances skipped because they descend from a furniture instance.
    pub skipped_furniture: usize,
    /// Enriched-recon checks: how many were possible, and the failures (`instance: detail`).
    pub door_checks: usize,
    pub door_mismatches: Vec<String>,
    pub pivot_checks: usize,
    pub pivot_mismatches: Vec<String>,
    pub local_checks: usize,
    pub local_mismatches: Vec<String>,
}

impl Report {
    pub fn failures(&self) -> Vec<&Match> {
        self.matches
            .iter()
            .filter(|m| m.pos_err_m > POS_TOL_M || m.yaw_err_deg > YAW_TOL_DEG)
            .collect()
    }
    pub fn worst_pos_m(&self) -> f64 {
        self.matches.iter().map(|m| m.pos_err_m).fold(0.0, f64::max)
    }
    pub fn worst_yaw_deg(&self) -> f64 {
        self.matches
            .iter()
            .map(|m| m.yaw_err_deg)
            .fold(0.0, f64::max)
    }
    pub fn ok(&self) -> bool {
        self.failures().is_empty()
            && self.unmatched.is_empty()
            && self.door_mismatches.is_empty()
            && self.pivot_mismatches.is_empty()
            && self.local_mismatches.is_empty()
    }
}

pub fn wrap_deg(d: f64) -> f64 {
    let mut x = d % 360.0;
    if x > 180.0 {
        x -= 360.0;
    }
    if x <= -180.0 {
        x += 360.0;
    }
    x
}

/// Match under one yaw-sign hypothesis: greedy nearest position within each group.
fn evaluate(instances: &[&InstanceRecord], recon: &ReconFile, yaw_sign: f64) -> Report {
    let root = Rigid::from_enfusion([0.0; 3], [0.0, yaw_sign * recon.root_angles[1], 0.0], 1.0);
    let inv = root.inverse();
    let locals: Vec<([f64; 3], f64, Group)> = recon
        .children
        .iter()
        .map(|c| {
            (
                inv.point(c.rel_pos),
                wrap_deg(yaw_sign * (c.yaw_deg - recon.root_angles[1])),
                c.group(),
            )
        })
        .collect();
    let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
    for (ii, inst) in instances.iter().enumerate() {
        let g = instance_group(inst.kind);
        for (ci, (p, _, cg)) in locals.iter().enumerate() {
            if *cg != g {
                continue;
            }
            let d = (0..3)
                .map(|a| (p[a] - inst.local.pos[a]).powi(2))
                .sum::<f64>()
                .sqrt();
            if d <= MATCH_CAP_M {
                pairs.push((d, ii, ci));
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut inst_used = vec![false; instances.len()];
    let mut child_used = vec![false; recon.children.len()];
    let mut report = Report {
        yaw_sign,
        ..Report::default()
    };
    for (d, ii, ci) in pairs {
        if inst_used[ii] || child_used[ci] {
            continue;
        }
        inst_used[ii] = true;
        child_used[ci] = true;
        let yaw = wrap_deg(instances[ii].local.rigid().yaw_deg());
        report.matches.push(Match {
            instance: instances[ii].id.clone(),
            child_index: ci,
            group: locals[ci].2,
            pos_err_m: d,
            yaw_err_deg: wrap_deg(locals[ci].1 - yaw).abs(),
        });
    }
    for (ii, inst) in instances.iter().enumerate() {
        if !inst_used[ii] {
            report
                .unmatched
                .push(format!("{} ({:?}, {})", inst.id, inst.kind, inst.prefab));
        }
    }
    for (ci, used) in child_used.iter().enumerate() {
        if !used {
            report.extra.push((ci, locals[ci].2));
        }
    }
    report.matches.sort_by(|a, b| a.instance.cmp(&b.instance));
    report
}

/// Instances placed from XOB sockets that the world nests under the building — everything
/// except furniture instances and their descendants.
fn architectural(file: &InstancesFile) -> (Vec<&InstanceRecord>, usize) {
    let kinds: HashMap<&str, InstanceKind> = file
        .instances
        .iter()
        .map(|i| (i.id.as_str(), i.kind))
        .collect();
    let parents: HashMap<&str, Option<&str>> = file
        .instances
        .iter()
        .map(|i| (i.id.as_str(), i.parent.as_deref()))
        .collect();
    let under_furniture = |id: &str| {
        if kinds.get(id) == Some(&InstanceKind::Furniture) {
            return true;
        }
        let mut cur = parents.get(id).copied().flatten();
        let mut hops = 0;
        while let Some(p) = cur {
            if kinds.get(p) == Some(&InstanceKind::Furniture) {
                return true;
            }
            hops += 1;
            if hops > 64 {
                break;
            }
            cur = parents.get(p).copied().flatten();
        }
        false
    };
    let mut skipped = 0;
    let kept = file
        .instances
        .iter()
        .filter(|i| i.source == PlacementSource::XobSocket)
        .filter(|i| {
            if under_furniture(&i.id) {
                skipped += 1;
                false
            } else {
                true
            }
        })
        .collect();
    (kept, skipped)
}

/// Try both handedness hypotheses; keep the one with the smaller total position error.
pub fn verify(file: &InstancesFile, recon: &ReconFile) -> Report {
    let (socketed, skipped) = architectural(file);
    let mut a = evaluate(&socketed, recon, 1.0);
    let mut b = evaluate(&socketed, recon, -1.0);
    a.skipped_furniture = skipped;
    b.skipped_furniture = skipped;
    let score = |r: &Report| {
        r.matches.iter().map(|m| m.pos_err_m).sum::<f64>() + r.unmatched.len() as f64 * MATCH_CAP_M
    };
    let mut best = if score(&b) < score(&a) { b } else { a };
    enrichment_checks(file, recon, &mut best);
    best
}

/// The T-090.11.3 enrichment, when the dump carries it: a leaf's hinge params must equal the
/// instance's `DoorRecord`, the child's `pivotId` must be the instance id's last segment, and
/// its parent-frame origin must match the instance placed under its parent (2 cm).
fn enrichment_checks(file: &InstancesFile, recon: &ReconFile, report: &mut Report) {
    let by_id: HashMap<&str, &InstanceRecord> =
        file.instances.iter().map(|i| (i.id.as_str(), i)).collect();
    for m in &report.matches {
        let Some(inst) = by_id.get(m.instance.as_str()) else {
            continue;
        };
        let child = &recon.children[m.child_index];
        if let Some(d) = child.door {
            report.door_checks += 1;
            match inst.door {
                Some(r)
                    if (r.angle_range_deg - d.angle_range).abs() < 1e-3
                        && (r.closed_angle_deg - d.closed_angle).abs() < 1e-3
                        && (r.initial_angle_deg - d.initial_angle).abs() < 1e-3 => {}
                other => report.door_mismatches.push(format!(
                    "{}: recon door {:?} vs instance {:?}",
                    inst.id, d, other
                )),
            }
        }
        if !child.pivot_id.is_empty() {
            report.pivot_checks += 1;
            let tail = inst.id.rsplit('/').next().unwrap_or(&inst.id);
            if !tail.eq_ignore_ascii_case(&child.pivot_id) {
                report.pivot_mismatches.push(format!(
                    "{}: recon pivot {:?} vs id tail {:?}",
                    inst.id, child.pivot_id, tail
                ));
            }
        }
        if let (Some(lp), Some(parent_id)) = (child.local_pos, inst.parent.as_deref()) {
            if let Some(parent) = by_id.get(parent_id) {
                report.local_checks += 1;
                let mine = parent.local.rigid().inverse().point(inst.local.pos);
                let d = (0..3)
                    .map(|a| (mine[a] - lp[a]).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if d > POS_TOL_M {
                    report.local_mismatches.push(format!(
                        "{}: parent-frame origin {:?} vs recon localPos {:?} ({:.4} m)",
                        inst.id, mine, lp, d
                    ));
                }
            }
        }
    }
}

pub fn load(instances: &PathBuf, recon: &PathBuf) -> Result<(InstancesFile, ReconFile)> {
    let file: InstancesFile = serde_json::from_str(
        &fs::read_to_string(instances).with_context(|| instances.display().to_string())?,
    )
    .context("parse instances JSON")?;
    let dump: ReconFile = serde_json::from_str(
        &fs::read_to_string(recon).with_context(|| recon.display().to_string())?,
    )
    .context("parse recon JSON")?;
    if dump.children.is_empty() {
        bail!("recon dump has no children");
    }
    Ok((file, dump))
}

pub fn run_instances_verify(args: &[String]) -> Result<u8> {
    let mut instances: Option<PathBuf> = None;
    let mut recon: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--instances" if i + 1 < args.len() => {
                instances = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--recon" if i + 1 < args.len() => {
                recon = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            other => {
                eprintln!(
                    "instances-verify: unknown arg {other} (usage: --instances <slug>.instances.json --recon <slug>_children.json)"
                );
                return Ok(1);
            }
        }
    }
    let instances = instances.context("--instances <file> is required")?;
    let recon = recon.context("--recon <file> is required")?;
    let (file, dump) = load(&instances, &recon)?;
    let report = verify(&file, &dump);
    let failures = report.failures();
    println!(
        "instances-verify {} vs recon {}: {} socket instances ({} furniture descendants skipped) · {} matched · {} unmatched · {} recon children unclaimed · yaw sign {:+} · worst pos {:.4} m · worst yaw {:.3}° · {} over tolerance ({} m / {}°)",
        file.prefab_id,
        dump.slug,
        report.matches.len() + report.unmatched.len(),
        report.skipped_furniture,
        report.matches.len(),
        report.unmatched.len(),
        report.extra.len(),
        report.yaw_sign,
        report.worst_pos_m(),
        report.worst_yaw_deg(),
        failures.len(),
        POS_TOL_M,
        YAW_TOL_DEG
    );
    let mut per_group: HashMap<Group, (usize, f64, f64)> = HashMap::new();
    for m in &report.matches {
        let e = per_group.entry(m.group).or_insert((0, 0.0, 0.0));
        e.0 += 1;
        e.1 = e.1.max(m.pos_err_m);
        e.2 = e.2.max(m.yaw_err_deg);
    }
    let mut groups: Vec<_> = per_group.into_iter().collect();
    groups.sort_by_key(|(g, _)| format!("{g:?}"));
    for (g, (n, p, y)) in groups {
        println!("  {g:?}: {n} matched · worst pos {p:.4} m · worst yaw {y:.3}°");
    }
    for m in &failures {
        let c = &dump.children[m.child_index];
        println!(
            "  MISMATCH {} ↔ child #{} ({:?}, depth {}, size {:?}, name {:?}, resource {:?}): pos {:.3} m · yaw {:.2}°",
            m.instance,
            m.child_index,
            m.group,
            c.depth,
            c.size,
            c.name,
            c.resource,
            m.pos_err_m,
            m.yaw_err_deg
        );
    }
    println!(
        "  enrichment: door params {}/{} ok · pivot ids {}/{} ok · parent-frame origins {}/{} ok",
        report.door_checks - report.door_mismatches.len(),
        report.door_checks,
        report.pivot_checks - report.pivot_mismatches.len(),
        report.pivot_checks,
        report.local_checks - report.local_mismatches.len(),
        report.local_checks
    );
    for m in report
        .door_mismatches
        .iter()
        .chain(&report.pivot_mismatches)
        .chain(&report.local_mismatches)
    {
        println!("  ENRICHMENT MISMATCH {m}");
    }
    for u in &report.unmatched {
        println!("  UNMATCHED {u}");
    }
    for (ci, g) in &report.extra {
        let c = &dump.children[*ci];
        println!(
            "  UNCLAIMED child #{ci} ({g:?}, depth {}, size {:?}, relPos {:?}, angles {:?})",
            c.depth, c.size, c.rel_pos, c.angles_deg
        );
    }
    Ok(u8::from(!report.ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use map_engine_core::building_compound::{CoverTier, LocalTransform};

    fn inst(
        id: &str,
        kind: InstanceKind,
        pos: [f64; 3],
        yaw: f64,
        parent: Option<&str>,
    ) -> InstanceRecord {
        InstanceRecord {
            id: id.into(),
            kind,
            prefab: format!("Prefabs/{id}.et"),
            blas: "blas/x.bvh".into(),
            xob: None,
            local: LocalTransform::from_rigid(&Rigid::from_enfusion(pos, [0.0, yaw, 0.0], 1.0)),
            door: None,
            cover: CoverTier::None,
            source: PlacementSource::XobSocket,
            parent: parent.map(str::to_string),
        }
    }

    fn child(depth: u32, class: &str, comps: &[&str], rel: [f64; 3], yaw: f64) -> ReconChild {
        ReconChild {
            depth,
            name: String::new(),
            class: class.into(),
            resource: String::new(),
            rel_pos: rel,
            yaw_deg: yaw,
            size: [1.0, 1.0, if class == "Building" { 0.5 } else { 0.0 }],
            components: comps.iter().map(|s| s.to_string()).collect(),
            pivot_id: String::new(),
            local_pos: None,
            angles_deg: None,
            door: None,
        }
    }

    fn file(instances: Vec<InstanceRecord>) -> InstancesFile {
        InstancesFile {
            schema_version: "1.0.0".into(),
            prefab_id: "T".into(),
            resource_name: "Prefabs/T.et".into(),
            shell_bvh: "T.bvh".into(),
            instances,
            notes: vec![],
        }
    }

    #[test]
    fn recon_groups_from_class_and_components() {
        assert_eq!(
            child(1, "Building", &["MeshObject"], [0.0; 3], 0.0).group(),
            Group::WindowFrame
        );
        assert_eq!(
            child(1, "GenericEntity", &["DoorSlotComponent"], [0.0; 3], 0.0).group(),
            Group::DoorFrame
        );
        assert_eq!(
            child(2, "GenericEntity", &["DoorComponent"], [0.0; 3], 0.0).group(),
            Group::DoorLeaf
        );
        assert_eq!(
            child(
                2,
                "GenericEntity",
                &["SCR_DestructionMultiPhaseComponent"],
                [0.0; 3],
                0.0
            )
            .group(),
            Group::Glass
        );
        assert_eq!(
            child(1, "StaticModelEntity", &[], [0.0; 3], 0.0).group(),
            Group::Prop
        );
        assert_eq!(wrap_deg(370.0), 10.0);
        assert_eq!(wrap_deg(-190.0), 170.0);
    }

    #[test]
    fn matches_through_the_building_yaw_and_skips_furniture_descendants() {
        // Building yawed 90° in the world: a local child sits at world offset R_y(90)·local.
        let root_yaw = 90.0;
        let world =
            |p: [f64; 3]| Rigid::from_enfusion([0.0; 3], [0.0, root_yaw, 0.0], 1.0).point(p);
        let f = file(vec![
            inst(
                "win_1",
                InstanceKind::WindowFrame,
                [2.0, 0.0, -1.0],
                30.0,
                None,
            ),
            inst(
                "win_2",
                InstanceKind::WindowFrame,
                [-3.0, 1.0, 4.0],
                -90.0,
                None,
            ),
            inst(
                "door_1",
                InstanceKind::DoorFrame,
                [0.5, 0.0, 6.0],
                0.0,
                None,
            ),
            inst(
                "door_1/leaf",
                InstanceKind::DoorLeaf,
                [0.9, 0.0, 6.0],
                0.0,
                Some("door_1"),
            ),
            inst(
                "cupboard",
                InstanceKind::Furniture,
                [1.0, 0.0, 1.0],
                0.0,
                None,
            ),
            inst(
                "cupboard/pane",
                InstanceKind::Glass,
                [1.0, 1.0, 1.0],
                0.0,
                Some("cupboard"),
            ),
        ]);
        let recon = ReconFile {
            slug: "T".into(),
            root_angles: [0.0, root_yaw, 0.0],
            children: vec![
                child(
                    1,
                    "Building",
                    &["MeshObject"],
                    world([2.0, 0.0, -1.0]),
                    root_yaw + 30.0,
                ),
                child(
                    1,
                    "Building",
                    &["MeshObject"],
                    world([-3.0, 1.0, 4.0]),
                    root_yaw - 90.0,
                ),
                child(
                    1,
                    "GenericEntity",
                    &["DoorSlotComponent"],
                    world([0.5, 0.0, 6.0]),
                    root_yaw,
                ),
                child(
                    2,
                    "GenericEntity",
                    &["DoorComponent"],
                    world([0.9, 0.0, 6.0]),
                    root_yaw,
                ),
            ],
        };
        let r = verify(&f, &recon);
        assert_eq!(r.yaw_sign, 1.0);
        assert_eq!(r.matches.len(), 4, "{r:?}");
        assert!(r.failures().is_empty(), "{:?}", r.matches);
        assert!(r.unmatched.is_empty());
        assert!(r.extra.is_empty());
        assert_eq!(r.skipped_furniture, 2, "cupboard + its pane");
        assert!(r.ok());
        // A displaced child is reported; a missing child leaves the instance unmatched.
        let mut moved = recon;
        moved.children[0].rel_pos[1] += 0.05;
        moved.children.pop();
        let r = verify(&f, &moved);
        assert_eq!(r.failures().len(), 1);
        assert_eq!(
            r.unmatched,
            vec!["door_1/leaf (DoorLeaf, Prefabs/door_1/leaf.et)"]
        );
        assert!(!r.ok());
    }

    /// The T-090.11.3 socket pin: the committed farmhouse instances against the committed
    /// Workbench recon dump (88 architectural children, 2026-09-03).
    #[test]
    fn farmhouse_sockets_match_the_workbench_recon() {
        let root = crate::root::find_repo_root().unwrap();
        let instances = root.join(
            "packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json",
        );
        let recon = root.join("xtask/tests/fixtures/FarmHouse_E_1L01_Wood_children.json");
        let (file, dump) = load(&instances, &recon).unwrap();
        assert_eq!(dump.children.len(), 88);
        let r = verify(&file, &dump);
        assert_eq!(
            r.yaw_sign, 1.0,
            "handedness pin: Rigid::from_enfusion yaw sign"
        );
        assert_eq!(
            r.skipped_furniture, 2,
            "the two cupboard panes live under the furniture composition"
        );
        assert_eq!(r.matches.len(), 88, "unmatched: {:?}", r.unmatched);
        assert!(
            r.extra.is_empty(),
            "unclaimed recon children: {:?}",
            r.extra
        );
        assert!(
            r.failures().is_empty(),
            "worst pos {:.4} m · worst yaw {:.3}° · {:?}",
            r.worst_pos_m(),
            r.worst_yaw_deg(),
            r.failures()
        );
        // The enriched dump (Workbench restarted 2026-09-04): every leaf's hinge params, every
        // child's socket name and every nested child's parent-frame origin agree with the
        // prefab + XOB decode.
        assert_eq!(r.door_checks, 7);
        assert_eq!(r.pivot_checks, 88);
        assert!(r.local_checks >= 60, "{}", r.local_checks);
        assert!(r.door_mismatches.is_empty(), "{:?}", r.door_mismatches);
        assert!(r.pivot_mismatches.is_empty(), "{:?}", r.pivot_mismatches);
        assert!(r.local_mismatches.is_empty(), "{:?}", r.local_mismatches);
        assert!(r.ok());
    }
}
