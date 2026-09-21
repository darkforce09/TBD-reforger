use super::*;

use std::fs;

use std::path::PathBuf;

/// The injected clock every test stamps with — determinism is the whole point.
const CLOCK: &str = "2026-08-14T12:00:00Z";

fn scratch_root(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-tickets-ops-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(crate::repository::TICKETS_DIR))
        .expect("mkdir scratch tickets dir");
    dir
}

/// Scratch work ticket. `owns` defaults NONEMPTY so status flips into the live set
/// do not trip the owns gate unless a test empties it on purpose, and `created_at`
/// defaults PRESENT so ships do not trip the birth-stamp refusal unless a
/// test removes it on purpose. The same convention extends to the body
/// tiers: `main_goal` and the six ready-tier fields default NONEMPTY so live
/// flips and ships pass the tier gates unless a test empties them on purpose.
fn work(id: &str, status: Status) -> WorkTicket {
    WorkTicket {
        id: id.into(),
        title: format!("{id} title"),
        summary: format!("{id} summary"),
        class: Some("chore".into()),
        status,
        executor: Some("claude-code".into()),
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
        main_goal: Some(format!("{id} goal")),
        context: vec![format!("{id} context")],
        requirement: vec![format!("{id} requirement")],
        current_state: vec![format!("{id} current state")],
        approach: vec![format!("{id} approach")],
        verify: vec![format!("{id} verify")],
        acceptance: vec![format!("{id} acceptance")],
        citations: vec![],
        shipped_at: None,
        priority: None,
        created_at: Some("2026-08-01T09:00:00Z".into()),
        completed_at: None,
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec![format!("{id}.surface")],
        pack_last: None,
    }
}

fn program(id: &str, status: Status, children: &[&str], active: Option<&str>) -> Ticket {
    Ticket::Program(ProgramTicket {
        id: id.into(),
        title: format!("{id} title"),
        summary: format!("{id} summary"),
        class: None,
        status,
        executor: Some("claude-code".into()),
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        children: children.iter().map(|s| (*s).to_string()).collect(),
        active: active.map(str::to_string),
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

fn corpus(tickets: Vec<Ticket>) -> Corpus {
    let mut c = Corpus::new("/nonexistent-ops-root");
    for t in tickets {
        c.tickets.insert(t.id().to_string(), t);
    }
    c
}

fn child_of(parent: &str, id: &str, status: Status) -> Ticket {
    let mut w = work(id, status);
    w.parent = Some(parent.into());
    Ticket::Work(w)
}

mod status_and_shipping_tests;

mod hierarchy_and_body_tests;
