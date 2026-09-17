//! Runner.

use super::*;

pub fn check(root: &Path, registry: &serde_json::Value, strict: bool) -> Vec<String> {
    // Schema first: structural/enum contract from .ai/tickets/schema.json (T-237 / T-273).
    // Hand-rolled checks below add business rules (order, phantoms, on-disk specs, markers).
    let mut errors = validate_registry_schema(root, registry);
    errors.extend(validate_registry(registry));
    errors.extend(check_open_work_owns(root));
    // T-917.2: class required on every work ticket; surface required on live work with
    // a component (the estimated-marker escape documented on the fns).
    errors.extend(check_work_class(root));
    errors.extend(check_live_work_surface(root));
    // T-917.3: body word caps (summary ≤40 on work, migration_legacy-exempt; body
    // lines ≤30; citations ≤8), anti-blend rules, and the post-cutover
    // quarantine-mint tripwire.
    errors.extend(check_body_rules(root));
    // T-917.4: an estimated[] stamp entry must have a present field — except
    // shipped_at, which may be absent+marked with the gap named in estimate_note.
    errors.extend(check_estimated_stamp_coherence(root));
    // T-916.2: children[] must reference on-disk files and child files must reference on-disk
    // parents — the referential rule the save_tree delete-pass retirement makes load-bearing.
    errors.extend(check_children_integrity(root));
    // T-912.2: the committed wave.lock must match the tickets it was compiled from, and a
    // MISSING lock is a DidNotRun refusal — wired into the base check so every registry mutator
    // preflight and CI's `ticket check --strict` cover it.
    errors.extend(crate::wave_lock::check_as_errors(root));
    // T-913.2: every run receipt under .ai/tickets/metrics/ must satisfy
    // .ai/tickets/metrics.schema.json plus the token-sum / RFC 3339 UTC invariants —
    // a malformed receipt is red, named by file.
    errors.extend(crate::metrics::check_as_errors(root));
    // T-917.5: every token estimate under .ai/tickets/estimates/ must satisfy
    // estimates.schema.json + the business rules (factor == the documented constant,
    // diff_loc arithmetic, shipped-only, receipt/estimate mutual exclusion, and the
    // "tokens" marker ⇔ file coherence). Structurally OUTSIDE metrics/ so an
    // estimate can never impersonate a receipt (T-913).
    errors.extend(crate::metrics::estimates::check_as_errors(root));
    // T-917.6: THE hard ship gate — shipped requires created_at + completed_at +
    // SHA-shaped shipped_at (or the marked-absent asymmetry) + token accounting
    // (receipt XOR estimate; the NEITHER arm) — and the plan ready-gate: ready-class
    // work carries a plan document that exists on disk.
    errors.extend(check_ship_gate(root));
    errors.extend(check_plan_ready_gate(root));
    // T-920.1 tiered body obligations (t920 spec Decisions log #2), composed without
    // double-reporting: (a) idea tier — title nonempty on every work ticket,
    // corpus-wide; (b) queued tier — main_goal metered by MAIN_GOAL_DEBT_PIN (plus
    // the title-debt meter), drift-red both ways, new offenders refused in ops;
    // (c) ready tier — the six body fields nonempty on ready-class work,
    // corpus-wide, quarantine-exempt.
    errors.extend(check_work_title_nonempty(root));
    errors.extend(check_ready_tier_body(root));
    errors.extend(check_debt_pins(root));
    // T-917.1/.2: the scope vocabulary (.ai/tickets/scope-vocab.toml) must EXIST and be
    // shape-valid — BASE tier since the S.2 cutover (S.1 parked existence at --strict
    // only while pre-v2 scratch registries still lacked the file): scope legality now
    // rides every corpus load, so a missing vocabulary is red wherever check runs.
    errors.extend(crate::validation::vocabulary::check_as_errors(root));
    errors.extend(fossil_paths_check(root));

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        if let Some(targets) = row.get("targets").and_then(|t| t.as_array()) {
            for tgt in targets {
                if let Some(s) = tgt.as_str()
                    && !VALID_TARGETS.contains(&s)
                {
                    errors.push(format!("{tid}: invalid target '{s}'"));
                }
            }
        }
        if let Some(ex) = opt_str(row, "executor")
            && !VALID_EXECUTORS.contains(&ex)
        {
            errors.push(format!("{tid}: invalid executor '{ex}'"));
        }
        if let Some(stream) = opt_str(row, "stream")
            && !VALID_STREAMS.contains(&stream)
        {
            errors.push(format!("{tid}: invalid stream '{stream}'"));
        }
        if let Some(plan) = row.get("slice_plan").and_then(|p| p.as_object()) {
            for (sid, meta) in plan {
                if let Some(targets) = meta.get("targets").and_then(|t| t.as_array()) {
                    for tgt in targets {
                        if let Some(s) = tgt.as_str()
                            && !VALID_TARGETS.contains(&s)
                        {
                            errors.push(format!("{tid} slice {sid}: invalid target '{s}'"));
                        }
                    }
                }
                let ex_ok = meta
                    .get("executor")
                    .and_then(|e| e.as_str())
                    .map(|e| VALID_EXECUTORS.contains(&e))
                    .unwrap_or(false);
                if !ex_ok {
                    errors.push(format!("{tid} slice {sid}: invalid executor"));
                }
            }
        }
    }

    for tid in FORBIDDEN_PHANTOM_IDS {
        if ticket_by_id(registry, tid).is_some() {
            errors.push(format!("Forbidden phantom ticket row: {tid}"));
        }
    }

    for row in tickets(registry) {
        let tid = str_field(row, "id");
        let spec = opt_str(row, "spec").unwrap_or("").trim().to_string();
        let status = opt_str(row, "status").unwrap_or("");
        if !spec.is_empty()
            && status != "idea"
            && status != "cancelled"
            && !root.join(&spec).is_file()
        {
            errors.push(format!("{tid}: spec missing on disk: {spec}"));
        }
    }

    let roadmap = root.join("docs/specs/Mission_Creator_Architecture/ROADMAP.md");
    if roadmap.is_file() {
        let text = fs::read_to_string(&roadmap).unwrap_or_default();
        if !text.contains(NEXT_MARKER_START) || !text.contains(NEXT_MARKER_END) {
            let rel = roadmap.strip_prefix(root).unwrap_or(&roadmap);
            errors.push(format!("Missing markers in {}", rel.display()));
        }
    }

    if let Err(e) = test_gap_analysis_round_trip(root) {
        errors.push(e.to_string());
    }

    if strict {
        let hits = scan_legacy_ids(root);
        for (path, matches) in hits {
            errors.push(format!("Legacy ID in {path}: {} match(es)", matches.len()));
        }
        let gap = gap_analysis_path(root);
        if gap.is_file() {
            let text = fs::read_to_string(&gap).unwrap_or_default();
            if text.contains("| priority |") || PRIORITY_P.is_match(&text) {
                errors.push("gap_analysis still has priority column or numbered P backlog".into());
            }
        }
    }

    errors
}
