//! T-090.12.1 — `instances-verify --world-row --chunk <cx_cy.json.gz> --prefabs <prefabs.json.gz>`:
//! the committed chunk row that places the recon's building, composed with every socket
//! instance, must land each matched child on the recon's absolute `worldPos` within
//! [`POS_TOL_M`]. That pins the chunk wire v2 transform end to end on the world data itself —
//! the map→engine axis mapping (`x, y_north, z_up` → `[x, y_up, z_north]`), the heading
//! normalisation, the 2-dp rounding and `Rigid::from_enfusion`'s composition — not on a
//! synthetic scene.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use anyhow::{Context, Result, bail};
use map_engine_core::building_compound::InstancesFile;
use map_engine_core::geometry::rigid::Rigid;
use serde_json::Value;

use super::verify::{POS_TOL_M, ReconFile, Report};

/// The catalogue's cell size (`manifest.objects.chunkSizeM`).
pub const CHUNK_M: f64 = 512.0;

/// One chunk row in the map frame (`x`, `y_north`, `z_up`), angles in Enfusion `GetAngles()`
/// degrees (`pitch` about X, `yaw` = heading about Y, `roll` about Z), uniform `scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldRow {
    pub pid: u64,
    pub x: f64,
    pub y_north: f64,
    pub z_up: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
    pub scale: f64,
    /// 5 (v1, or v2 with trivial trailers) or 8.
    pub width: usize,
}

impl WorldRow {
    /// Entity → world transform in the Enfusion frame `[x, y_up, z_north]` — the frame every
    /// BLAS and `CompoundBuilding` speaks.
    #[must_use]
    pub fn rigid(&self) -> Rigid {
        Rigid::from_enfusion(
            [self.x, self.z_up, self.y_north],
            [self.pitch, self.yaw, self.roll],
            self.scale,
        )
    }
}

fn gunzip_json(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).with_context(|| path.display().to_string())?;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes.as_slice())
        .read_to_end(&mut out)
        .with_context(|| format!("gunzip {}", path.display()))?;
    serde_json::from_slice(&out).with_context(|| format!("parse {}", path.display()))
}

/// `prefabId → resourceName` from `objects/prefabs.json.gz`.
pub fn load_prefab_names(prefabs_gz: &Path) -> Result<HashMap<u64, String>> {
    let doc = gunzip_json(prefabs_gz)?;
    let rows = doc["prefabs"]
        .as_array()
        .context("prefabs.json.gz: no prefabs array")?;
    let mut out = HashMap::with_capacity(rows.len());
    for p in rows {
        if let (Some(id), Some(rn)) = (p["prefabId"].as_u64(), p["resourceName"].as_str()) {
            out.insert(id, rn.to_string());
        }
    }
    if out.is_empty() {
        bail!("prefabs.json.gz: no prefab rows");
    }
    Ok(out)
}

/// Strip the `{GUID}` prefix a catalogue `resourceName` carries.
fn resource_tail(rn: &str) -> &str {
    rn.split_once('}').map_or(rn, |(_, tail)| tail)
}

/// The prefab id whose catalogue `resourceName` (`{GUID}Prefabs/…/X.et`) names `resource_name`
/// (`Prefabs/…/X.et`, as the instances file carries it). The lowest id wins a tie.
#[must_use]
pub fn pid_for_resource(names: &HashMap<u64, String>, resource_name: &str) -> Option<u64> {
    let want = resource_tail(resource_name);
    names
        .iter()
        .filter(|(_, rn)| resource_tail(rn) == want)
        .map(|(id, _)| *id)
        .min()
}

