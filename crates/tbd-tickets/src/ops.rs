//! T-916.1 — pure registry mutation ops over the typed [`Corpus`].
//!
//! Design authority: `docs/platform/t915_ticketboard_design.md` §Write path. Semantics
//! are replicated from `xtask/src/cmds.rs` (`cmd_set_status`, `cmd_ship`,
//! `cmd_mark_ready`, `cmd_add`, `cmd_remove`, `cmd_reorder`, `cmd_advance_slice`) with
//! exactly the divergences that design sanctions:
//!
//! - **Refuse up-front instead of wedging mid-save.** The Value path pokes a status
//!   onto the ticket and only discovers at `save_tree` time that the image is
//!   unparseable ("idea must not carry order", "ready-class requires order", empty
//!   `user_story`, …) — after earlier files in the iteration were already rewritten.
//!   Here every transition that would need data the ticket lacks refuses BEFORE any
//!   mutation, naming what is missing.
//! - **Reorder collisions refuse instead of writing red state.** `cmd_reorder` happily
//!   writes a duplicate live order; `validate_registry` then reds the tree and every
//!   subsequent verb refuses until the operator hand-edits. Post-image validation kills
//!   the class: the colliding write never lands.
//! - **New invariants** (all named in the design): `ship` resolves child ids and clears
//!   a program whose `active` names the shipped ticket; `add_child` refuses a work
//!   parent unless `promote` performs the atomic work→program rewrite; `remove` of a
//!   program refuses unless `force` cascades deliberately; duplicate `children[]`
//!   entries, non-dotted child ids and dangling `children[]` references refuse at the
//!   post-image gate.
//!
//! **Injected clock.** Every op takes `now_utc: &str` (validated RFC 3339 UTC,
//! precedent `metrics::stamp_land_at`) and NEVER reads the wall clock — two runs with
//! the same stamp are byte-identical, so tests never race time. The parameter is
//! uniform across ops on purpose: ops that do not stamp still validate it, so a caller
//! wiring bug (feeding a garbage clock) surfaces on the first op, not on the first
//! cancel.
//!
//! **The general invariant: no op may write a corpus its own preflight would refuse.**
//! Ops mutate a clone of the map, run [`validate_post_image`] on the result, and only
//! then swap it into the corpus and report the changed/deleted id sets — the caller
//! feeds those to [`Corpus::write_back`] / [`Corpus::delete_files`]. A refused op
//! leaves the in-memory corpus untouched.

use crate::store::Corpus;
use crate::{
    Domain, ProgramTicket, ScopeV2, Status, StatusName, Ticket, WorkTicket, parse_ticket_toml,
    render_ticket_toml,
};
use std::collections::{BTreeMap, BTreeSet};

/// The 8-value status enum in registry spelling — mirrors `VALID_TICKET_STATUSES` in
/// `xtask/src/cmds.rs` (itself a mirror of `.ai/tickets/schema.json` `$defs.status`).
pub const VALID_STATUS_NAMES: &[&str] = &[
    "idea",
    "queued",
    "ready",
    "running",
    "review",
    "shipped",
    "deferred",
    "cancelled",
];

/// What an op did: `changed` is the exact [`Corpus::write_back`] argument, `deleted`
/// the exact [`Corpus::delete_files`] argument (nonempty only for [`remove`]). Both
/// sorted and deduplicated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OpOutcome {
    pub changed: Vec<String>,
    pub deleted: Vec<String>,
}

fn validate_clock(now_utc: &str) -> Result<(), String> {
    crate::validate_rfc3339_utc("now_utc", now_utc)
}

/// Exact legacy refusal string (`cmds.rs::unknown_ticket`) so T-916.2 can pass op
/// errors through verbatim.
fn unknown(id: &str) -> String {
    format!("Unknown ticket: {id}")
}

fn set_ticket_status(t: &mut Ticket, s: Status) {
    match t {
        Ticket::Program(p) => p.status = s,
        Ticket::Work(w) => w.status = s,
    }
}

fn set_completed_at(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.completed_at = v,
        Ticket::Work(w) => w.completed_at = v,
    }
}

fn created_at_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.created_at.as_deref(),
        Ticket::Work(w) => w.created_at.as_deref(),
    }
}

fn plan_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.plan.as_deref(),
        Ticket::Work(w) => w.plan.as_deref(),
    }
}

fn set_plan(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.plan = v,
        Ticket::Work(w) => w.plan = v,
    }
}

fn spec_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.spec.as_deref(),
        Ticket::Work(w) => w.spec.as_deref(),
    }
}

fn set_spec(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.spec = v,
        Ticket::Work(w) => w.spec = v,
    }
}

fn user_story_of(t: &Ticket) -> Option<&str> {
    match t {
        Ticket::Program(p) => p.user_story.as_deref(),
        Ticket::Work(w) => w.user_story.as_deref(),
    }
}

fn set_user_story(t: &mut Ticket, v: Option<String>) {
    match t {
        Ticket::Program(p) => p.user_story = v,
        Ticket::Work(w) => w.user_story = v,
    }
}

fn acceptance_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Program(p) => &p.acceptance,
        Ticket::Work(w) => &w.acceptance,
    }
}

fn set_acceptance(t: &mut Ticket, v: Vec<String>) {
    match t {
        Ticket::Program(p) => p.acceptance = v,
        Ticket::Work(w) => w.acceptance = v,
    }
}

fn summary_of(t: &Ticket) -> &str {
    match t {
        Ticket::Program(p) => &p.summary,
        Ticket::Work(w) => &w.summary,
    }
}

fn title_of(t: &Ticket) -> &str {
    match t {
        Ticket::Program(p) => &p.title,
        Ticket::Work(w) => &w.title,
    }
}

fn depends_on_of(t: &Ticket) -> &[String] {
    match t {
        Ticket::Program(p) => &p.depends_on,
        Ticket::Work(w) => &w.depends_on,
    }
}

/// The `shipped_at` value a `status = "shipped"` image must carry to preserve bytes:
/// work tickets keep their standalone field (a parsed work ticket always has field ==
/// status copy), programs only ever carry it inside [`Status::Shipped`].
fn current_shipped_at(t: &Ticket) -> Option<String> {
    match t {
        Ticket::Work(w) => w.shipped_at.clone(),
        Ticket::Program(p) => match &p.status {
            Status::Shipped { shipped_at, .. } => shipped_at.clone(),
            Status::Idea
            | Status::Queued { .. }
            | Status::Ready { .. }
            | Status::Running { .. }
            | Status::Review { .. }
            | Status::Deferred { .. }
            | Status::Cancelled { .. } => None,
        },
    }
}

/// Live orders in a corpus image: `order → ids` over queued/ready/running/review (the
/// same live set `validate_registry` and `wave repack` use).
fn live_order_sets(map: &BTreeMap<String, Ticket>) -> BTreeMap<i64, BTreeSet<String>> {
    let mut out: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    for (id, t) in map {
        let status = t.status();
        if status.name().is_live()
            && let Some(order) = status.order()
        {
            out.entry(order).or_default().insert(id.clone());
        }
    }
    out
}

