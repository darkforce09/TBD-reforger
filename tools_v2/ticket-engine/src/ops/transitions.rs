//! Transitions.

use super::*;

/// Validate the candidate image, then commit it into the corpus and report the id
/// sets. A refusal leaves `c` untouched.
pub(super) fn commit(
    c: &mut Corpus,
    post: BTreeMap<String, Ticket>,
    changed: BTreeSet<String>,
    deleted: BTreeSet<String>,
    made_live: BTreeSet<String>,
) -> Result<OpOutcome, String> {
    validate_post_image(&c.tickets, &post, &changed, &made_live)?;
    c.tickets = post;
    Ok(OpOutcome {
        changed: changed.into_iter().collect(),
        deleted: deleted.into_iter().collect(),
    })
}

/// The order a ticket carries into `shipped`, `deferred` or `cancelled`: its own order when it has
/// one. An order-less parent ticket gets the [`append_order`] of `tickets`, because `ticket check`
/// requires an order on every parent whose status is not `idea`, and in these statuses the order
/// carries no dispatch meaning (the wave packer and `queue.json` read live tickets only). An
/// order-less child stays order-less: the check walks parent tickets only.
fn order_for_non_live_status(tickets: &BTreeMap<String, Ticket>, t: &Ticket) -> Option<i64> {
    t.status()
        .order()
        .or_else(|| crate::store::is_parent_id(t.id()).then(|| append_order(tickets)))
}

/// Build the typed [`Status`] a transition to `name` must carry, refusing up front and by name
/// anything the ticket lacks or forbids, so no refusal lands half-way through a save. Order
/// follows `ticket check`, which requires one on every parent ticket whose status is not `idea`:
/// - `queued`, `ready`, `running` and `review` refuse a ticket without an order: there the order
///   is dispatch priority, chosen deliberately with `ticket reorder`;
/// - `shipped`, `deferred` and `cancelled` take [`order_for_non_live_status`], which mints the
///   append order for an order-less parent;
/// - `idea` refuses a ticket that carries an order, since the idea status has none.
pub(super) fn status_for_transition(
    tickets: &BTreeMap<String, Ticket>,
    t: &Ticket,
    name: StatusName,
) -> Result<Status, String> {
    let id = t.id();
    let cur = t.status();
    match name {
        StatusName::Idea => {
            if let Some(n) = cur.order() {
                return Err(format!(
                    "refusing set-status {id}: idea must not carry order and the ticket has order {n} — a mid-save wedge is the alternative; clear the order deliberately first"
                ));
            }
            Ok(Status::Idea)
        }
        StatusName::Queued => {
            let order = cur.order().ok_or_else(|| {
                format!(
                    "refusing set-status {id}: queued requires order and the ticket has none — `ticket reorder {id} <anchor>` mints one (a mid-save wedge is the alternative)"
                )
            })?;
            Ok(Status::Queued { order })
        }
        StatusName::Ready | StatusName::Running | StatusName::Review => {
            let mut missing: Vec<&str> = Vec::new();
            if cur.order().is_none() {
                missing.push("order");
            }
            if spec_of(t).unwrap_or("").trim().is_empty() {
                missing.push("spec");
            }
            if main_goal_of(t).unwrap_or("").trim().is_empty() {
                missing.push("main_goal");
            }
            if acceptance_of(t).iter().all(|s| s.trim().is_empty()) {
                missing.push("acceptance");
            }
            if !missing.is_empty() {
                return Err(format!(
                    "refusing set-status {id}: status {} needs order/spec/main_goal/acceptance and the ticket lacks {} — a mid-save wedge is the alternative; set the fields (or use mark-ready) first",
                    name.as_str(),
                    missing.join(", ")
                ));
            }
            Status::live_ready(
                name,
                cur.order().expect("checked above"),
                spec_of(t).expect("checked above").to_string(),
                main_goal_of(t).expect("checked above").to_string(),
                acceptance_of(t).to_vec(),
            )
            .map_err(|e| format!("refusing set-status {id}: {e}"))
        }
        StatusName::Shipped => Ok(Status::Shipped {
            shipped_at: current_shipped_at(t),
            order: order_for_non_live_status(tickets, t),
        }),
        StatusName::Deferred => Ok(Status::Deferred {
            order: order_for_non_live_status(tickets, t),
        }),
        StatusName::Cancelled => Ok(Status::Cancelled {
            order: order_for_non_live_status(tickets, t),
        }),
    }
}

