use super::*;

#[test]
fn parse_render_work_queued() {
    let t = Ticket::Work(WorkTicket {
        id: "T-905".into(),
        title: "x".into(),
        summary: "hello \" \\\\ world".into(),
        class: Some("chore".into()),
        status: Status::Queued { order: 5850 },
        executor: Some("claude-code".into()),
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: None,
        scope: ScopeV2 {
            domain: Domain::Repo,
            layer: "ci".into(),
            component: None,
            surface: vec![],
        },
        main_goal: None,
        context: vec![],
        requirement: vec![],
        current_state: vec![],
        approach: vec![],
        verify: vec![],
        acceptance: vec![],
        citations: vec![],
        shipped_at: None,
        priority: None,
        created_at: None,
        completed_at: None,
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec![],
        pack_last: None,
    });
    let s = render_ticket_toml(&t).unwrap();
    assert!(s.contains("status = \"queued\""));
    assert!(s.contains("order = 5850"));
    assert!(s.contains("[scope]"), "{s}");
    assert!(s.contains("domain = \"repo\""), "{s}");
    assert!(s.contains("layer = \"ci\""), "{s}");
    assert!(!s.contains("component"), "None component omitted:\n{s}");
    assert!(!s.contains("surface"), "empty surface omitted:\n{s}");
    let back = parse_ticket_toml(&s).unwrap();
    assert_eq!(t, back);
}

#[test]
fn flat_scope_full_depth_roundtrip() {
    let t = parse_ticket_toml(
        r#"
id = "T-816"
kind = "work"
title = "esc"
summary = "esc"
class = "feature"
status = "queued"
order = 4980
executor = "claude-code"

[scope]
domain = "website"
layer = "frontend"
component = "mission_creator"
surface = ["attr_panel", "toolbelt"]
"#,
    )
    .unwrap();
    match &t {
        Ticket::Work(w) => {
            assert_eq!(w.scope.domain, Domain::Website);
            assert_eq!(w.scope.layer, "frontend");
            assert_eq!(w.scope.component.as_deref(), Some("mission_creator"));
            assert_eq!(w.scope.surface, vec!["attr_panel", "toolbelt"]);
        }
        Ticket::Program(_) => panic!("work"),
    }
    let s = render_ticket_toml(&t).unwrap();
    assert!(s.contains("component = \"mission_creator\""), "{s}");
    assert_eq!(t, parse_ticket_toml(&s).unwrap());
}

/// The v1 nested scope tree must REFUSE under v2 types — the migrator's whole
/// reason to work Value→Value.
#[test]
fn v1_nested_scope_refuses() {
    let err = parse_ticket_toml(
        r#"
id = "T-001"
kind = "work"
title = "x"
status = "idea"

[scope.repo]
layers = ["docs"]
"#,
    )
    .unwrap_err();
    assert!(
        err.contains("domain") || err.contains("unknown field") || err.contains("repo"),
        "v1 scope must not parse as v2: {err}"
    );
}

#[test]
fn class_and_estimated_values_are_validated() {
    let base = |class: &str, estimated: &str| {
        format!(
            r#"
id = "T-901"
kind = "work"
title = "x"
status = "idea"
{class}
{estimated}

[scope]
domain = "repo"
layer = "docs"
"#
        )
    };
    let err = parse_ticket_toml(&base("class = \"epic\"", "")).unwrap_err();
    assert!(
        err.contains("T-901") && err.contains("epic") && err.contains("bug|feature"),
        "{err}"
    );
    let err = parse_ticket_toml(&base("", "estimated = [\"vibes\"]")).unwrap_err();
    assert!(
        err.contains("T-901") && err.contains("vibes") && err.contains("tokens"),
        "{err}"
    );
    for c in crate::CLASS_VALUES {
        parse_ticket_toml(&base(&format!("class = \"{c}\""), ""))
            .unwrap_or_else(|e| panic!("class {c} legal: {e}"));
    }
    for e in crate::ESTIMATED_VALUES {
        parse_ticket_toml(&base("", &format!("estimated = [\"{e}\"]")))
            .unwrap_or_else(|err| panic!("estimated {e} legal: {err}"));
    }
}

#[test]
fn surface_requires_component() {
    let err = parse_ticket_toml(
        r#"
id = "T-902"
kind = "work"
title = "x"
status = "idea"

[scope]
domain = "website"
layer = "frontend"
surface = ["map_canvas"]
"#,
    )
    .unwrap_err();
    assert!(
        err.contains("T-902") && err.contains("surface requires scope.component"),
        "{err}"
    );
}

