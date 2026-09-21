use super::*;

use crate::{Domain, ScopeV2, Status, WorkTicket};

/// Real repo root, the xtask-tests precedent: `CARGO_MANIFEST_DIR/../..`.
fn repo_root() -> PathBuf {
    crate::repository::find_repo_root().expect("repository root")
}

/// Minimal vocabulary every scratch TREE carries (T-917.2: `Corpus::load` is
/// fail-closed on the vocab file). Memory-only corpora (`Corpus::new`) never read it.
const MINI_VOCAB: &str = "[repo.docs]\n";

fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("tbd-tickets-store-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch tickets dir");
    fs::write(dir.join(crate::vocab::VOCAB_REL), MINI_VOCAB).expect("write scratch vocab");
    dir
}

fn work(id: &str, status: Status) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.into(),
        title: format!("{id} title"),
        summary: format!("{id} summary"),
        class: Some("chore".into()),
        status,
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

/// The only live-tree files whose bytes deviate from the canonical
/// `render_ticket_toml` form — both hand-edited outside any writer, both
/// VALUE-equal after re-parse (measured 2026-08-14 over 1182 files):
///
/// EMPTY since the T-916.1 land commit canonicalized the last two hand-edit
/// deviations (`T-911.1` shipped_at slot, `T-916.2` inline layers array) as
/// operator bookkeeping riding the same commit. The pin stays SELF-TIGHTENING
/// both ways: a deviation outside the list fails the test, and a listed file
/// that has become canonical ALSO fails until the entry is removed in the same
/// commit — the `frozen_unmappable_is_49` exact-accounting pattern. The list
/// may only ever shrink.
const HAND_EDITED_NOT_CANONICAL: &[&str] = &[];

/// Shrink-only ratchet: the number of tickets carrying a nonempty `migration_legacy`
/// — parked wall summaries awaiting decomposition, measured on the live tree
/// (instrument: typed corpus scan, `!migration_legacy.is_empty()`). The
/// `HAND_EDITED_NOT_CANONICAL` self-tightening pattern, red BOTH ways:
///
/// - **Growth is impossible by rule**: new tickets never park a wall — a
///   post-cutover mint is red in `ticket check`, the quarantine-mint
///   tripwire pinned by `quarantine_mint_past_cutover_is_red` — and the
///   ops post-image gate refuses new wall summaries outright, so nothing can
///   legitimately add a carrier. A count above the pin means somebody hand-minted
///   the field.
/// - **Every Program T drain batch SHRINKS this pin in the same commit** (spec
///   §Programs, Program T: decompose the wall into the typed fields, delete
///   `migration_legacy`, shrink the pin by exactly the batch size).
const MIGRATION_LEGACY_PIN: usize = 0;

mod corpus_storage_tests;
