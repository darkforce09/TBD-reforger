use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::check::require_check_ok;
use crate::gap::test_gap_analysis_round_trip;
use crate::prompt::extract_prompt;
use crate::registry::*;
use crate::sync::{cmd_sync, generate_queue_json, refuse_empty_write};

/// Ticket status enum — mirrors `.ai/tickets/schema.json` `$defs.status` (T-383).
const VALID_TICKET_STATUSES: &[&str] = &[
    "idea",
    "queued",
    "ready",
    "running",
    "review",
    "shipped",
    "deferred",
    "cancelled",
];

pub fn cmd_brief(_root: &Path, registry: &Value, id: &str) -> Result<()> {
    let t = require_ticket(registry, id);
    let tid = str_field(t, "id");
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{tid}"));
    let active = opt_str(t, "active_slice").unwrap_or("").to_string();
    let spec = slice_spec(t);
    let shipped = shipped_slices(t);
    println!("{tid} · {}", opt_str(t, "title").unwrap_or(""));
    if !active.is_empty() {
        println!("SLICE: {active}");
    }
    println!("READ: {spec} (slice spec — only source of truth for this slice)");
    if let Some(hub) = opt_str(t, "spec") {
        if hub != spec {
            println!("HUB: {hub} (program context only)");
        }
    }
    println!("BRANCH: {branch}");
    println!(
        "EXECUTION: Default ship on main. Parallel tickets use worktree .ai/artifacts/worktrees/TBD-{tid} @ {branch} (merge to main when done). Docs-only slices (cursor-docs) may commit on main. See .ai/tickets/README.md."
    );
    println!("TARGETS: {}", slice_targets(t).join(", "));
    println!("DO NOT: edit documentation");
    if !shipped.is_empty() {
        println!("DO NOT REOPEN (shipped): {}", shipped.join(", "));
    }

    match (active.as_str(), tid.as_str()) {
        ("T-090.1.2.2", _) => {
            println!(
                "SCOPE: SAP cell seam repair — analyze 256 m grid edges, feather/blend in stitch-sap-ortho.mjs, rebuild lossless z0–6 pyramid"
            );
            println!(
                "DO NOT REOPEN: T-090.1.2 decode contract, T-090.1.2.1 lossless pyramid encode (reuse --lossless rebuild)"
            );
            println!(
                "PREFLIGHT: git lfs pull && make map-assets-link && cargo run -q -p xtask -- ticket brief T-090"
            );
            println!("HANDOFF: .ai/artifacts/t090_1_2_2_claude_code_handoff.md");
            println!(
                "VERIFY: analyze-sap-seams + verify-sap-seams + verify-sap-ortho + EXPECT_LOSSLESS=1 verify-tile-pyramid + cargo xtask ci verify-terrain"
            );
            println!("MANUAL: S1 operator seam location invisible at max zoom");
        }
        ("T-090.1.2.3", _) => {
            println!(
                "SCOPE: basemap tile prefetch + cache — fix pan ~40 fps flicker; useTerrainBasemapLayer.ts (+ basemapTileCache.ts)"
            );
            println!("PARALLEL: frontend only — safe alongside T-090.1.2.2");
            println!("HANDOFF: .ai/artifacts/t090_1_2_3_claude_code_handoff.md");
            println!(
                "RESUME: docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md"
            );
            println!("VERIFY: cargo xtask mk ci-local-leptos");
            println!("MANUAL: P1 no pop-in; P2 pan fps ≥55");
        }
        ("T-090.1.2.5", _) => {
            println!(
                "SCOPE: satellite water — ocean + inland on SAP ortho; P0 mask spike, composite-water-ortho.mjs, lossless pyramid rebuild"
            );
            println!("DEPENDS: run after T-090.1.2.2 seam ortho when possible");
            println!("HANDOFF: .ai/artifacts/t090_1_2_5_claude_code_handoff.md");
            println!(
                "RESUME: docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md"
            );
            println!(
                "VERIFY: water spike + verify-sap-ortho + EXPECT_LOSSLESS=1 verify-tile-pyramid"
            );
            println!("MANUAL: W1 coast water; W2 inland lakes/rivers");
        }
        ("T-090.1.2.1", _) => {
            println!(
                "SCOPE: lossless WebP z0–6 pyramid from staged SAP ortho — build-tile-pyramid.sh --lossless, verify VP8L, manifest maxZoom 6"
            );
            println!(
                "DO NOT REOPEN: T-090.1.2 decode/stitch/orientation (shipped @ c2730a3) unless verify-sap-ortho fails"
            );
            println!(
                "ORTH: packages/map-assets/everon/staging/sap/everon-sap-ortho.png (12800² — already built; do NOT re-stitch)"
            );
            println!(
                "PREFLIGHT: git lfs pull && make map-assets-link && cargo run -q -p xtask -- ticket brief T-090"
            );
            println!("HANDOFF: .ai/artifacts/t090_1_2_1_claude_code_handoff.md");
            println!(
                "VERIFY: node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon && EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon && cargo xtask ci verify-terrain && cargo xtask mk ci-local-leptos"
            );
            println!(
                "MANUAL: L1 max-zoom field/road pixel-sharp; L2 north-up; L3 alignment; L4 ≥55 fps"
            );
        }
        (_, "T-122") => {
            println!(
                "SCOPE: ALL findings in docs/platform/CODEBASE_AUDIT_2026.md (C/R/T/M/D) — one branch"
            );
            println!(
                "MAY EDIT: docs/platform/CODEBASE_AUDIT_2026.md (append shipped SHA under §Verification)"
            );
            println!("DO NOT: edit registry or other docs");
            println!("VERIFY: cargo xtask db test-it && cargo xtask mk ci-local-leptos");
        }
        (_, "T-123") => {
            println!(
                "AUTHORITY: docs/platform/DOCUMENTATION_STANDARDS.md (normative — already written)"
            );
            println!("SCOPE: roll out in-code @contract/@route/@model + Godoc/TSDoc/Enfusion tags");
            println!("OUT OF SCOPE: markdownlint only");
            println!(
                "SLICES: .0 doc hub → .1 Go → .2 TS → .3 Enfusion → .4 codegen → .5 Go JSON validation → .6 CI"
            );
            println!("SPEC: docs/platform/t123_documentation_standards_rollout.md");
        }
        (_, "T-124") => {
            println!("STATUS: shipped @ cd11db0 — historical replay only");
            println!("SPEC: docs/platform/t124_dependency_upgrade.md");
            println!("DO NOT REOPEN unless dependency regression");
        }
        ("T-124.1", _) => println!("SHIPPED @ 1d85f46 — do not reopen"),
        ("T-124.2", _) => println!("SHIPPED @ d81ed9c — do not reopen"),
        ("T-124.3", _) => println!("SHIPPED @ cd11db0 — do not reopen"),
        (_, "T-125") => {
            println!("AUTHORITY: new CODING_STANDARDS.md (T-125.0) + DOCUMENTATION_STANDARDS.md");
            println!(
                "SCOPE: full CI gate, golangci full set, TS strict, @route completion, error policy"
            );
            println!("PREREQ: T-124 shipped (met @ cd11db0)");
            println!("SPEC: docs/platform/t125_coding_standards_enforcement.md");
        }
        ("T-125.0", _) => println!(
            "SCOPE: write docs/platform/CODING_STANDARDS.md — style/structure/errors/tests"
        ),
        ("T-125.1", _) => {
            println!(
                "SCOPE: .github/workflows/ci.yml + cargo xtask ci ci-local; Postgres 18 service"
            )
        }
        ("T-125.2", _) => println!(
            "SCOPE: golangci errcheck/govet/staticcheck; remove only-new-issues; fix all Go lint"
        ),
        ("T-125.3", _) => {
            println!("SCOPE: tsconfig strict:true + eslint @contract/@model enforcement + fixes")
        }
        ("T-125.4", _) => println!(
            "SCOPE: @route on all handlers; error-handling; Enfusion DTO fixture gate in validate.mjs"
        ),
        ("T-125.5", _) => println!("SCOPE: .editorconfig + optional Prettier"),
        ("T-125.6", _) => {
            println!(
                "EXECUTOR: cursor-docs — registry shipped, hub links, CLAUDE §Done, ticket sync"
            );
            println!("DO NOT: Claude executes this slice");
        }
        ("T-123.0", _) => {
            println!(
                "SCOPE: AGENT_COMMIT_CHECKLIST link, platform README, handoff artifact — docs only"
            );
            println!("DO NOT: edit apps/website/, apps/mod/, packages/tbd-schema/ source");
            println!(
                "VERIFY: cargo run -q -p xtask -- ticket sync && cargo run -q -p xtask -- ticket check --strict"
            );
        }
        ("T-123.1", _) => {
            println!(
                "SCOPE: Go internal/models + handlers — Godoc + @contract/@route on cross-boundary symbols"
            );
            println!("FIX: schemaVersion int drift → string per DOCUMENTATION_STANDARDS §2.2");
            println!("DO NOT: edit docs/ or registry");
            println!("VERIFY: cargo xtask db test-it && go build ./...");
        }
        ("T-123.2", _) => {
            println!(
                "SCOPE: frontend tsdoc.json + TSDoc on types/api/hooks + @model/@contract/@route"
            );
            println!("NOTE: eslint jsdoc CI lands in T-123.6 — add tags here first");
            println!("VERIFY: cargo xtask mk ci-local-leptos");
        }
        ("T-123.4", _) => {
            println!(
                "SCOPE: schema codegen — internal/contract/ + frontend/src/types/contract/ + regen script"
            );
            println!("SCHEMAS: registry-items, loadout-export, mission export defs first");
            println!("VERIFY: cargo xtask ci schema-validate && cargo xtask db test-it");
        }
        ("T-123.5", _) => {
            println!("SCOPE: CreateVersion validates against mission.schema.json before persist");
            println!("LIB: santhosh-tekuri/jsonschema or equivalent; 400 on invalid payload");
            println!("VERIFY: cargo xtask db test-it (golden pass + invalid fixture fail cases)");
        }
        ("T-123.6", _) => {
            println!(
                "SCOPE: CI — revive exported, eslint jsdoc, verify-contract-citations.mjs, schema.yml"
            );
            println!("VERIFY: local golangci-lint + FE lint + citation script exit 0");
        }
        ("T-123.3", _) => {
            println!(
                "SCOPE: Enfusion Backend/Gamemode — //! headers, DTO field docs, @authority/@rpc/@replicated"
            );
            println!("PREFLIGHT: enfusion-mcp before any .c edit");
            println!("VERIFY: Workbench compile on touched scripts (human note)");
        }
        ("T-090.1", _) => {
            println!(
                "SCOPE: aligned WebP tile basemap — TileLayer / manifest tiles[]; see t090_1_aligned_basemap.md"
            );
            println!(
                "DO NOT REOPEN: T-091 dem/* + ydoc Z wiring (shipped @ dde589e) unless regression"
            );
            println!(
                "PREFLIGHT: make map-assets-link && cargo run -q -p xtask -- ticket brief T-090"
            );
        }
        ("T-091.2", _) => {
            println!(
                "DO NOT REOPEN: T-091.0 plugin/export, T-091.1 dem/* loader (shipped @ 2c56c2e) unless regression fix"
            );
            println!(
                "SCOPE: ydoc z sample (addSlot/pasteSlots/moveEntities/updateSlotPosition X/Y), TacticalMap CUR z, BottomToolbelt 3dp Z, useDemLayer hillshade (BitmapLayer ≤1024px), MissionSettings toggles, meta.environment showGrid/showHillshade"
            );
            println!(
                "CONSUME: sampleElevation/isDemReady/isDemDegraded from tactical-map/dem — do not redo loader"
            );
            println!(
                "PREFLIGHT: cargo xtask ci lfs-dem && cargo run -q -p xtask -- ticket brief T-091"
            );
            println!(
                "VERIFY: cargo xtask mk ci-local-leptos && cargo xtask ci verify-terrain-strict"
            );
            println!(
                "MANUAL: M1 CUR Z >5m; M3 Save z=123.456; M5/M6 toggles; M7 degraded; M8 Attributes X→Z re-sample"
            );
        }
        ("T-091.1", _) => {
            println!(
                "DO NOT: TBD_TerrainExportPlugin.c, Workbench, MCP terrain export, re-export everon-dem-16bit.png, anchor probes, or packages/map-assets/ edits"
            );
            println!(
                "SCOPE (React-era, shipped; app retired at T-159.29.3): tactical-map/dem/* + DemController wiring"
            );
            println!(
                "REFERENCE (port, do not re-run): packages/tbd-schema/scripts/lib/dem-sample.mjs"
            );
            println!(
                "PREFLIGHT: cargo xtask ci lfs-dem && cargo run -q -p xtask -- ticket brief T-091"
            );
            println!(
                "VERIFY: cargo xtask mk ci-local-leptos && cargo xtask ci verify-terrain-strict"
            );
        }
        _ => {
            println!("VERIFY: cargo xtask mk ci-local-leptos");
        }
    }

    if let Some(acc) = t.get("acceptance").and_then(|a| a.as_array()) {
        println!("ACCEPTANCE:");
        for a in acc {
            if let Some(s) = a.as_str() {
                println!("  - {s}");
            }
        }
    }
    Ok(())
}

