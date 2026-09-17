use super::*;

use std::path::PathBuf;

use crate::{Domain, ScopeV2, Status, WorkTicket, render_ticket_toml};

fn scratch_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-quarantine-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch");
    fs::write(dir.join(".ai/tickets/scope-vocab.toml"), "[repo.docs]\n").expect("vocab");
    dir
}

fn work(id: &str, title: &str, summary: &str) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.into(),
        title: title.into(),
        summary: summary.into(),
        class: Some("chore".into()),
        status: Status::Idea,
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
        shipped_at: None,
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

fn words(n: usize, stem: &str) -> String {
    (1..=n)
        .map(|i| format!("{stem}{i}"))
        .collect::<Vec<_>>()
        .join(" ")
}

mod body_quarantine_tests;
