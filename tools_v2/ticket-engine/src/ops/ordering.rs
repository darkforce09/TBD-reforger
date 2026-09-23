//! Ordering.

use super::*;

/// `cmd_remove` semantics, extended to the full corpus. A work ticket: delete its
/// file; when its `parent` names a program in the corpus, scrub it from that
/// program's `children[]` — refusing when the scrub would empty the list (programs
/// require children; remove the program itself, or add another child first). A
/// program: REFUSE unless `force`, which cascade-deletes every descendant file
/// (closure over `children[]` edges AND work `parent` back-edges) — the deliberate,
/// documented divergence from the old save path, which silently cascade-deleted via
/// the `save_tree` stale-file pass (design Decisions log #3).
///
/// Any OTHER program still listing a removed id (double listings exist: two programs
/// listing one child) makes the post-image referential check refuse the whole
/// op — fail-closed, naming the listing program — rather than strand a dangling
/// `children[]` entry.
pub fn remove(c: &mut Corpus, id: &str, force: bool, now_utc: &str) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let target = c.tickets.get(id).ok_or_else(|| unknown(id))?;
    let mut post = c.tickets.clone();
    let mut changed: BTreeSet<String> = BTreeSet::new();
    let mut deleted: BTreeSet<String> = BTreeSet::new();
    match target {
        Ticket::Work(w) => {
            deleted.insert(id.to_string());
            post.remove(id);
            if let Some(pid) = w.parent.clone()
                && let Some(Ticket::Program(p)) = post.get_mut(&pid)
            {
                p.children.retain(|cid| cid != id);
                if p.children.is_empty() {
                    return Err(format!(
                        "removing {id} would leave program {pid} with no children — a program requires children; remove the program itself (force cascades) or add another child first"
                    ));
                }
                changed.insert(pid);
            }
        }
        Ticket::Program(p) => {
            if !force {
                return Err(format!(
                    "{id} is a program — removing it cascade-deletes every descendant file ({} children listed); pass force to do that deliberately",
                    p.children.len()
                ));
            }
            let mut queue = vec![id.to_string()];
            while let Some(current) = queue.pop() {
                if !deleted.insert(current.clone()) {
                    continue;
                }
                if let Some(Ticket::Program(cp)) = post.get(&current) {
                    for child in &cp.children {
                        if !deleted.contains(child) {
                            queue.push(child.clone());
                        }
                    }
                }
                for (other_id, other) in &post {
                    if deleted.contains(other_id) {
                        continue;
                    }
                    if let Ticket::Work(ow) = other
                        && ow.parent.as_deref() == Some(current.as_str())
                    {
                        queue.push(other_id.clone());
                    }
                }
            }
            for gone in &deleted {
                post.remove(gone);
            }
        }
    }
    commit(c, post, changed, deleted, BTreeSet::new())
}

/// The order [`reorder`] gives a ticket anchored after `anchor_order`: the next integer.
fn order_after(anchor_order: i64) -> i64 {
    anchor_order + 1
}

/// The order that places a ticket after every ordered ticket in `tickets`, parents and children
/// alike: the value [`reorder`] mints when anchored after the highest-ordered ticket. `ticket
/// check` reads an order of 0 as absent, so the anchor floors at 0 and a corpus without a
/// positive order yields 1.
pub(super) fn append_order(tickets: &BTreeMap<String, Ticket>) -> i64 {
    let highest = tickets
        .values()
        .filter_map(|t| t.status().order())
        .fold(0, i64::max);
    order_after(highest)
}

/// `cmd_reorder` semantics: the anchor must exist AND carry an order (both failure
/// modes print the same string), the new order is `order_after` the anchor's, and an `idea`
/// ticket flips to `queued` — every other status keeps its variant and only moves its order.
/// The one sanctioned divergence: a resulting duplicate LIVE order refuses at the
/// post-image gate instead of landing red state on disk (the wedge that motivated
/// post-image validation — `validate_registry` reds duplicate live orders and every
/// subsequent verb then refuses until a hand-edit).
pub fn reorder(c: &mut Corpus, id: &str, after: &str, now_utc: &str) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let t = c.tickets.get(id).ok_or_else(|| unknown(id))?;
    let anchor_order = c
        .tickets
        .get(after)
        .and_then(|a| a.status().order())
        .ok_or_else(|| format!("Unknown anchor ticket: {after}"))?;
    let new_order = order_after(anchor_order);
    let was_idea = matches!(t.status(), Status::Idea);
    let new_status = match t.status().clone() {
        Status::Idea => Status::Queued { order: new_order },
        Status::Queued { .. } => Status::Queued { order: new_order },
        Status::Ready {
            spec,
            main_goal,
            acceptance,
            ..
        } => Status::Ready {
            order: new_order,
            spec,
            main_goal,
            acceptance,
        },
        Status::Running {
            spec,
            main_goal,
            acceptance,
            ..
        } => Status::Running {
            order: new_order,
            spec,
            main_goal,
            acceptance,
        },
        Status::Review {
            spec,
            main_goal,
            acceptance,
            ..
        } => Status::Review {
            order: new_order,
            spec,
            main_goal,
            acceptance,
        },
        Status::Shipped { shipped_at, .. } => Status::Shipped {
            shipped_at,
            order: Some(new_order),
        },
        Status::Deferred { .. } => Status::Deferred {
            order: Some(new_order),
        },
        Status::Cancelled { .. } => Status::Cancelled {
            order: Some(new_order),
        },
    };
    let mut post = c.tickets.clone();
    set_ticket_status(post.get_mut(id).expect("looked up above"), new_status);
    let mut made_live = BTreeSet::new();
    if was_idea {
        made_live.insert(id.to_string());
    }
    let changed = BTreeSet::from([id.to_string()]);
    commit(c, post, changed, BTreeSet::new(), made_live)
}

/// `cmd_advance_slice` semantics over the typed [`ProgramTicket::children`] (the Value
/// path read the mirrored `slices` key): no active → first child; else the next child
/// after the current one; refuse past the end and refuse an active that is not in the
/// list. Refusal strings come back verbatim so the command layer can pass them
/// through.
pub fn advance_slice(c: &mut Corpus, id: &str, now_utc: &str) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let t = c.tickets.get(id).ok_or_else(|| unknown(id))?;
    let p = match t {
        Ticket::Program(p) => p,
        Ticket::Work(_) => return Err(format!("{id} has no slices[]")),
    };
    if p.children.is_empty() {
        return Err(format!("{id} has no slices[]"));
    }
    let new_active = match &p.active {
        None => p.children[0].clone(),
        Some(active) => {
            let idx = p
                .children
                .iter()
                .position(|child| child == active)
                .ok_or_else(|| format!("active_slice {active} not in slices[]"))?;
            if idx + 1 >= p.children.len() {
                return Err(format!("{id}: no slice after {active}"));
            }
            p.children[idx + 1].clone()
        }
    };
    let mut post = c.tickets.clone();
    if let Some(Ticket::Program(program)) = post.get_mut(id) {
        program.active = Some(new_active);
    }
    let changed = BTreeSet::from([id.to_string()]);
    commit(c, post, changed, BTreeSet::new(), BTreeSet::new())
}
