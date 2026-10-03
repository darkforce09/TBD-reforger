//! Minting new tickets.
//!
//! **Role:** [`add`] mints a new parent work ticket and [`add_child`] a dotted child under a
//! parent, promoting a work parent to a program when asked.
//! **Position:** under the operations module; the `ticket add` and `ticket add-child` verbs call
//! it.
//! **Signals & state:** mutates only the corpus it is given.
//! **Invariants:** a minted ticket is work, status `idea`, scope `repo`/`docs`, with `created_at`
//! from the given clock and a `class` from [`ticket_model::classify_work`]; parent numbers come
//! from parent ids only, never from children.

use super::*;

/// Mint `T-{next:03}`, where next is the highest parent number plus one (children never
/// affect it, as in `derive_next_id`): kind work, status idea, `scope.repo.layers = ["docs"]`,
/// `created_at` stamped from the given clock, summary falling back to the title. Returns the
/// minted id alongside the outcome.
pub fn add(
    c: &mut Corpus,
    title: &str,
    summary: &str,
    now_utc: &str,
) -> Result<(TicketId, OpOutcome), String> {
    validate_clock(now_utc)?;
    let tid = TicketId::new(format!("T-{:03}", c.derive_next_parent_id()));
    let mut post = c.tickets.clone();
    post.insert(
        tid.clone(),
        minted_work(&tid, None, title, summary, now_utc),
    );
    let changed = BTreeSet::from([tid.clone()]);
    let outcome = commit(c, post, changed, BTreeSet::new(), BTreeSet::new())?;
    Ok((tid, outcome))
}

/// The one shape both minters (`add`, `add_child`) produce: scope `repo`/`docs` (legal in the
/// scope vocabulary and component-free, so the surface rule leaves ideas mintable), and a
/// `class` from the deterministic [`ticket_model::classify_work`] triage, so the check's
/// class-required-on-work rule holds from birth.
pub(super) fn minted_work(
    id: &TicketId,
    parent: Option<&TicketId>,
    title: &str,
    summary: &str,
    now_utc: &str,
) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.clone(),
        title: title.to_string(),
        summary: if summary.is_empty() {
            title.to_string()
        } else {
            summary.to_string()
        },
        class: Some(ticket_model::classify_work(&format!("{title} {summary}")).to_string()),
        status: Status::Idea,
        executor: None,
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: parent.map(TicketId::to_string),
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
        created_at: Some(now_utc.to_string()),
        completed_at: None,
        estimated: vec![],
        estimate_note: None,
        migration_legacy: vec![],
        owns: vec![],
        pack_last: None,
    })
}

/// Append a freshly minted child under `parent_id`. The child id is the next free dotted
/// extension of the parent id; the child inherits nothing but its `parent` field (status idea,
/// `created_at` stamped, the repo/docs scope [`add`] mints, since a work ticket must carry a
/// scope). Returns the minted child id alongside the outcome.
///
/// A `kind = "work"` parent REFUSES unless `promote` — the encoding hard-refuses
/// work-with-children and program-without-children, so a first child can only exist if
/// the parent's kind flips in the same op. Promotion preserves every field a program
/// can carry and refuses, by name, the two it cannot: `[scope]` is dropped (programs
/// forbid scope), while a `parent` field or a stray
/// `shipped_at` on a non-shipped status refuse promotion outright rather than silently
/// losing data.
pub fn add_child(
    c: &mut Corpus,
    parent_id: &TicketId,
    title: &str,
    summary: &str,
    promote: bool,
    now_utc: &str,
) -> Result<(TicketId, OpOutcome), String> {
    validate_clock(now_utc)?;
    let parent = c.tickets.get(parent_id).ok_or_else(|| unknown(parent_id))?;
    let child_id = c.next_child_id(parent_id);
    let mut post = c.tickets.clone();
    match parent {
        Ticket::Program(_) => {
            if let Some(Ticket::Program(p)) = post.get_mut(parent_id) {
                p.children.push(child_id.to_string());
            }
        }
        Ticket::Work(w) => {
            if !promote {
                return Err(format!(
                    "{parent_id} is kind work — a work ticket cannot carry children (the encoding refuses work-with-children); pass promote to atomically rewrite it work→program and add the first child (its [scope] is dropped: programs forbid scope)"
                ));
            }
            if let Some(grandparent) = &w.parent {
                return Err(format!(
                    "{parent_id}: cannot promote work→program: it has parent {grandparent} and a program cannot carry a parent field — add the child under {grandparent} instead (the flat-tree convention) or detach the parent first"
                ));
            }
            if w.shipped_at.is_some() && !matches!(w.status, Status::Shipped { .. }) {
                return Err(format!(
                    "{parent_id}: cannot promote work→program: shipped_at is set but status is {} — a program carries shipped_at only inside status shipped",
                    w.status.name().as_str()
                ));
            }
            let promoted = ProgramTicket {
                id: w.id.clone(),
                title: w.title.clone(),
                summary: w.summary.clone(),
                class: w.class.clone(),
                status: w.status.clone(),
                executor: w.executor.clone(),
                notes: w.notes.clone(),
                spec: w.spec.clone(),
                plan: w.plan.clone(),
                depends_on: w.depends_on.clone(),
                unblocks: w.unblocks.clone(),
                children: vec![child_id.to_string()],
                active: None,
                main_goal: w.main_goal.clone(),
                context: w.context.clone(),
                requirement: w.requirement.clone(),
                current_state: w.current_state.clone(),
                approach: w.approach.clone(),
                verify: w.verify.clone(),
                acceptance: w.acceptance.clone(),
                citations: w.citations.clone(),
                priority: w.priority,
                created_at: w.created_at.clone(),
                completed_at: w.completed_at.clone(),
                estimated: w.estimated.clone(),
                estimate_note: w.estimate_note.clone(),
                migration_legacy: w.migration_legacy.clone(),
                owns: w.owns.clone(),
                pack_last: w.pack_last,
            };
            post.insert(parent_id.clone(), Ticket::Program(promoted));
        }
    }
    post.insert(
        child_id.clone(),
        minted_work(&child_id, Some(parent_id), title, summary, now_utc),
    );
    let changed = BTreeSet::from([parent_id.clone(), child_id.clone()]);
    let outcome = commit(c, post, changed, BTreeSet::new(), BTreeSet::new())?;
    Ok((child_id, outcome))
}
