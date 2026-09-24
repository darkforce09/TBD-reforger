use super::*;

#[test]
fn tip_registry_passes_schema() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let errs = validate_registry_schema(&root, &registry);
    assert!(
        errs.is_empty(),
        "tip registry must PASS schema; got:\n{}",
        errs.join("\n")
    );
}

#[test]
fn tip_registry_full_check_ok() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let errs = check(&root, &registry, false);
    assert!(
        errs.is_empty(),
        "tip registry must PASS full check; got:\n{}",
        errs.join("\n")
    );
}

#[test]
fn perturbed_ticket_field_fails_schema() {
    let root = worktree_root();
    let mut registry = load_registry(&root).expect("load tip registry");
    let tickets = registry
        .get_mut("tickets")
        .and_then(|t| t.as_array_mut())
        .expect("tickets array");
    let first = tickets.first_mut().expect("at least one ticket");
    first
        .as_object_mut()
        .expect("ticket object")
        .remove("title");
    let errs = validate_registry_schema(&root, &registry);
    assert!(
        !errs.is_empty(),
        "removing required title must make schema check RED"
    );
    assert!(
        errs.iter().any(|e| e.contains("schema")),
        "errors should be schema-tagged: {errs:?}"
    );
}

/// The id pattern admits parents of three or more digits, so the first four-digit id is legal,
/// and still refuses a two-digit parent or a malformed suffix.
#[test]
fn schema_admits_ids_of_three_or_more_digits() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let errors_with_first_id = |id: &str| {
        let mut registry = registry.clone();
        registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .and_then(|t| t.first_mut())
            .and_then(|t| t.as_object_mut())
            .expect("first ticket object")
            .insert("id".into(), json!(id));
        validate_registry_schema(&root, &registry)
    };
    for id in ["T-649", "T-1000", "T-1000.2", "T-12345.6.7"] {
        let errs = errors_with_first_id(id);
        assert!(errs.is_empty(), "{id} must pass: {}", errs.join("\n"));
    }
    for id in ["T-99", "T-99.1", "T-1000.", "T-1000a"] {
        assert!(
            !errors_with_first_id(id).is_empty(),
            "{id} must fail the id pattern"
        );
    }
}

#[test]
fn perturbed_schema_rejects_tip_registry() {
    let root = worktree_root();
    let registry = load_registry(&root).expect("load tip registry");
    let schema_path = ticket_schema_path(&root);
    let schema_text = fs::read_to_string(&schema_path).expect("read schema");
    let mut schema: Value = serde_json::from_str(&schema_text).expect("parse schema");
    // Narrow root type to array — tip registry is an object → must fail.
    schema
        .as_object_mut()
        .expect("schema object")
        .insert("type".into(), json!("array"));
    let validator = jsonschema::validator_for(&schema).expect("perturbed schema still compiles");
    let errs: Vec<_> = validator.iter_errors(&registry).collect();
    assert!(
        !errs.is_empty(),
        "type=array schema must reject object registry"
    );
}

