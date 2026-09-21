//! Mutations.

use super::*;

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
    // Typed op: mints max PARENT numeric + 1 (children never affect it —
    // `derive_next_id` semantics preserved), kind work, status idea, repo/docs scope.
    // Every minted ticket gets its birth stamp (RFC 3339 UTC). Existing tickets
    // get NO backfill — only the minting verbs write created_at.
    let (tid, outcome) = ops::add(&mut corpus, title, summary, &crate::now_utc_rfc3339())
        .map_err(anyhow::Error::msg)?;
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Added {tid}: {title}");
    Ok(())
}

/// New verb: `ticket add-child <PARENT> <TITLE> [--summary S] [--promote]`.
/// Appends a freshly minted child (next free dotted extension, status idea, created_at
/// stamped) under an existing program. A `kind = "work"` parent refuses unless `--promote`
/// performs the atomic work→program rewrite plus first child in one op (design Decisions #4;
/// the refusal text comes from the op). Syncs like `add`; no repack — a minted idea child is
/// in wave limbo, and a `--promote` of a LIVE work parent is reconciled by the next
/// `cargo xtask wave repack` exactly like a `remove` of a live ticket (neither verb repacked
/// before the typed ops either).
pub fn cmd_add_child(
    root: &Path,
    registry: &mut Value,
    parent_id: &str,
    title: &str,
    summary: &str,
    promote: bool,
) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(parent_id).is_none() {
        unknown_ticket(parent_id);
    }
    require_check_ok(root, registry, &format!("add-child {parent_id}"))?;

    let was_work = matches!(corpus.get(parent_id), Some(Ticket::Work(_)));
    let (cid, outcome) = ops::add_child(
        &mut corpus,
        parent_id,
        title,
        summary,
        promote,
        &crate::now_utc_rfc3339(),
    )
    .map_err(anyhow::Error::msg)?;
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    if was_work {
        println!("Added {cid}: {title} ({parent_id} promoted work -> program)");
    } else {
        println!("Added {cid}: {title}");
    }
    Ok(())
}

pub fn cmd_remove(root: &Path, registry: &mut Value, id: &str, force: bool) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
    }
    // Refuse delete when the registry fails ticket check (same bar as
    // add / set-status). Check runs first so a red registry never loses
    // a row on disk.
    require_check_ok(root, registry, &format!("remove {id}"))?;

    // Typed op: a work ticket deletes surgically and scrubs its parent's
    // children[]; a program REFUSES unless --force cascade-deletes the descendant closure
    // deliberately (design Decisions #3 — the documented divergence from the old save path,
    // whose stale-file pass cascade-deleted silently).
    let outcome = ops::remove(&mut corpus, id, force, &crate::now_utc_rfc3339())
        .map_err(anyhow::Error::msg)?;
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;
    corpus
        .delete_files(&outcome.deleted)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Removed {id}");
    Ok(())
}

pub fn cmd_reorder(root: &Path, registry: &mut Value, id: &str, after: &str) -> Result<()> {
    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
    }
    // Reorder may flip idea→queued; refuse when check is red.
    require_check_ok(root, registry, &format!("reorder {id}"))?;

    // Typed op: order = anchor + 1, idea flips to queued, every other status keeps
    // its variant. "Unknown anchor ticket: {after}" comes back verbatim on the exit
    // path; the op's OTHER refusal — duplicate live order — is the sanctioned divergence
    // where the old CLI wrote red state on disk (the cmd_reorder wedge).
    let outcome = match ops::reorder(&mut corpus, id, after, &crate::now_utc_rfc3339()) {
        Ok(o) => o,
        Err(msg) => refuse_verbatim(&msg),
    };
    let new_order = corpus
        .get(id)
        .and_then(|t| t.status().order())
        .expect("reorder always lands an order");
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} order -> {new_order} (after {after})");
    Ok(())
}

pub fn cmd_advance_slice(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    // Refuse advance when the registry fails ticket check (same bar as
    // add/remove/set-status/mark-ready/reorder/ship).
    // Check runs first so a red registry never gets an active write + sync.
    require_check_ok(root, registry, &format!("advance-slice {id}"))?;

    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
    }
    // Typed op: walks `ProgramTicket::children` (the Value path read the mirrored
    // `slices` key) — no active → first child, else the next one; the refusals
    // ("{id} has no slices[]", "active_slice {a} not in slices[]", "{id}: no slice after {a}")
    // come back verbatim on the exit path.
    let outcome = match ops::advance_slice(&mut corpus, id, &crate::now_utc_rfc3339()) {
        Ok(o) => o,
        Err(msg) => refuse_verbatim(&msg),
    };
    let new_active = match corpus.get(id) {
        Some(Ticket::Program(p)) => p.active.clone().unwrap_or_default(),
        Some(Ticket::Work(_)) | None => String::new(),
    };
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    reload_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} active_slice -> {new_active}");
    Ok(())
}
