//! Tests for [`super`] — a synthetic catalogue over a loose-directory source built from the
//! xob test helpers (no game content), plus the committed-library pins.

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::map_blueprint::pak::DirSource;
use crate::map_blueprint::xob::tests::{coll_box_record_with_material, with_coll};
use crate::map_blueprint::xob_nodes::XobNode;
use crate::map_blueprint::xob_nodes::tests::{synth_head, wrap_xob};

fn write(dir: &Path, rel: &str, bytes: &[u8]) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, bytes).unwrap();
}

fn et_with_mesh(model: &str) -> String {
    format!(
        "GenericEntity {{\n ID \"1\"\n components {{\n  MeshObject \"{{6000000000000012}}\" {{\n   Object \"{{6000000000000021}}{model}\"\n  }}\n }}\n}}\n"
    )
}

/// A HEAD-only XOB (no COLL): root node, `Building` layer name, one gamemat.
fn head_only(gamemat: &str) -> Vec<u8> {
    let strings = ["Mat", "{A}Assets/a.emat", "Scene_Root", "Building", gamemat];
    let nodes = [XobNode {
        name_idx: 2,
        pos: [0.0; 3],
        quat: [0.0, 0.0, 0.0, 1.0],
        next_sibling: 0xFFFF,
        first_child: 0xFFFF,
    }];
    wrap_xob(&synth_head(&strings, &nodes))
}

/// The same with one box collider on layer `Building` (index 3) and the gamemat (index 4).
fn boxed(gamemat: &str, half: [f32; 3]) -> Vec<u8> {
    with_coll(
        head_only(gamemat),
        &coll_box_record_with_material([0.0, half[1], 0.0], half, 3, 4),
    )
}

/// Synthetic catalogue: a wood barrel (prop), a second prop on the SAME model, a building on a
/// box, a decal without COLL, a sound source without a mesh, a foliage-collider tree and a
/// wood-only tree whose canopy fallback has no visual LOD to read.
fn synth_library(dir: &Path) -> (Vec<PrefabRow>, HashMap<u32, u64>) {
    write(
        dir,
        "Assets/Barrel.xob",
        &boxed("{B}Common/Materials/Game/wood.gamemat", [1.0, 1.0, 1.0]),
    );
    write(
        dir,
        "Assets/Decal.xob",
        &head_only("{B}Common/Materials/Game/wood.gamemat"),
    );
    write(
        dir,
        "Assets/Tree.xob",
        &boxed("{F}Common/Materials/Game/foliage.gamemat", [2.0, 4.0, 2.0]),
    );
    write(
        dir,
        "Assets/Trunk.xob",
        &boxed("{B}Common/Materials/Game/wood.gamemat", [0.3, 5.0, 0.3]),
    );
    write(
        dir,
        "Prefabs/Props/Barrel.et",
        et_with_mesh("Assets/Barrel.xob").as_bytes(),
    );
    write(
        dir,
        "Prefabs/Props/Barrel_Red.et",
        et_with_mesh("Assets/Barrel.xob").as_bytes(),
    );
    write(
        dir,
        "Prefabs/Structures/Shed.et",
        et_with_mesh("Assets/Barrel.xob").as_bytes(),
    );
    write(
        dir,
        "Prefabs/Props/Decal.et",
        et_with_mesh("Assets/Decal.xob").as_bytes(),
    );
    write(dir, "Prefabs/Props/Sound.et", b"GenericEntity {\n ID \"2\"\n components {\n  RplComponent \"{6000000000000030}\" {\n   Enabled 1\n  }\n }\n}\n");
    write(
        dir,
        "Prefabs/Vegetation/Tree.et",
        et_with_mesh("Assets/Tree.xob").as_bytes(),
    );
    write(
        dir,
        "Prefabs/Vegetation/Trunk.et",
        et_with_mesh("Assets/Trunk.xob").as_bytes(),
    );
    let row = |pid: u32, rn: &str, kind: &str| PrefabRow {
        pid,
        resource_name: format!("{{{pid:016X}}}{rn}"),
        kind: kind.into(),
    };
    let rows = vec![
        row(0, "Prefabs/Props/Barrel.et", "prop"),
        row(1, "Prefabs/Props/Barrel_Red.et", "prop"),
        row(2, "Prefabs/Structures/Shed.et", "building"),
        row(3, "Prefabs/Props/Decal.et", "prop"),
        row(4, "Prefabs/Props/Sound.et", "prop"),
        row(5, "Prefabs/Vegetation/Tree.et", "tree"),
        row(6, "Prefabs/Vegetation/Trunk.et", "tree"),
        row(7, "Prefabs/Missing/Ghost.et", "prop"),
    ];
    let census: HashMap<u32, u64> = [(0, 10), (1, 400), (2, 3), (3, 900), (5, 250), (6, 5)]
        .into_iter()
        .collect();
    (rows, census)
}