/// The owns rule sees CHILD ticket files. The live tree must be green, and an
/// owns-empty queued work ticket dropped into a synthetic tickets dir must go red — including
/// a dotted child id the parents-only registry view never loads.
#[test]
fn open_work_without_owns_is_red() {
    let root = worktree_root();
    let errs = check_open_work_owns(&root);
    assert!(
        errs.is_empty(),
        "live tree must have owns on every open work ticket; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t912-owns-check");
    let bad = r#"id = "T-001.1"
kind = "work"
title = "x"
summary = "x"
class = "chore"
status = "queued"
order = 10

[scope]
domain = "repo"
layer = "docs"
"#;
    fs::write(dir.join("T-001.1.toml"), bad).unwrap();
    let errs = check_open_work_owns(&tmp);
    assert_eq!(
        errs,
        vec!["T-001.1: owns required for queued work ticket".to_string()],
        "owns-empty queued child must be red"
    );

    let good = bad.replace("order = 10\n", "order = 10\nowns = [\"docs/README.md\"]\n");
    fs::write(dir.join("T-001.1.toml"), good).unwrap();
    assert!(
        check_open_work_owns(&tmp).is_empty(),
        "nonempty owns must restore green"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// Class is required on every work ticket — the live tree is green (the
/// migrator triaged all of history), and a planted class-less work ticket reds
/// naming ticket + the legal set; restoring class restores green.
#[test]
fn work_without_class_is_red() {
    let root = worktree_root();
    let errs = check_work_class(&root);
    assert!(
        errs.is_empty(),
        "live tree must carry class on every work ticket; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-class-check");
    let bare = r#"id = "T-001"
kind = "work"
title = "x"
summary = "x"
status = "idea"

[scope]
domain = "repo"
layer = "docs"
"#;
    fs::write(dir.join("T-001.toml"), bare).unwrap();
    let errs = check_work_class(&tmp);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001")
            && errs[0].contains("class required")
            && errs[0].contains("bug|feature|chore|audit|docs"),
        "{}",
        errs[0]
    );
    fs::write(
        dir.join("T-001.toml"),
        bare.replace("summary = \"x\"\n", "summary = \"x\"\nclass = \"chore\"\n"),
    )
    .unwrap();
    assert!(check_work_class(&tmp).is_empty(), "class restores green");
    fs::remove_dir_all(&tmp).unwrap();
}

/// Live work with a component but no surface is red unless the migrator's
/// `"scope" ∈ estimated[]` escape is recorded; component-free scope is exempt.
#[test]
fn live_work_component_without_surface_is_red() {
    let root = worktree_root();
    let errs = check_live_work_surface(&root);
    assert!(
        errs.is_empty(),
        "live tree must satisfy the surface rule; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-surface-check");
    let bare = r#"id = "T-001"
kind = "work"
title = "x"
summary = "x"
class = "feature"
status = "queued"
order = 10
owns = ["a.rs"]

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
"#;
    fs::write(dir.join("T-001.toml"), bare).unwrap();
    let errs = check_live_work_surface(&tmp);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001")
            && errs[0].contains("surface required")
            && errs[0].contains("mission_creator"),
        "{}",
        errs[0]
    );
    // The migrator's honest escape…
    fs::write(
        dir.join("T-001.toml"),
        bare.replace(
            "owns = [\"a.rs\"]\n",
            "owns = [\"a.rs\"]\nestimated = [\"scope\"]\n",
        ),
    )
    .unwrap();
    assert!(
        check_live_work_surface(&tmp).is_empty(),
        "scope ∈ estimated must pass"
    );
    // …a real surface…
    fs::write(
        dir.join("T-001.toml"),
        bare.replace(
            "component = \"mission_creator\"\n",
            "component = \"mission_creator\"\nsurface = [\"map_canvas\"]\n",
        ),
    )
    .unwrap();
    assert!(
        check_live_work_surface(&tmp).is_empty(),
        "nonempty surface must pass"
    );
    // …component-free scope…
    fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "[scope]\ndomain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\n",
                "[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n",
            ),
        )
        .unwrap();
    assert!(
        check_live_work_surface(&tmp).is_empty(),
        "component-free scope is exempt"
    );
    // …and a component whose vocabulary surface list is EMPTY (mod.scripts.backend
    // a shape the live tree carries) are all green: the rule cannot require a
    // surface the vocabulary does not offer.
    fs::write(
            dir.join("T-001.toml"),
            bare.replace(
                "[scope]\ndomain = \"website\"\nlayer = \"frontend\"\ncomponent = \"mission_creator\"\n",
                "[scope]\ndomain = \"mod\"\nlayer = \"scripts\"\ncomponent = \"backend\"\n",
            ),
        )
        .unwrap();
    assert!(
        check_live_work_surface(&tmp).is_empty(),
        "empty vocab surface list is exempt until widened"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The body cap rules. The live tree is green (post-quarantine); a
/// planted 41-word summary reds naming ticket, field, count and cap; a 31-word
/// context line, a 9-word citation and an owns-duplicating citation each red; a
/// command-shaped acceptance line WARNS (never errors); nonempty
/// migration_legacy exempts the summary cap ONLY.
#[test]
fn body_caps_red_green_and_warning_channel() {
    let root = worktree_root();
    let errs = check_body_rules(&root);
    assert!(
        errs.is_empty(),
        "live tree must satisfy the body caps; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-body-caps");
    let with_summary = |summary: &str, extra: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"short title\"\nsummary = \"{summary}\"\nclass = \"chore\"\nstatus = \"idea\"\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    let wall41 = (1..=41)
        .map(|i| format!("w{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    let load = |tmp: &Path| crate::Corpus::load(tmp).expect("scratch corpus loads");

    // 41-word summary → red naming ticket, field, count, cap.
    fs::write(dir.join("T-001.toml"), with_summary(&wall41, "")).unwrap();
    let (errs, warns) = body_findings(&load(&tmp));
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001") && errs[0].contains("summary is 41 words (cap 40)"),
        "{}",
        errs[0]
    );
    assert!(warns.is_empty(), "{warns:?}");

    // Nonempty migration_legacy exempts the summary cap — and ONLY the summary
    // cap: a 31-word context line on the same ticket still reds.
    let line31 = (1..=31)
        .map(|i| format!("c{i}"))
        .collect::<Vec<_>>()
        .join(" ");
    fs::write(
        dir.join("T-001.toml"),
        with_summary(
            &wall41,
            &format!(
                "migration_legacy = [\"parked wall\"]\ncontext = [\"why now\", \"{line31}\"]\n"
            ),
        ),
    )
    .unwrap();
    let (errs, _) = body_findings(&load(&tmp));
    assert_eq!(
        errs.len(),
        1,
        "summary exempt, context line still red: {errs:?}"
    );
    assert!(
        errs[0].contains("T-001") && errs[0].contains("context[1] is 31 words (cap 30)"),
        "{}",
        errs[0]
    );

    // 9-word citation and an owns-duplicating citation each red.
    fs::write(
            dir.join("T-001.toml"),
            with_summary(
                "fine",
                "owns = [\"docs/README.md\"]\ncitations = [\"one two three four five six seven eight nine\", \"docs/README.md\"]\n",
            ),
        )
        .unwrap();
    let (errs, _) = body_findings(&load(&tmp));
    assert_eq!(errs.len(), 2, "{errs:?}");
    assert!(
        errs.iter()
            .any(|e| e.contains("citations[0] is 9 words (cap 8)")),
        "{errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("citations[1]") && e.contains("duplicates an owns[] entry")),
        "{errs:?}"
    );

    // Command-shaped acceptance → WARNING pointing at verify[], never an error.
    fs::write(
            dir.join("T-001.toml"),
            with_summary(
                "fine",
                "acceptance = [\"cargo xtask ticket check prints check OK\", \"board renders ten fields\"]\n",
            ),
        )
        .unwrap();
    let (errs, warns) = body_findings(&load(&tmp));
    assert!(errs.is_empty(), "warning must not be an error: {errs:?}");
    assert_eq!(warns.len(), 1, "{warns:?}");
    assert!(
        warns[0].contains("T-001")
            && warns[0].contains("acceptance[0]")
            && warns[0].contains("verify[]"),
        "{}",
        warns[0]
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The quarantine is one-shot history migration — a work ticket carrying
/// migration_legacy with created_at past the 2026-08-15 cutover is red (new
/// tickets never quarantine); a pre-cutover stamp (or no stamp) stays green.
#[test]
fn quarantine_mint_past_cutover_is_red() {
    let (tmp, dir) = scratch_tickets_dir("t917-quarantine-mint");
    let quarantined = |created: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"short title\"\nsummary = \"short title\"\nclass = \"chore\"\nstatus = \"idea\"\n{created}migration_legacy = [\"parked wall\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    fs::write(
        dir.join("T-001.toml"),
        quarantined("created_at = \"2026-08-16T00:00:00Z\"\n"),
    )
    .unwrap();
    let (errs, _) = body_findings(&crate::Corpus::load(&tmp).unwrap());
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001")
            && errs[0].contains("2026-08-16T00:00:00Z")
            && errs[0].contains("cutover"),
        "{}",
        errs[0]
    );
    for green in [
        "created_at = \"2026-08-14T23:59:59Z\"\n",
        "", // stampless history is exempt (988 shipped lack created_at until S.4)
    ] {
        fs::write(dir.join("T-001.toml"), quarantined(green)).unwrap();
        let (errs, _) = body_findings(&crate::Corpus::load(&tmp).unwrap());
        assert!(errs.is_empty(), "{green:?} must be green: {errs:?}");
    }
    fs::remove_dir_all(&tmp).unwrap();
}

/// The estimated[]-vs-field coherence rule. Live tree green; a ticket
/// listing created_at/completed_at in estimated[] with the field ABSENT is red
/// naming ticket + field; shipped_at absent+marked is legal ONLY with an
/// estimate_note naming the gap; present fields restore green.
#[test]
fn estimated_marker_without_field_is_red() {
    let root = worktree_root();
    let errs = check_estimated_stamp_coherence(&root);
    assert!(
        errs.is_empty(),
        "live tree must satisfy estimated[] coherence; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-estimated-coherence");
    let with = |extra: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"shipped\"\norder = 10\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    // created_at marked but absent → red naming ticket + field.
    fs::write(
        dir.join("T-001.toml"),
        with("estimated = [\"created_at\"]\n"),
    )
    .unwrap();
    let errs = check_estimated_stamp_coherence(&tmp);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001") && errs[0].contains("created_at") && errs[0].contains("absent"),
        "{}",
        errs[0]
    );
    // completed_at marked but absent → red.
    fs::write(
        dir.join("T-001.toml"),
        with("estimated = [\"completed_at\"]\n"),
    )
    .unwrap();
    let errs = check_estimated_stamp_coherence(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("completed_at"),
        "{errs:?}"
    );
    // shipped_at absent+marked WITHOUT a note → red; WITH the gap named → green.
    fs::write(
        dir.join("T-001.toml"),
        with("estimated = [\"shipped_at\"]\n"),
    )
    .unwrap();
    let errs = check_estimated_stamp_coherence(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("shipped_at") && errs[0].contains("estimate_note"),
        "{errs:?}"
    );
    fs::write(
        dir.join("T-001.toml"),
        with(
            "estimated = [\"shipped_at\"]\nestimate_note = \"no subject commits; no SHA mined\"\n",
        ),
    )
    .unwrap();
    assert!(
        check_estimated_stamp_coherence(&tmp).is_empty(),
        "absent-marked shipped_at with the gap named is the legal asymmetry"
    );
    // All three present + marked → green (the backfill's normal output shape).
    fs::write(
            dir.join("T-001.toml"),
            with(
                "shipped_at = \"abcd1234\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\nestimated = [\"created_at\", \"completed_at\", \"shipped_at\"]\nestimate_note = \"mined git_subject\"\n",
            ),
        )
        .unwrap();
    assert!(check_estimated_stamp_coherence(&tmp).is_empty());
    fs::remove_dir_all(&tmp).unwrap();
}

/// THE ship gate, arm by arm. Live tree green (the S.2–S.5 passes plus
/// this slice's data fixes made it satisfiable); each planted violation reds
/// naming ticket + field (and the offending value for the SHA-shape arm); the
/// absent-marked-with-note asymmetry and receipt-or-estimate accounting are green.
/// Deliberately calls the gate fn directly — the double-report splits against the
/// The coherence and mutual-exclusion rules are documented on the fn and exercised by the full-check test
/// on the live tree.
#[test]
fn ship_gate_red_green_per_arm() {
    let root = worktree_root();
    let errs = check_ship_gate(&root);
    assert!(
        errs.is_empty(),
        "live tree must satisfy the ship gate; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-ship-gate");
    let shipped = |extra: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"shipped\"\norder = 10\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    let full = "shipped_at = \"abcdef12\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n";
    let estimate_path = tmp
        .join(crate::repository::ESTIMATES_DIR)
        .join("T-001.json");
    fs::create_dir_all(estimate_path.parent().unwrap()).unwrap();
    let with_estimate = || fs::write(&estimate_path, "{}").unwrap();

    // Fully stamped + estimate file → green.
    fs::write(dir.join("T-001.toml"), shipped(full)).unwrap();
    with_estimate();
    assert!(check_ship_gate(&tmp).is_empty(), "full stamps are green");

    // Missing completed_at → red naming ticket + field.
    fs::write(
        dir.join("T-001.toml"),
        shipped("shipped_at = \"abcdef12\"\ncreated_at = \"2026-07-01T10:00:00Z\"\n"),
    )
    .unwrap();
    let errs = check_ship_gate(&tmp);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001") && errs[0].contains("without completed_at"),
        "{}",
        errs[0]
    );

    // Missing created_at → red naming ticket + field.
    fs::write(
        dir.join("T-001.toml"),
        shipped("shipped_at = \"abcdef12\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n"),
    )
    .unwrap();
    let errs = check_ship_gate(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("without created_at"),
        "{errs:?}"
    );

    // Non-SHA-shaped shipped_at → red naming the VALUE (the branch-stray class).
    fs::write(
            dir.join("T-001.toml"),
            shipped("shipped_at = \"slice/T-197\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n"),
        )
        .unwrap();
    let errs = check_ship_gate(&tmp);
    assert!(
        errs.len() == 1
            && errs[0].contains("T-001")
            && errs[0].contains("\"slice/T-197\"")
            && errs[0].contains("not a commit SHA"),
        "{errs:?}"
    );

    // Absent + UNMARKED shipped_at → red pointing at stamp-sha.
    fs::write(
        dir.join("T-001.toml"),
        shipped("created_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n"),
    )
    .unwrap();
    let errs = check_ship_gate(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("without shipped_at") && errs[0].contains("stamp-sha"),
        "{errs:?}"
    );

    // Absent + marked WITH a note naming the gap → the legal asymmetry, green.
    fs::write(
            dir.join("T-001.toml"),
            shipped("created_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\nestimated = [\"shipped_at\"]\nestimate_note = \"no subject commits; no SHA mined\"\n"),
        )
        .unwrap();
    assert!(
        check_ship_gate(&tmp).is_empty(),
        "absent-marked-with-note is the legal asymmetry"
    );

    // No token accounting (neither receipt dir nor estimate file) → red.
    fs::remove_file(&estimate_path).unwrap();
    fs::write(dir.join("T-001.toml"), shipped(full)).unwrap();
    let errs = check_ship_gate(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("no token accounting"),
        "{errs:?}"
    );
    // A receipt dir with one file satisfies the arm too.
    let rdir = tmp.join(crate::repository::METRICS_DIR).join("T-001");
    fs::create_dir_all(&rdir).unwrap();
    fs::write(rdir.join("r.json"), "{}").unwrap();
    assert!(
        check_ship_gate(&tmp).is_empty(),
        "receipt satisfies the accounting arm"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The plan ready-gate: a ready/running/review WORK ticket without a
/// plan key reds naming the fix; a plan key satisfies the gate whether or not its file
/// exists, because a missing file is the spec-and-plan file rule's one finding;
/// programs and non-ready work are exempt.
#[test]
fn plan_ready_gate_red_green() {
    let root = worktree_root();
    let errs = check_plan_ready_gate(&root);
    assert!(
        errs.is_empty(),
        "live ready tickets must carry plans; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t917-plan-gate");
    let ready = |status: &str, plan_line: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"{status}\"\norder = 10\nspec = \"docs/spec.md\"\n{plan_line}main_goal = \"story\"\ncontext = [\"why\"]\nrequirement = [\"ask\"]\ncurrent_state = [\"today\"]\napproach = [\"steps\"]\nverify = [\"cargo test\"]\nacceptance = [\"gate\"]\nowns = [\"a.rs\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    for status in ["ready", "running", "review"] {
        fs::write(dir.join("T-001.toml"), ready(status, "")).unwrap();
        let errs = check_plan_ready_gate(&tmp);
        assert_eq!(errs.len(), 1, "{status}: {errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains(status)
                && errs[0].contains("requires plan"),
            "{}",
            errs[0]
        );
    }
    // Plan key present but the file is missing → the gate is satisfied, and the missing
    // file is reported once, by the spec-and-plan file rule.
    fs::write(
        dir.join("T-001.toml"),
        ready(
            "ready",
            "plan = \"documentation_v2/tickets/plans/t-001_plan.md\"\n",
        ),
    )
    .unwrap();
    assert!(
        check_plan_ready_gate(&tmp).is_empty(),
        "the gate checks the plan key, not the file"
    );
    let errs = check_spec_and_plan_files_exist(&tmp);
    assert!(
        errs.contains(
            &"T-001: plan missing on disk: documentation_v2/tickets/plans/t-001_plan.md"
                .to_string()
        ),
        "{errs:?}"
    );
    // Queued work is exempt (the gate binds on ready-class only).
    fs::write(
            dir.join("T-001.toml"),
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"queued\"\norder = 10\nowns = [\"a.rs\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n",
        )
        .unwrap();
    assert!(
        check_plan_ready_gate(&tmp).is_empty(),
        "queued work is exempt"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The idea-tier title rule: live tree green (measured zero empty
/// titles); a planted empty-title work ticket reds naming it; a real title
/// restores green. Programs are not this rule's business (work-shaped tier
/// table), and title != id / word-cap arms deliberately do NOT red here — they
/// are pin-metered debt.
#[test]
fn work_title_nonempty_red_green() {
    let root = worktree_root();
    let errs = check_work_title_nonempty(&root);
    assert!(
        errs.is_empty(),
        "live tree must carry a title on every work ticket; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t920-title-check");
    let with_title = |title: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"{title}\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"idea\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    fs::write(dir.join("T-001.toml"), with_title("   ")).unwrap();
    let errs = check_work_title_nonempty(&tmp);
    assert_eq!(errs.len(), 1, "{errs:?}");
    assert!(
        errs[0].contains("T-001") && errs[0].contains("title required"),
        "{}",
        errs[0]
    );
    // An id-as-title is NOT this rule's red (pin-metered debt, ops-gated).
    fs::write(dir.join("T-001.toml"), with_title("T-001")).unwrap();
    assert!(
        check_work_title_nonempty(&tmp).is_empty(),
        "id-as-title is debt, not an emptiness red"
    );
    fs::remove_dir_all(&tmp).unwrap();
}
