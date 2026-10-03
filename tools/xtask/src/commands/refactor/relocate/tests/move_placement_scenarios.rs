//! Whole relocation runs on throwaway checkouts for where the moves land: folder rows that share a
//! destination parent each land exactly at their `to` in any manifest order, rows that would put
//! two things in one place are refused with both lines before anything is written, and an apply
//! that fails at any step leaves the index and the working tree byte-identical.

use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use super::apply;
use super::dry_run;
use super::fixture_repository::FixtureRepository;
use super::manifest::parse_manifest;
use super::path_mapping::PathMapping;
use super::plan_application::{apply_plan, apply_plan_with_checkpoint};
use super::planned_tree::PlannedTree;
use super::relocation_plan::{PlanRefusal, RelocationPlan, build_plan};
use super::repository_files::{RepositorySnapshot, TrackedTree};

/// The camera folders of one crate: the math folder becomes the source root, and the two camera
/// folders land inside it, listed first so the folders sharing `crates/camera_math/src` run in an
/// order the manifest does not give.
const CAMERA_ROWS: &str = "path\tcamera/ortho\tcrates/camera_math/src/ortho\t\n\
                           path\tcamera/orbit\tcrates/camera_math/src/orbit\t\n\
                           path\tcamera/math\tcrates/camera_math/src\t\n\
                           path\tcamera/math/mod.rs\tcrates/camera_math/src/lib.rs\t\n\
                           path\tcamera/math/glmat4.rs\tcrates/camera_math/src/matrix4.rs\t\n";

fn camera_checkout(tag: &str) -> FixtureRepository {
    let repo = FixtureRepository::new(tag);
    repo.write("camera/math/mod.rs", "pub mod glmat4;\n")
        .write("camera/math/glmat4.rs", "pub fn identity() {}\n")
        .write("camera/math/tests/README.md", "# Math tests\n")
        .write("camera/math/tests/glmat4_tests.rs", "#[test]\nfn identity() {}\n")
        .write("camera/ortho/mod.rs", "pub mod view;\n")
        .write("camera/ortho/view.rs", "pub fn view() {}\n")
        .write("camera/orbit/mod.rs", "pub fn orbit() {}\n")
        .write(
            "docs/cameras.md",
            "[math](/camera/math/mod.rs), [ortho](/camera/ortho/view.rs), [orbit](/camera/orbit/mod.rs)\n",
        )
        .track();
    repo
}

/// The plan of the manifest at `manifest` over `repo`, as `--dry-run` and `--apply` build it.
fn plan_of(repo: &FixtureRepository, manifest: &Path) -> Result<RelocationPlan, PlanRefusal> {
    let text = std::fs::read_to_string(manifest).expect("read the manifest");
    let rows = parse_manifest(&text).expect("a valid manifest");
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the checkout");
    build_plan(&snapshot, &rows, None)
}

/// The refusal messages of a plan the checkout cannot take.
fn refusals(repo: &FixtureRepository, manifest: &Path) -> Vec<String> {
    match plan_of(repo, manifest) {
        Err(PlanRefusal::Refused(errors)) => errors,
        Err(PlanRefusal::NotRun(cause)) => panic!("the plan did not run: {cause:?}"),
        Ok(_) => panic!("the plan was accepted"),
    }
}

/// Every byte the apply may touch: the index file, then every folder and file of the working
/// tree outside `.git`, each file with its mode and content.
fn fingerprint(root: &Path) -> Vec<u8> {
    let mut out = std::fs::read(root.join(".git/index")).expect("read the index");
    let mut stack = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(folder) = stack.pop() {
        for entry in std::fs::read_dir(&folder).expect("read a folder") {
            let path = entry.expect("a folder entry").path();
            let relative = path.strip_prefix(root).expect("inside the checkout");
            if relative == Path::new(".git") {
                continue;
            }
            let metadata = std::fs::symlink_metadata(&path).expect("stat an entry");
            if metadata.is_dir() {
                entries.push(format!("folder {}", relative.display()).into_bytes());
                stack.push(path);
            } else {
                let mut line = format!(
                    "file {} {:o} ",
                    relative.display(),
                    metadata.permissions().mode()
                )
                .into_bytes();
                line.extend(std::fs::read(&path).expect("read a file"));
                entries.push(line);
            }
        }
    }
    entries.sort();
    for entry in entries {
        out.extend(entry);
        out.push(0);
    }
    out
}

#[test]
fn relocate_folder_rows_sharing_a_parent_each_land_at_their_to() {
    let repo = camera_checkout("shared-parent");
    let manifest = repo.manifest(CAMERA_ROWS);
    let plan = plan_of(&repo, &manifest).expect("the plan builds");
    let text = std::fs::read_to_string(&manifest).expect("read the manifest");
    let rows = parse_manifest(&text).expect("a valid manifest");
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the checkout");
    let mapping = PathMapping::from_rows(&rows);
    let planned: Vec<String> = PlannedTree::new(&snapshot, &mapping, &plan)
        .paths()
        .files()
        .cloned()
        .collect();

    assert_eq!(dry_run(repo.root(), &manifest), 0);
    assert_eq!(apply(repo.root(), &manifest), 0);
    let expected = [
        "crates/camera_math/src/lib.rs",
        "crates/camera_math/src/matrix4.rs",
        "crates/camera_math/src/orbit/mod.rs",
        "crates/camera_math/src/ortho/mod.rs",
        "crates/camera_math/src/ortho/view.rs",
        "crates/camera_math/src/tests/README.md",
        "crates/camera_math/src/tests/glmat4_tests.rs",
        "docs/cameras.md",
    ];
    assert_eq!(repo.tracked(), expected);
    assert_eq!(planned, expected, "the dry run's planned tree is the tree");
    assert!(!repo.exists("crates/camera_math/src/math"));
    assert_eq!(
        repo.read("docs/cameras.md"),
        "[math](/crates/camera_math/src/lib.rs), [ortho](/crates/camera_math/src/ortho/view.rs), \
         [orbit](/crates/camera_math/src/orbit/mod.rs)\n"
    );
}

