use super::*;

use crate::validation::require_check_ok;

use crate::registry::load_registry;

use serde_json::json;

use std::path::PathBuf;

fn worktree_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// Break a required enum so schema check goes red (in-memory only).
fn red_registry(root: &Path) -> Value {
    let mut registry = load_registry(root).expect("load tip registry");
    registry
        .get_mut("tickets")
        .and_then(|t| t.as_array_mut())
        .expect("tickets")
        .first_mut()
        .expect("ticket")
        .as_object_mut()
        .expect("obj")
        .insert("status".into(), json!("not-a-real-status"));
    registry
}

/// Pinned-identity git for scratch registries — the check preflight runs `git grep`
/// (fossil guard) and `wave repack` derives its ledger base from history, so mutator
/// fixtures must be real repos (the wave_lock test pattern).
fn git_in_dir(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=t916@test",
            "-c",
            "user.name=t916",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// T-916.2 scratch registry — a real git repo carrying the REAL `.ai/tickets/schema.json`
/// plus a minimal 4-ticket tree (program T-001 with a ready active child and an idea
/// child; ready parent T-002), wave.lock freshly repacked and everything committed.
/// Mutator tests run HERE only: the live registry gets zero writes from the suite.
/// T-917.2: the tree carries the minimal scope vocabulary (Corpus::load resolves
/// legality fail-closed) and every work ticket a class (check requires it).
fn scratch_registry(tag: &str) -> PathBuf {
    use crate::{Domain, ProgramTicket, ScopeV2, Status, Ticket, WorkTicket};
    let dir = std::env::temp_dir().join(format!("t916-cmds-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai/tickets")).unwrap();
    fs::create_dir_all(dir.join("docs")).unwrap();
    fs::write(
        dir.join(".ai/tickets/ROOT"),
        "# ticket-registry root marker\n",
    )
    .unwrap();
    // The real schema: a stub would silently weaken the very preflight these tests keep
    // in front of the typed ops.
    fs::copy(
        worktree_root().join(".ai/tickets/schema.json"),
        dir.join(".ai/tickets/schema.json"),
    )
    .unwrap();
    fs::write(dir.join(".ai/tickets/scope-vocab.toml"), "[repo.docs]\n").unwrap();
    // T-917.5/.6: the estimates schema rides along so a stamp-sha-generated
    // estimate validates under the REAL contract inside the scratch too.
    fs::copy(
        worktree_root().join(crate::metrics::estimates::ESTIMATES_SCHEMA_REL),
        dir.join(crate::metrics::estimates::ESTIMATES_SCHEMA_REL),
    )
    .unwrap();
    fs::write(dir.join("docs/spec.md"), "# spec\n").unwrap();
    fs::write(dir.join("docs/child-spec.md"), "# child spec\n").unwrap();
    // T-917.6 plan ready-gate: every ready-class WORK ticket carries a plan that
    // exists on disk (the live-tree contract this fixture must now mirror).
    fs::create_dir_all(dir.join("docs/plans")).unwrap();
    for plan in ["t-001_1_plan.md", "t-002_plan.md"] {
        fs::write(
            dir.join("docs/plans").join(plan),
            "# plan\n\n## Context\n\n## Approach\n\n## Risks\n\n## Verification\n",
        )
        .unwrap();
    }
    let ready = |order: i64, spec: &str| Status::Ready {
        order,
        spec: spec.into(),
        main_goal: "story".into(),
        acceptance: vec!["gate".into()],
    };
    let work =
        |id: &str, status: Status, spec: Option<&str>, parent: Option<&str>, owns: &[&str]| {
            // Parsed work tickets carry the ready-class prose BOTH in the status and in the
            // standalone fields — mirror that or the write_back round-trip gate refuses.
            let ready_class = matches!(status, Status::Ready { .. });
            Ticket::Work(WorkTicket {
                id: id.into(),
                title: format!("{id} title"),
                summary: format!("{id} summary"),
                class: Some("chore".into()),
                status,
                executor: Some("claude-code".into()),
                notes: None,
                spec: spec.map(str::to_string),
                plan: ready_class.then(|| crate::ops::default_plan_path(id)),
                depends_on: vec![],
                unblocks: vec![],
                parent: parent.map(str::to_string),
                scope: ScopeV2 {
                    domain: Domain::Repo,
                    layer: "docs".into(),
                    component: None,
                    surface: vec![],
                },
                main_goal: ready_class.then(|| "story".to_string()),
                // T-920.1 ready-tier rule: ready-class work carries the six
                // body fields nonempty (check_ready_tier_body) — the fixture
                // mirrors the live-tree contract like it does for plans.
                context: if ready_class {
                    vec!["why".into()]
                } else {
                    vec![]
                },
                requirement: if ready_class {
                    vec!["ask".into()]
                } else {
                    vec![]
                },
                current_state: if ready_class {
                    vec!["today".into()]
                } else {
                    vec![]
                },
                approach: if ready_class {
                    vec!["steps".into()]
                } else {
                    vec![]
                },
                verify: if ready_class {
                    vec!["cargo test".into()]
                } else {
                    vec![]
                },
                acceptance: if ready_class {
                    vec!["gate".into()]
                } else {
                    vec![]
                },
                citations: vec![],
                shipped_at: None,
                priority: None,
                // T-917.6: birth stamps present, or ops::ship refuses the flip.
                created_at: Some("2026-08-01T09:00:00Z".into()),
                completed_at: None,
                estimated: vec![],
                estimate_note: None,
                migration_legacy: vec![],
                owns: owns.iter().map(|s| (*s).to_string()).collect(),
                pack_last: None,
            })
        };
    let mut corpus = Corpus::new(&dir);
    for t in [
        Ticket::Program(ProgramTicket {
            id: "T-001".into(),
            title: "T-001 title".into(),
            summary: "T-001 summary".into(),
            class: None,
            status: ready(10, "docs/spec.md"),
            executor: Some("claude-code".into()),
            notes: None,
            spec: Some("docs/spec.md".into()),
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            children: vec!["T-001.1".into(), "T-001.2".into()],
            active: Some("T-001.1".into()),
            main_goal: Some("story".into()),
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec!["gate".into()],
            citations: vec![],
            priority: None,
            created_at: None,
            completed_at: None,
            estimated: vec![],
            estimate_note: None,
            migration_legacy: vec![],
            owns: vec![],
            pack_last: None,
        }),
        work(
            "T-001.1",
            ready(20, "docs/child-spec.md"),
            Some("docs/child-spec.md"),
            Some("T-001"),
            &["a.rs"],
        ),
        work("T-001.2", Status::Idea, None, Some("T-001"), &[]),
        work(
            "T-002",
            ready(30, "docs/spec.md"),
            Some("docs/spec.md"),
            None,
            &["b.rs"],
        ),
    ] {
        corpus.tickets.insert(t.id().to_string(), t);
    }
    let all: Vec<String> = corpus.tickets.keys().cloned().collect();
    corpus.write_back(&all).expect("seed scratch tree");
    git_in_dir(&dir, &["init", "-q"]);
    git_in_dir(&dir, &["add", "-A"]);
    git_in_dir(&dir, &["commit", "-q", "-m", "seed scratch registry"]);
    crate::wave_lock::repack_quiet(&dir).expect("seed wave.lock");
    // Seed queue.json + generated docs so the fixture starts from a synced state.
    let reg = load_registry(&dir).expect("load scratch registry");
    crate::sync::cmd_sync(&dir, &reg).expect("seed sync");
    git_in_dir(&dir, &["add", "-A"]);
    git_in_dir(&dir, &["commit", "-q", "-m", "seed lock + sync outputs"]);
    dir
}

fn parse_scratch_ticket(root: &Path, id: &str) -> crate::Ticket {
    crate::parse_ticket_toml(
        &fs::read_to_string(root.join(format!(".ai/tickets/{id}.toml"))).unwrap(),
    )
    .unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn queue_rows(root: &Path) -> Vec<(String, String)> {
    let queue: Value =
        serde_json::from_str(&fs::read_to_string(root.join(".ai/tickets/queue.json")).unwrap())
            .unwrap();
    queue["tickets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["id"].as_str().unwrap_or("").to_string(),
                t["spec"].as_str().unwrap_or("").to_string(),
            )
        })
        .collect()
}

mod command_mutation_tests;

mod execution_boundaries_tests;
