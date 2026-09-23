use super::*;

/// One program file whose `fields` land between its status line and its children.
fn program(status: &str, fields: &str) -> String {
    format!(
        "id = \"T-009\"\nkind = \"program\"\ntitle = \"x\"\nsummary = \"x\"\nstatus = \"{status}\"\n{fields}children = [\"T-009.1\"]\n"
    )
}

/// The program's one dotted child, whose `fields` land between its status line and its parent.
fn child(status: &str, fields: &str) -> String {
    format!(
        "id = \"T-009.1\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"{status}\"\n{fields}parent = \"T-009\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
    )
}

fn write_tree(dir: &Path, program_file: &str, child_file: &str) {
    fs::write(dir.join("T-009.toml"), program_file).unwrap();
    fs::write(dir.join("T-009.1.toml"), child_file).unwrap();
}

/// The live tree is green: every spec and plan that a parent or child file names exists.
#[test]
fn live_tree_names_only_existing_spec_and_plan_files() {
    let errs = check_spec_and_plan_files_exist(&worktree_root());
    assert!(
        errs.is_empty(),
        "live ticket files name missing documents:\n{}",
        errs.join("\n")
    );
}

/// A child naming a missing spec is red for every status that binds, naming the child and the
/// path; the file landing restores green.
#[test]
fn child_spec_missing_on_disk_is_red_until_the_file_exists() {
    let (tmp, dir) = scratch_tickets_dir("spec-plan-files-child-spec");
    for (status, order) in [
        ("queued", "order = 10\n"),
        ("shipped", ""),
        ("deferred", ""),
    ] {
        write_tree(
            &dir,
            &program("idea", ""),
            &child(status, &format!("{order}spec = \"docs/child_spec.md\"\n")),
        );
        assert_eq!(
            check_spec_and_plan_files_exist(&tmp),
            vec!["T-009.1: spec missing on disk: docs/child_spec.md".to_string()],
            "{status} child"
        );
    }
    fs::create_dir_all(tmp.join("docs")).unwrap();
    fs::write(tmp.join("docs/child_spec.md"), "# spec\n").unwrap();
    assert!(
        check_spec_and_plan_files_exist(&tmp).is_empty(),
        "a spec that exists is green"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// A child naming a missing plan is red the same way, and green once the plan exists.
#[test]
fn child_plan_missing_on_disk_is_red_until_the_file_exists() {
    let (tmp, dir) = scratch_tickets_dir("spec-plan-files-child-plan");
    for (status, order) in [
        ("queued", "order = 10\n"),
        ("shipped", ""),
        ("deferred", ""),
    ] {
        write_tree(
            &dir,
            &program("idea", ""),
            &child(
                status,
                &format!("{order}plan = \"docs/plans/t-009_1_plan.md\"\n"),
            ),
        );
        assert_eq!(
            check_spec_and_plan_files_exist(&tmp),
            vec!["T-009.1: plan missing on disk: docs/plans/t-009_1_plan.md".to_string()],
            "{status} child"
        );
    }
    fs::create_dir_all(tmp.join("docs/plans")).unwrap();
    fs::write(tmp.join("docs/plans/t-009_1_plan.md"), "# plan\n").unwrap();
    assert!(
        check_spec_and_plan_files_exist(&tmp).is_empty(),
        "a plan that exists is green"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// Parents bind exactly like children: a shipped program reports each missing field once.
#[test]
fn parent_spec_and_plan_missing_on_disk_are_red() {
    let (tmp, dir) = scratch_tickets_dir("spec-plan-files-parent");
    write_tree(
        &dir,
        &program(
            "shipped",
            "spec = \"docs/program_spec.md\"\nplan = \"docs/plans/t-009_plan.md\"\n",
        ),
        &child("idea", ""),
    );
    assert_eq!(
        check_spec_and_plan_files_exist(&tmp),
        vec![
            "T-009: spec missing on disk: docs/program_spec.md".to_string(),
            "T-009: plan missing on disk: docs/plans/t-009_plan.md".to_string(),
        ]
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// `idea` and `cancelled` tickets may name documents that do not exist, parent and child alike.
#[test]
fn idea_and_cancelled_tickets_are_exempt() {
    let (tmp, dir) = scratch_tickets_dir("spec-plan-files-exempt");
    let missing = "spec = \"docs/absent_spec.md\"\nplan = \"docs/plans/absent_plan.md\"\n";
    for status in ["idea", "cancelled"] {
        write_tree(&dir, &program(status, missing), &child(status, missing));
        assert!(
            check_spec_and_plan_files_exist(&tmp).is_empty(),
            "{status} tickets are exempt"
        );
    }
    fs::remove_dir_all(&tmp).unwrap();
}

/// Fail-closed: an unparseable ticket file reports the load error, never a clean scan.
#[test]
fn an_unloadable_corpus_reports_the_load_error() {
    let (tmp, dir) = scratch_tickets_dir("spec-plan-files-unloadable");
    fs::write(dir.join("T-011.toml"), "id = \"T-011\"\nkind = \"nope\"\n").unwrap();
    let errs = check_spec_and_plan_files_exist(&tmp);
    assert_eq!(errs.len(), 1, "one load refusal: {errs:?}");
    assert!(errs[0].contains("T-011"), "must name the file: {}", errs[0]);
    fs::remove_dir_all(&tmp).unwrap();
}