/// `cmd_set_status` semantics: trim, refuse empty, refuse a non-enum value, write the
/// status; `cancelled` stamps `completed_at` (the ONLY set-status target that stamps —
/// `ship`/`done` own the shipped stamp). Deliberately does
/// NOT clear `active` (that is `ship`'s job). The ticket keeps its order; the one order this
/// verb mints is the append order an order-less parent takes into `shipped`, `deferred` or
/// `cancelled` ([`status_for_transition`]), so no set-status leaves `ticket check` red for lack
/// of an order.
pub fn set_status(
    c: &mut Corpus,
    id: &str,
    status: &str,
    now_utc: &str,
) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let status = status.trim();
    if status.is_empty() {
        return Err(format!(
            "refusing set-status {id}: status must be non-empty (refusing to write \"\" over the registry)"
        ));
    }
    let Some(name) = StatusName::parse(status) else {
        return Err(format!(
            "refusing set-status {id}: invalid status `{status}` (expected one of: {})",
            VALID_STATUS_NAMES.join(", ")
        ));
    };
    let pre = c.tickets.get(id).ok_or_else(|| unknown(id))?;
    let was_live = pre.status().name().is_live();
    let new_status = status_for_transition(&c.tickets, pre, name)?;
    let mut post = c.tickets.clone();
    let t = post.get_mut(id).expect("looked up above");
    set_ticket_status(t, new_status);
    if matches!(name, StatusName::Cancelled) {
        set_completed_at(t, Some(now_utc.to_string()));
    }
    let mut made_live = BTreeSet::new();
    if !was_live && name.is_live() {
        made_live.insert(id.to_string());
    }
    let changed = BTreeSet::from([id.to_string()]);
    commit(c, post, changed, BTreeSet::new(), made_live)
}

/// `cmd_ship` semantics: status→shipped preserving the existing `shipped_at` value and
/// order (an order-less parent takes the append order through [`order_for_non_live_status`];
/// ship never invents the SHA — that stays hand-edited), stamp `completed_at`,
/// clear the ticket's own `active`. `id` may be a dotted child id as well as a parent id:
/// the lookup runs against `c.tickets`, which holds every ticket on disk. And — the
/// invariant — ship clears any program whose `active` still names the shipped ticket; that
/// program counts as changed.
///
/// **The ship-gate lifecycle** (spec §The gate, §stamp-sha closes the loop).
/// A shipped ticket must end with `created_at` + `completed_at` + a SHA-shaped
/// `shipped_at` + token accounting, but those arrive at DIFFERENT moments:
///
/// 1. `ship` stamps `completed_at` (this op) and REFUSES pre-write when `created_at`
///    is absent — that stamp can never arrive later honestly (`created_at` is minted
///    by `ticket add` at birth, so an un-stamped ticket needs a deliberate hand-stamp
///    naming a date the operator can defend);
/// 2. the operator commits — only now does the landing SHA exist;
/// 3. `ticket stamp-sha <id> <sha>` ([`stamp_sha`]) closes `shipped_at` and the token
///    estimate.
///
/// So `ship` deliberately does NOT require `shipped_at` or tokens (they cannot exist
/// yet); the `ticket check` ship gate is what holds committed trees to the full
/// contract — the working tree is transiently gate-red between steps 1 and 3 by
/// design, and step 3 closes it.
pub fn ship(c: &mut Corpus, id: &str, now_utc: &str) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let Some(t) = c.tickets.get(id) else {
        return Err(unknown(id));
    };
    if created_at_of(t).is_none() {
        return Err(format!(
            "refusing ship {id}: created_at is absent — the ship gate requires it and ship cannot \
             invent a birth date; created_at is minted by `ticket add`, so an old un-stamped \
             ticket needs a deliberate hand-stamp: its file's first-commit author date in UTC"
        ));
    }
    // (t920 spec Decisions log #2, shipped row): a FUTURE ship carries the
    // full ready-tier body — main_goal plus the six fields — refused pre-write
    // naming each empty one. Work-only (the tier table is work-shaped; a program
    // aggregates its children's bodies) and quarantine-exempt like every body-tier
    // rule. `main_goal` is checked HERE and not in `empty_ready_tier_fields` because
    // ship can jump from queued/idea, where the ready-class parse guarantee does not
    // exist yet. Shipped HISTORY stays untouched: check never reds old ships until
    // the debt drain finishes — this arm binds only the ship verb.
    if let Ticket::Work(w) = t
        && w.migration_legacy.is_empty()
    {
        let mut missing: Vec<&str> = Vec::new();
        if w.main_goal.as_deref().unwrap_or("").trim().is_empty() {
            missing.push("main_goal");
        }
        missing.extend(crate::empty_ready_tier_fields(w));
        if !missing.is_empty() {
            return Err(format!(
                "refusing ship {id}: ready-tier body fields empty: {} — a ship needs the full body (t920 spec Decisions log #2); fill them first (thin evidence yields thin honest lines, never padding)",
                missing.join(", ")
            ));
        }
    }
    let order = order_for_non_live_status(&c.tickets, t);
    let mut post = c.tickets.clone();
    let mut changed = BTreeSet::from([id.to_string()]);
    {
        let t = post.get_mut(id).expect("checked above");
        let shipped_at = current_shipped_at(t);
        set_ticket_status(t, Status::Shipped { shipped_at, order });
        set_completed_at(t, Some(now_utc.to_string()));
        if let Ticket::Program(p) = t {
            p.active = None;
        }
    }
    let mut stale_active: Vec<String> = Vec::new();
    for (pid, t) in &post {
        if pid == id {
            continue;
        }
        if let Ticket::Program(p) = t
            && p.active.as_deref() == Some(id)
        {
            stale_active.push(pid.clone());
        }
    }
    for pid in stale_active {
        if let Some(Ticket::Program(p)) = post.get_mut(&pid) {
            p.active = None;
        }
        changed.insert(pid);
    }
    commit(c, post, changed, BTreeSet::new(), BTreeSet::new())
}

