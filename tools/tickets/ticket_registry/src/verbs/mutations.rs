//! `ticket add`, `add-child`, `remove`, `reorder` and `advance-slice`.
//!
//! **Role:** the writing verbs that mint, delete, move and advance tickets.
//! **Position:** called by the xtask `ticket` command group; each runs the check preflight, one
//! typed operation, the file writes, a registry reload and `ticket sync`.
//! **Signals & state:** none; writes ticket files and the sync outputs, and prints the result.
//! **Invariants:** none of these verbs repacks the wave lock: a minted idea is not dispatchable,
//! and the next `cargo xtask wave repack` reconciles a removal or promotion.

use super::*;

/// Mint a new work ticket titled `title`, summary falling back to the title, then sync and
/// print `Added <id>: <title>`. `program`, `surfaces` and `impact` are accepted and ignored:
/// the typed ticket has no such fields.
///
/// # Errors
/// When the check preflight is red, the corpus does not load, or a write fails.
pub fn cmd_add(
    root: &Path,
    registry: &mut Value,
    title: &str,
    program: &str,
    surfaces: &str,
    impact: &str,
    summary: &str,
) -> Result<()> {
    // Refuse insert when the registry fails ticket check (same bar as
    // set-status/mark-ready/reorder/ship). Check runs first so a
    // red registry never gets a row write + sync.
    require_check_ok(root, registry, "add")?;

    let mut corpus = load_corpus(root)?;
    let _ = (program, surfaces, impact);
    // Typed op: mints max PARENT numeric + 1 (children never affect it, as in
    // `derive_next_id`), kind work, status idea, repo/docs scope.
    // Every minted ticket gets its birth stamp (RFC 3339 UTC). Existing tickets
    // get NO backfill — only the minting verbs write created_at.
    let (tid, outcome) = ops::add(&mut corpus, title, summary, &time_source::now_utc_rfc3339())
        .map_err(Error::msg)?;
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Added {tid}: {title}");
    Ok(())
}

/// `ticket add-child <PARENT> <TITLE> [--summary S] [--promote]`.
/// Appends a freshly minted child (next free dotted extension, status idea, created_at
/// stamped) under an existing program. A `kind = "work"` parent refuses unless `--promote`
/// performs the atomic work→program rewrite plus first child in one op (the refusal text comes
/// from the op). Syncs like `add`; no repack — a minted idea child is not dispatchable, and a
/// `--promote` of a LIVE work parent is reconciled by the next `cargo xtask wave repack`
/// exactly like a `remove` of a live ticket.
pub fn cmd_add_child(
    root: &Path,
    registry: &mut Value,
    parent_id: &TicketId,
    title: &str,
    summary: &str,
    promote: bool,
) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(parent_id).is_none() {
        return Err(unknown_ticket(parent_id));
    }
    require_check_ok(root, registry, &format!("add-child {parent_id}"))?;

    let was_work = matches!(corpus.get(parent_id), Some(Ticket::Work(_)));
    let (cid, outcome) = ops::add_child(
        &mut corpus,
        parent_id,
        title,
        summary,
        promote,
        &time_source::now_utc_rfc3339(),
    )
    .map_err(Error::msg)?;
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    if was_work {
        println!("Added {cid}: {title} ({parent_id} promoted work -> program)");
    } else {
        println!("Added {cid}: {title}");
    }
    Ok(())
}

/// Remove ticket `id` (a program only with `force`, with every descendant), then sync and print
/// `Removed <id>`.
///
/// # Errors
/// When the ticket is unknown, the check preflight is red, the operation refuses, or a write or
/// delete fails.
pub fn cmd_remove(root: &Path, registry: &mut Value, id: &TicketId, force: bool) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        return Err(unknown_ticket(id));
    }
    // Refuse delete when the registry fails ticket check (same bar as
    // add / set-status). Check runs first so a red registry never loses
    // a row on disk.
    require_check_ok(root, registry, &format!("remove {id}"))?;

    // Typed op: a work ticket deletes surgically and scrubs its parent's
    // children[]; a program REFUSES unless --force cascade-deletes the descendant closure
    // deliberately; nothing is ever deleted as a side effect.
    let outcome =
        ops::remove(&mut corpus, id, force, &time_source::now_utc_rfc3339()).map_err(Error::msg)?;
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;
    corpus.delete_files(&outcome.deleted).map_err(Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Removed {id}");
    Ok(())
}

/// Move `id` to the order after `after` (an idea becomes queued), then sync and print
/// `<id> order -> <order> (after <after>)`.
///
/// # Errors
/// When the ticket is unknown, the check preflight is red, or a write fails; the operation's
/// refusals (unknown anchor, duplicate live order) come back as [`Error::Refused`].
pub fn cmd_reorder(
    root: &Path,
    registry: &mut Value,
    id: &TicketId,
    after: &TicketId,
) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        return Err(unknown_ticket(id));
    }
    // Reorder may flip idea→queued; refuse when check is red.
    require_check_ok(root, registry, &format!("reorder {id}"))?;

    // Typed op: order = anchor + 1, idea flips to queued, every other status keeps
    // its variant. "Unknown anchor ticket: {after}" comes back verbatim on the exit
    // path; the op's OTHER refusal — duplicate live order — keeps red state off the disk.
    let outcome = match ops::reorder(&mut corpus, id, after, &time_source::now_utc_rfc3339()) {
        Ok(o) => o,
        Err(msg) => return Err(refuse_verbatim(msg)),
    };
    let new_order = corpus
        .get(id)
        .and_then(|t| t.status().order())
        .expect("reorder always lands an order");
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} order -> {new_order} (after {after})");
    Ok(())
}

/// Move program `id`'s `active` to its next child (the first when none is active), then sync
/// and print `<id> active_slice -> <child>`.
///
/// # Errors
/// When the check preflight is red, the ticket is unknown, or a write fails; the operation's
/// slice-walk refusals come back as [`Error::Refused`].
pub fn cmd_advance_slice(root: &Path, registry: &mut Value, id: &TicketId) -> Result<()> {
    // Refuse advance when the registry fails ticket check (same bar as
    // add/remove/set-status/mark-ready/reorder/ship).
    // Check runs first so a red registry never gets an active write + sync.
    require_check_ok(root, registry, &format!("advance-slice {id}"))?;

    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        return Err(unknown_ticket(id));
    }
    // Typed op: walks `ProgramTicket::children` — no active → first child, else the next
    // one; the refusals
    // ("{id} has no slices[]", "active_slice {a} not in slices[]", "{id}: no slice after {a}")
    // come back verbatim on the exit path.
    let outcome = match ops::advance_slice(&mut corpus, id, &time_source::now_utc_rfc3339()) {
        Ok(o) => o,
        Err(msg) => return Err(refuse_verbatim(msg)),
    };
    let new_active = match corpus.get(id) {
        Some(Ticket::Program(p)) => p.active.clone().unwrap_or_default(),
        Some(Ticket::Work(_)) | None => String::new(),
    };
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} active_slice -> {new_active}");
    Ok(())
}