/// Every row of one chunk file (`{instances: [[pid, x, y, z, yaw(, pitch, roll, scale)], …]}`).
pub fn load_rows(chunk_gz: &Path) -> Result<Vec<WorldRow>> {
    let doc = gunzip_json(chunk_gz)?;
    let rows = doc["instances"]
        .as_array()
        .context("chunk: no instances array")?;
    let mut out = Vec::with_capacity(rows.len());
    for (i, r) in rows.iter().enumerate() {
        let a = r
            .as_array()
            .with_context(|| format!("row {i}: not an array"))?;
        if a.len() != 5 && a.len() != 8 {
            bail!("row {i}: {} elements (want exactly 5 or 8)", a.len());
        }
        let n = |k: usize| {
            a[k].as_f64()
                .with_context(|| format!("row {i}[{k}]: not a number"))
        };
        let wide = a.len() == 8;
        out.push(WorldRow {
            pid: a[0].as_u64().with_context(|| format!("row {i}: pid"))?,
            x: n(1)?,
            y_north: n(2)?,
            z_up: n(3)?,
            yaw: n(4)?,
            pitch: if wide { n(5)? } else { 0.0 },
            roll: if wide { n(6)? } else { 0.0 },
            scale: if wide { n(7)? } else { 1.0 },
            width: a.len(),
        });
    }
    Ok(out)
}

/// Rows of `pid` whose map position is within `tol_m` of (`x`, `y_north`).
#[must_use]
pub fn rows_near(rows: &[WorldRow], pid: u64, x: f64, y_north: f64, tol_m: f64) -> Vec<WorldRow> {
    rows.iter()
        .copied()
        .filter(|r| r.pid == pid && (r.x - x).hypot(r.y_north - y_north) <= tol_m)
        .collect()
}

/// Chunk id (`{cx}_{cy}`) of a map position (the converter's floor partition).
#[must_use]
pub fn chunk_id_of(x: f64, y_north: f64) -> String {
    format!(
        "{}_{}",
        (x / CHUNK_M).floor() as i64,
        (y_north / CHUNK_M).floor() as i64
    )
}

#[derive(Debug, Default)]
pub struct WorldRowReport {
    pub row: Option<WorldRow>,
    /// Matched children with a recon `worldPos` that were compared.
    pub checked: usize,
    pub worst_pos_m: f64,
    pub mismatches: Vec<String>,
    /// Matched children the (older) recon dump carries no `worldPos` for.
    pub children_without_world_pos: usize,
}

impl WorldRowReport {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.row.is_some() && self.checked > 0 && self.mismatches.is_empty()
    }
}

/// Compose `row` with every matched instance's building-frame origin and compare with the
/// recon child's absolute `worldPos` (2 cm).
#[must_use]
pub fn check(
    row: WorldRow,
    file: &InstancesFile,
    recon: &ReconFile,
    matches: &Report,
) -> WorldRowReport {
    let world = row.rigid();
    let by_id: HashMap<&str, _> = file.instances.iter().map(|i| (i.id.as_str(), i)).collect();
    let mut rep = WorldRowReport {
        row: Some(row),
        ..Default::default()
    };
    for m in &matches.matches {
        let Some(inst) = by_id.get(m.instance.as_str()) else {
            continue;
        };
        let child = &recon.children[m.child_index];
        let Some(wp) = child.world_pos else {
            rep.children_without_world_pos += 1;
            continue;
        };
        let mine = world.point(inst.local.pos);
        let d = (0..3)
            .map(|a| (mine[a] - wp[a]).powi(2))
            .sum::<f64>()
            .sqrt();
        rep.checked += 1;
        rep.worst_pos_m = rep.worst_pos_m.max(d);
        if d > POS_TOL_M {
            rep.mismatches.push(format!(
                "{}: row-placed {:?} vs recon worldPos {:?} ({d:.4} m)",
                inst.id, mine, wp
            ));
        }
    }
    rep
}