fn tmp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-library-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn schema_dir() -> PathBuf {
    crate::root::find_repo_root()
        .unwrap()
        .join("packages/tbd-schema/schema")
}

#[test]
fn descriptors_carry_blocks_reasons_kinds_and_canopy() {
    let dir = tmp("reasons");
    let (rows, census) = synth_library(&dir);
    let source = DirSource { root: dir.clone() };
    let opts = LibraryOptions {
        terrain: "synth".into(),
        only_kinds: vec![],
        limit: None,
        hot: 3,
    };
    let lib = build_library(&source, &rows, &census, &opts).unwrap();
    let d = |pid: u32| lib.descriptors.iter().find(|d| d.prefab_id == pid).unwrap();
    // Props keep the walker's kind; the building's root is the shell.
    assert!(d(0).blocks && d(0).instances[0].kind == InstanceKind::Prop);
    assert_eq!(d(0).shell_bvh, "blas/Barrel.bvh");
    assert!(d(2).blocks && d(2).instances[0].kind == InstanceKind::Shell);
    assert_eq!(d(2).instances[0].id, "Shed");
    // Placed bounds are the collider box.
    let b = d(0).local_bounds.unwrap();
    assert!(
        (b.min[0] + 1.0).abs() < 1e-6 && (b.max[1] - 2.0).abs() < 1e-6,
        "{b:?}"
    );
    // blocks:false with the reason that names the hole.
    assert_eq!(
        (d(3).blocks, d(3).reason.as_deref()),
        (false, Some("no-coll"))
    );
    assert_eq!(
        (d(4).blocks, d(4).reason.as_deref()),
        (false, Some("no-mesh"))
    );
    assert_eq!(
        (d(7).blocks, d(7).reason.as_deref()),
        (false, Some("unresolved"))
    );
    assert!(d(3).instances.is_empty() && d(3).shell_bvh.is_empty());
    // Trees: foliage in the COLL → canopy; wood-only with no visual LOD → trunk only, noted.
    assert!(d(5).canopy && d(5).blocks);
    assert!(!d(6).canopy && d(6).blocks, "{:?}", d(6));
    assert!(
        d(6).notes.iter().any(|n| n.contains("canopy hull")),
        "{:?}",
        d(6).notes
    );
    assert_eq!(d(5).instances[0].kind, InstanceKind::Tree);
    let t = &lib.manifest.totals;
    assert_eq!((t.prefabs, t.blocks, t.canopy, t.canopy_hull), (8, 5, 1, 0));
    let prop = &t.by_kind["prop"];
    assert_eq!(
        (
            prop.prefabs,
            prop.blocks,
            prop.no_coll,
            prop.no_mesh,
            prop.unresolved
        ),
        (5, 2, 1, 1, 1)
    );
}

