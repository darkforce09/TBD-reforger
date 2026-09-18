//! Brief.

use super::*;

/// Ticket status enum — mirrors `.ai/tickets/schema.json` `$defs.status` (T-383).
pub(super) const VALID_TICKET_STATUSES: &[&str] = &[
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
    if let Some(hub) = opt_str(t, "spec")
        && hub != spec
    {
        println!("HUB: {hub} (program context only)");
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
                "ORTH: assets_v2/scratch/everon/sap/everon-sap-ortho.png (12800² — already built; do NOT re-stitch)"
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
            println!("DO NOT: edit apps/website/, apps/mod/, contracts_v2/ source");
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
                "DO NOT: TBD_TerrainExportPlugin.c, Workbench, MCP terrain export, re-export everon-dem-16bit.png, anchor probes, or assets_v2/terrains/ edits"
            );
            println!(
                "SCOPE (React-era, shipped; app retired at T-159.29.3): tactical-map/dem/* + DemController wiring"
            );
            println!(
                "REFERENCE (port, do not re-run): contracts_v2/scripts/lib/dem-sample.mjs"
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
