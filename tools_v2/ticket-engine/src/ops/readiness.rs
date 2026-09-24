//! Readiness.

use super::*;

/// A ticket's own plan document. [`mark_ready`] defaults an unset `plan` field to this path.
pub fn default_plan_path(id: &str) -> String {
    crate::repository::documentation::plan_path(id)
}

/// `cmd_mark_ready` semantics: set `spec` when the argument is nonempty; refuse when
/// the resulting spec is empty ("Ticket {id} needs a spec path") or missing on disk
/// under the corpus root ("Spec file not found: …"); refuse while any `depends_on`
/// target present in the corpus is neither shipped nor cancelled ("Blocked by …");
/// then status→ready with the exact backfills: empty `main_goal` becomes
/// summary→title→id, all-empty `acceptance` becomes `["See spec."]`.
///
/// **Ready-tier gate** (t920 spec Decisions log #2): a WORK ticket whose
/// `migration_legacy` is empty refuses promotion while any of the six ready-tier
/// body fields ([`crate::empty_ready_tier_fields`]) is empty — naming each. The
/// backfills therefore only ever fire on quarantined tickets (and on `main_goal`,
/// which the queued tier owns and the backfill fills summary→title→id as before).
///
/// **Plan ready-gate** (spec §Plan documents, Decisions log #9): nothing goes
/// ready without its own plan document. `plan_arg` (nonempty) sets the `plan` field;
/// otherwise an already-set `plan` stands; otherwise the field defaults to
/// [`default_plan_path`]. Whatever path results must EXIST on disk under the corpus
/// root or the op refuses naming it ("Plan file not found: …") — the spec-on-disk
/// gate pattern, extended. The resolved path is WRITTEN to the ticket so the
/// check-level plan rule can see it. `plan` ≠ `spec`: spec stays the shared program
/// authority; plan is this ticket's own four-section document
/// (`.ai/tickets/plan_template.md`).
///
/// One divergence inside the backfill, sanctioned by the refuse-up-front rule: the
/// Value path takes `summary` even when it is the empty string (the key exists), which
/// then wedges mid-save on the empty `main_goal`; the typed backfill takes the first
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
            "Plan file not found: {} — nothing goes ready without its own plan document; copy \
             {} to {resolved_plan} and fill the four sections",
            plan_path.display(),
            crate::repository::documentation::PLAN_TEMPLATE
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
            "refusing mark-ready {id}: ready requires order and the ticket has none — a mid-save wedge is the alternative; reorder it into the queue first"
        )
    })?;
    // Ready-tier gate (t920 spec Decisions log #2): promotion refuses with
    // any of the six body fields empty, naming each — pre-write, corpus untouched
    // (the shared refusal pattern). Work-only, quarantine-exempt: a nonempty
    // migration_legacy means the content exists unprocessed (the drain fills
    // the fields when it decomposes the wall) — the story/acceptance
    // backfills below still serve exactly that path.
    if let Ticket::Work(w) = &snapshot
        && w.migration_legacy.is_empty()
    {
        let missing = crate::empty_ready_tier_fields(w);
        if !missing.is_empty() {
            return Err(format!(
                "refusing mark-ready {id}: ready-tier body fields empty: {} — ready requires context/requirement/current_state/approach/verify/acceptance nonempty (t920 spec Decisions log #2); fill them from the spec and plan, honestly",
                missing.join(", ")
            ));
        }
    }
    let story = {
        let existing = main_goal_of(&snapshot).unwrap_or("");
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
    set_main_goal(t, Some(story));
    set_acceptance(t, acceptance);
    set_ticket_status(t, new_status);
    let mut made_live = BTreeSet::new();
    if !was_live {
        made_live.insert(id.to_string());
    }
    let changed = BTreeSet::from([id.to_string()]);
    commit(c, post, changed, BTreeSet::new(), made_live)
}
