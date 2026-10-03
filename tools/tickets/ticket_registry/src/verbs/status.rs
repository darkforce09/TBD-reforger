//! `ticket ready-ids` and `ticket set-status`.
//!
//! **Role:** lists the dispatchable ready ids, and sets one ticket's status.
//! **Position:** called by the xtask `ticket` command group; `set-status` runs the check
//! preflight, the typed operation, the queue regeneration and the wave lock repack.
//! **Signals & state:** none; prints, and `set-status` writes the ticket file, `queue.json` and
//! the wave lock.
//! **Invariants:** an empty or unknown status is refused before any write; `set-status`
//! regenerates only `queue.json` among the sync outputs and always repacks the wave lock.

use super::*;

/// Print, one per line, up to `limit` (default `batch_size`) ids from `queue.json` that are
/// `ready`, carry a spec, use the `claude-code` executor and match `stream` when given.
///
/// # Errors
/// When `queue.json` exists but cannot be read or parsed.
pub fn cmd_ready_ids(
    root: &Path,
    registry: &Value,
    limit: Option<usize>,
    stream: Option<&str>,
) -> Result<()> {
    let queue_path = root.join(repository_layout::QUEUE_JSON);
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let limit = limit.unwrap_or_else(|| {
        data.get("batch_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize
    });
    let mut ids = vec![];
    if let Some(arr) = data.get("tickets").and_then(|t| t.as_array()) {
        for t in arr {
            if opt_str(t, "status") != Some("ready") {
                continue;
            }
            let spec = opt_str(t, "spec").unwrap_or("").trim();
            if spec.is_empty() {
                continue;
            }
            let tid = opt_str(t, "id").unwrap_or("");
            let row = match ticket_by_id(registry, &TicketId::new(tid)) {
                Some(r) => r,
                None => continue,
            };
            if slice_executor(row) != "claude-code" {
                continue;
            }
            if let Some(s) = stream
                && !s.is_empty()
                && opt_str(row, "stream") != Some(s)
            {
                continue;
            }
            ids.push(tid.to_string());
            if ids.len() >= limit {
                break;
            }
        }
    }
    println!("{}", ids.join("\n"));
    Ok(())
}

/// Set ticket `id`'s status to `status` (trimmed), then regenerate `queue.json` and repack the
/// wave lock.
///
/// # Errors
/// When the status is empty or not one of the eight names, the ticket is unknown, the check
/// preflight is red, the typed operation refuses, or a write fails.
pub fn cmd_set_status(
    root: &Path,
    registry: &mut Value,
    id: &TicketId,
    status: &str,
) -> Result<()> {
    // Reject empty / invalid status before any write — never stamp `""` over registry.
    let status = status.trim();
    refuse_empty_write(
        &format!("set-status {id}"),
        status.is_empty(),
        "status must be non-empty (refusing to write \"\" over registry)",
    )?;
    if !VALID_TICKET_STATUSES.contains(&status) {
        return Err(Error::msg(format!(
            "refusing set-status {id}: invalid status `{status}` \
             (expected one of: {})",
            VALID_TICKET_STATUSES.join(", ")
        )));
    }

    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        return Err(unknown_ticket(id));
    }
    // Refuse status writes when the registry fails ticket check
    // (same bar as ship/done). No silent escape hatch; a red registry
    // must be fixed before any status mutator may write.
    require_check_ok(root, registry, &format!("set-status {id}"))?;

    // Typed op: the enum gate again (second net); a cancel is a
    // completion — the op stamps completed_at in the same mutation, before the wave-lock
    // refresh below; other set-status targets do not stamp (`ticket ship` / `ticket done`
    // own the shipped stamp) and `active` is untouched (also ship's job). Transitions the
    // ticket lacks data for refuse UP FRONT instead of wedging mid-save.
    let outcome = ops::set_status(&mut corpus, id, status, &time_source::now_utc_rfc3339())
        .map_err(Error::msg)?;
    corpus.write_back(&outcome.changed).map_err(Error::msg)?;

    // set-status regenerates queue.json and repacks wave.lock ONLY — no full cmd_sync: the
    // roadmap next-work block and the gap-analysis ticket column refresh on the next full sync
    // (`ticket sync` or a mutator that runs cmd_sync). Reload first: queue.json must come from
    // the post-state, not the pre-mutation Value.
    reload_registry(root, registry)?;
    let queue = generate_queue_json(registry);
    write_json_ascii(&root.join(repository_layout::QUEUE_JSON), &queue)?;
    // `set-status cancelled` (and `shipped`) must repack or `wave check` goes red on a
    // correct registry. Run for EVERY status — a demotion out of the dispatchable set (queued →
    // idea/deferred) strands the id in the lock's open waves just as surely as a cancel, and a
    // dispatchability-neutral write recompiles to the identical bytes.
    refresh_wave_lock(root)?;
    Ok(())
}
