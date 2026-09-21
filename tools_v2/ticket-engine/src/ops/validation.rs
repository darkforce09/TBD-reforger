//! Validation.

use super::*;

/// Post-image validation — the private gate every op runs on its candidate image
/// before any byte can land. THE invariant: no op may write a corpus its own preflight
/// would refuse.
///
/// Scoping decisions, each measured against the live tree on 2026-08-14:
///
/// - **Live-order collisions: refuse only NEW colliding pairs.** The live tree already
///   carries parent↔child live-order collisions (order 900 across one program family,
///   one order shared by a parent and its children) that `validate_registry` never reds —
///   its walk is parents-only — so a literal corpus-wide refusal would wedge every op
///   on a tree the check calls green. Refusing collisions the op *introduces* kills
///   exactly the `cmd_reorder` red-write wedge the design names, and never
///   retro-polices preexisting state.
/// - **Empty `owns` on live work: only ids this op made live.** Same
///   don't-retro-police carve-out, stated verbatim in the design.
/// - **Child-id shape (`{parent}.{suffix}`): changed programs only.** Measured
///   preexisting violation: a parked program cross-lists another program's child. A
///   corpus-wide rule would refuse every op on the live tree; scoping to programs the
///   op touched still guarantees ops never *produce* a non-dotted child.
/// - **Duplicate `children[]` entries and dangling `children[]` references:
///   corpus-wide.** The live tree is clean on both (measured), so these cannot wedge —
///   and dangling-reference checking must be corpus-wide anyway, or a `remove` could
///   strand a listing in an untouched program.
pub(super) fn validate_post_image(
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
    // No op may write a NEW summary wall. Scoped to `changed` — the same
    // don't-retro-police carve-out the fn header documents: the live tree carries no
    // unquarantined wall, so this binds exactly on prose an op introduces (`add` /
    // `add_child` summaries, or a summary-editing verb). Nonempty `migration_legacy`
    // exempts exactly the summary cap (a parked ticket carries `summary := title`,
    // which may itself exceed the cap); no op mints that field, and a mint dated after
    // the quarantine cutover is red in `ticket check`, not here.
    // Work-only: program summaries are uncapped this pass (spec §Wall quarantine).
    for id in changed {
        if let Some(Ticket::Work(w)) = post.get(id)
            && w.migration_legacy.is_empty()
        {
            let words = w.summary.split_whitespace().count();
            if words > crate::SUMMARY_WORD_CAP {
                return Err(format!(
                    "post-image {id}: summary is {words} words (cap {}) — write the ten typed body fields instead of a wall (caps: spec §Body)",
                    crate::SUMMARY_WORD_CAP
                ));
            }
        }
    }
    // Title gate (t920 spec Decisions log #4): no op may write a ticket —
    // either kind — whose title is empty, its own id, or over TITLE_WORD_CAP words.
    // Scoped to `changed`, the same don't-retro-police carve-out: the 440 history
    // titles are metered debt (TITLE_DEBT_PIN) drained batch by batch;
    // an op that rewrites a debt ticket must repair the title in the same breath.
    // The two nonempty arms are exactly [`crate::title_is_debt`] — one instrument.
    for id in changed {
        if let Some(t) = post.get(id) {
            let title = title_of(t);
            if title.trim().is_empty() {
                return Err(format!(
                    "post-image {id}: title is empty — every ticket carries a real title (t920 spec Decisions log #4)"
                ));
            }
            if title == id {
                return Err(format!(
                    "post-image {id}: title equals the ticket id — write a real title; id-as-title is the measured debt class the drain batches shrink, and ops never add to it (t920 spec Decisions log #4)"
                ));
            }
            let words = title.split_whitespace().count();
            if words > crate::TITLE_WORD_CAP {
                return Err(format!(
                    "post-image {id}: title is {words} words (cap {}) — a title is a scannable one-liner; move the prose into the body fields (t920 spec Decisions log #4)",
                    crate::TITLE_WORD_CAP
                ));
            }
        }
    }
    // Queued-tier main_goal (t920 spec Decisions log #1): a changed LIVE
    // (queued/ready/running/review) work ticket must carry main_goal. Quarantine-
    // exempt (nonempty migration_legacy — content exists, unprocessed; the
    // drain fills main_goal when it decomposes the wall). Scoped to `changed`: the
    // history debt is metered by MAIN_GOAL_DEBT_PIN, never retro-policed — this arm
    // is what makes NEW offenders impossible while the pin drains.
    for id in changed {
        if let Some(Ticket::Work(w)) = post.get(id)
            && w.status.name().is_live()
            && w.migration_legacy.is_empty()
            && w.main_goal.as_deref().unwrap_or("").trim().is_empty()
        {
            return Err(format!(
                "post-image {id}: {} work ticket without main_goal — queued and above state one main goal, rendered first (t920 spec Decisions log #1); write main_goal",
                w.status.name().as_str()
            ));
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
                "duplicate live order {order} on {} — refusing to write a red corpus; pick a different anchor",
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
            // Surface rule (spec Decisions log #3: surface REQUIRED on
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