/// The CLI arm: locate the building's row in `chunk_gz` (by the instances file's resource name
/// and the recon's `rootWorldPos`), run [`check`], print one report line. Errors when the row
/// is missing or ambiguous; `Ok(report)` otherwise (the caller decides the exit code).
pub fn run_world_row(
    chunk_gz: &Path,
    prefabs_gz: &Path,
    file: &InstancesFile,
    recon: &ReconFile,
    matches: &Report,
) -> Result<WorldRowReport> {
    let names = load_prefab_names(prefabs_gz)?;
    let pid = pid_for_resource(&names, &file.resource_name)
        .with_context(|| format!("no catalogue prefab named {}", file.resource_name))?;
    let rwp = recon
        .root_world_pos
        .context("recon dump carries no rootWorldPos")?;
    // The recon's origin decides the cell; a chunk from anywhere else cannot hold the row.
    let want = chunk_id_of(rwp[0], rwp[2]);
    let stem = chunk_gz
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if !stem.starts_with(&format!("{want}.")) {
        bail!(
            "recon rootWorldPos ({:.2}, {:.2}) lies in chunk {want}, not {stem}",
            rwp[0],
            rwp[2]
        );
    }
    let rows = load_rows(chunk_gz)?;
    let near = rows_near(&rows, pid, rwp[0], rwp[2], 0.05);
    let row = match near.as_slice() {
        [r] => *r,
        [] => bail!(
            "no row of pid {pid} within 5 cm of ({:.2}, {:.2}) in {}",
            rwp[0],
            rwp[2],
            chunk_gz.display()
        ),
        many => bail!("{} rows of pid {pid} within 5 cm: {many:?}", many.len()),
    };
    let rep = check(row, file, recon, matches);
    println!(
        "world-row {}: pid {} at ({:.2}, {:.2}, {:.2}) yaw {:.2} pitch {:.2} roll {:.2} scale {:.3} ({}-wide) · {} children checked · worst {:.4} m · {} over {POS_TOL_M} m · {} without worldPos",
        chunk_gz
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default(),
        row.pid,
        row.x,
        row.y_north,
        row.z_up,
        row.yaw,
        row.pitch,
        row.roll,
        row.scale,
        row.width,
        rep.checked,
        rep.worst_pos_m,
        rep.mismatches.len(),
        rep.children_without_world_pos
    );
    for m in &rep.mismatches {
        println!("  WORLD-ROW MISMATCH {m}");
    }
    Ok(rep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_blueprint::tests::fixture;
    use crate::map_blueprint::verify::{load, verify};
    use crate::root::find_repo_root;

    fn objects_dir() -> std::path::PathBuf {
        find_repo_root()
            .unwrap()
            .join("packages/map-assets/everon/objects")
    }

    #[test]
    fn pid_lookup_ignores_the_guid_prefix() {
        let names: HashMap<u64, String> = [
            (3, "{AAAA}Prefabs/A/x.et".to_string()),
            (7, "{BBBB}Prefabs/B/y.et".to_string()),
        ]
        .into_iter()
        .collect();
        assert_eq!(pid_for_resource(&names, "Prefabs/B/y.et"), Some(7));
        assert_eq!(pid_for_resource(&names, "{CCCC}Prefabs/A/x.et"), Some(3));
        assert_eq!(pid_for_resource(&names, "Prefabs/C/z.et"), None);
    }

    #[test]
    fn chunk_id_is_the_floor_partition() {
        assert_eq!(chunk_id_of(9363.58, 285.598), "18_0");
        assert_eq!(chunk_id_of(512.0, 511.999), "1_0");
        assert_eq!(chunk_id_of(0.0, 0.0), "0_0");
    }

    /// The T-090.12.1 transform pin: the farmhouse's committed chunk row (18_0), composed with
    /// the 88 socket instances of the T-090.11 pipeline, lands every child on the Workbench
    /// recon's absolute `worldPos` within 2 cm. Its pitch and roll are 0, so the row must have
    /// stayed 5-wide (trivial trailers are never padded).
    #[test]
    fn farmhouse_chunk_row_places_every_socket_child_within_2cm() {
        let root = find_repo_root().unwrap();
        let instances = root.join(
            "packages/map-assets/everon/prefabs/buildings/FarmHouse_E_1L01_Wood.instances.json",
        );
        let recon = fixture("FarmHouse_E_1L01_Wood_children.json");
        let (file, dump) = load(&instances, &recon).unwrap();
        let matches = verify(&file, &dump);
        assert_eq!(matches.matches.len(), 88);
        let names = load_prefab_names(&objects_dir().join("prefabs.json.gz")).unwrap();
        let pid = pid_for_resource(&names, &file.resource_name).expect("farmhouse pid");
        let rwp = dump.root_world_pos.expect("recon rootWorldPos");
        let id = chunk_id_of(rwp[0], rwp[2]);
        assert_eq!(id, "18_0");
        let rows = load_rows(&objects_dir().join(format!("chunks/{id}.json.gz"))).unwrap();
        let near = rows_near(&rows, pid, rwp[0], rwp[2], 0.05);
        assert_eq!(near.len(), 1, "{near:?}");
        let row = near[0];
        assert_eq!(row.width, 5, "pitch/roll 0 → the row stays 5-wide: {row:?}");
        assert_eq!(row.yaw, 38.46, "{row:?}");
        assert_eq!((row.pitch, row.roll, row.scale), (0.0, 0.0, 1.0));
        let r = check(row, &file, &dump, &matches);
        assert_eq!(r.checked, 88, "{r:?}");
        assert_eq!(r.children_without_world_pos, 0);
        assert!(
            r.worst_pos_m <= POS_TOL_M,
            "worst {:.4} m: {:?}",
            r.worst_pos_m,
            r.mismatches
        );
        assert!(r.ok());
    }

    /// The tilted-prop pin: the GarbageContainer_01 the T-090.11.3 rotation fixture recorded at
    /// world (9878.51, 6.753, 236.2) with angles (−3.044, −104.126, −4.754) is an 8-wide row in
    /// chunk 19_0 carrying pitch −3.04 / roll −4.75 (round2) and heading 255.87
    /// (`norm_heading(−104.126)`). Scale is 1.0 while the catalogue is built from the July 2026
    /// export (no `scale` field) — re-blessed when the v2 export lands (T-090.12.1b).
    #[test]
    fn garbage_container_row_carries_pitch_and_roll() {
        let fx: Value = serde_json::from_str(
            &std::fs::read_to_string(fixture("rotation_pin_GarbageContainer_01.json")).unwrap(),
        )
        .unwrap();
        let wp: Vec<f64> = fx["parent"]["worldPos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let ang: Vec<f64> = fx["parent"]["anglesDeg"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(ang, vec![-3.044, -104.126, -4.754]);
        let names = load_prefab_names(&objects_dir().join("prefabs.json.gz")).unwrap();
        let pid = pid_for_resource(&names, fx["parent"]["prefab"].as_str().unwrap())
            .expect("garbage container pid");
        let id = chunk_id_of(wp[0], wp[2]);
        assert_eq!(id, "19_0");
        let rows = load_rows(&objects_dir().join(format!("chunks/{id}.json.gz"))).unwrap();
        let near = rows_near(&rows, pid, wp[0], wp[2], 0.05);
        assert_eq!(near.len(), 1, "{near:?}");
        let row = near[0];
        assert_eq!(row.width, 8, "{row:?}");
        assert_eq!(row.z_up, 6.75);
        assert_eq!(row.yaw, 255.87);
        assert_eq!(row.pitch, -3.04);
        assert_eq!(row.roll, -4.75);
        assert_eq!(
            row.scale, 1.0,
            "scale absent from the July export (re-bless at 1b)"
        );
        // The engine-frame placement puts the origin where the fixture says it is (5 mm rounding).
        let t = row.rigid().point([0.0; 3]);
        assert!(
            (t[0] - wp[0]).abs() < 0.006
                && (t[1] - wp[1]).abs() < 0.006
                && (t[2] - wp[2]).abs() < 0.006,
            "{t:?}"
        );
    }
}