pub fn unknown_ticket(id: &str) -> ! {
    eprintln!("Unknown ticket: {id}");
    std::process::exit(1);
}

pub fn require_ticket<'a>(registry: &'a Value, id: &str) -> &'a Value {
    match ticket_by_id(registry, id) {
        Some(t) => t,
        None => unknown_ticket(id),
    }
}

pub fn cmd_show(registry: &Value, id: &str) -> Result<()> {
    let t = require_ticket(registry, id);
    let surfaces = string_list(t, "surfaces").unwrap_or_default().join(", ");
    let impact = string_list(t, "impact").unwrap_or_default().join(", ");
    println!(
        "### {} · {}",
        str_field(t, "id"),
        opt_str(t, "title").unwrap_or("")
    );
    println!(
        "**Program:** {} · **Where:** {surfaces}",
        opt_str(t, "program").unwrap_or("")
    );
    if let Some(route) = opt_str(t, "route") {
        println!("**Route:** {route}");
    }
    println!(
        "**Impact:** {impact} · **Status:** {} · **Order:** {}",
        opt_str(t, "status").unwrap_or(""),
        match t.get("order") {
            Some(v) if !matches!(v, Value::Null) => {
                if let Some(n) = v.as_i64() {
                    n.to_string()
                } else {
                    v.to_string()
                }
            }
            _ => "—".into(),
        }
    );
    println!("**Summary:** {}", opt_str(t, "summary").unwrap_or(""));
    if let Some(deps) = string_list(t, "depends_on") {
        if !deps.is_empty() {
            println!("**Needs:** {}", deps.join(", "));
        }
    }
    if let Some(unblocks) = string_list(t, "unblocks") {
        if !unblocks.is_empty() {
            println!("**Blocks:** {}", unblocks.join(", "));
        }
    }
    if let Some(spec) = opt_str(t, "spec") {
        println!("**Spec:** `{spec}`");
    }
    Ok(())
}

