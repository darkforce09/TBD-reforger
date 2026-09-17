use super::*;

use std::path::PathBuf;

use crate::{Domain, ProgramTicket, ScopeV2, WorkTicket};

fn scratch_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-backfill-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch");
    fs::write(dir.join(".ai/tickets/scope-vocab.toml"), "[repo.docs]\n").expect("vocab");
    dir
}

fn shipped_work(id: &str, shipped_at: Option<&str>) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.into(),
        title: format!("{id} title"),
        summary: format!("{id} summary"),
        class: Some("chore".into()),
        status: Status::Shipped {
            shipped_at: shipped_at.map(str::to_string),
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
            domain: Domain::Repo,
            layer: "docs".into(),
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
        shipped_at: shipped_at.map(str::to_string),
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

fn with_stamps(t: Ticket, created: &str, completed: &str) -> Ticket {
    match t {
        Ticket::Work(mut w) => {
            w.created_at = Some(created.into());
            w.completed_at = Some(completed.into());
            Ticket::Work(w)
        }
        Ticket::Program(mut p) => {
            p.created_at = Some(created.into());
            p.completed_at = Some(completed.into());
            Ticket::Program(p)
        }
    }
}

fn sc(sha: &str, date: &str) -> SubjectCommit {
    SubjectCommit {
        sha: sha.into(),
        date_utc: to_utc_z(date).expect("test date"),
    }
}

mod timestamp_provenance_tests;