/// Post-image validation — the private gate every op runs on its candidate image
/// before any byte can land. THE invariant: no op may write a corpus its own preflight
/// would refuse.
///
/// Scoping decisions, each measured against the live tree on 2026-08-14:
///
/// - **Live-order collisions: refuse only NEW colliding pairs.** The live tree already
///   carries parent↔child live-order collisions (order 900 across the T-090 family,
///   4310 on T-674.1/.2, 4320 on T-675.1/.2) that `validate_registry` never reds —
///   its walk is parents-only — so a literal corpus-wide refusal would wedge every op
///   on a tree the check calls green. Refusing collisions the op *introduces* kills
///   exactly the `cmd_reorder` red-write wedge the design names, and never
///   retro-polices preexisting state.
/// - **Empty `owns` on live work: only ids this op made live.** Same
///   don't-retro-police carve-out, stated verbatim in the design.
/// - **Child-id shape (`{parent}.{suffix}`): changed programs only.** Measured
///   preexisting violation: T-111 (frozen-unmappable parking) lists T-067.1. A
///   corpus-wide rule would refuse every op on the live tree; scoping to programs the
///   op touched still guarantees ops never *produce* a non-dotted child.
/// - **Duplicate `children[]` entries and dangling `children[]` references:
///   corpus-wide.** The live tree is clean on both (measured), so these cannot wedge —
///   and dangling-reference checking must be corpus-wide anyway, or a `remove` could
///   strand a listing in an untouched program.
fn validate_post_image(
    pre: &BTreeMap<String, Ticket>,
    post: &BTreeMap<String, Ticket>,
    changed: &BTreeSet<String>,
    made_live: &BTreeSet<String>,
) -> Result<(), String> {
    // Render + re-parse + round-trip equality for every ticket this op rewrote.
    for id in changed {
        let t = post
            .get(id)
            .ok_or_else(|| format!("post-image: changed id {id} has no corpus entry"))?;
        let text = render_ticket_toml(t).map_err(|e| format!("post-image {id}: {e}"))?;
        let back = parse_ticket_toml(&text)
            .map_err(|e| format!("post-image {id}: rendered TOML does not re-parse: {e}"))?;
        if back != *t {
            return Err(format!(
                "post-image {id}: render → re-parse does not round-trip to the same ticket"
            ));
        }
    }
    // T-917.3: no op may write a NEW summary wall. Scoped to `changed` — the same
    // don't-retro-police carve-out the fn header documents: after the one-shot
    // `ticket quarantine-walls` pass the live tree carries no unquarantined wall, so
    // this binds exactly on prose an op introduces (`add`/`add_child` summaries, or a
    // future summary-editing verb). Nonempty `migration_legacy` exempts exactly the
    // summary cap (quarantined tickets carry `summary := title`, which may itself
    // exceed the cap); the field is minted only by the quarantine pass — a
    // post-cutover mint is red in `ticket check`, not here (ops never set it).
    // Work-only: program summaries are uncapped this pass (spec §Wall quarantine).
    for id in changed {
        if let Some(Ticket::Work(w)) = post.get(id)
            && w.migration_legacy.is_empty()
        {
            let words = w.summary.split_whitespace().count();
            if words > crate::SUMMARY_WORD_CAP {
                return Err(format!(
                    "post-image {id}: summary is {words} words (cap {}) — write the ten typed body fields instead of a wall (caps: T-917 spec §Body)",
                    crate::SUMMARY_WORD_CAP
                ));
            }
        }
    }
    // Structural children rules.
    for (pid, t) in post {
        if let Ticket::Program(p) = t {
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for c in &p.children {
                if !seen.insert(c.as_str()) {
                    return Err(format!(
                        "program {pid} lists duplicate child {c} — the 4a2f3426 duplicate-children class; fix children[]"
                    ));
                }
                if !post.contains_key(c) {
                    return Err(format!(
                        "program {pid} children[] names {c}, which has no corpus entry after this op — fix {pid} first"
                    ));
                }
            }
            if changed.contains(pid) {
                for c in &p.children {
                    let dotted = c
                        .strip_prefix(&format!("{pid}."))
                        .is_some_and(|s| !s.is_empty());
                    if !dotted {
                        return Err(format!(
                            "program {pid} lists child {c}, which is not {pid}.<suffix> — child ids must be dotted extensions of their parent"
                        ));
                    }
                }
            }
        }
    }
    // Live-order collisions the op would introduce.
    let pre_live = live_order_sets(pre);
    let post_live = live_order_sets(post);
    for (order, ids) in &post_live {
        if ids.len() < 2 {
            continue;
        }
        let preexisting = pre_live.get(order);
        let all_preexisting = ids
            .iter()
            .all(|id| preexisting.is_some_and(|s| s.contains(id)));
        if !all_preexisting {
            let list: Vec<&str> = ids.iter().map(String::as_str).collect();
            return Err(format!(
                "duplicate live order {order} on {} — refusing to write a red corpus (the legacy cmd_reorder wedge class); pick a different anchor",
                list.join(" and ")
            ));
        }
    }
    // Work made live by THIS op must own a collision surface.
    for id in made_live {
        if let Some(Ticket::Work(w)) = post.get(id) {
            if w.owns.is_empty() {
                return Err(format!(
                    "{id}: owns required for {} work ticket — this op would make it live with empty owns[] (the wave packer cannot see an owns-empty ticket)",
                    w.status.name().as_str()
                ));
            }
            // T-917.2 surface rule (spec Decisions log #3: surface REQUIRED on
            // live/new work), same made-live-only scoping as owns. Binds only when
            // the scope names a component: component-free vocabulary positions
            // (repo/docs, engine layers, …) carry no surfaces to require, and
            // `"scope" ∈ estimated[]` is the migrator's honest escape for
            // owns-uninferable history. Deliberately STRICTER than the check-level
            // rule (which also exempts components whose vocabulary surface list is
            // empty — ops cannot read the vocab from a memory-only corpus): ops
            // being stricter than check is the safe direction of the "no op may
            // write a corpus its own preflight would refuse" invariant, and making
            // a component-bearing ticket live without naming a surface is exactly
            // the decision point where the operator should widen the vocabulary or
            // record the marker deliberately.
            if let Some(component) = &w.scope.component
                && w.scope.surface.is_empty()
                && !w.estimated.iter().any(|e| e == "scope")
            {
                return Err(format!(
                    "{id}: surface required for {} work ticket — scope names component {component} but surface is empty; set [scope] surface (vocabulary: .ai/tickets/scope-vocab.toml) or record \"scope\" in estimated[]",
                    w.status.name().as_str()
                ));
            }
        }
    }
    Ok(())
}

