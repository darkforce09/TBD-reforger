use super::*;

pub(super) fn instance_group(kind: InstanceKind) -> Group {
    match kind {
        InstanceKind::WindowFrame => Group::WindowFrame,
        InstanceKind::DoorFrame => Group::DoorFrame,
        InstanceKind::DoorLeaf => Group::DoorLeaf,
        InstanceKind::Glass => Group::Glass,
        InstanceKind::Prop => Group::Prop,
        _ => Group::Other,
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
pub(super) fn evaluate(instances: &[&InstanceRecord], recon: &ReconFile, yaw_sign: f64) -> Report {
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
pub(super) fn architectural(file: &InstancesFile) -> (Vec<&InstanceRecord>, usize) {
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

/// The enrichment, when the dump carries it: a leaf's hinge params must equal the
/// instance's `DoorRecord`, the child's `pivotId` must be the instance id's last segment, and
/// its parent-frame origin must match the instance placed under its parent (2 cm).
pub(super) fn enrichment_checks(file: &InstancesFile, recon: &ReconFile, report: &mut Report) {
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
        if let (Some(lp), Some(parent_id)) = (child.local_pos, inst.parent.as_deref())
            && let Some(parent) = by_id.get(parent_id)
        {
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

pub fn run_instances_verify(_root: &std::path::Path, args: &[String]) -> Result<u8> {
    let mut instances: Option<PathBuf> = None;
    let mut recon: Option<PathBuf> = None;
    // `--world-row --chunk <cx_cy.json.gz> --prefabs <prefabs.json.gz>`.
    let mut world_row = false;
    let mut chunk: Option<PathBuf> = None;
    let mut prefabs: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--world-row" => {
                world_row = true;
                i += 1;
            }
            "--chunk" if i + 1 < args.len() => {
                chunk = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--prefabs" if i + 1 < args.len() => {
                prefabs = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
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
                    "instances-verify: unknown arg {other} (usage: --instances <slug>.instances.json --recon <slug>_children.json [--world-row --chunk <cx_cy.json.gz> --prefabs <prefabs.json.gz>])"
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
    // The committed chunk row must place every matched child on its recon worldPos.
    let mut world_ok = true;
    if world_row {
        let chunk = chunk.context("--world-row needs --chunk <cx_cy.json.gz>")?;
        let prefabs = prefabs.context("--world-row needs --prefabs <prefabs.json.gz>")?;
        let rep = super::super::world_row::run_world_row(&chunk, &prefabs, &file, &dump, &report)?;
        world_ok = rep.ok();
    }
    Ok(u8::from(!(report.ok() && world_ok)))
}