#[test]
fn blas_dedup_by_stem_manifest_entries_and_hot_order() {
    let dir = tmp("dedup");
    let (rows, census) = synth_library(&dir);
    let source = DirSource { root: dir.clone() };
    let opts = LibraryOptions {
        terrain: "synth".into(),
        only_kinds: vec![],
        limit: None,
        hot: 2,
    };
    let lib = build_library(&source, &rows, &census, &opts).unwrap();
    // Box, Box_Red and Shed share one model → one BLAS; Tree and Trunk have their own.
    let paths: Vec<&str> = lib.blas.keys().map(String::as_str).collect();
    assert_eq!(
        paths,
        vec!["blas/Barrel.bvh", "blas/Tree.bvh", "blas/Trunk.bvh"]
    );
    let m = &lib.manifest;
    assert_eq!(m.blas.len(), 3);
    let tree = m.blas("blas/Tree.bvh").unwrap();
    assert_eq!(tree.tris, 12);
    assert_eq!(tree.kinds, [0, 0, 12]);
    assert_eq!(m.blas("blas/Barrel.bvh").unwrap().kinds, [12, 0, 0]);
    assert_eq!(
        m.blas("blas/Barrel.bvh").unwrap().bytes,
        lib.blas["blas/Barrel.bvh"].len() as u64
    );
    // Descriptor entries are sorted by pid and carry the world census.
    let pids: Vec<u32> = m.descriptors.iter().map(|d| d.pid).collect();
    assert_eq!(pids, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert_eq!(m.descriptor(1).unwrap().instances_in_world, 400);
    assert_eq!(m.descriptor(4).unwrap().instances_in_world, 0);
    assert_eq!(
        m.descriptor(1).unwrap().blas,
        vec!["blas/Barrel.bvh".to_string()]
    );
    // Hot: the most-placed BLOCKING pids (900 places the decal, but it never blocks).
    assert_eq!(m.hot, vec![1, 5]);
    // Bytes are attributed to the kind that first references a BLAS.
    assert_eq!(
        m.totals.by_kind["prop"].bytes,
        lib.blas["blas/Barrel.bvh"].len() as u64
    );
    assert_eq!(m.totals.by_kind["building"].bytes, 0);
    assert_eq!(m.totals.blas_files, 3);
}

#[test]
fn write_is_schema_valid_and_deterministic() {
    let dir = tmp("write");
    let (rows, census) = synth_library(&dir);
    let source = DirSource { root: dir.clone() };
    let opts = LibraryOptions {
        terrain: "synth".into(),
        only_kinds: vec![],
        limit: None,
        hot: 3,
    };
    let lib = build_library(&source, &rows, &census, &opts).unwrap();
    let out = dir.join("prefabs");
    let first = write_library(&out, &lib, &schema_dir(), false).unwrap();
    assert_eq!(first, 8 + 3 + 1, "8 descriptors + 3 BLAS + manifest");
    let again = write_library(&out, &lib, &schema_dir(), false).unwrap();
    assert_eq!(again, 0, "a re-emit that changes nothing writes nothing");
    // Round trip through the files.
    let m: BlasManifest =
        serde_json::from_str(&fs::read_to_string(out.join("blas-manifest.json")).unwrap()).unwrap();
    assert_eq!(m, lib.manifest);
    let d: PrefabDescriptor =
        serde_json::from_str(&fs::read_to_string(out.join("descriptors/2.json")).unwrap()).unwrap();
    assert_eq!(d, lib.descriptors[2]);
    // A filtered run never writes a manifest.
    let partial_out = dir.join("partial");
    let n = write_library(&partial_out, &lib, &schema_dir(), true).unwrap();
    assert_eq!(n, 11);
    assert!(!partial_out.join("blas-manifest.json").exists());
    // The schemas reject a padded document.
    let mut bad = serde_json::to_value(&lib.descriptors[0]).unwrap();
    bad["blocks"] = Value::from("yes");
    assert!(
        validate_against(
            &bad,
            &schema_dir().join("prefab-descriptor.schema.json"),
            "bad"
        )
        .is_err()
    );
}

#[test]
fn only_kind_and_limit_select_rows() {
    let dir = tmp("filter");
    let (rows, census) = synth_library(&dir);
    let source = DirSource { root: dir.clone() };
    let opts = LibraryOptions {
        terrain: "synth".into(),
        only_kinds: vec!["tree".into()],
        limit: Some(1),
        hot: 3,
    };
    let lib = build_library(&source, &rows, &census, &opts).unwrap();
    assert_eq!(lib.descriptors.len(), 1);
    assert_eq!(lib.descriptors[0].prefab_id, 5);
}

/// The committed library reproduces the T-090.11 farmhouse: the descriptor's root is the shell
/// (its sidecar byte-identical to the committed `buildings/FarmHouse_E_1L01_Wood.bvh`) and every
/// record of `FarmHouse_E_1L01_Wood.instances.json` is in the descriptor unchanged.
#[test]
fn committed_farmhouse_descriptor_reproduces_the_t090_11_instances() {
    use map_engine_core::building_compound::InstancesFile;
    let root = crate::root::find_repo_root().unwrap();
    let prefabs = root.join("packages/map-assets/everon/prefabs");
    let manifest: BlasManifest =
        serde_json::from_str(&fs::read_to_string(prefabs.join("blas-manifest.json")).unwrap())
            .unwrap();
    let file: InstancesFile = serde_json::from_str(
        &fs::read_to_string(prefabs.join("buildings/FarmHouse_E_1L01_Wood.instances.json"))
            .unwrap(),
    )
    .unwrap();
    let names =
        load_prefab_rows(&root.join("packages/map-assets/everon/objects/prefabs.json.gz")).unwrap();
    let pid = names
        .iter()
        .find(|r| strip_guid(&r.resource_name) == file.resource_name)
        .map(|r| r.pid)
        .expect("farmhouse pid");
    let entry = manifest.descriptor(pid).expect("manifest entry");
    assert!(entry.blocks && entry.kind == "building");
    let d: PrefabDescriptor =
        serde_json::from_str(&fs::read_to_string(prefabs.join(&entry.path)).unwrap()).unwrap();
    assert_eq!(d.instances[0].kind, InstanceKind::Shell);
    assert_eq!(d.shell_bvh, d.instances[0].blas);
    let shell = fs::read(prefabs.join(&d.shell_bvh)).unwrap();
    let committed = fs::read(prefabs.join("buildings").join(&file.shell_bvh)).unwrap();
    assert_eq!(shell, committed, "root BLAS == the T-090.11 shell sidecar");
    for rec in &file.instances {
        let mine = d
            .instances
            .iter()
            .find(|i| i.id == rec.id)
            .unwrap_or_else(|| panic!("{} missing from the descriptor", rec.id));
        assert_eq!(mine, rec, "{}", rec.id);
    }
    assert_eq!(d.instances.len(), file.instances.len() + 1);
    for p in d.blas_paths() {
        assert!(manifest.blas(p).is_some(), "{p} not in the manifest");
        assert!(prefabs.join(p).is_file(), "{p} missing");
    }
}

/// The canopy hull never sees a full visual mesh: `hull_sample` keeps at most the 26 extreme
/// points (one per axis / diagonal direction), always including the axis extremes.
#[test]
fn hull_sample_keeps_at_most_26_extreme_points() {
    // A dense cloud on a sphere of radius 3 around (1, 2, 3), plus six axis spikes.
    let mut pts: Vec<[f64; 3]> = Vec::new();
    for i in 0..2000 {
        let a = i as f64 * 0.618_033_988_7 * std::f64::consts::TAU;
        let z = -1.0 + 2.0 * (i as f64 + 0.5) / 2000.0;
        let r = (1.0 - z * z).sqrt();
        pts.push([
            1.0 + 3.0 * r * a.cos(),
            2.0 + 3.0 * r * a.sin(),
            3.0 + 3.0 * z,
        ]);
    }
    for (axis, sign) in [
        (0, 1.0),
        (0, -1.0),
        (1, 1.0),
        (1, -1.0),
        (2, 1.0),
        (2, -1.0),
    ] {
        let mut p = [1.0, 2.0, 3.0];
        p[axis] += 5.0 * sign;
        pts.push(p);
    }
    let sample = hull_sample(&pts);
    assert!(sample.len() <= 26 && sample.len() >= 6, "{}", sample.len());
    assert!(sample.contains(&[6.0, 2.0, 3.0]) && sample.contains(&[-4.0, 2.0, 3.0]));
    assert!(sample.contains(&[1.0, 7.0, 3.0]) && sample.contains(&[1.0, 2.0, -2.0]));
    let tris = crate::map_blueprint::hull::hull_triangles(&sample);
    assert!(!tris.is_empty(), "the sample hulls");
    assert!(hull_sample(&[]).is_empty());
}
