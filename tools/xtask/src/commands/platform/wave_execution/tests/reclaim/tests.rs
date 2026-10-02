use super::*;

#[test]
fn positive_identification_spares_everything_it_cannot_parse() {
    // The three dirs a looser rule would have eaten, measured at MAIN_ROOT.
    assert_eq!(slice_token("target-T-454"), Some("T-454".into()));
    assert_eq!(slice_token("target-T-582-api"), Some("T-582".into()));
    assert_eq!(slice_token("target-t742"), Some("t742".into()));
    assert_eq!(slice_token("target-dev-api"), None);
    assert_eq!(slice_token("target-ci"), None);
    // A GATE dir that CONTAINS a ticket id — anchoring at the first component is what makes
    // this unparseable even without the explicit target-gate-* exclusion.
    assert_eq!(slice_token("target-gate-schema-T422"), None);
    // A dotted id with an `-api` suffix is not `(-.*)?$` after the digits.
    assert_eq!(slice_token("target-T-068.13-api"), None);
}

#[test]
fn adhoc_pattern_is_uppercase_t_with_a_dash_only() {
    assert_eq!(adhoc_token("tbd-target-T-742"), Some("T-742".into()));
    assert_eq!(adhoc_token("tbd-target-T-742-x"), Some("T-742".into()));
    assert_eq!(adhoc_token("tbd-target-t742"), None);
    assert_eq!(adhoc_token("tbd-target-wave138-verify"), None);
    // The shared cache must never parse as a slice dir.
    assert_eq!(adhoc_token("tbd-target"), None);
}

#[test]
fn key_matches_the_tr_pipeline() {
    assert_eq!(key_of("T-702"), "t702");
    assert_eq!(key_of("v2-target-T-1"), "v2targett1");
}

/// Every kind of folder the build output sweeps can meet, each holding a `live` file.
const SCRATCH_FOLDERS: &[&str] = &[
    "target",
    "target/debug",
    "target/run-main",
    "target/dev-api",
    "target/ci",
    "target/gate-check",
    "target/gate-dist-frontend",
    "target/gate-slice-frontend-T-1",
    "target-dev-api",
    "target-ci",
    "target-gate-check",
    "target-gate-slice-frontend-T-1",
    "dist-gate-frontend",
    "target-T-454",
    "target-container",
    "apps/frontend/dist",
];

/// A scratch main checkout holding [`SCRATCH_FOLDERS`], unique per test and process.
fn scratch_checkout(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("reclaim-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for d in SCRATCH_FOLDERS {
        std::fs::create_dir_all(root.join(d)).unwrap();
        std::fs::write(root.join(d).join("live"), "x").unwrap();
    }
    root
}

fn assert_live(root: &Path, folders: &[&str]) {
    for d in folders {
        assert!(root.join(d).join("live").is_file(), "{d} must survive");
    }
}

fn assert_gone(root: &Path, folders: &[&str]) {
    for d in folders {
        assert!(!root.join(d).exists(), "{d} must be gone");
    }
}

const RETIRED: &[&str] = &[
    "target-dev-api",
    "target-ci",
    "target-gate-check",
    "target-gate-slice-frontend-T-1",
    "dist-gate-frontend",
];

const GATE_SET: &[&str] = &[
    "target/gate-check",
    "target/gate-dist-frontend",
    "target/gate-slice-frontend-T-1",
];

const NEVER_SWEPT_HERE: &[&str] = &[
    "target",
    "target/debug",
    "target/run-main",
    "target/dev-api",
    "target/ci",
    "target-T-454",
    "target-container",
    "apps/frontend/dist",
];

/// The retired root-level folders go by default; nothing else at the root or in `target/` does.
#[test]
fn retired_root_level_folders_are_deleted_and_nothing_else() {
    let root = scratch_checkout("retired");
    sweep_retired_root_level_folders(&root);
    assert_gone(&root, RETIRED);
    assert_live(&root, GATE_SET);
    assert_live(&root, NEVER_SWEPT_HERE);
    // Idempotent: nothing left to free.
    assert_eq!(sweep_retired_root_level_folders(&root), 0);
    let _ = std::fs::remove_dir_all(&root);
}

/// The gate set is exactly the `target/gate-*` subfolders, in collation order.
#[test]
fn gate_folders_are_the_gate_subfolders_of_target_only() {
    let root = scratch_checkout("gate-list");
    let found: Vec<PathBuf> = gate_folders(&root);
    let expected: Vec<PathBuf> = GATE_SET.iter().map(|d| root.join(d)).collect();
    assert_eq!(found, expected);
    let _ = std::fs::remove_dir_all(&root);
}

/// Without `--gate-dirs` the gate set is only measured; with it the gate set goes and the rest of
/// `target/` and every root-level folder stays. A positive minimum age spares a fresh folder.
#[test]
fn gate_sweep_is_opt_in_and_spares_the_rest_of_target() {
    let root = scratch_checkout("gate-sweep");
    assert_eq!(sweep_gate_folders(&root, false, 0), 0);
    assert_live(&root, GATE_SET);
    sweep_gate_folders(&root, true, 1);
    assert_live(&root, GATE_SET);
    sweep_gate_folders(&root, true, 0);
    assert_gone(&root, GATE_SET);
    assert_live(&root, NEVER_SWEPT_HERE);
    assert_live(&root, RETIRED);
    let _ = std::fs::remove_dir_all(&root);
}