/// T-917.2: every new key lands in its canonical slot — class after summary, plan
/// after spec, the body lists between main_goal and acceptance (in order),
/// citations after acceptance, estimated/estimate_note after completed_at,
/// migration_legacy immediately before owns, [scope] trailing. Extends the
/// T-913.1 `timestamps_roundtrip_in_canonical_slot` pattern.
#[test]
fn v2_keys_land_in_canonical_slots() {
    let t = Ticket::Work(WorkTicket {
        id: "T-917".into(),
        title: "slotted".into(),
        summary: "slotted".into(),
        class: Some("feature".into()),
        status: Status::Shipped {
            shipped_at: Some("abc123def".into()),
            order: Some(6000),
        },
        executor: Some("claude-code".into()),
        notes: Some("n".into()),
        spec: Some("docs/spec.md".into()),
        plan: Some("docs/plans/T-917_plan.md".into()),
        depends_on: vec!["T-1".into()],
        unblocks: vec!["T-2".into()],
        parent: None,
        scope: ScopeV2 {
            domain: Domain::Website,
            layer: "frontend".into(),
            component: Some("mission_creator".into()),
            surface: vec!["attr_panel".into()],
        },
        main_goal: Some("story".into()),
        context: vec!["why".into()],
        requirement: vec!["ask".into()],
        current_state: vec!["today".into()],
        approach: vec!["steps".into()],
        verify: vec!["cargo test".into()],
        acceptance: vec!["done".into()],
        citations: vec!["docs/x.md".into()],
        shipped_at: Some("abc123def".into()),
        priority: Some(1),
        created_at: Some("2026-08-14T10:00:00Z".into()),
        completed_at: Some("2026-08-14T11:30:00Z".into()),
        estimated: vec!["tokens".into()],
        estimate_note: Some("no receipts era".into()),
        migration_legacy: vec!["old wall".into()],
        owns: vec!["tools_v2/xtask/src/cmds.rs".into()],
        pack_last: None,
    });
    let s = render_ticket_toml(&t).unwrap();
    let pos = |needle: &str| {
        s.find(needle)
            .unwrap_or_else(|| panic!("{needle} in:\n{s}"))
    };
    let order = [
        "summary = ",
        "class = ",
        "status = ",
        "spec = ",
        "plan = ",
        "executor = ",
        "main_goal = ",
        "context = ",
        "requirement = ",
        "current_state = ",
        "approach = ",
        "verify = ",
        "acceptance = ",
        "citations = ",
        "shipped_at = ",
        "created_at = ",
        "completed_at = ",
        "estimated = ",
        "estimate_note = ",
        "migration_legacy = ",
        "owns = ",
        "[scope]",
    ];
    for pair in order.windows(2) {
        assert!(
            pos(pair[0]) < pos(pair[1]),
            "canonical slot violated: {} must precede {}:\n{s}",
            pair[0],
            pair[1]
        );
    }
    assert_eq!(t, parse_ticket_toml(&s).unwrap());
}

/// T-913.1: stamps round-trip and land in the canonical slot — after `shipped_at`
/// (still a bare SHA), before `owns`.
#[test]
fn timestamps_roundtrip_in_canonical_slot() {
    let t = Ticket::Work(WorkTicket {
        id: "T-914".into(),
        title: "stamped".into(),
        summary: "stamped".into(),
        class: Some("chore".into()),
        status: Status::Shipped {
            shipped_at: Some("abc123def".into()),
            order: Some(6000),
        },
        executor: Some("claude-code".into()),
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: None,
        scope: ScopeV2 {
            domain: Domain::Repo,
            layer: "tickets".into(),
            component: None,
            surface: vec![],
        },
        main_goal: None,
        context: vec![],
        requirement: vec![],
        current_state: vec![],
        approach: vec![],
        verify: vec![],
        acceptance: vec![],
        citations: vec![],
        shipped_at: Some("abc123def".into()),
        priority: None,
        created_at: Some("2026-08-14T10:00:00Z".into()),
        completed_at: Some("2026-08-14T11:30:00+00:00".into()),
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec!["tools_v2/xtask/src/cmds.rs".into()],
        pack_last: None,
    });
    let s = render_ticket_toml(&t).unwrap();
    let pos = |needle: &str| {
        s.find(needle)
            .unwrap_or_else(|| panic!("{needle} in:\n{s}"))
    };
    assert!(
        pos("shipped_at = ") < pos("created_at = ")
            && pos("created_at = ") < pos("completed_at = ")
            && pos("completed_at = ") < pos("owns = "),
        "canonical slot violated:\n{s}"
    );
    assert_eq!(t, parse_ticket_toml(&s).unwrap());
}