/// `ticket stamp-sha` step 3 of the ship lifecycle (see [`ship`]): write the
/// landing commit SHA onto a SHIPPED ticket, canonically, through both storage arms
/// (work tickets carry the `shipped_at` field mirrored into [`Status::Shipped`];
/// programs carry it inside the status only — the `current_shipped_at` asymmetry).
///
/// Refusals, each pre-write with the corpus untouched:
/// - `sha` not 7–40 lowercase hex ([`crate::is_sha_shaped`] — empty/garbage refuses);
/// - ticket not SHIPPED (stamp-sha closes a ship, it never implies one);
/// - `shipped_at` already carries a DIFFERENT value — `shipped_at` is never
///   overwritten by any verb (the backfill's present-fields rule); if the stamp is
///   truly wrong the operator deletes the value by hand, deliberately, first.
///
/// Idempotent-ish: re-stamping the SAME sha is a no-op — `Ok` with an empty
/// `changed` set, so the caller can still (re)generate the token estimate for a
/// ticket whose stamp landed but whose accounting did not (the
/// window). A successful write also REMOVES a stale `"shipped_at"` entry from
/// `estimated[]`: the operator-supplied landing SHA is measured provenance, not an
/// estimate (the marker + gap-note state was the miner's honest absence, now closed).
pub fn stamp_sha(c: &mut Corpus, id: &str, sha: &str, now_utc: &str) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    let sha = sha.trim();
    if !crate::is_sha_shaped(sha) {
        return Err(format!(
            "refusing stamp-sha {id}: {sha:?} is not a commit SHA (7-40 lowercase hex)"
        ));
    }
    let Some(t) = c.tickets.get(id) else {
        return Err(unknown(id));
    };
    let status = t.status().name();
    if status != StatusName::Shipped {
        return Err(format!(
            "refusing stamp-sha {id}: status is {}, not shipped — stamp-sha closes a shipped \
             ticket's landing commit; `ticket ship {id}` first",
            status.as_str()
        ));
    }
    match current_shipped_at(t) {
        Some(existing) if existing == sha => Ok(OpOutcome::default()),
        Some(existing) => Err(format!(
            "refusing stamp-sha {id}: shipped_at is already {existing:?} — shipped_at is never \
             overwritten; if the recorded stamp is truly wrong, delete the value by hand first \
             and re-run"
        )),
        None => {
            let mut post = c.tickets.clone();
            let t = post.get_mut(id).expect("looked up above");
            match t {
                Ticket::Work(w) => {
                    w.shipped_at = Some(sha.to_string());
                    if let Status::Shipped { shipped_at, .. } = &mut w.status {
                        *shipped_at = Some(sha.to_string());
                    }
                    w.estimated.retain(|e| e != "shipped_at");
                }
                Ticket::Program(p) => {
                    if let Status::Shipped { shipped_at, .. } = &mut p.status {
                        *shipped_at = Some(sha.to_string());
                    }
                    p.estimated.retain(|e| e != "shipped_at");
                }
            }
            let changed = BTreeSet::from([id.to_string()]);
            commit(c, post, changed, BTreeSet::new(), BTreeSet::new())
        }
    }
}
