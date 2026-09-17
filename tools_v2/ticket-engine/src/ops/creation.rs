//! Creation.

use super::*;

/// `cmd_add` semantics: mint `T-{next:03}` where next is max PARENT numeric + 1
/// (children never affect it — `derive_next_id` preserved exactly), kind work, status
/// idea, `scope.repo.layers = ["docs"]`, `created_at` stamped from the injected clock,
/// summary falling back to the title. Returns the minted id alongside the outcome.
pub fn add(
    c: &mut Corpus,
    title: &str,
    summary: &str,
    now_utc: &str,
) -> Result<(String, OpOutcome), String> {
    validate_clock(now_utc)?;
    let tid = format!("T-{:03}", c.derive_next_parent_id());
    let mut post = c.tickets.clone();
    post.insert(
        tid.clone(),
        minted_work(&tid, None, title, summary, now_utc),
    );
    let changed = BTreeSet::from([tid.clone()]);
    let outcome = commit(c, post, changed, BTreeSet::new(), BTreeSet::new())?;
    Ok((tid, outcome))
}

/// The one shape both minters (`add`, `add_child`) produce — `cmd_add`'s row, typed.
/// T-917.2: mints v2 — flat scope `repo`/`docs` (the vocab-legal mint default,
/// component-free so the surface rule leaves ideas mintable), and a `class` from the
/// conservative-deterministic [`crate::classify_work`] triage so the check-level
/// class-required-on-work rule holds from birth (idea status is otherwise exempt from
/// nothing — every work ticket carries a class).
pub(super) fn minted_work(
    id: &str,
    parent: Option<&str>,
    title: &str,
    summary: &str,
    now_utc: &str,
) -> Ticket {
    Ticket::Work(WorkTicket {
        id: id.to_string(),
        title: title.to_string(),
        summary: if summary.is_empty() {
            title.to_string()
        } else {
            summary.to_string()
        },
        class: Some(crate::classify_work(&format!("{title} {summary}")).to_string()),
        status: Status::Idea,
        executor: None,
        notes: None,
        spec: None,
        plan: None,
        depends_on: vec![],
        unblocks: vec![],
        parent: parent.map(str::to_string),
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

/// New verb (design §Write path): append a freshly minted child under `parent_id`.
/// The child id is the next free dotted extension of the parent id; the child inherits
/// nothing but its `parent` field (status idea, `created_at` stamped, the `cmd_add`
/// repo/docs scope — a work ticket must carry SOME scope and the `add` minting default
/// is the precedent). Returns the minted child id alongside the outcome.
///
/// A `kind = "work"` parent REFUSES unless `promote` — the encoding hard-refuses
/// work-with-children and program-without-children, so a first child can only exist if
/// the parent's kind flips in the same op. Promotion preserves every field a program
/// can carry and refuses, by name, the two it cannot: `[scope]` is dropped (programs
/// forbid scope — sanctioned by the design), while a `parent` field or a stray
/// `shipped_at` on a non-shipped status refuse promotion outright rather than silently
/// losing data.
pub fn add_child(
    c: &mut Corpus,
    parent_id: &str,
    title: &str,
    summary: &str,
    promote: bool,
    now_utc: &str,
) -> Result<(String, OpOutcome), String> {
    validate_clock(now_utc)?;
    let parent = c.tickets.get(parent_id).ok_or_else(|| unknown(parent_id))?;
    let child_id = c.next_child_id(parent_id);
    let mut post = c.tickets.clone();
    match parent {
        Ticket::Program(_) => {
            if let Some(Ticket::Program(p)) = post.get_mut(parent_id) {
                p.children.push(child_id.clone());
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
                children: vec![child_id.clone()],
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
            post.insert(parent_id.to_string(), Ticket::Program(promoted));
        }
    }
    post.insert(
        child_id.clone(),
        minted_work(&child_id, Some(parent_id), title, summary, now_utc),
    );
    let changed = BTreeSet::from([parent_id.to_string(), child_id.clone()]);
    let outcome = commit(c, post, changed, BTreeSet::new(), BTreeSet::new())?;
    Ok((child_id, outcome))
}