/// T-913.1: program arm carries the stamps too — and (T-917.2) the class/body keys
/// are LEGAL on programs while scope stays forbidden.
#[test]
fn program_timestamps_and_v2_fields_roundtrip() {
    let toml_in = r#"
id = "T-913"
kind = "program"
title = "metrics"
summary = "metrics"
class = "chore"
status = "queued"
order = 5920
children = ["T-913.1"]
context = ["why now"]
verify = ["cargo xtask ticket check"]
created_at = "2026-08-10T09:00:00Z"
completed_at = "2026-08-14T12:00:00Z"
"#;
    let t = parse_ticket_toml(toml_in).unwrap();
    match &t {
        Ticket::Program(p) => {
            assert_eq!(p.created_at.as_deref(), Some("2026-08-10T09:00:00Z"));
            assert_eq!(p.completed_at.as_deref(), Some("2026-08-14T12:00:00Z"));
            assert_eq!(p.class.as_deref(), Some("chore"));
            assert_eq!(p.context, vec!["why now"]);
            assert_eq!(p.verify, vec!["cargo xtask ticket check"]);
        }
        Ticket::Work(_) => panic!("T-913 must parse as Program"),
    }
    let rendered = render_ticket_toml(&t).unwrap();
    assert_eq!(t, parse_ticket_toml(&rendered).unwrap());

    let err = parse_ticket_toml(&format!(
        "{toml_in}\n[scope]\ndomain = \"repo\"\nlayer = \"docs\"\n"
    ))
    .unwrap_err();
    assert!(err.contains("program forbids [scope]"), "{err}");
}

/// T-913.1: malformed stamps are load errors that NAME the ticket — never now.
#[test]
fn malformed_timestamp_is_parse_error_naming_ticket() {
    for bad in [
        "2026-13-99T25:61:00Z",
        "2026-08-14 10:00",
        "2026-08-14T10:00:00+05:00",
    ] {
        let err = parse_ticket_toml(&format!(
            r#"
id = "T-901"
kind = "work"
title = "x"
summary = "x"
status = "queued"
order = 1
created_at = "{bad}"
owns = ["docs/x.md"]

[scope]
domain = "repo"
layer = "docs"
"#
        ))
        .unwrap_err();
        assert!(err.contains("T-901"), "must name the ticket: {err}");
        assert!(err.contains("created_at"), "must name the field: {err}");
    }
}

/// T-920.1 — the rename roundtrip: a pre-rename blob carrying `user_story`
/// parses via the serde alias into `main_goal`, and the render emits ONLY
/// `main_goal`, in the same canonical slot (after `active`-tier keys, before
/// `context`). A load + write_back of a carrier IS the migration.
#[test]
fn user_story_alias_parses_and_emits_main_goal() {
    let legacy = r#"
id = "T-919"
kind = "work"
title = "Wall triage drain"
summary = "s"
class = "chore"
status = "queued"
order = 5990
spec = "docs/spec.md"
user_story = "the goal, pre-rename spelling"
context = ["why"]
acceptance = ["gate"]

[scope]
domain = "repo"
layer = "docs"
"#;
    let t = parse_ticket_toml(legacy).expect("user_story alias parses");
    match &t {
        Ticket::Work(w) => assert_eq!(
            w.main_goal.as_deref(),
            Some("the goal, pre-rename spelling")
        ),
        Ticket::Program(_) => panic!("work"),
    }
    let rendered = render_ticket_toml(&t).expect("render");
    assert!(
        rendered.contains("main_goal = \"the goal, pre-rename spelling\""),
        "{rendered}"
    );
    for line in rendered.lines() {
        assert!(
            !line.starts_with("user_story = "),
            "render must never emit the dead spelling:\n{rendered}"
        );
    }
    // Same canonical slot: spec < main_goal < context < acceptance.
    let pos = |needle: &str| {
        rendered
            .find(needle)
            .unwrap_or_else(|| panic!("{needle} in:\n{rendered}"))
    };
    assert!(
        pos("spec = ") < pos("main_goal = ")
            && pos("main_goal = ") < pos("context = ")
            && pos("context = ") < pos("acceptance = "),
        "canonical slot violated:\n{rendered}"
    );
    assert_eq!(t, parse_ticket_toml(&rendered).unwrap());
    // Carrying BOTH spellings is a serde duplicate-field refusal, not a silent
    // pick — the alias-class discipline the 4a2f3426 pin established.
    let both = legacy.replace(
        "user_story = \"the goal, pre-rename spelling\"",
        "user_story = \"old\"\nmain_goal = \"new\"",
    );
    let err = parse_ticket_toml(&both).expect_err("both spellings must refuse");
    assert!(err.contains("duplicate"), "{err}");
}

#[test]
fn idea_rejects_order() {
    let err = parse_ticket_toml(
        r#"
id = "T-001"
kind = "work"
title = "x"
status = "idea"
order = 1

[scope]
domain = "repo"
layer = "docs"
"#,
    )
    .unwrap_err();
    assert!(err.contains("idea must not carry order"));
}
