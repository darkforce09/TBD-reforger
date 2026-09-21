//! Status.

use super::*;

pub fn cmd_ready_ids(
    root: &Path,
    registry: &Value,
    limit: Option<usize>,
    stream: Option<&str>,
) -> Result<()> {
    let queue_path = root.join(crate::repository::QUEUE_JSON);
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
            let row = match ticket_by_id(registry, tid) {
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

pub fn cmd_set_status(root: &Path, registry: &mut Value, id: &str, status: &str) -> Result<()> {
    // Reject empty / invalid status before any write — never stamp `""` over registry.
    let status = status.trim();
    refuse_empty_write(
        &format!("set-status {id}"),
        status.is_empty(),
        "status must be non-empty (refusing to write \"\" over registry)",
    )?;
    if !VALID_TICKET_STATUSES.contains(&status) {
        bail!(
            "refusing set-status {id}: invalid status `{status}` \
             (expected one of: {})",
            VALID_TICKET_STATUSES.join(", ")
        );
    }

    let mut corpus = load_corpus(root)?;
    if corpus.get(id).is_none() {
        unknown_ticket(id);
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
    let outcome = ops::set_status(&mut corpus, id, status, &crate::now_utc_rfc3339())
        .map_err(anyhow::Error::msg)?;
    corpus
        .write_back(&outcome.changed)
        .map_err(anyhow::Error::msg)?;

    // Preserved asymmetry (t915 design §Write path): set-status regenerates queue.json +
    // repacks ONLY — no full cmd_sync (generated docs go stale by current design;
    // rationalizing that is its own ticket). Reload first: queue.json must come from the
    // post-state, not the pre-mutation Value.
    reload_registry(root, registry)?;
    let queue = generate_queue_json(registry);
    write_json_ascii(&root.join(crate::repository::QUEUE_JSON), &queue)?;
    // `set-status cancelled` (and `shipped`) must repack or `wave check` goes red on a
    // correct registry. Run for EVERY status — a demotion out of the dispatchable set (queued →
    // idea/deferred) strands the id in the lock's open waves just as surely as a cancel, and a
    // dispatchability-neutral write recompiles to the identical bytes.
    refresh_wave_lock(root)?;
    Ok(())
}