pub fn cmd_next(registry: &Value) -> Result<()> {
    if let Some(slice_row) = tickets(registry).iter().find(|t| {
        t.get("active_slice")
            .map(|v| is_truthy(Some(v)))
            .unwrap_or(false)
    }) {
        println!(
            "ACTIVE: {} slice {}",
            str_field(slice_row, "id"),
            opt_str(slice_row, "active_slice").unwrap_or("")
        );
    }
    let mut open_t: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| matches!(opt_str(t, "status"), Some("ready" | "queued")) && order_truthy(t))
        .collect();
    open_t.sort_by_key(|t| ticket_sort_key(t));
    for t in open_t.into_iter().take(5) {
        println!(
            "  {} — {} ({})",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or("")
        );
    }
    Ok(())
}

pub fn cmd_prompt(
    root: &Path,
    registry: &Value,
    id: &str,
    slice: Option<&str>,
    header: bool,
) -> Result<()> {
    let t = require_ticket(registry, id);
    let slice_id = slice
        .map(|s| s.to_string())
        .or_else(|| opt_str(t, "active_slice").map(|s| s.to_string()));
    let plan = t.get("slice_plan").and_then(|p| p.as_object());
    let spec_rel = if let Some(s) = slice {
        let plan = plan.with_context(|| format!("Unknown slice {s} on {id}"))?;
        if !plan.contains_key(s) {
            eprintln!("Unknown slice {s} on {id}");
            std::process::exit(1);
        }
        plan.get(s)
            .and_then(|r| r.get("spec"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        slice_spec(t)
    };
    if spec_rel.is_empty() {
        let sid = slice_id
            .as_deref()
            .map(|s| format!(" slice {s}"))
            .unwrap_or_default();
        eprintln!("No spec for {id}{sid}");
        std::process::exit(1);
    }
    let spec_path = root.join(&spec_rel);
    if !spec_path.is_file() {
        eprintln!("Spec not found: {spec_rel}");
        std::process::exit(1);
    }
    let text = fs::read_to_string(&spec_path)?;
    let prompt = match extract_prompt(&text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    if header {
        let handoff = slice_handoff_path(t, slice_id.as_deref());
        let label = slice_id.unwrap_or_else(|| id.to_string());
        println!("# Prompt for {label} — from {spec_rel}");
        println!("# Handoff: {handoff}");
        println!();
    }
    println!("{prompt}");
    Ok(())
}

pub fn cmd_list(root: &Path, registry: &Value) -> Result<()> {
    let queue_path = root.join(".ai/tickets/queue.json");
    let data = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let batch = data
        .get("batch_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(10);
    let conc = data
        .get("concurrency")
        .and_then(|v| v.as_i64())
        .unwrap_or(3);
    println!("batch_size={batch} concurrency={conc}");
    println!("{:<8} {:<10} {:<50} TITLE", "ID", "STATUS", "SPEC");
    println!("{}", "-".repeat(100));
    if let Some(arr) = data.get("tickets").and_then(|t| t.as_array()) {
        for t in arr {
            let id = opt_str(t, "id").unwrap_or("");
            let status = opt_str(t, "status").unwrap_or("");
            let spec = opt_str(t, "spec").unwrap_or("");
            let spec_trunc: String = spec.chars().take(48).collect();
            let title = opt_str(t, "title").unwrap_or("");
            println!("{id:<8} {status:<10} {spec_trunc:<50} {title}");
        }
    }
    Ok(())
}

pub fn cmd_milestone(registry: &Value, milestone: &str) -> Result<()> {
    let milestone = milestone.to_uppercase();
    let mut rows: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| opt_str(t, "milestone") == Some(milestone.as_str()))
        .collect();
    rows.sort_by_key(|t| ticket_sort_key(t));
    if rows.is_empty() {
        println!("No tickets tagged milestone={milestone}");
        return Ok(());
    }
    let shipped = rows
        .iter()
        .filter(|t| opt_str(t, "status") == Some("shipped"))
        .count();
    println!("## Milestone {milestone}: {shipped}/{} shipped", rows.len());
    for t in rows {
        println!(
            "  [{:<8}] {} — {}",
            opt_str(t, "status").unwrap_or(""),
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or("")
        );
    }
    Ok(())
}

pub fn cmd_plan_batch(registry: &Value) -> Result<()> {
    let mut queued: Vec<&Value> = tickets(registry)
        .iter()
        .filter(|t| matches!(opt_str(t, "status"), Some("queued" | "ready")) && order_truthy(t))
        .collect();
    queued.sort_by_key(|t| ticket_sort_key(t));
    println!("Next batch candidates (top 10 by order):");
    for t in queued.into_iter().take(10) {
        let spec = opt_str(t, "spec").unwrap_or("(no spec yet)");
        println!(
            "  {} — {} [{}] — {spec}",
            str_field(t, "id"),
            opt_str(t, "title").unwrap_or(""),
            opt_str(t, "status").unwrap_or("")
        );
    }
    Ok(())
}

pub fn cmd_sparse_paths(registry: &Value, id: &str) -> Result<()> {
    let t = require_ticket(registry, id);
    let mut paths = std::collections::BTreeSet::new();
    paths.insert(".github".to_string());
    for tgt in slice_targets(t) {
        match tgt.as_str() {
            "website" => {
                paths.insert("apps/website".into());
            }
            "mod" => {
                paths.insert("apps/mod".into());
            }
            "shared" => {
                paths.insert("packages/tbd-schema".into());
            }
            "root" => {
                // `Makefile` sat in this list until T-897 deleted it. Its successor is `xtask/`:
                // a root slice needs that checked out or `cargo xtask` cannot build, which is
                // now the whole task surface rather than a 504-line file at the top level.
                for p in [
                    "scripts",
                    ".ai/tickets",
                    "docs",
                    ".ai/artifacts",
                    "xtask",
                    "README.md",
                    "CLAUDE.md",
                ] {
                    paths.insert(p.into());
                }
            }
            _ => {}
        }
    }
    for p in paths {
        println!("{p}");
    }
    Ok(())
}

pub fn cmd_gap_round_trip(root: &Path) -> Result<()> {
    test_gap_analysis_round_trip(root)?;
    println!("round-trip OK");
    Ok(())
}

/// T-912.2 lifecycle hook: every registry STATUS writer refreshes the committed wave.lock with
/// the one legal writer, so a bookkeeping ship/cancel never leaves `wave check` red on a
/// correct registry. The refresh rides whatever commit carries the status change — statuses and
/// the lock are working-tree writes the operator commits together.
fn refresh_wave_lock(root: &Path) -> Result<()> {
    crate::wave_lock::repack_quiet(root)
        .map(|_| ())
        .context("refresh wave.lock after status write (`cargo xtask wave repack`)")
}

pub fn cmd_ship(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    // T-237: refuse to mark shipped when the registry fails ticket check
    // (including Draft 2020-12 .ai/tickets/schema.json). Check runs first so a
    // red registry never gets a status write + sync.
    let _ = require_ticket(registry, id);
    require_check_ok(root, registry, &format!("ship {id}"))?;

    let t = ticket_by_id_mut(registry, id).unwrap_or_else(|| unknown_ticket(id));
    if let Some(obj) = t.as_object_mut() {
        obj.insert("status".into(), json!("shipped"));
        // T-913.1: completed_at rides the same mutation as the status write (and lands
        // before refresh_wave_lock — timestamps are not lock fields, so the lock bytes
        // only change if the STATUS change moves waves). `shipped_at` stays a bare SHA.
        obj.insert("completed_at".into(), json!(tbd_tickets::now_utc_rfc3339()));
        obj.remove("active");
        obj.remove("active_slice");
    }
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    refresh_wave_lock(root)?;
    println!("{id} -> shipped");
    Ok(())
}

pub fn cmd_mark_ready(
    root: &Path,
    registry: &mut Value,
    id: &str,
    spec_arg: Option<&str>,
) -> Result<()> {
    // T-451: refuse ready promotion when the registry fails ticket check.
    let _ = require_ticket(registry, id);
    require_check_ok(root, registry, &format!("mark-ready {id}"))?;
    {
        let t = ticket_by_id_mut(registry, id).unwrap_or_else(|| unknown_ticket(id));
        if let Some(s) = spec_arg {
            if !s.is_empty() {
                if let Some(obj) = t.as_object_mut() {
                    obj.insert("spec".into(), json!(s));
                }
            }
        }
    }
    let t = require_ticket(registry, id);
    let spec = opt_str(t, "spec").unwrap_or("").trim().to_string();
    if spec.is_empty() {
        eprintln!("Ticket {id} needs a spec path");
        std::process::exit(1);
    }
    if !root.join(&spec).is_file() {
        eprintln!("Spec file not found: {}", root.join(&spec).display());
        std::process::exit(1);
    }
    if let Some(deps) = string_list(t, "depends_on") {
        for dep in deps {
            if let Some(dep_row) = ticket_by_id(registry, &dep) {
                let st = opt_str(dep_row, "status").unwrap_or("");
                if st != "shipped" && st != "cancelled" {
                    eprintln!("Blocked by {dep} (status={st})");
                    std::process::exit(1);
                }
            }
        }
    }
    if let Some(t) = ticket_by_id_mut(registry, id) {
        if let Some(obj) = t.as_object_mut() {
            obj.insert("status".into(), json!("ready"));
            let story_empty = match obj.get("user_story") {
                Some(Value::String(s)) => s.trim().is_empty(),
                _ => true,
            };
            if story_empty {
                let fallback = obj
                    .get("summary")
                    .and_then(Value::as_str)
                    .or_else(|| obj.get("title").and_then(Value::as_str))
                    .unwrap_or(id)
                    .to_string();
                obj.insert("user_story".into(), json!(fallback));
            }
            let acc_empty = match obj.get("acceptance") {
                Some(Value::Array(a)) => a
                    .iter()
                    .all(|s| s.as_str().is_none_or(|x| x.trim().is_empty())),
                _ => true,
            };
            if acc_empty {
                obj.insert("acceptance".into(), json!(["See spec."]));
            }
        }
    }
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} -> ready ({spec})");
    Ok(())
}

pub fn cmd_add(
    root: &Path,
    registry: &mut Value,
    title: &str,
    program: &str,
    surfaces: &str,
    impact: &str,
    summary: &str,
) -> Result<()> {
    // T-455: refuse insert when the registry fails ticket check (same bar as
    // set-status/mark-ready/reorder/ship — T-451 / T-237). Check runs first so a
    // red registry never gets a next_id bump + row write + sync.
    require_check_ok(root, registry, "add")?;

    let next_id = registry
        .get("next_id")
        .and_then(|n| n.as_u64())
        .unwrap_or(1);
    let tid = format!("T-{:03}", next_id);
    if let Some(obj) = registry.as_object_mut() {
        obj.insert("next_id".into(), json!(next_id + 1));
    }
    let _ = (program, surfaces, impact);
    // T-913.1: every minted ticket gets its birth stamp (RFC 3339 UTC). Existing tickets
    // get NO backfill — only `ticket add` writes created_at.
    let row = json!({
        "id": tid,
        "kind": "work",
        "title": title,
        "summary": if summary.is_empty() { title } else { summary },
        "status": "idea",
        "created_at": tbd_tickets::now_utc_rfc3339(),
        "scope": { "repo": { "layers": ["docs"] } },
    });
    tickets_mut(registry)?.push(row);
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Added {tid}: {title}");
    Ok(())
}

pub fn cmd_remove(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    // T-455: refuse delete when the registry fails ticket check (same bar as
    // add / set-status — T-451). Check runs first so a red registry never loses
    // a row on disk.
    let _ = require_ticket(registry, id);
    require_check_ok(root, registry, &format!("remove {id}"))?;

    let before = tickets(registry).len();
    let list = tickets_mut(registry)?;
    list.retain(|t| opt_str(t, "id") != Some(id));
    if list.len() == before {
        unknown_ticket(id);
    }
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("Removed {id}");
    Ok(())
}

pub fn cmd_reorder(root: &Path, registry: &mut Value, id: &str, after: &str) -> Result<()> {
    // T-451: reorder may flip idea→queued; refuse when check is red.
    let _ = require_ticket(registry, id);
    require_check_ok(root, registry, &format!("reorder {id}"))?;

    let anchor_order = {
        let anchor = ticket_by_id(registry, after);
        match anchor {
            Some(a) if a.get("order").is_some() && !matches!(a.get("order"), Some(Value::Null)) => {
                order_or(a, 0)
            }
            _ => {
                eprintln!("Unknown anchor ticket: {after}");
                std::process::exit(1);
            }
        }
    };
    let was_idea = opt_str(require_ticket(registry, id), "status") == Some("idea");
    let t = ticket_by_id_mut(registry, id).unwrap_or_else(|| unknown_ticket(id));
    let new_order = anchor_order + 1;
    if let Some(obj) = t.as_object_mut() {
        obj.insert("order".into(), json!(new_order));
        if was_idea {
            obj.insert("status".into(), json!("queued"));
        }
    }
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} order -> {new_order} (after {after})");
    Ok(())
}

pub fn cmd_advance_slice(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    // T-459: refuse advance when the registry fails ticket check (same bar as
    // add/remove/set-status/mark-ready/reorder/ship — T-455 / T-451 / T-237).
    // Check runs first so a red registry never gets an active_slice write + sync.
    require_check_ok(root, registry, &format!("advance-slice {id}"))?;

    let (slices, active) = {
        let t = require_ticket(registry, id);
        let slices: Vec<String> = t
            .get("slices")
            .and_then(|s| s.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let active = opt_str(t, "active_slice").map(|s| s.to_string());
        (slices, active)
    };
    if slices.is_empty() {
        eprintln!("{id} has no slices[]");
        std::process::exit(1);
    }
    let new_active = if active.is_none() {
        slices[0].clone()
    } else {
        let a = active.unwrap();
        let idx = match slices.iter().position(|s| s == &a) {
            Some(i) => i,
            None => {
                eprintln!("active_slice {a} not in slices[]");
                std::process::exit(1);
            }
        };
        if idx + 1 >= slices.len() {
            eprintln!("{id}: no slice after {a}");
            std::process::exit(1);
        }
        slices[idx + 1].clone()
    };
    if let Some(t) = ticket_by_id_mut(registry, id) {
        if let Some(obj) = t.as_object_mut() {
            obj.insert("active".into(), json!(new_active));
            obj.insert("active_slice".into(), json!(new_active));
        }
    }
    save_registry(root, registry)?;
    cmd_sync(root, registry)?;
    println!("{id} active_slice -> {new_active}");
    Ok(())
}

pub fn cmd_ready_ids(
    root: &Path,
    registry: &Value,
    limit: Option<usize>,
    stream: Option<&str>,
) -> Result<()> {
    let queue_path = root.join(".ai/tickets/queue.json");
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
            if let Some(s) = stream {
                if !s.is_empty() && opt_str(row, "stream") != Some(s) {
                    continue;
                }
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
    // T-383: reject empty / invalid status before any write — never stamp `""` over registry.
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

    // T-451: refuse status writes when the registry fails ticket check
    // (same bar as ship/done — T-237). No silent escape hatch; a red registry
    // must be fixed before any status mutator may write.
    let _ = require_ticket(registry, id);
    require_check_ok(root, registry, &format!("set-status {id}"))?;

    let t = ticket_by_id_mut(registry, id).unwrap_or_else(|| unknown_ticket(id));
    if let Some(obj) = t.as_object_mut() {
        obj.insert("status".into(), json!(status));
        // T-913.1: a cancel is a completion — stamp it in the same mutation as the status
        // write, before the wave-lock refresh below. Other set-status targets do not
        // stamp; `ticket ship` / `ticket done` own the shipped stamp.
        if status == "cancelled" {
            obj.insert("completed_at".into(), json!(tbd_tickets::now_utc_rfc3339()));
        }
    }
    save_registry(root, registry)?;
    let queue = generate_queue_json(registry);
    write_json_ascii(&root.join(".ai/tickets/queue.json"), &queue)?;
    // T-912.2: `set-status cancelled` (and `shipped`) must repack or `wave check` goes red on a
    // correct registry. Run for EVERY status — a demotion out of the dispatchable set (queued →
    // idea/deferred) strands the id in the lock's open waves just as surely as a cancel, and a
    // dispatchability-neutral write recompiles to the identical bytes.
    refresh_wave_lock(root)?;
    Ok(())
}

pub fn cmd_get(registry: &Value, id: &str, field: Option<&str>) -> Result<()> {
    let t = require_ticket(registry, id);
    if let Some(field) = field {
        let mut val = t.get(field).cloned().unwrap_or(json!(""));
        if field == "branch" && (val.is_null() || val == json!("") || val == json!(null)) {
            val = json!(format!("ticket/{id}"));
        }
        match val {
            Value::String(s) => println!("{s}"),
            Value::Null => println!(),
            other => {
                if let Some(s) = other.as_str() {
                    println!("{s}");
                } else {
                    println!("{other}");
                }
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(t)?);
    }
    Ok(())
}

pub fn cmd_config(root: &Path, registry: &Value, key: &str) -> Result<()> {
    let queue_path = root.join(".ai/tickets/queue.json");
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let defaults = [
        ("batch_size", "10"),
        ("concurrency", "3"),
        ("worktree_base", ".ai/artifacts/worktrees"),
        ("git_base", "main"),
    ];
    if let Some(v) = data.get(key) {
        match v {
            Value::String(s) => println!("{s}"),
            Value::Number(n) => println!("{n}"),
            other => println!("{other}"),
        }
    } else {
        let d = defaults
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| *v)
            .unwrap_or("");
        println!("{d}");
    }
    Ok(())
}

pub fn cmd_clean(root: &Path, registry: &Value, id: &str) -> Result<()> {
    let t = require_ticket(registry, id);
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{id}"));
    // resolve worktree base
    let queue_path = root.join(".ai/tickets/queue.json");
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path)?)?
    } else {
        generate_queue_json(registry)
    };
    let base = data
        .get("worktree_base")
        .and_then(|v| v.as_str())
        .unwrap_or(".ai/artifacts/worktrees");
    let wt = if Path::new(base).is_absolute() {
        Path::new(base).join(format!("TBD-{id}"))
    } else {
        root.join(base).join(format!("TBD-{id}"))
    };
    if wt.is_dir() {
        let status = Command::new("git")
            .args(["worktree", "remove", "--force"])
            .arg(&wt)
            .current_dir(root)
            .status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            let _ = fs::remove_dir_all(&wt);
        }
        println!("Removed worktree {}", wt.display());
    }
    let check = Command::new("git")
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{branch}"))
        .current_dir(root)
        .status()?;
    if check.success() {
        Command::new("git")
            .args(["branch", "-D"])
            .arg(&branch)
            .current_dir(root)
            .status()?;
        println!("Deleted local branch {branch}");
    }
    Ok(())
}

