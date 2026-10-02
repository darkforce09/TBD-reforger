use super::*;

/// The ready-tier body rule: live tree green (ready-class plus
/// shipped after the drain); a planted ready or shipped work ticket with
/// the six fields empty reds NAMING EACH missing field; the quarantine exemption
/// and the queued tier stay green.
#[test]
fn ready_tier_body_red_green_and_quarantine_exempt() {
    let root = worktree_root();
    let errs = check_ready_tier_body(&root);
    assert!(
        errs.is_empty(),
        "live ready-class and shipped work must carry the six body fields; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t920-ready-tier");
    let ready = |status: &str, extra: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"{status}\"\norder = 10\nspec = \"docs/spec.md\"\nmain_goal = \"goal\"\n{extra}acceptance = [\"gate\"]\nowns = [\"a.rs\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    for status in ["ready", "running", "review"] {
        fs::write(dir.join("T-001.toml"), ready(status, "")).unwrap();
        let errs = check_ready_tier_body(&tmp);
        assert_eq!(errs.len(), 1, "{status}: {errs:?}");
        assert!(
            errs[0].contains("T-001")
                && errs[0].contains(status)
                && errs[0].contains("context, requirement, current_state, approach, verify"),
            "must name each empty field: {}",
            errs[0]
        );
    }
    // Partial fill: exactly the still-empty fields are named.
    fs::write(
        dir.join("T-001.toml"),
        ready("ready", "context = [\"why\"]\nrequirement = [\"ask\"]\n"),
    )
    .unwrap();
    let errs = check_ready_tier_body(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("fields: current_state, approach, verify —"),
        "{errs:?}"
    );
    // Full fill: green.
    fs::write(
            dir.join("T-001.toml"),
            ready(
                "ready",
                "context = [\"why\"]\nrequirement = [\"ask\"]\ncurrent_state = [\"today\"]\napproach = [\"steps\"]\nverify = [\"cargo test\"]\n",
            ),
        )
        .unwrap();
    assert!(check_ready_tier_body(&tmp).is_empty(), "filled is green");
    // Quarantine exemption: the same empty-bodied ready ticket with a nonempty
    // migration_legacy is green (content exists, unprocessed).
    fs::write(
        dir.join("T-001.toml"),
        ready("ready", "migration_legacy = [\"parked wall\"]\n"),
    )
    .unwrap();
    assert!(
        check_ready_tier_body(&tmp).is_empty(),
        "quarantined ready ticket is exempt"
    );
    // Queued work is the pin-metered tier, not this rule's.
    fs::write(
            dir.join("T-001.toml"),
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"queued\"\norder = 10\nowns = [\"a.rs\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n",
        )
        .unwrap();
    assert!(
        check_ready_tier_body(&tmp).is_empty(),
        "queued is exempt from the ready tier"
    );
    // Zeroing: shipped joins this rule. Empty-bodied shipped reds;
    // quarantine exemption still holds; filled shipped is green.
    let shipped = |extra: &str| {
        format!(
            "id = \"T-001\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"shipped\"\norder = 10\nspec = \"docs/spec.md\"\nmain_goal = \"goal\"\nshipped_at = \"abcdef12\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n{extra}acceptance = [\"gate\"]\nowns = [\"a.rs\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    fs::write(dir.join("T-001.toml"), shipped("")).unwrap();
    let errs = check_ready_tier_body(&tmp);
    assert_eq!(errs.len(), 1, "shipped empty: {errs:?}");
    assert!(
        errs[0].contains("T-001")
            && errs[0].contains("shipped")
            && errs[0].contains("context, requirement, current_state, approach, verify"),
        "shipped empty must name each field: {}",
        errs[0]
    );
    fs::write(
        dir.join("T-001.toml"),
        shipped("migration_legacy = [\"parked wall\"]\n"),
    )
    .unwrap();
    assert!(
        check_ready_tier_body(&tmp).is_empty(),
        "quarantined shipped ticket is exempt"
    );
    fs::write(
            dir.join("T-001.toml"),
            shipped(
                "context = [\"why\"]\nrequirement = [\"ask\"]\ncurrent_state = [\"today\"]\napproach = [\"steps\"]\nverify = [\"cargo test\"]\n",
            ),
        )
        .unwrap();
    assert!(
        check_ready_tier_body(&tmp).is_empty(),
        "filled shipped is green"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The pin growth verdict: equality is silent; growth blames the
/// offender (never the pin); below-pin stays green HERE because check runs on
/// arbitrary roots (a 4-ticket scratch measures 0 against the live pin) — the
/// shrink direction is the store ratchet tests' exact-equality red, on the live
/// tree, in CI (`title_debt_ratchet_pin` / `main_goal_debt_ratchet_pin`).
#[test]
fn debt_pin_growth_verdict() {
    assert!(pin_growth_finding("TITLE_DEBT_PIN", 440, 440, "i").is_empty());
    let grown = pin_growth_finding("TITLE_DEBT_PIN", 441, 440, "the instrument text");
    assert_eq!(grown.len(), 1);
    assert!(
        grown[0].contains("441 > pin 440")
            && grown[0].contains("fix the ticket, never the pin")
            && grown[0].contains("the instrument text"),
        "{}",
        grown[0]
    );
    assert!(
        pin_growth_finding("MAIN_GOAL_DEBT_PIN", 0, 53, "i").is_empty(),
        "below-pin is the scratch-tree case — green in check, red in the ratchet test"
    );
}

/// The strict honesty counters over a scratch fixture whose numbers are
/// hand-computable: 4 shipped — one receipted+measured, one diff_loc-estimated
/// (git_subject stamps), one cohort_median-estimated (id_interpolation stamps),
/// one measured-stamps with a receipt missing tokens accounting entirely (the
/// counters COUNT, they do not police — the gate rule reds it separately).
#[test]
fn honesty_counters_fixture_math() {
    let (tmp, dir) = scratch_tickets_dir("t917-counters");
    let shipped = |id: &str, extra: &str| {
        format!(
            "id = \"{id}\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"shipped\"\norder = 10\nshipped_at = \"abcdef12\"\ncreated_at = \"2026-07-01T10:00:00Z\"\ncompleted_at = \"2026-07-02T10:00:00Z\"\n{extra}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    // The first fixture ticket: receipt, measured stamps.
    fs::write(dir.join("T-001.toml"), shipped("T-001", "")).unwrap();
    let rdir = tmp.join(crate::repository::METRICS_DIR).join("T-001");
    fs::create_dir_all(&rdir).unwrap();
    fs::write(rdir.join("r.json"), "{}").unwrap();
    // The second: diff_loc estimate, git_subject-mined stamps.
    fs::write(
            dir.join("T-002.toml"),
            shipped(
                "T-002",
                "estimated = [\"created_at\", \"completed_at\", \"tokens\"]\nestimate_note = \"created_at/completed_at git_subject-mined from 2 commit subject(s)\"\n",
            ),
        )
        .unwrap();
    // The third: cohort_median estimate, interpolated stamps (no git_subject token).
    fs::write(
            dir.join("T-003.toml"),
            shipped(
                "T-003",
                "estimated = [\"created_at\", \"completed_at\", \"tokens\"]\nestimate_note = \"no subject commits; created_at/completed_at id-interpolated between T-001 and T-005\"\n",
            ),
        )
        .unwrap();
    // The fourth: measured stamps, NO accounting (counted 0/0 — the gate rule reds it).
    fs::write(dir.join("T-004.toml"), shipped("T-004", "")).unwrap();
    // A queued ticket must not count anywhere.
    fs::write(
            dir.join("T-005.toml"),
            "id = \"T-005\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"queued\"\norder = 11\nowns = [\"a.rs\"]\nestimated = [\"created_at\"]\ncreated_at = \"2026-07-01T10:00:00Z\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n",
        )
        .unwrap();
    let est_dir = tmp.join(crate::repository::ESTIMATES_DIR);
    fs::create_dir_all(&est_dir).unwrap();
    fs::write(
            est_dir.join("T-002.json"),
            "{\n  \"derived_from_shas\": [\n    \"aaaa111122223333\"\n  ],\n  \"factor\": 150,\n  \"generated_at\": \"2026-08-15T00:00:00Z\",\n  \"id\": \"T-002\",\n  \"loc_changed\": 10,\n  \"source\": \"diff_loc\",\n  \"tokens_estimated\": 1500\n}\n",
        )
        .unwrap();
    fs::write(
            est_dir.join("T-003.json"),
            "{\n  \"cohort\": {\n    \"class\": \"chore\"\n  },\n  \"cohort_size\": 3,\n  \"factor\": 150,\n  \"generated_at\": \"2026-08-15T00:00:00Z\",\n  \"id\": \"T-003\",\n  \"source\": \"cohort_median\",\n  \"tokens_estimated\": 3000\n}\n",
        )
        .unwrap();

    let lines = strict_honesty_counters(&tmp).expect("counters over a loadable tree");
    assert_eq!(
        lines,
        vec![
            "shipped tokens measured/estimated: 1/2 (diff_loc 1, cohort_median 1)".to_string(),
            "stamps: measured 2-tickets, estimated 2-tickets (git_subject 1, id_interpolation 1)"
                .to_string(),
        ],
        "counter math must equal the hand computation"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

/// The referential-integrity rule. The live tree must be green (measured: zero
/// violations, the cross-listed child included — file and parent both exist); a children[] entry
/// without a file and a child whose parent file is missing must each go red naming BOTH
/// ids; restoring the files restores green.
#[test]
fn children_integrity_red_green() {
    let root = worktree_root();
    let errs = check_children_integrity(&root);
    assert!(
        errs.is_empty(),
        "live tree must be referentially intact; got:\n{}",
        errs.join("\n")
    );

    let (tmp, dir) = scratch_tickets_dir("t916-refint-check");
    let program = r#"id = "T-009"
kind = "program"
title = "x"
summary = "x"
status = "idea"
children = [
    "T-009.1",
    "T-009.2",
]
"#;
    let child = |id: &str, parent: &str| {
        format!(
            "id = \"{id}\"\nkind = \"work\"\ntitle = \"x\"\nsummary = \"x\"\nclass = \"chore\"\nstatus = \"idea\"\nparent = \"{parent}\"\n\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
        )
    };
    fs::write(dir.join("T-009.toml"), program).unwrap();
    fs::write(dir.join("T-009.1.toml"), child("T-009.1", "T-009")).unwrap();
    // A child listed but missing on disk → red naming lister and child.
    let errs = check_children_integrity(&tmp);
    assert_eq!(
        errs,
        vec![
            "T-009: children[] names T-009.2, which has no .ai/tickets/T-009.2.toml on disk"
                .to_string()
        ],
        "dangling children[] entry must be red"
    );
    fs::write(dir.join("T-009.2.toml"), child("T-009.2", "T-009")).unwrap();
    assert!(
        check_children_integrity(&tmp).is_empty(),
        "restored child file must be green"
    );

    // A child whose parent file is absent → red naming child and parent.
    fs::write(dir.join("T-010.4.toml"), child("T-010.4", "T-010")).unwrap();
    let errs = check_children_integrity(&tmp);
    assert_eq!(
        errs,
        vec!["T-010.4: parent T-010 has no .ai/tickets/T-010.toml on disk".to_string()],
        "orphaned child must be red"
    );
    fs::remove_file(dir.join("T-010.4.toml")).unwrap();
    assert!(check_children_integrity(&tmp).is_empty());

    // Fail-closed: an unparseable corpus reports the load error, never a clean scan.
    fs::write(dir.join("T-011.toml"), "id = \"T-011\"\nkind = \"nope\"\n").unwrap();
    let errs = check_children_integrity(&tmp);
    assert_eq!(errs.len(), 1, "one load refusal: {errs:?}");
    assert!(errs[0].contains("T-011"), "must name the file: {}", errs[0]);
    fs::remove_dir_all(&tmp).unwrap();
}

/// A malformed lifecycle stamp is a parse error that names the ticket — the
/// every-file walk (`check_open_work_owns` reuses `parse_ticket_toml`) goes red, and
/// nothing coerces the value to now. Valid stamps restore green.
#[test]
fn malformed_timestamp_is_red() {
    let (tmp, dir) = scratch_tickets_dir("t913-timestamp-check");
    let bad = r#"id = "T-001.1"
kind = "work"
title = "x"
summary = "x"
class = "chore"
status = "queued"
order = 10
created_at = "2026-13-99T25:61:00Z"
owns = ["docs/README.md"]

[scope]
domain = "repo"
layer = "docs"
"#;
    fs::write(dir.join("T-001.1.toml"), bad).unwrap();
    let errs = check_open_work_owns(&tmp);
    assert_eq!(errs.len(), 1, "exactly one parse error: {errs:?}");
    assert!(
        errs[0].contains("T-001.1") && errs[0].contains("created_at"),
        "error must name ticket and field: {}",
        errs[0]
    );

    let naive = bad.replace("2026-13-99T25:61:00Z", "2026-08-14 10:00");
    fs::write(dir.join("T-001.1.toml"), naive).unwrap();
    let errs = check_open_work_owns(&tmp);
    assert!(
        errs.len() == 1 && errs[0].contains("created_at"),
        "naive datetime must be red: {errs:?}"
    );

    let good = bad.replace("2026-13-99T25:61:00Z", "2026-08-14T10:00:00Z");
    fs::write(dir.join("T-001.1.toml"), good).unwrap();
    assert!(
        check_open_work_owns(&tmp).is_empty(),
        "valid RFC 3339 UTC must restore green"
    );
    fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn require_check_ok_blocks_invalid_registry() {
    let root = worktree_root();
    let mut registry = load_registry(&root).expect("load tip registry");
    registry
        .get_mut("tickets")
        .and_then(|t| t.as_array_mut())
        .expect("tickets")
        .first_mut()
        .expect("ticket")
        .as_object_mut()
        .expect("obj")
        .insert("status".into(), json!("not-a-real-status"));
    let errs = check(&root, &registry, false);
    assert!(
        !errs.is_empty(),
        "invalid status must fail check (ship/set-status preflight relies on this)"
    );
    assert!(
        errs.iter().any(|e| e.contains("schema")),
        "expected schema error for bogus status: {errs:?}"
    );
    let refuse = require_check_ok(&root, &registry, "set-status T-001");
    assert!(
        refuse.is_err(),
        "require_check_ok must Err on red registry (T-451)"
    );
    let msg = format!("{:#}", refuse.unwrap_err());
    assert!(
        msg.contains("refusing set-status T-001"),
        "refuse message missing: {msg}"
    );
}