/// Validate the candidate image, then commit it into the corpus and report the id
/// sets. A refusal leaves `c` untouched.
fn commit(
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

/// Build the typed [`Status`] a transition to `name` must carry, sourcing every datum
/// from the ticket itself — and refusing, up front and by name, anything the ticket
/// lacks (or forbids). This is where the mid-save wedge class of the Value path dies.
fn status_for_transition(t: &Ticket, name: StatusName) -> Result<Status, String> {
    let id = t.id();
    let cur = t.status();
    match name {
        StatusName::Idea => {
            if let Some(n) = cur.order() {
                return Err(format!(
                    "refusing set-status {id}: idea must not carry order and the ticket has order {n} — the legacy CLI wedges mid-save here; clear the order deliberately first"
                ));
            }
            Ok(Status::Idea)
        }
        StatusName::Queued => {
            let order = cur.order().ok_or_else(|| {
                format!(
                    "refusing set-status {id}: queued requires order and the ticket has none — `ticket reorder {id} <anchor>` mints one (the legacy CLI wedges mid-save here)"
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
            if user_story_of(t).unwrap_or("").trim().is_empty() {
                missing.push("user_story");
            }
            if acceptance_of(t).iter().all(|s| s.trim().is_empty()) {
                missing.push("acceptance");
            }
            if !missing.is_empty() {
                return Err(format!(
                    "refusing set-status {id}: status {} needs order/spec/user_story/acceptance and the ticket lacks {} — the legacy CLI wedges mid-save on this; set the fields (or use mark-ready) first",
                    name.as_str(),
                    missing.join(", ")
                ));
            }
            Status::live_ready(
                name,
                cur.order().expect("checked above"),
                spec_of(t).expect("checked above").to_string(),
                user_story_of(t).expect("checked above").to_string(),
                acceptance_of(t).to_vec(),
            )
            .map_err(|e| format!("refusing set-status {id}: {e}"))
        }
        StatusName::Shipped => Ok(Status::Shipped {
            shipped_at: current_shipped_at(t),
            order: cur.order(),
        }),
        StatusName::Deferred => Ok(Status::Deferred { order: cur.order() }),
        StatusName::Cancelled => Ok(Status::Cancelled { order: cur.order() }),
    }
}

/// `cmd_set_status` semantics: trim, refuse empty, refuse a non-enum value, write the
/// status; `cancelled` stamps `completed_at` (the ONLY set-status target that stamps —
/// `ship`/`done` own the shipped stamp, `cmds.rs` comment T-913.1). Deliberately does
/// NOT clear `active` (that is `ship`'s job) and does NOT touch order fields beyond
/// what the target status can carry.
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
    let new_status = status_for_transition(pre, name)?;
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
/// order (ship never invents the SHA — that stays hand-edited), stamp `completed_at`,
/// clear the ticket's own `active`. NOW resolves child ids (the full-corpus map is the
/// fix for the "`ticket ship T-912.2` → Unknown ticket" hole), and — new invariant —
/// clears any program whose `active` still names the shipped ticket; that parent
/// counts as changed.
///
/// **T-917.6 — the ship-gate lifecycle** (spec §The gate, §stamp-sha closes the loop).
/// A shipped ticket must end with `created_at` + `completed_at` + a SHA-shaped
/// `shipped_at` + token accounting, but those arrive at DIFFERENT moments:
///
/// 1. `ship` stamps `completed_at` (this op) and REFUSES pre-write when `created_at`
///    is absent — that stamp can never arrive later honestly (`created_at` is minted
///    by `ticket add` at birth; an old un-stamped ticket needs a backfill first —
///    `ticket backfill-stamps` for shipped history, a deliberate hand-stamp for a
///    pre-T-913 ticket being shipped today);
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
             ticket needs a backfill first (`ticket backfill-stamps` mines shipped history; a \
             live pre-stamp ticket gets a deliberate hand-stamp)"
        ));
    }
    let mut post = c.tickets.clone();
    let mut changed = BTreeSet::from([id.to_string()]);
    {
        let t = post.get_mut(id).expect("checked above");
        let shipped_at = current_shipped_at(t);
        let order = t.status().order();
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

/// T-917.6 — `ticket stamp-sha` step 3 of the ship lifecycle (see [`ship`]): write the
/// landing commit SHA onto a SHIPPED ticket, canonically, through both storage arms
/// (work tickets carry the `shipped_at` field mirrored into [`Status::Shipped`];
/// programs carry it inside the status only — the [`current_shipped_at`] asymmetry).
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
/// ticket whose stamp landed but whose accounting did not (the T-917.5→T-917.6
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

/// The T-917.6 per-ticket plan-path convention (spec §Plan documents): lowercase id,
/// dots to underscores — `T-917.6` → `docs/plans/t-917_6_plan.md`. [`mark_ready`]
/// defaults an unset `plan` to this path; the S.6 plan docs land at exactly these
/// paths so the default resolves.
pub fn default_plan_path(id: &str) -> String {
    format!("docs/plans/{}_plan.md", id.to_lowercase().replace('.', "_"))
}

/// `cmd_mark_ready` semantics: set `spec` when the argument is nonempty; refuse when
/// the resulting spec is empty ("Ticket {id} needs a spec path") or missing on disk
/// under the corpus root ("Spec file not found: …"); refuse while any `depends_on`
/// target present in the corpus is neither shipped nor cancelled ("Blocked by …");
/// then status→ready with the exact backfills: empty `user_story` becomes
/// summary→title→id, all-empty `acceptance` becomes `["See spec."]`.
///
/// **T-917.6 plan ready-gate** (spec §Plan documents, Decisions log #9): nothing goes
/// ready without its own plan document. `plan_arg` (nonempty) sets the `plan` field;
/// otherwise an already-set `plan` stands; otherwise the field defaults to
/// [`default_plan_path`]. Whatever path results must EXIST on disk under the corpus
/// root or the op refuses naming it ("Plan file not found: …") — the spec-on-disk
/// gate pattern, extended. The resolved path is WRITTEN to the ticket so the
/// check-level plan rule can see it. `plan` ≠ `spec`: spec stays the shared program
/// authority; plan is this ticket's own four-section document
/// (`docs/plans/TEMPLATE.md`).
///
/// One divergence inside the backfill, sanctioned by the refuse-up-front rule: the
/// Value path takes `summary` even when it is the empty string (the key exists), which
/// then wedges mid-save on the empty `user_story`; the typed backfill takes the first
/// NONEMPTY of summary→title, else the id. And ready requires an order the ticket must
/// already carry — an order-less ticket refuses up front where the CLI wedged.
pub fn mark_ready(
    c: &mut Corpus,
    id: &str,
    spec_arg: Option<&str>,
    plan_arg: Option<&str>,
    now_utc: &str,
) -> Result<OpOutcome, String> {
    validate_clock(now_utc)?;
    if !c.tickets.contains_key(id) {
        return Err(unknown(id));
    }
    let mut post = c.tickets.clone();
    if let Some(s) = spec_arg
        && !s.is_empty()
    {
        set_spec(
            post.get_mut(id).expect("checked above"),
            Some(s.to_string()),
        );
    }
    // Plan resolution: explicit arg > existing field > the id-derived default. The
    // resolved value lands on the ticket either way.
    let resolved_plan = match plan_arg {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => plan_of(post.get(id).expect("checked above"))
            .map(str::to_string)
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| default_plan_path(id)),
    };
    set_plan(
        post.get_mut(id).expect("checked above"),
        Some(resolved_plan.clone()),
    );
    let snapshot = post.get(id).expect("checked above").clone();
    let spec_trimmed = spec_of(&snapshot).unwrap_or("").trim().to_string();
    if spec_trimmed.is_empty() {
        return Err(format!("Ticket {id} needs a spec path"));
    }
    let spec_path = c.root().join(&spec_trimmed);
    if !spec_path.is_file() {
        return Err(format!("Spec file not found: {}", spec_path.display()));
    }
    let plan_path = c.root().join(&resolved_plan);
    if !plan_path.is_file() {
        return Err(format!(
            "Plan file not found: {} — nothing goes ready without its own plan document \
             (T-917.6 ready-gate); copy docs/plans/TEMPLATE.md to {resolved_plan} and fill \
             the four sections",
            plan_path.display()
        ));
    }
    for dep in depends_on_of(&snapshot) {
        if let Some(dep_ticket) = post.get(dep) {
            let dep_status = dep_ticket.status().name();
            if !matches!(dep_status, StatusName::Shipped | StatusName::Cancelled) {
                return Err(format!("Blocked by {dep} (status={})", dep_status.as_str()));
            }
        }
    }
    let was_live = snapshot.status().name().is_live();
    let order = snapshot.status().order().ok_or_else(|| {
        format!(
            "refusing mark-ready {id}: ready requires order and the ticket has none — the legacy CLI wedges mid-save here; reorder it into the queue first"
        )
    })?;
    let story = {
        let existing = user_story_of(&snapshot).unwrap_or("");
        if !existing.trim().is_empty() {
            existing.to_string()
        } else if !summary_of(&snapshot).trim().is_empty() {
            summary_of(&snapshot).to_string()
        } else if !title_of(&snapshot).trim().is_empty() {
            title_of(&snapshot).to_string()
        } else {
            id.to_string()
        }
    };
    let acceptance = if acceptance_of(&snapshot)
        .iter()
        .any(|s| !s.trim().is_empty())
    {
        acceptance_of(&snapshot).to_vec()
    } else {
        vec!["See spec.".to_string()]
    };
    let stored_spec = spec_of(&snapshot).expect("nonempty above").to_string();
    let new_status = Status::live_ready(
        StatusName::Ready,
        order,
        stored_spec,
        story.clone(),
        acceptance.clone(),
    )
    .map_err(|e| format!("refusing mark-ready {id}: {e}"))?;
    let t = post.get_mut(id).expect("checked above");
    set_user_story(t, Some(story));
    set_acceptance(t, acceptance);
    set_ticket_status(t, new_status);
    let mut made_live = BTreeSet::new();
    if !was_live {
        made_live.insert(id.to_string());
    }
    let changed = BTreeSet::from([id.to_string()]);
    commit(c, post, changed, BTreeSet::new(), made_live)
}

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
fn minted_work(
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
        user_story: None,
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
                user_story: w.user_story.clone(),
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

/// `cmd_remove` semantics, extended to the full corpus. A work ticket: delete its
/// file; when its `parent` names a program in the corpus, scrub it from that
/// program's `children[]` — refusing when the scrub would empty the list (programs
/// require children; remove the program itself, or add another child first). A
/// program: REFUSE unless `force`, which cascade-deletes every descendant file
/// (closure over `children[]` edges AND work `parent` back-edges) — the deliberate,
/// documented divergence from the old save path, which silently cascade-deleted via
/// the `save_tree` stale-file pass (design Decisions log #3).
///
/// Any OTHER program still listing a removed id (double listings exist: T-067 and
/// T-111 both list T-067.1) makes the post-image referential check refuse the whole
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

/// `cmd_reorder` semantics: the anchor must exist AND carry an order (both failure
/// modes print the same legacy string), new order = anchor + 1, and an `idea` ticket
/// flips to `queued` — every other status keeps its variant and only moves its order.
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
    let new_order = anchor_order + 1;
    let was_idea = matches!(t.status(), Status::Idea);
    let new_status = match t.status().clone() {
        Status::Idea => Status::Queued { order: new_order },
        Status::Queued { .. } => Status::Queued { order: new_order },
        Status::Ready {
            spec,
            user_story,
            acceptance,
            ..
        } => Status::Ready {
            order: new_order,
            spec,
            user_story,
            acceptance,
        },
        Status::Running {
            spec,
            user_story,
            acceptance,
            ..
        } => Status::Running {
            order: new_order,
            spec,
            user_story,
            acceptance,
        },
        Status::Review {
            spec,
            user_story,
            acceptance,
            ..
        } => Status::Review {
            order: new_order,
            spec,
            user_story,
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
/// list. Refusal strings are the legacy ones verbatim so T-916.2 can pass them
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// The injected clock every test stamps with — determinism is the whole point.
    const CLOCK: &str = "2026-08-14T12:00:00Z";

    fn scratch_root(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("tbd-tickets-ops-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".ai/tickets")).expect("mkdir scratch tickets dir");
        dir
    }

    /// Scratch work ticket. `owns` defaults NONEMPTY so status flips into the live set
    /// do not trip the owns gate unless a test empties it on purpose, and `created_at`
    /// defaults PRESENT so ships do not trip the T-917.6 birth-stamp refusal unless a
    /// test removes it on purpose.
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
            user_story: None,
            context: vec![],
            requirement: vec![],
            current_state: vec![],
            approach: vec![],
            verify: vec![],
            acceptance: vec![],
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
            user_story: None,
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

    /// A malformed injected clock refuses on entry — including in ops that never stamp
    /// (uniform contract: caller wiring bugs surface on the first op).
    #[test]
    fn ops_refuse_malformed_clock() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
        for bad in ["2026-08-14 12:00", "2026-08-14T12:00:00+05:00", ""] {
            let err = ship(&mut c, "T-1", bad).expect_err("bad clock must refuse");
            assert!(err.contains("now_utc"), "{err}");
            let err = advance_slice(&mut c, "T-1", bad).expect_err("bad clock must refuse");
            assert!(err.contains("now_utc"), "{err}");
        }
    }

    #[test]
    fn set_status_refuses_empty_and_invalid() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
        let before = c.clone();
        let err = set_status(&mut c, "T-1", "  ", CLOCK).expect_err("empty refuses");
        assert!(err.contains("non-empty"), "{err}");
        let err = set_status(&mut c, "T-1", "not-a-real-status", CLOCK).expect_err("enum gate");
        assert!(
            err.contains("invalid status `not-a-real-status`") && err.contains("cancelled"),
            "{err}"
        );
        assert_eq!(before, c, "refused ops must leave the corpus untouched");
    }

    /// T-916.1 acceptance 2 — →ready on a ticket without order (an idea) refuses up
    /// front, naming the missing data, instead of the legacy mid-save wedge.
    #[test]
    fn set_status_ready_without_order_refuses() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Idea))]);
        let before = c.clone();
        let err = set_status(&mut c, "T-1", "ready", CLOCK).expect_err("must refuse");
        assert!(err.contains("order"), "must name the missing order: {err}");
        assert!(err.contains("wedges mid-save"), "{err}");
        assert_eq!(before, c);
    }

    #[test]
    fn set_status_cancelled_stamps_completed_at() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
        let out = set_status(&mut c, "T-1", "cancelled", CLOCK).expect("cancel");
        assert_eq!(out.changed, vec!["T-1".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(w.status, Status::Cancelled { order: Some(5) });
                assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
            }
            Ticket::Program(_) => panic!("T-1 must stay work"),
        }
    }

    /// Preserved asymmetry: `set-status shipped` neither stamps `completed_at` nor
    /// clears `active` — `ship` owns both (cmds.rs T-913.1 comment).
    #[test]
    fn set_status_shipped_keeps_active_and_does_not_stamp() {
        let mut c = corpus(vec![
            program(
                "T-1",
                Status::Queued { order: 5 },
                &["T-1.1"],
                Some("T-1.1"),
            ),
            child_of("T-1", "T-1.1", Status::Idea),
        ]);
        set_status(&mut c, "T-1", "shipped", CLOCK).expect("shipped");
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => {
                assert_eq!(
                    p.status,
                    Status::Shipped {
                        shipped_at: None,
                        order: Some(5)
                    }
                );
                assert_eq!(
                    p.active.as_deref(),
                    Some("T-1.1"),
                    "set-status must not clear active"
                );
                assert_eq!(p.completed_at, None, "only ship/cancel stamp completed_at");
            }
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
    }

    #[test]
    fn set_status_idea_with_order_refuses() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Queued { order: 5 }))]);
        let err = set_status(&mut c, "T-1", "idea", CLOCK).expect_err("must refuse");
        assert!(err.contains("idea must not carry order"), "{err}");
    }

    /// T-916.1 acceptance 4 — ship of a dotted child id succeeds at the op layer (the
    /// legacy "Unknown ticket" hole), and the new invariant: a parent whose `active`
    /// names the shipped child is cleared and counted as changed.
    #[test]
    fn ship_dotted_child_clears_matching_parent_active() {
        let mut c = corpus(vec![
            program(
                "T-1",
                Status::Queued { order: 5 },
                &["T-1.1", "T-1.2"],
                Some("T-1.1"),
            ),
            child_of("T-1", "T-1.1", Status::Queued { order: 7 }),
            child_of("T-1", "T-1.2", Status::Idea),
        ]);
        let out = ship(&mut c, "T-1.1", CLOCK).expect("ship child");
        assert_eq!(
            out.changed,
            vec!["T-1".to_string(), "T-1.1".to_string()],
            "parent counts as changed"
        );
        assert!(out.deleted.is_empty());
        match c.get("T-1.1").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(
                    w.status,
                    Status::Shipped {
                        shipped_at: None,
                        order: Some(7)
                    },
                    "order preserved; no SHA invented"
                );
                assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
            }
            Ticket::Program(_) => panic!("T-1.1 must stay work"),
        }
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => assert_eq!(p.active, None, "stale active cleared"),
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
    }

    /// Ship preserves an existing `shipped_at` value and the order — it never invents
    /// the SHA (that stays hand-edited; design §Explicit leftovers).
    #[test]
    fn ship_preserves_shipped_at_and_order() {
        let mut w = work("T-2", Status::Queued { order: 7 });
        w.shipped_at = Some("beefcafe".into());
        let mut c = corpus(vec![Ticket::Work(w)]);
        ship(&mut c, "T-2", CLOCK).expect("ship");
        match c.get("T-2").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(
                    w.status,
                    Status::Shipped {
                        shipped_at: Some("beefcafe".into()),
                        order: Some(7)
                    }
                );
                assert_eq!(w.shipped_at.as_deref(), Some("beefcafe"));
                assert_eq!(w.completed_at.as_deref(), Some(CLOCK));
            }
            Ticket::Program(_) => panic!("T-2 must stay work"),
        }
    }

    /// T-917.6 — ship REFUSES a created_at-less ticket pre-write (the birth stamp can
    /// never arrive later honestly), naming the field and the fix; the corpus is
    /// byte-untouched. Both kinds refuse.
    #[test]
    fn ship_refuses_created_at_less_pre_write() {
        let mut unstamped = work("T-3", Status::Queued { order: 4 });
        unstamped.created_at = None;
        let mut unstamped_prog = match program("T-4", Status::Queued { order: 5 }, &["T-4.1"], None)
        {
            Ticket::Program(p) => p,
            Ticket::Work(_) => unreachable!(),
        };
        unstamped_prog.created_at = None;
        let mut c = corpus(vec![
            Ticket::Work(unstamped),
            Ticket::Program(unstamped_prog),
            child_of("T-4", "T-4.1", Status::Idea),
        ]);
        let before = c.clone();
        for id in ["T-3", "T-4"] {
            let err = ship(&mut c, id, CLOCK).expect_err("created_at-less must refuse");
            assert!(
                err.contains(id) && err.contains("created_at") && err.contains("backfill"),
                "must name ticket, field and fix: {err}"
            );
        }
        assert_eq!(before, c, "refused ship must leave the corpus untouched");
    }

    /// T-917.6 — stamp_sha writes the landing SHA through both arms, is a no-op on
    /// the same sha, refuses a different sha / a non-shipped ticket / a garbage sha,
    /// and drops a stale "shipped_at" estimated[] marker when it closes the field.
    #[test]
    fn stamp_sha_writes_noops_and_refuses() {
        let mut absent_marked = work(
            "T-1",
            Status::Shipped {
                shipped_at: None,
                order: Some(7),
            },
        );
        absent_marked.estimated = vec!["shipped_at".into()];
        absent_marked.estimate_note = Some("no subject commits; no SHA mined".into());
        let mut c = corpus(vec![
            Ticket::Work(absent_marked),
            Ticket::Work(work("T-2", Status::Queued { order: 9 })),
            program(
                "T-5",
                Status::Shipped {
                    shipped_at: None,
                    order: Some(11),
                },
                &["T-5.1"],
                None,
            ),
            child_of("T-5", "T-5.1", Status::Idea),
        ]);
        let before = c.clone();
        // Garbage shapes refuse (empty, too short, uppercase, branch-shaped).
        for bad in ["", "abc123", "ABCDEF12", "slice/T-197", "2026-07-26"] {
            let err = stamp_sha(&mut c, "T-1", bad, CLOCK).expect_err("garbage sha");
            assert!(
                err.contains("refusing stamp-sha T-1") && err.contains("lowercase hex"),
                "{err}"
            );
        }
        // Non-shipped refuses naming the status.
        let err = stamp_sha(&mut c, "T-2", "abcdef12", CLOCK).expect_err("not shipped");
        assert!(
            err.contains("refusing stamp-sha T-2") && err.contains("queued"),
            "{err}"
        );
        assert_eq!(before, c, "refusals must not mutate");
        // Work write: both arms + marker dropped (measured provenance now).
        let out = stamp_sha(&mut c, "T-1", "abcdef12", CLOCK).expect("stamp work");
        assert_eq!(out.changed, vec!["T-1".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(w.shipped_at.as_deref(), Some("abcdef12"));
                assert_eq!(
                    w.status,
                    Status::Shipped {
                        shipped_at: Some("abcdef12".into()),
                        order: Some(7)
                    }
                );
                assert!(
                    !w.estimated.iter().any(|e| e == "shipped_at"),
                    "stale absent-marker must drop: {:?}",
                    w.estimated
                );
            }
            Ticket::Program(_) => panic!("work"),
        }
        // Program write: the status arm (programs carry no standalone field).
        stamp_sha(&mut c, "T-5", "beadfeed", CLOCK).expect("stamp program");
        match c.get("T-5").unwrap() {
            Ticket::Program(p) => assert_eq!(
                p.status,
                Status::Shipped {
                    shipped_at: Some("beadfeed".into()),
                    order: Some(11)
                }
            ),
            Ticket::Work(_) => panic!("program"),
        }
        // Same sha again: no-op with an empty changed set.
        let out = stamp_sha(&mut c, "T-1", "abcdef12", CLOCK).expect("re-stamp same sha");
        assert!(out.changed.is_empty(), "no-op must report nothing changed");
        // Different sha: refuse — shipped_at is never overwritten.
        let err = stamp_sha(&mut c, "T-1", "00000001", CLOCK).expect_err("different sha");
        assert!(
            err.contains("never overwritten") && err.contains("abcdef12"),
            "{err}"
        );
    }

    /// A parent whose `active` names a DIFFERENT child is untouched by a child ship.
    #[test]
    fn ship_leaves_unrelated_parent_active() {
        let mut c = corpus(vec![
            program(
                "T-1",
                Status::Queued { order: 5 },
                &["T-1.1", "T-1.2"],
                Some("T-1.2"),
            ),
            child_of("T-1", "T-1.1", Status::Idea),
            child_of("T-1", "T-1.2", Status::Idea),
        ]);
        let out = ship(&mut c, "T-1.1", CLOCK).expect("ship");
        assert_eq!(out.changed, vec!["T-1.1".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.2")),
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
    }

    /// `mark_ready`: spec argument lands, deps gate fires exactly like cmd_mark_ready
    /// ("Blocked by …"), story backfills summary→title→id, acceptance backfills
    /// `["See spec."]`, and (T-917.6) the `plan` field lands on the default path. A
    /// queued ticket with empty owns stays legal — queued was already live, so this
    /// op did not MAKE it live (no retro-policing).
    #[test]
    fn mark_ready_backfills_and_gates() {
        let root = scratch_root("mark-ready");
        fs::create_dir_all(root.join("docs/plans")).unwrap();
        fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
        fs::write(root.join("docs/plans/t-1_plan.md"), "# plan\n").unwrap();
        fs::write(root.join("docs/plans/t-2_plan.md"), "# plan\n").unwrap();
        let mut c = Corpus::new(&root);
        let mut t1 = work("T-1", Status::Queued { order: 10 });
        t1.owns = vec![];
        c.tickets.insert("T-1".into(), Ticket::Work(t1));
        let mut t2 = work("T-2", Status::Queued { order: 11 });
        t2.depends_on = vec!["T-1".into(), "T-404".into()];
        t2.spec = Some("docs/spec.md".into());
        c.tickets.insert("T-2".into(), Ticket::Work(t2));

        let err = mark_ready(&mut c, "T-2", None, None, CLOCK).expect_err("dep gate");
        assert_eq!(err, "Blocked by T-1 (status=queued)");

        let out = mark_ready(&mut c, "T-1", Some("docs/spec.md"), None, CLOCK).expect("ready");
        assert_eq!(out.changed, vec!["T-1".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(
                    w.status,
                    Status::Ready {
                        order: 10,
                        spec: "docs/spec.md".into(),
                        user_story: "T-1 summary".into(),
                        acceptance: vec!["See spec.".into()],
                    }
                );
                assert_eq!(w.spec.as_deref(), Some("docs/spec.md"));
                assert_eq!(
                    w.plan.as_deref(),
                    Some("docs/plans/t-1_plan.md"),
                    "unset plan defaults to the id-derived path and is WRITTEN"
                );
                assert_eq!(w.user_story.as_deref(), Some("T-1 summary"));
                assert_eq!(w.acceptance, vec!["See spec.".to_string()]);
            }
            Ticket::Program(_) => panic!("T-1 must stay work"),
        }
        // T-1 ready (not shipped/cancelled) still blocks T-2; a cancel unblocks.
        let err = mark_ready(&mut c, "T-2", None, None, CLOCK).expect_err("still blocked");
        assert_eq!(err, "Blocked by T-1 (status=ready)");
        set_status(&mut c, "T-1", "cancelled", CLOCK).expect("cancel");
        mark_ready(&mut c, "T-2", None, None, CLOCK)
            .expect("deps satisfied; T-404 absent is skipped");
    }

    /// T-917.6 plan ready-gate: mark-ready without the plan file refuses naming the
    /// path (corpus untouched); an explicit PLAN argument overrides the default and
    /// must exist too; an existing `plan` field is honored over the default.
    #[test]
    fn mark_ready_plan_gate_refuses_and_resolves() {
        let root = scratch_root("mark-ready-plan");
        fs::create_dir_all(root.join("docs/plans")).unwrap();
        fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
        let mut c = Corpus::new(&root);
        c.tickets.insert(
            "T-9.1".into(),
            Ticket::Work(work("T-9.1", Status::Queued { order: 10 })),
        );
        let before = c.clone();
        // No plan file anywhere → refuse naming the DEFAULT path (dots → underscores).
        let err =
            mark_ready(&mut c, "T-9.1", Some("docs/spec.md"), None, CLOCK).expect_err("no plan");
        assert!(
            err.starts_with("Plan file not found: ") && err.contains("docs/plans/t-9_1_plan.md"),
            "{err}"
        );
        assert_eq!(before, c, "refusal must not mutate");
        // Explicit PLAN argument that is missing → refuse naming THAT path.
        let err = mark_ready(
            &mut c,
            "T-9.1",
            Some("docs/spec.md"),
            Some("docs/plans/custom.md"),
            CLOCK,
        )
        .expect_err("explicit plan missing");
        assert!(err.contains("docs/plans/custom.md"), "{err}");
        // Present explicit plan lands and is written to the field.
        fs::write(root.join("docs/plans/custom.md"), "# plan\n").unwrap();
        mark_ready(
            &mut c,
            "T-9.1",
            Some("docs/spec.md"),
            Some("docs/plans/custom.md"),
            CLOCK,
        )
        .expect("explicit plan present");
        match c.get("T-9.1").unwrap() {
            Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some("docs/plans/custom.md")),
            Ticket::Program(_) => panic!("work"),
        }
        // An already-set plan field is honored when no argument is passed.
        set_status(&mut c, "T-9.1", "queued", CLOCK).expect("back to queued");
        mark_ready(&mut c, "T-9.1", None, None, CLOCK).expect("field plan honored");
        match c.get("T-9.1").unwrap() {
            Ticket::Work(w) => assert_eq!(w.plan.as_deref(), Some("docs/plans/custom.md")),
            Ticket::Program(_) => panic!("work"),
        }
        assert_eq!(default_plan_path("T-917.6"), "docs/plans/t-917_6_plan.md");
        assert_eq!(default_plan_path("T-090.4"), "docs/plans/t-090_4_plan.md");
    }

    #[test]
    fn mark_ready_refuses_missing_spec_and_missing_file() {
        let root = scratch_root("mark-ready-missing");
        let mut c = Corpus::new(&root);
        c.tickets.insert(
            "T-1".into(),
            Ticket::Work(work("T-1", Status::Queued { order: 1 })),
        );
        let err = mark_ready(&mut c, "T-1", None, None, CLOCK).expect_err("no spec");
        assert_eq!(err, "Ticket T-1 needs a spec path");
        let err =
            mark_ready(&mut c, "T-1", Some("docs/nope.md"), None, CLOCK).expect_err("file missing");
        assert!(err.starts_with("Spec file not found: "), "{err}");
        assert!(err.contains("docs/nope.md"), "{err}");
    }

    #[test]
    fn mark_ready_without_order_refuses() {
        let root = scratch_root("mark-ready-order");
        fs::create_dir_all(root.join("docs/plans")).unwrap();
        fs::write(root.join("docs/spec.md"), "# spec\n").unwrap();
        fs::write(root.join("docs/plans/t-1_plan.md"), "# plan\n").unwrap();
        let mut c = Corpus::new(&root);
        c.tickets
            .insert("T-1".into(), Ticket::Work(work("T-1", Status::Idea)));
        let err =
            mark_ready(&mut c, "T-1", Some("docs/spec.md"), None, CLOCK).expect_err("no order");
        assert!(err.contains("order"), "{err}");
    }

    /// `add` mints max-parent+1 (children never affect it), stamps `created_at` from
    /// the injected clock, defaults scope to repo/docs, and falls back summary→title.
    #[test]
    fn add_mints_next_parent_id_and_stamps() {
        let mut c = corpus(vec![
            Ticket::Work(work("T-001", Status::Idea)),
            Ticket::Work(work("T-910", Status::Idea)),
            child_of("T-910", "T-910.7", Status::Idea),
        ]);
        let (tid, out) = add(&mut c, "New thing", "", CLOCK).expect("add");
        assert_eq!(tid, "T-911");
        assert_eq!(out.changed, vec!["T-911".to_string()]);
        match c.get("T-911").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(w.status, Status::Idea);
                assert_eq!(w.summary, "New thing", "summary falls back to title");
                assert_eq!(w.created_at.as_deref(), Some(CLOCK));
                assert_eq!(
                    w.scope,
                    ScopeV2 {
                        domain: Domain::Repo,
                        layer: "docs".into(),
                        component: None,
                        surface: vec![],
                    },
                    "T-917.2 mint default: flat repo/docs, component-free"
                );
                assert_eq!(
                    w.class.as_deref(),
                    Some("feature"),
                    "minted class comes from the classify_work triage"
                );
            }
            Ticket::Program(_) => panic!("minted ticket must be work"),
        }
    }

    /// T-917.2 — the surface rule mirrors the owns rule: an op that makes a
    /// component-bearing, surface-less work ticket live refuses naming the fix;
    /// a surface or the migrator's `"scope"` estimated-marker passes; component-free
    /// scope is exempt (no vocabulary surfaces exist to require).
    #[test]
    fn made_live_component_without_surface_refuses() {
        let mut bare = work("T-2", Status::Idea);
        bare.scope = ScopeV2 {
            domain: Domain::Website,
            layer: "frontend".into(),
            component: Some("mission_creator".into()),
            surface: vec![],
        };
        let mut marked = work("T-4", Status::Idea);
        marked.scope = bare.scope.clone();
        marked.estimated = vec!["scope".into()];
        let mut surfaced = work("T-5", Status::Idea);
        surfaced.scope = ScopeV2 {
            surface: vec!["attr_panel".into()],
            ..bare.scope.clone()
        };
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            Ticket::Work(bare),
            Ticket::Work(work("T-3", Status::Idea)),
            Ticket::Work(marked),
            Ticket::Work(surfaced),
        ]);
        let before = c.clone();
        let err = reorder(&mut c, "T-2", "T-1", CLOCK).expect_err("surface-less made live");
        assert!(
            err.contains("surface required") && err.contains("mission_creator"),
            "{err}"
        );
        assert_eq!(before, c, "refusal must not mutate");
        // Component-free scope (the mint default) stays mintable → live.
        reorder(&mut c, "T-3", "T-1", CLOCK).expect("component-free exempt");
        // The migrator's honest escape passes…
        reorder(&mut c, "T-4", "T-3", CLOCK).expect("scope ∈ estimated passes");
        // …and so does a real surface.
        reorder(&mut c, "T-5", "T-4", CLOCK).expect("surfaced ticket passes");
    }

    /// T-916.1 acceptance 2 — add-child onto a work parent refuses without `promote`;
    /// with `promote` the parent atomically becomes a program (scope dropped, every
    /// other field preserved) carrying the freshly minted first child.
    #[test]
    fn add_child_onto_work_refuses_then_promotes() {
        let mut c = corpus(vec![Ticket::Work(work("T-5", Status::Queued { order: 9 }))]);
        let before = c.clone();
        let err = add_child(&mut c, "T-5", "First slice", "", false, CLOCK)
            .expect_err("work parent without promote must refuse");
        assert!(
            err.contains("kind work") && err.contains("promote"),
            "{err}"
        );
        assert_eq!(before, c, "refusal must not mutate");

        let (cid, out) = add_child(&mut c, "T-5", "First slice", "", true, CLOCK)
            .expect("promote rewrites work→program");
        assert_eq!(cid, "T-5.1");
        assert_eq!(out.changed, vec!["T-5".to_string(), "T-5.1".to_string()]);
        match c.get("T-5").unwrap() {
            Ticket::Program(p) => {
                assert_eq!(p.children, vec!["T-5.1".to_string()]);
                assert_eq!(p.active, None);
                assert_eq!(p.status, Status::Queued { order: 9 }, "status preserved");
                assert_eq!(
                    p.executor.as_deref(),
                    Some("claude-code"),
                    "fields preserved"
                );
                assert_eq!(p.owns, vec!["T-5.surface".to_string()], "owns preserved");
                let rendered = render_ticket_toml(c.get("T-5").unwrap()).unwrap();
                assert!(rendered.contains("kind = \"program\""), "{rendered}");
                assert!(
                    !rendered.contains("[scope"),
                    "scope must be dropped:\n{rendered}"
                );
            }
            Ticket::Work(_) => panic!("T-5 must be a program now"),
        }
        match c.get("T-5.1").unwrap() {
            Ticket::Work(w) => {
                assert_eq!(w.parent.as_deref(), Some("T-5"));
                assert_eq!(w.status, Status::Idea);
                assert_eq!(w.created_at.as_deref(), Some(CLOCK));
                assert_eq!(w.summary, "First slice", "summary falls back to title");
            }
            Ticket::Program(_) => panic!("minted child must be work"),
        }
    }

    /// Promotion refuses, by name, the two fields a program cannot carry.
    #[test]
    fn promote_refuses_parented_and_stray_shipped_at() {
        let mut parented = work("T-6.1", Status::Idea);
        parented.parent = Some("T-6".into());
        let mut c = corpus(vec![
            program("T-6", Status::Idea, &["T-6.1"], None),
            Ticket::Work(parented),
        ]);
        let err = add_child(&mut c, "T-6.1", "x", "", true, CLOCK).expect_err("parented");
        assert!(err.contains("parent") && err.contains("T-6"), "{err}");

        let mut stray = work("T-7", Status::Queued { order: 3 });
        stray.shipped_at = Some("abc123".into());
        let mut c = corpus(vec![Ticket::Work(stray)]);
        let err = add_child(&mut c, "T-7", "x", "", true, CLOCK).expect_err("stray shipped_at");
        assert!(err.contains("shipped_at"), "{err}");
    }

    #[test]
    fn add_child_onto_program_appends_next_free_id() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1"], None),
            child_of("T-1", "T-1.1", Status::Idea),
        ]);
        let (cid, out) = add_child(&mut c, "T-1", "Second", "sum", false, CLOCK).expect("append");
        assert_eq!(cid, "T-1.2");
        assert_eq!(out.changed, vec!["T-1".to_string(), "T-1.2".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => {
                assert_eq!(p.children, vec!["T-1.1".to_string(), "T-1.2".to_string()]);
            }
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
    }

    /// T-916.1 acceptance 2 — duplicate `children[]` entries refuse at the post-image
    /// gate (corpus-wide; the 4a2f3426 class), whatever op tries to write.
    #[test]
    fn duplicate_child_refuses_any_op() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1", "T-1.1"], None),
            child_of("T-1", "T-1.1", Status::Idea),
        ]);
        let before = c.clone();
        let err = set_status(&mut c, "T-1.1", "deferred", CLOCK).expect_err("dup children");
        assert!(
            err.contains("duplicate child") && err.contains("T-1.1"),
            "{err}"
        );
        assert_eq!(before, c);
    }

    /// T-916.1 acceptance 2/3 — remove of a program refuses without `force`; with
    /// `force` it cascade-deletes the full descendant closure (children[] edges AND
    /// work parent back-edges, nested programs included).
    #[test]
    fn remove_program_refuses_then_force_cascades() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
            child_of("T-1", "T-1.1", Status::Idea),
            program("T-1.2", Status::Idea, &["T-1.2.1"], None),
            child_of("T-1.2", "T-1.2.1", Status::Idea),
        ]);
        let before = c.clone();
        let err = remove(&mut c, "T-1", false, CLOCK).expect_err("program refuses");
        assert!(err.contains("force") && err.contains("cascade"), "{err}");
        assert_eq!(before, c);

        let out = remove(&mut c, "T-1", true, CLOCK).expect("force cascades");
        assert_eq!(
            out.deleted,
            vec![
                "T-1".to_string(),
                "T-1.1".to_string(),
                "T-1.2".to_string(),
                "T-1.2.1".to_string()
            ]
        );
        assert!(out.changed.is_empty());
        assert!(c.tickets.is_empty(), "closure removes every descendant");
    }

    /// T-916.1 acceptance 2 — removing the last child of a program refuses, naming the
    /// fix (programs require children).
    #[test]
    fn remove_last_child_of_program_refuses() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1"], None),
            child_of("T-1", "T-1.1", Status::Idea),
        ]);
        let before = c.clone();
        let err = remove(&mut c, "T-1.1", false, CLOCK).expect_err("last child");
        assert!(
            err.contains("no children") && err.contains("T-1"),
            "must name program and fix: {err}"
        );
        assert_eq!(before, c);
    }

    #[test]
    fn remove_work_scrubs_parent_children() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
            child_of("T-1", "T-1.1", Status::Idea),
            child_of("T-1", "T-1.2", Status::Idea),
        ]);
        let out = remove(&mut c, "T-1.1", false, CLOCK).expect("remove child");
        assert_eq!(out.deleted, vec!["T-1.1".to_string()]);
        assert_eq!(out.changed, vec!["T-1".to_string()]);
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => assert_eq!(p.children, vec!["T-1.2".to_string()]),
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
        assert!(c.get("T-1.1").is_none());
    }

    /// A work ticket double-listed by a second program (the live T-067.1 shape) cannot
    /// be removed while that listing dangles — the corpus-wide referential check
    /// refuses, naming the listing program.
    #[test]
    fn remove_double_listed_child_refuses() {
        let mut c = corpus(vec![
            program("T-8", Status::Idea, &["T-8.1", "T-8.2"], None),
            child_of("T-8", "T-8.1", Status::Idea),
            child_of("T-8", "T-8.2", Status::Idea),
            program("T-9", Status::Idea, &["T-8.1"], None),
        ]);
        let before = c.clone();
        let err = remove(&mut c, "T-8.1", false, CLOCK).expect_err("dangling listing");
        assert!(err.contains("T-9") && err.contains("T-8.1"), "{err}");
        assert_eq!(before, c);
    }

    /// T-916.1 acceptance 2 — a reorder that would land on an occupied live order
    /// refuses instead of writing red state (the sanctioned wedge fix).
    #[test]
    fn reorder_collision_refuses() {
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            Ticket::Work(work("T-2", Status::Queued { order: 11 })),
            Ticket::Work(work("T-3", Status::Queued { order: 20 })),
        ]);
        let before = c.clone();
        let err = reorder(&mut c, "T-3", "T-1", CLOCK).expect_err("11 is taken");
        assert!(err.contains("duplicate live order 11"), "{err}");
        assert!(err.contains("T-2") && err.contains("T-3"), "{err}");
        assert_eq!(before, c, "the colliding write never lands");
        // A free slot works and flips nothing else.
        let out = reorder(&mut c, "T-3", "T-2", CLOCK).expect("12 is free");
        assert_eq!(out.changed, vec!["T-3".to_string()]);
        match c.get("T-3").unwrap() {
            Ticket::Work(w) => assert_eq!(w.status, Status::Queued { order: 12 }),
            Ticket::Program(_) => panic!("T-3 must stay work"),
        }
    }

    /// Reorder flips idea→queued (cmd_reorder), which makes the ticket live — so an
    /// owns-empty work idea refuses (the op made it live), while an owned one flips.
    #[test]
    fn reorder_flips_idea_and_requires_owns() {
        let mut bare = work("T-2", Status::Idea);
        bare.owns = vec![];
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            Ticket::Work(bare),
            Ticket::Work(work("T-3", Status::Idea)),
        ]);
        let err = reorder(&mut c, "T-2", "T-1", CLOCK).expect_err("owns-empty made live");
        assert!(err.contains("owns required"), "{err}");
        let out = reorder(&mut c, "T-3", "T-1", CLOCK).expect("owned idea flips");
        assert_eq!(out.changed, vec!["T-3".to_string()]);
        match c.get("T-3").unwrap() {
            Ticket::Work(w) => assert_eq!(w.status, Status::Queued { order: 11 }),
            Ticket::Program(_) => panic!("T-3 must stay work"),
        }
    }

    /// Don't retro-police: the live tree already carries parent↔child live-order
    /// collisions the parents-only `validate_registry` never reds (order 900 across
    /// the T-090 family, measured 2026-08-14). Ops that do not introduce a NEW
    /// collision must keep working on such a corpus.
    #[test]
    fn preexisting_collision_is_not_retro_policed() {
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 900 })),
            Ticket::Work(work("T-2", Status::Queued { order: 900 })),
            Ticket::Work(work("T-3", Status::Queued { order: 10 })),
        ]);
        let out = set_status(&mut c, "T-3", "deferred", CLOCK)
            .expect("op away from the collision must not be blocked by preexisting red");
        assert_eq!(out.changed, vec!["T-3".to_string()]);
        // But JOINING the preexisting collision is still a new pair — refuse.
        let mut c2 = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 899 })),
            Ticket::Work(work("T-2", Status::Queued { order: 900 })),
            Ticket::Work(work("T-4", Status::Queued { order: 900 })),
            Ticket::Work(work("T-3", Status::Queued { order: 10 })),
        ]);
        let err = reorder(&mut c2, "T-3", "T-1", CLOCK).expect_err("joining 900 refuses");
        assert!(err.contains("duplicate live order 900"), "{err}");
    }

    /// Both anchor failure modes print the exact legacy string (cmd_reorder prints
    /// "Unknown anchor ticket" for missing AND for order-less anchors).
    #[test]
    fn reorder_unknown_anchor_message() {
        let mut c = corpus(vec![
            Ticket::Work(work("T-1", Status::Queued { order: 10 })),
            Ticket::Work(work("T-2", Status::Idea)),
        ]);
        let err = reorder(&mut c, "T-1", "T-404", CLOCK).expect_err("missing anchor");
        assert_eq!(err, "Unknown anchor ticket: T-404");
        let err = reorder(&mut c, "T-1", "T-2", CLOCK).expect_err("order-less anchor");
        assert_eq!(err, "Unknown anchor ticket: T-2");
    }

    /// cmd_advance_slice walk over typed children: first child when no active, next
    /// after the current one, refuse past the end, refuse active-not-in-children —
    /// legacy refusal strings verbatim.
    #[test]
    fn advance_slice_walks_and_refuses() {
        let mut c = corpus(vec![
            program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
            child_of("T-1", "T-1.1", Status::Idea),
            child_of("T-1", "T-1.2", Status::Idea),
            Ticket::Work(work("T-2", Status::Idea)),
        ]);
        advance_slice(&mut c, "T-1", CLOCK).expect("first child");
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.1")),
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
        advance_slice(&mut c, "T-1", CLOCK).expect("next child");
        match c.get("T-1").unwrap() {
            Ticket::Program(p) => assert_eq!(p.active.as_deref(), Some("T-1.2")),
            Ticket::Work(_) => panic!("T-1 must stay program"),
        }
        let err = advance_slice(&mut c, "T-1", CLOCK).expect_err("past end");
        assert_eq!(err, "T-1: no slice after T-1.2");
        let err = advance_slice(&mut c, "T-2", CLOCK).expect_err("work has no children");
        assert_eq!(err, "T-2 has no slices[]");
        if let Some(Ticket::Program(p)) = c.tickets.get_mut("T-1") {
            p.active = Some("T-9.9".into());
        }
        let err = advance_slice(&mut c, "T-1", CLOCK).expect_err("bogus active");
        assert_eq!(err, "active_slice T-9.9 not in slices[]");
    }

    /// T-916.1 acceptance 3 — the 4a2f3426 alias-class regression pin. The bug: the
    /// Value path mirrored `children`→`slices` and `active`→`active_slice`, and a
    /// value carrying BOTH spellings blew up serde's alias handling ("duplicate field
    /// `children`"), breaking every mutator. Through the typed path the mirrored-keys
    /// condition is unrepresentable: legacy spellings on disk still PARSE (serde
    /// aliases), but no rendered output of any op ever contains a `slices =` or
    /// `active_slice =` line — there is nothing to clash.
    #[test]
    fn mirrored_keys_unrepresentable_4a2f3426_pin() {
        // Legacy spellings parse via alias into the canonical typed fields…
        let legacy = r#"
id = "T-1"
kind = "program"
title = "t"
summary = "s"
status = "idea"
slices = ["T-1.1", "T-1.2"]
active_slice = "T-1.1"
"#;
        let t = parse_ticket_toml(legacy).expect("aliases parse");
        let rendered = render_ticket_toml(&t).expect("render");
        assert!(rendered.contains("children = ["), "{rendered}");
        assert!(rendered.contains("active = \"T-1.1\""), "{rendered}");
        // …and after real ops, NO ticket in the corpus renders a mirrored key.
        let mut c = corpus(vec![
            t,
            child_of("T-1", "T-1.1", Status::Idea),
            child_of("T-1", "T-1.2", Status::Idea),
        ]);
        advance_slice(&mut c, "T-1", CLOCK).expect("advance");
        ship(&mut c, "T-1.1", CLOCK).expect("ship child");
        for (id, ticket) in &c.tickets {
            let out = render_ticket_toml(ticket).expect("render");
            for line in out.lines() {
                assert!(
                    !line.starts_with("slices = ") && !line.starts_with("active_slice = "),
                    "{id} rendered a mirrored legacy key:\n{out}"
                );
            }
        }
    }

    /// Injected-clock determinism: the same op sequence with the same stamp produces
    /// byte-identical renders — nothing inside ops ever reads the wall clock.
    #[test]
    fn injected_clock_determinism() {
        let build = || {
            corpus(vec![
                Ticket::Work(work("T-1", Status::Queued { order: 10 })),
                Ticket::Work(work("T-2", Status::Idea)),
            ])
        };
        let run = |c: &mut Corpus| {
            let (tid, _) = add(c, "Minted", "", CLOCK).expect("add");
            assert_eq!(tid, "T-003", "cmd_add zero-pads: T-{{:03}}");
            add_child(c, "T-2", "Slice one", "", true, CLOCK).expect("promote");
            reorder(c, "T-003", "T-1", CLOCK).expect_err("minted idea has empty owns");
            set_status(c, "T-1", "cancelled", CLOCK).expect("cancel");
            ship(c, "T-2.1", CLOCK).expect("ship child");
            let mut all = String::new();
            for ticket in c.tickets.values() {
                all.push_str(&render_ticket_toml(ticket).expect("render"));
                all.push('\n');
            }
            all
        };
        let (mut a, mut b) = (build(), build());
        let (ra, rb) = (run(&mut a), run(&mut b));
        assert_eq!(ra, rb, "same clock, same bytes");
        assert!(ra.contains(CLOCK), "stamps came from the injected clock");
    }

    /// End-to-end on disk: op outcome feeds write_back + delete_files, and the tree
    /// reflects exactly the changed/deleted sets — nothing else.
    #[test]
    fn remove_cascade_end_to_end_on_disk() {
        let root = scratch_root("remove-e2e");
        let mut c = Corpus::new(&root);
        for t in [
            program("T-1", Status::Idea, &["T-1.1", "T-1.2"], None),
            child_of("T-1", "T-1.1", Status::Idea),
            child_of("T-1", "T-1.2", Status::Idea),
            Ticket::Work(work("T-2", Status::Idea)),
        ] {
            c.tickets.insert(t.id().to_string(), t);
        }
        let all: Vec<String> = c.tickets.keys().cloned().collect();
        c.write_back(&all).expect("seed tree");
        let out = remove(&mut c, "T-1", true, CLOCK).expect("cascade");
        c.write_back(&out.changed).expect("write changed");
        c.delete_files(&out.deleted).expect("delete files");
        let left: Vec<String> = fs::read_dir(root.join(".ai/tickets"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            left,
            vec!["T-2.toml".to_string()],
            "only the survivor remains"
        );
    }

    /// T-917.3: an op may not mint a NEW summary wall — `add` with a >40-word summary
    /// refuses pre-write (corpus byte-untouched, the ops refusal-test pattern), naming
    /// the count and the cap.
    #[test]
    fn add_refuses_wall_summary_pre_write() {
        let mut c = corpus(vec![Ticket::Work(work("T-1", Status::Idea))]);
        let before = c.clone();
        let wall = "wall ".repeat(crate::SUMMARY_WORD_CAP + 1);
        let err = add(&mut c, "Short title", wall.trim(), CLOCK).expect_err("wall must refuse");
        assert!(
            err.contains("41 words") && err.contains("cap 40"),
            "must name count and cap: {err}"
        );
        assert_eq!(c, before, "refused op must leave the corpus untouched");
    }

    /// T-917.3: nonempty `migration_legacy` exempts exactly the summary cap — an op
    /// touching a quarantined ticket (summary := title, possibly >40 words) commits.
    #[test]
    fn quarantined_ticket_is_exempt_from_summary_cap() {
        let mut w = work("T-1", Status::Queued { order: 5 });
        w.summary = "word ".repeat(50).trim().to_string();
        w.migration_legacy = vec!["the original wall".into()];
        let mut c = corpus(vec![Ticket::Work(w)]);
        set_status(&mut c, "T-1", "deferred", CLOCK).expect("quarantined ticket must stay mutable");

        // The same summary WITHOUT the quarantine marker refuses.
        let mut unmarked = work("T-2", Status::Queued { order: 6 });
        unmarked.summary = "word ".repeat(50).trim().to_string();
        let mut c = corpus(vec![Ticket::Work(unmarked)]);
        let err = set_status(&mut c, "T-2", "deferred", CLOCK)
            .expect_err("unquarantined wall in a changed ticket must refuse");
        assert!(err.contains("50 words") && err.contains("cap 40"), "{err}");
    }
}