pub fn cmd_done(root: &Path, registry: &mut Value, id: &str) -> Result<()> {
    cmd_clean(root, registry, id)?;
    cmd_ship(root, registry, id)?;
    Ok(())
}

pub fn cmd_run(root: &Path, registry: &Value, dry_run: bool, stream: Option<&str>) -> Result<()> {
    // Port of bash cmd_run — invoke cargo xtask for sub-ops
    let conc: usize = {
        cmd_config_value(root, registry, "concurrency")
            .parse()
            .unwrap_or(3)
    };
    let batch: usize = cmd_config_value(root, registry, "batch_size")
        .parse()
        .unwrap_or(10);
    let mut ready = vec![];
    // replicate ready-ids
    {
        let queue_path = root.join(".ai/tickets/queue.json");
        let data: Value = if queue_path.is_file() {
            serde_json::from_str(&fs::read_to_string(&queue_path)?)?
        } else {
            generate_queue_json(registry)
        };
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
                if let Some(s) = stream {
                    if !s.is_empty() && opt_str(row, "stream") != Some(s) {
                        continue;
                    }
                }
                ready.push(tid.to_string());
                if ready.len() >= batch {
                    break;
                }
            }
        }
    }
    if ready.is_empty() {
        eprintln!("No ready tickets. Steps:");
        eprintln!("  1. Composer 2.5: write specs for next batch, commit to main");
        eprintln!("  2. cargo run -q -p xtask -- ticket mark-ready T-0xx path/to/spec.md");
        std::process::exit(1);
    }
    println!(
        "Running {} ticket(s), concurrency={conc} (dry_run={})",
        ready.len(),
        if dry_run { 1 } else { 0 }
    );
    // Sequential for Rust port (bash used parallel jobs). Document in verify.
    for id in &ready {
        run_one(root, registry, id, dry_run)?;
    }
    println!("Batch run finished. cargo run -q -p xtask -- ticket list");
    Ok(())
}