#[test]
fn relocate_rows_that_collide_are_refused_with_both_lines() {
    let shared = FixtureRepository::new("same-to");
    shared
        .write("scheduler/tests/chunk_tests.rs", "\n")
        .write("camera/tests/grid_tests.rs", "\n")
        .track();
    let same_to = shared.manifest(
        "path\tscheduler/tests\tcoordinates/src/tests\t\n\
         path\tcamera/tests\tcoordinates/src/tests\t\n",
    );
    let errors = refusals(&shared, &same_to);
    assert!(
        errors
            .iter()
            .any(|error| error.contains("lines 2 and 3") && error.contains("coordinates/src/tests")),
        "{errors:?}"
    );
    assert_eq!(dry_run(shared.root(), &same_to), 2);
    assert_eq!(apply(shared.root(), &same_to), 2);
    assert!(
        shared.exists("scheduler/tests/chunk_tests.rs"),
        "nothing moved"
    );

    let occupied = FixtureRepository::new("occupied-to");
    occupied
        .write("camera/math/mod.rs", "\n")
        .write("camera/math/ortho/perspective.rs", "\n")
        .write("camera/ortho/mod.rs", "\n")
        .track();
    let nested = occupied.manifest(
        "path\tcamera/math\tcamera_math/src\t\n\
         path\tcamera/ortho\tcamera_math/src/ortho\t\n",
    );
    let errors = refusals(&occupied, &nested);
    assert!(
        errors.iter().any(|error| error.contains("lines 2 and 3")),
        "{errors:?}"
    );
    assert_eq!(apply(occupied.root(), &nested), 2);
    assert!(
        occupied.exists("camera/math/ortho/perspective.rs"),
        "nothing moved"
    );

    let swapped = FixtureRepository::new("swap");
    swapped
        .write("left/a.rs", "\n")
        .write("right/b.rs", "\n")
        .track();
    let swap = swapped.manifest("path\tleft\tright\t\npath\tright\tleft\t\n");
    let errors = refusals(&swapped, &swap);
    assert!(
        errors.iter().any(|error| error.contains("lines 2 and 3")),
        "{errors:?}"
    );
    assert_eq!(apply(swapped.root(), &swap), 2);
    assert_eq!(swapped.tracked(), ["left/a.rs", "right/b.rs"]);
}

#[test]
fn relocate_failed_apply_leaves_index_and_tree_byte_identical() {
    let probe = camera_checkout("undo-count");
    let probe_manifest = probe.manifest(CAMERA_ROWS);
    let probe_plan = plan_of(&probe, &probe_manifest).expect("the plan builds");
    let steps = probe_plan.moves.len() + probe_plan.rewrites.len();
    assert_eq!(steps, 6, "five moves and one rewrite");
    for stop in 1..=steps {
        let repo = camera_checkout(&format!("undo-step-{stop}"));
        let manifest = repo.manifest(CAMERA_ROWS);
        let plan = plan_of(&repo, &manifest).expect("the plan builds");
        let before = fingerprint(repo.root());
        let outcome = apply_plan_with_checkpoint(repo.root(), &plan, &mut |done| {
            if done == stop {
                Err(format!("failure injected after step {done}"))
            } else {
                Ok(())
            }
        });
        assert!(outcome.is_err(), "step {stop} must stop the apply");
        assert!(
            fingerprint(repo.root()) == before,
            "after step {stop}: the checkout differs after the undo ({outcome:?})"
        );
    }

    let repo = camera_checkout("undo-occupied");
    let manifest = repo.manifest(CAMERA_ROWS);
    let plan = plan_of(&repo, &manifest).expect("the plan builds");
    // An untracked folder planted at a destination after planning stops the apply at that move
    // instead of letting `git mv` nest the moved folder inside it.
    repo.write("crates/camera_math/src/planted.txt", "untracked\n");
    let before = fingerprint(repo.root());
    let outcome = apply_plan(repo.root(), &plan);
    assert!(
        outcome
            .as_ref()
            .is_err_and(|reason| reason.contains("would nest")),
        "{outcome:?}"
    );
    assert!(fingerprint(repo.root()) == before, "{outcome:?}");

    let repo = camera_checkout("undo-write");
    let manifest = repo.manifest(CAMERA_ROWS);
    let plan = plan_of(&repo, &manifest).expect("the plan builds");
    assert!(!plan.rewrites.is_empty());
    // A read-only rewritten file fails its write after every move has run.
    let locked = repo.root().join("docs/cameras.md");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444))
        .expect("lock the file");
    let before = fingerprint(repo.root());
    let outcome = apply_plan(repo.root(), &plan);
    assert!(outcome.is_err(), "the write must fail");
    assert!(
        fingerprint(repo.root()) == before,
        "the checkout differs after the undo of a failed write ({outcome:?})"
    );
}
