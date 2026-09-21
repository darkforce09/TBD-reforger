use super::*;

use crate::cli::commit_subjects::mine_subjects;
use serde_json::json;

use crate::{Domain, ProgramTicket, ScopeV2, Status, WorkTicket};

fn repo_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// Scratch tree with the vocab the fail-closed corpus load needs plus copies of
/// BOTH committed schemas, so scratch validation is exactly the repo's.
fn scratch_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-estimates-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch");
    fs::write(
        dir.join(".ai/tickets/scope-vocab.toml"),
        "[repo.docs]\n\n[website.backend]\n\n[website.frontend]\n",
    )
    .expect("vocab");
    for rel in [ESTIMATES_SCHEMA_REL, crate::metrics::METRICS_SCHEMA_REL] {
        fs::copy(repo_root().join(rel), dir.join(rel)).expect("copy schema");
    }
    dir
}

fn shipped_work(id: &str, class: &str, domain: Domain, layer: &str) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.into(),
        title: format!("{id} title"),
        summary: format!("{id} summary"),
        class: Some(class.into()),
        status: Status::Shipped {
            shipped_at: Some("abcdef12".into()),
            order: Some(10),
        },
        executor: None,
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: None,
        scope: ScopeV2 {
            domain,
            layer: layer.into(),
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
        shipped_at: Some("abcdef12".into()),
        priority: None,
        created_at: None,
        completed_at: None,
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec![],
        pack_last: None,
    })
}

fn shipped_program(id: &str) -> Ticket {
    Ticket::Program(ProgramTicket {
        id: id.into(),
        title: format!("{id} prog"),
        summary: format!("{id} prog"),
        class: None,
        status: Status::Shipped {
            shipped_at: Some("beadfeed".into()),
            order: Some(40),
        },
        executor: None,
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        children: vec![],
        active: None,
        main_goal: None,
        context: vec![],
        requirement: vec![],
        current_state: vec![],
        approach: vec![],
        verify: vec![],
        acceptance: vec![],
        citations: vec![],
        priority: None,
        created_at: None,
        completed_at: None,
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec![],
        pack_last: None,
    })
}

fn sc(sha: &str) -> SubjectCommit {
    SubjectCommit {
        sha: sha.into(),
        date_utc: "2026-08-01T10:00:00Z".into(),
    }
}

const NOW: &str = "2026-08-15T00:00:00Z";

mod estimate_provenance_tests;