fn cmd_config_value(root: &Path, registry: &Value, key: &str) -> String {
    let queue_path = root.join(".ai/tickets/queue.json");
    let data: Value = if queue_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&queue_path).unwrap_or_default())
            .unwrap_or(json!({}))
    } else {
        generate_queue_json(registry)
    };
    if let Some(v) = data.get(key) {
        return match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            other => other.to_string(),
        };
    }
    match key {
        "batch_size" => "10".into(),
        "concurrency" => "3".into(),
        "worktree_base" => ".ai/artifacts/worktrees".into(),
        "git_base" => "main".into(),
        _ => "".into(),
    }
}

fn run_one(root: &Path, registry: &Value, id: &str, dry_run: bool) -> Result<()> {
    let t = require_ticket(registry, id);
    let spec = slice_spec(t);
    let branch = opt_str(t, "branch")
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("ticket/{id}"));
    let executor = slice_executor(t);
    if executor != "claude-code" {
        eprintln!("[{id}] SKIP — executor is {executor} (not claude-code)");
        return Ok(());
    }
    if spec.is_empty() || !root.join(&spec).is_file() {
        eprintln!("[{id}] SKIP — spec missing: {spec}");
        return Ok(());
    }
    println!("[{id}] branch={branch} spec={spec} dry_run={dry_run}");
    if dry_run {
        return Ok(());
    }
    // Full Claude Code invoke is environment-specific; mark running + note
    eprintln!(
        "[{id}] run: invoke Claude Code manually / ticket pipeline (xtask run is scaffolding)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::require_check_ok;
    use crate::registry::load_registry;
    use serde_json::json;
    use std::path::PathBuf;

    fn worktree_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask parent = repo/worktree root")
            .to_path_buf()
    }

    /// Break a required enum so schema check goes red (in-memory only).
    fn red_registry(root: &Path) -> Value {
        let mut registry = load_registry(root).expect("load tip registry");
        registry
            .get_mut("tickets")
            .and_then(|t| t.as_array_mut())
            .expect("tickets")
            .first_mut()
            .expect("ticket")
            .as_object_mut()
            .expect("obj")
            .insert("status".into(), json!("not-a-real-status"));
        registry
    }

    #[test]
    fn set_status_refuses_empty_without_write() {
        // T-383 Class-R: empty status must not overwrite a live registry field.
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = load_registry(&root).expect("load tip registry");
        let status_before = opt_str(require_ticket(&registry, "T-001"), "status")
            .unwrap_or("")
            .to_string();

        let err = cmd_set_status(&root, &mut registry, "T-001", "")
            .expect_err("set-status must refuse empty status");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing empty write") || msg.contains("non-empty"),
            "expected empty refuse, got: {msg}"
        );

        let status_after = opt_str(require_ticket(&registry, "T-001"), "status")
            .unwrap_or("")
            .to_string();
        assert_eq!(
            status_before, status_after,
            "empty set-status must not mutate in-memory status"
        );
        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(before, after, "empty set-status must not write T-001.toml");
    }

    #[test]
    fn set_status_refuses_invalid_enum_without_write() {
        // T-383 Class-R: invalid enum must not overwrite a live registry field.
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = load_registry(&root).expect("load tip registry");

        let err = cmd_set_status(&root, &mut registry, "T-001", "not-a-real-status")
            .expect_err("set-status must refuse invalid enum");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("invalid status") && msg.contains("not-a-real-status"),
            "expected invalid-enum refuse, got: {msg}"
        );

        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(
            before, after,
            "invalid set-status must not write T-001.toml"
        );
    }

    #[test]
    fn set_status_refuses_invalid_registry_without_write() {
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = red_registry(&root);

        // Preflight must fail before any mutator body runs — if this Err is missing,
        // cmd_set_status would save_registry over the live tip (see T-451 perturbation).
        let preflight = require_check_ok(&root, &registry, "set-status T-001");
        assert!(
            preflight.is_err(),
            "preflight must be red before calling cmd_set_status"
        );

        let err = cmd_set_status(&root, &mut registry, "T-001", "shipped")
            .expect_err("set-status must refuse a schema-red registry");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing set-status T-001"),
            "expected refuse message, got: {msg}"
        );
        assert!(
            msg.contains("ticket check failed"),
            "expected check-failed note, got: {msg}"
        );

        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(
            before, after,
            "set-status must not write T-001.toml when check is red"
        );
    }

    #[test]
    fn mark_ready_refuses_invalid_registry() {
        let root = worktree_root();
        let mut registry = red_registry(&root);
        let err = cmd_mark_ready(&root, &mut registry, "T-001", None)
            .expect_err("mark-ready must refuse a schema-red registry");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing mark-ready T-001"),
            "expected refuse message, got: {msg}"
        );
    }

    #[test]
    fn add_refuses_invalid_registry_without_write() {
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = red_registry(&root);
        let next_before = registry
            .get("next_id")
            .and_then(|n| n.as_u64())
            .expect("next_id");
        let tickets_before = tickets(&registry).len();

        let err = cmd_add(
            &root,
            &mut registry,
            "should-not-land",
            "platform",
            "xtask",
            "ops",
            "",
        )
        .expect_err("add must refuse a schema-red registry");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing add"),
            "expected refuse message, got: {msg}"
        );
        assert!(
            msg.contains("ticket check failed"),
            "expected check-failed note, got: {msg}"
        );

        let next_after = registry
            .get("next_id")
            .and_then(|n| n.as_u64())
            .expect("next_id");
        assert_eq!(
            next_before, next_after,
            "add must not bump next_id when check is red"
        );
        assert_eq!(
            tickets_before,
            tickets(&registry).len(),
            "add must not push a row in-memory when check is red"
        );

        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(
            before, after,
            "add must not write T-001.toml when check is red"
        );
    }

    #[test]
    fn remove_refuses_invalid_registry_without_write() {
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = red_registry(&root);
        let tickets_before = tickets(&registry).len();

        let err = cmd_remove(&root, &mut registry, "T-001")
            .expect_err("remove must refuse a schema-red registry");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing remove T-001"),
            "expected refuse message, got: {msg}"
        );
        assert!(
            msg.contains("ticket check failed"),
            "expected check-failed note, got: {msg}"
        );
        assert_eq!(
            tickets_before,
            tickets(&registry).len(),
            "remove must not drop a row in-memory when check is red"
        );

        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(
            before, after,
            "remove must not write T-001.toml when check is red"
        );
    }

    #[test]
    fn advance_slice_refuses_invalid_registry_without_write() {
        // T-459: advance-slice must share the add/remove preflight — red registry
        // never mutates active_slice in-memory or on disk.
        let root = worktree_root();
        let registry_path = root.join(".ai/tickets/T-001.toml");
        let before = fs::read_to_string(&registry_path).expect("read registry before");
        let mut registry = red_registry(&root);

        let active_before =
            opt_str(require_ticket(&registry, "T-090"), "active_slice").map(|s| s.to_string());

        let err = cmd_advance_slice(&root, &mut registry, "T-090")
            .expect_err("advance-slice must refuse a schema-red registry");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("refusing advance-slice T-090"),
            "expected refuse message, got: {msg}"
        );
        assert!(
            msg.contains("ticket check failed"),
            "expected check-failed note, got: {msg}"
        );

        let active_after =
            opt_str(require_ticket(&registry, "T-090"), "active_slice").map(|s| s.to_string());
        assert_eq!(
            active_before, active_after,
            "advance-slice must not mutate active_slice in-memory when check is red"
        );

        let after = fs::read_to_string(&registry_path).expect("read registry after");
        assert_eq!(
            before, after,
            "advance-slice must not write T-001.toml when check is red"
        );
    }

    #[test]
    fn require_check_ok_err_matches_set_status_gate() {
        // Same gate surface ship/set-status/mark-ready/add/remove/advance-slice share
        // (T-237 / T-451 / T-455 / T-459).
        let root = worktree_root();
        let registry = red_registry(&root);
        let err = require_check_ok(&root, &registry, "set-status T-001")
            .expect_err("red registry must fail require_check_ok");
        assert!(format!("{err:#}").contains("refusing set-status T-001"));
    }
}
