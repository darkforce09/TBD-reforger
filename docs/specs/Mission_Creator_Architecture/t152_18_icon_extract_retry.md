# T-152.18 — Reforger icon EXTRACT retry (warm Workbench, operator-in-loop)

**Ticket:** T-152 · **Slice:** T-152.18 (remediation ladder #7)
**Status:** `deferred` · **Operator skip:** 2026-07-13 — no extractable icons; slice not run
**Executor:** **claude-code** (Claude Code) — **OPERATOR-IN-LOOP: do not start headless**
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §6.4 (S9, A7, D2 WRONG_PATH)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.18`**
**Depends on:** operator presence (Workbench warm via `scripts/mod/tbd-dev-bootstrap.sh`)

## In one sentence

Do what T-152.2 was supposed to do: with the operator's Workbench warm, actually **extract** Reforger map-icon art for the 21 LANDMARK_SET keys, redraw only per-key documented gaps **with explicit operator approval**, and make silent redraw structurally impossible.

---

## Problem

Audit §6.4 / ledger D2 = **WRONG_PATH**. T-152.2's single MCP search timed out ("MCP daemon not reachable … hung >30s", `t152_2_icon_discovery.json:5-7`) and the slice silently redrew all 21 keys (`source:redraw`, `reforgerRef:null` × 21). The hub's locked approach was "**Extract** Reforger icons **where possible**; redraw **gaps**"; the gate predicate accepted redraw wholesale, so automation stayed green. Operator explicitly rejects this. Known Workbench constraints apply: daemon must be warmed by the operator; new WorkbenchPlugin classes need a manual Script Editor compile.

---

## Goal

1. **Hard preflight (no fallback):** `scripts/mod/mcp-call.sh` liveness ping must succeed before anything else; on failure the slice **STOPS** — it does not redraw, does not proceed, reports "blocked: Workbench not warm".
2. **Discovery, properly:** systematic `api_search` sweep (map UI, icon, marker, editor imageset terms) + asset-browser walks to locate Reforger's map/editor icon textures (imagesets/`.edds`); record every query + result in `.ai/artifacts/t152_18_icon_discovery.json` (same shape as .2's file, now with real hits).
3. **Extraction:** for each LANDMARK_SET key, map to a Reforger source texture and export it (imageset slice → PNG; document the exact tool path — MCP resource export, imageset unpack, or Workbench-side script). Rebuild `packages/map-assets/glyphs/svg|atlas` entries with `source:"reforger"` + `reforgerRef` set.
4. **Gaps:** keys with no plausible Reforger source get a per-key gap entry (`reason`, `candidatesTried[]`) and are redrawn **only after** the operator approves the gap list (M1 checkbox in the verify log names each approved key).
5. **Gate hardening:** the coverage gate now requires `count(source=="reforger") ≥ 1` and `∀ redraw keys ∈ operator-approved list` — redraw-all can never pass again.
6. Atlas rebuild + wire check (28/29-glyph manifest consistency), verify log `.ai/artifacts/t152_18_verify_log.md`.

---

## Out of scope

- Landmark zoom policy (T-152.21).
- Non-LANDMARK_SET glyphs (trees/props) — unchanged.
- Any silent fallback path. There is none. That is the point.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | Preflight liveness gate: `mcp-call.sh` ping timeout 10 s, 2 retries → on failure **exit, slice blocked** (recorded in verify log) — never proceed to art work | Kill the T-152.2 failure mode |
| L2 | Discovery artifact is append-style JSON with every query, result count, and chosen refs — auditable | S6 provenance |
| L3 | `source:"reforger"` entries must carry `reforgerRef` (resource path) + extraction tool note | One-button trail |
| L4 | Redraw allowed **only** for keys on the operator-approved gap list (M1); approval = named keys in verify log | S9 remedy |
| L5 | Icon cell contract unchanged (128 px cells, 24×24-viewBox-equivalent framing, north-up); extracted art normalized to cell | No render-side churn |
| L6 | If Reforger art licensing/format blocks committing extracted textures, STOP and surface to operator (do not synthesize a workaround) | Honesty |
| L7 | Commit `T-152.18:` · tag `T-152.18` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| LANDMARK_SET | 21 keys | hub `:123-131` |
| Current state | 21/21 redraw, 0 extract | `t152_2_icon_discovery.json` |
| Atlas | 1024×512, 128 px cells, 29 manifest glyphs | `glyphs/atlas/world-glyphs.json` |

---

## Tasks

1. Preflight gate script (liveness) + operator coordination (Workbench warm; Script Editor compile if a new export plugin class is needed).
2. Discovery sweep + artifact.
3. Per-key extraction + normalization → SVG/PNG cell art; `source`/`reforgerRef` metadata.
4. Gap list → operator approval (M1) → approved redraws only.
5. Atlas + manifest rebuild; glyph wire check in harness (badge draw smoke).
6. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | Preflight: MCP liveness PASS recorded (or slice blocked — no art commits) | Anti-fallback |
| **G2** | Discovery artifact: ≥ 8 distinct queries with results logged | Provenance |
| **G3** | `count(keys with source=="reforger") ≥ 1`; target ≥ 15 (report actual) | Extract |
| **G4** | ∀ key with source=="redraw": key ∈ operator-approved gap list (M1 names match) | Approval |
| **G5** | ∀ key: `source ∈ {reforger, redraw}` ∧ (`reforger` ⇒ `reforgerRef ≠ null`) | Metadata |
| **G6** | Atlas/manifest rebuild consistent (make map-glyphs-verify or equivalent); harness badge smoke draws | Wire |
| **G7** | FE/wasm suites exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
scripts/mod/mcp-call.sh api_search "ping" || { echo "BLOCKED: Workbench not warm"; exit 1; }   # G1 pattern
node scripts/map-assets/build-glyph-atlas.mjs   # actual rebuild entry per repo
make map-glyphs-verify || make schema-validate
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1 (operator):** approve the gap list — each redraw key named + initialed in the verify log.
- **M2 (operator):** map view — landmark icons read as Reforger-familiar (visual pass at badge zoom).

---

## Documentation sync (Cursor, after merge)

Registry `T-152.18 → shipped`; hub row; discovery artifact linked; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.18 (copy-paste)

Authority: this spec. **Do not edit docs/registry. OPERATOR MUST BE PRESENT (Workbench warm).**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.18** — Reforger icon EXTRACT retry (operator-in-loop).

═══ PREFLIGHT (HARD GATE — NO FALLBACK) ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  scripts/mod/mcp-call.sh api_search "map icon" # must return JSON within 10 s
  # FAILURE ⇒ STOP THE SLICE. Report "blocked: Workbench not warm". DO NOT REDRAW ANYTHING.

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_18_icon_extract_retry.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §6.4
  3. .ai/artifacts/t152_2_icon_discovery.json (the failure being remediated)
  4. docs/mod/MCP_TOOLING.md (daemon + mcp-call.sh usage)
  5. packages/map-assets/glyphs/** (svg sources, atlas, manifest)
  6. T-152 hub LANDMARK_SET (21 keys)

═══ PROBLEM ═══
  T-152.2 timed out once and silently redrew all 21 icons (reforgerRef:null). Operator rejected.
  This slice extracts real Reforger art; redraw only per-key with operator approval.

═══ SHIPPED (do not reopen) ═══
  Atlas plumbing/render lanes. Current redraw art stays until replaced key-by-key.

═══ LANGUAGE GATE ═══
  No engine code expected. Node/bash tooling + assets + metadata only.

═══ LOCKED ═══
  - Liveness gate first; failure = blocked slice, zero art commits
  - Every query logged to t152_18_icon_discovery.json
  - reforger source ⇒ reforgerRef set; redraw ⇒ operator-approved gap list only (M1)
  - Coverage gate: ≥1 extracted (target ≥15); redraw-all structurally impossible
  - Licensing/format blocker ⇒ STOP + surface (L6)

═══ DO ═══
  1. Liveness gate
  2. Discovery sweep (≥8 queries) + artifact
  3. Extract + normalize per key; metadata
  4. Gap list → operator M1 → approved redraws
  5. Atlas/manifest rebuild + wire smoke
  6. Verify; .ai/artifacts/t152_18_verify_log.md; commit "T-152.18: ..."; tag T-152.18

═══ DO NOT ═══
  - Proceed past a dead MCP daemon (the one absolute rule of this slice)
  - Redraw without a named operator approval
  - Edit docs/**, .ai/tickets/**

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1 gap-list approval · M2 visual pass

═══ RETURN ═══
  - Commit SHA + tag; verify log path
  - Extract/redraw split (n/21 + refs)
  - Blocked report instead, if Workbench was never warm
```
