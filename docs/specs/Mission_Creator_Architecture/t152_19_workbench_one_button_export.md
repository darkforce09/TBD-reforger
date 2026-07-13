# T-152.19 — One-button Workbench label export (Path A E2E)

**Ticket:** T-152 · **Slice:** T-152.19 (remediation ladder #8)
**Status:** `queued`
**Executor:** **claude-code** (Claude Code) — **OPERATOR-IN-LOOP: Workbench warm + Script Editor compile required**
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §9 (S6, D8, A9, A10)
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.19`**
**Depends on:** operator presence · T-152.17 (locality/kind vocabulary settled, so Path A emits the final shape)

## In one sentence

Close the "press the button tomorrow" gap: fix and actually run the Path A `TBD_LocationsExportPlugin` end-to-end, hunt road-name/taxiway attributes in Workbench entity space (final verdict, not a glossed spike), and wire one make target that regenerates the label sidecars from `$profile` output.

---

## Problem

Audit §9 scorecard. Towns 2/5: the Path A plugin is authored but has **never produced an artifact** (no `$profile:TBD_LocationsExport.json` anywhere; spike `status:"blocked-ci"`), and it hardcodes `"importance":0.55` for every row (`TBD_LocationsExportPlugin.c:172`) — running it today would *regress* the curated importance ranking. Roads 0/5: no engine-side name source found (888 segments, `namedSegments 0`); the 6 shipped names are a hand list. Taxiways: absent in every checked export surface. Known constraint: new/changed WorkbenchPlugin classes need an **operator Script Editor compile** — `wb_reload` is not enough (deceptively keeps old actions).

**Operator evidence (2026-07-13) — the extraction target provably exists.** Reforger's own in-game map (World Editor play mode, `MapMenu` widgets) renders a rich name layer our export never captured: **"HORNBEAM VALLEY"** (area/valley name), **"Ramtop Meadows"**, **"Raccoon Rock"**, plus grid-anchored town names — screenshot `.ai/artifacts/t152_12_operator/workbench_ingame_map_names_reference.png`. These come from engine map descriptors (SCR_MapDescriptorComponent / MapWidget layer) — a concrete hunt target for this slice's discovery sweep: dump map-descriptor entities/components rather than only `World/Locations/*.et` compositions. Area/valley names would also enrich the locations kind vocabulary (`region`?) beyond towns/hills.

---

## Goal

1. **Plugin fix:** `TBD_LocationsExportPlugin.c` emits per-row importance from a name→importance table mirror (source of truth stays `lib/locations-export.mjs` `IMPORTANCE_BY_NAME`, exported to the plugin as generated constants or a sidecar the plugin reads) + the T-152.17 kind vocabulary (town/village/locality/hill/peak/natural/airport).
2. **Path A run (operator):** operator compiles in Script Editor + runs "Export TBD Locations" → `$profile:TBD_LocationsExport.json`; copy-in script lands it under `packages/map-assets/everon/staging/`.
3. **Merge tool:** `export-locations.mjs` gains `--from-workbench` mode: Path A rows ∪ CfgWorlds crosswalk supplement (the 4 towns with no Location `.et`) → same `locations.json` shape; **diff gate** proves Path A ⊇ Path B settlement set (names + coords within 25 m).
4. **Road-name / taxiway verdict:** with Workbench live, sweep entity space (`api_search` MapDescriptor / SCR_MapDescriptorComponent / location entities / road network attributes) for name strings and taxiway linework. Outcome A: source found → follow-up extract slice filed with evidence. Outcome B: definitively absent → curated `road-names.json` formally chartered (operator signs M2) and taxiway absence closed. Either way the verdict is recorded with query logs — no gloss.
5. **One button:** `make map-labels-everon` = copy-in + merge + regen (`locations.json`, `height-labels.json` via .16 exporter) + validators — one command after the Workbench button.
6. Verify log `.ai/artifacts/t152_19_verify_log.md`.

---

## Out of scope

- Icon extraction (T-152.18).
- Rendering changes (lanes read regenerated sidecars unchanged).
- Full P-phase world re-export.

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | MCP liveness preflight identical to T-152.18 L1 — dead daemon ⇒ blocked slice, no fake progress | Anti-gloss |
| L2 | Importance/kind source of truth stays in repo (`lib/locations-export.mjs`); plugin consumes generated mirror — no second hand-maintained table | One authority |
| L3 | Path A output is **staged input**, merged deterministically — committed `locations.json` remains reproducible from script + staged files | Provenance chain |
| L4 | Diff gate: Path A settlement set ⊇ Path B settlements; any regression (missing town, >25 m drift) FAILS | No silent data loss |
| L5 | Road/taxiway verdict must quote query→result logs; "not found" is acceptable **only** with the sweep recorded + operator sign (M2) | S6 remedy |
| L6 | Enfusion edits obey EnfScript gotchas (no `vanilla` ident, Format ≤9 params, `ref` members for FileHandle, etc.); operator compiles (Script Editor) before run | Workbench reality |
| L7 | Commit `T-152.19:` · tag `T-152.19` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| Plugin importance bug | hardcoded 0.55 | `TBD_LocationsExportPlugin.c:172` |
| Crosswalk-only towns | 4 (Gorey, Highstone, Raccoon Rock, Kermovan) | `lib/locations-export.mjs:54-87` |
| Road segments / named | 888 / 0 | `t152_9_road_name_spike.json` |
| Curated road names | 6 | `road-names.json` |

---

## Tasks

1. Plugin importance/kind fix + generated mirror; operator Script Editor compile.
2. Operator runs export; copy-in script + staging path.
3. `--from-workbench` merge mode + diff gate in `export-locations.mjs`/lib.
4. Road-name + taxiway Workbench sweep with logged queries → verdict artifact `.ai/artifacts/t152_19_name_source_verdict.json`.
5. `make map-labels-everon` target chaining copy-in → merge → .16 heights regen → validators.
6. Verify suite + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | MCP liveness PASS recorded (else slice blocked, nothing shipped) | Anti-fallback |
| **G2** | `$profile` export staged; plugin rows carry non-uniform importance (≥ 3 distinct values) + T-152.17 kinds | Plugin fix |
| **G3** | Diff gate: Path A ∪ crosswalk ⊇ Path B settlements; coords ≤ 25 m drift; 8 required towns present | Data parity |
| **G4** | `make map-labels-everon` exit 0 from clean checkout + staged export; regenerates byte-stable artifacts on second run | One button |
| **G5** | Verdict artifact: ≥ 10 logged road/taxiway queries with results; outcome A (source + follow-up filed) or B (operator-signed curated charter, M2) | Verdict |
| **G6** | `make schema-validate` + FE suites exit 0 | Regression |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
scripts/mod/mcp-call.sh api_search "MapDescriptor" || { echo "BLOCKED: Workbench not warm"; exit 1; }
make map-labels-everon
make map-labels-everon   # byte-stability (G4)
git diff --stat packages/map-assets/everon/   # expect empty on 2nd run
make schema-validate
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1 (operator):** Script Editor compile + "Export TBD Locations" button run — one button, artifact lands.
- **M2 (operator):** road-name verdict signed (extract path found / curated charter accepted).

---

## Documentation sync (Cursor, after merge)

Registry `T-152.19 → shipped`; hub provenance matrix updated to Path A; `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.19 (copy-paste)

Authority: this spec. **Do not edit docs/registry. OPERATOR MUST BE PRESENT.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.19** — one-button Workbench label export (Path A E2E).

═══ PREFLIGHT (HARD GATE) ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  scripts/mod/mcp-call.sh api_search "Location" # JSON within 10 s or STOP (blocked report)

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_19_workbench_one_button_export.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §9
  3. apps/mod/tbd-framework/Scripts/WorkbenchGame/TBD_LocationsExportPlugin.c
  4. scripts/map-assets/{export-locations.mjs,lib/locations-export.mjs,lib/road-names.mjs}
  5. .ai/artifacts/t152_6_locations_spike.json + t152_9_road_name_spike.json
  6. docs/mod/MCP_TOOLING.md

═══ PROBLEM ═══
  Path A plugin never run + flattens importance to 0.55; roads 100% curated; taxiways unproven.
  North star: operator presses Workbench button → artifacts regenerate. Close it or record the
  definitive verdict — no gloss.

═══ SHIPPED (do not reopen) ═══
  Path B pipeline (stays as fallback/merge base); .16/.17 lane policy.

═══ LANGUAGE GATE ═══
  EnfScript plugin + Node scripts + Makefile. Zero engine-crate/TS changes.
  EnfScript gotchas: no `vanilla` ident, Format ≤9 params, ref FileHandle members;
  operator Script Editor compile (wb_reload insufficient for new classes).

═══ LOCKED ═══
  - MCP liveness gate; dead daemon ⇒ blocked
  - Importance/kind SoT in lib/locations-export.mjs; plugin reads generated mirror
  - Diff gate Path A ⊇ Path B settlements (≤25 m); make map-labels-everon byte-stable
  - Road/taxiway verdict with logged queries; curated charter needs operator sign (M2)

═══ DO ═══
  1. Plugin fix + mirror gen (operator compiles)
  2. Operator export run → staging copy-in
  3. --from-workbench merge + diff gate
  4. Name/taxiway sweep → verdict artifact
  5. make map-labels-everon (chains merge + .16 heights regen + validators)
  6. Verify; .ai/artifacts/t152_19_verify_log.md; commit "T-152.19: ..."; tag T-152.19

═══ DO NOT ═══
  - Proceed past dead MCP / skipped compile
  - Hand-edit committed label artifacts
  - Edit docs/**, .ai/tickets/**

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1 button run · M2 verdict sign

═══ RETURN ═══
  - Commit SHA + tag; verify log + verdict artifact paths
  - Path A vs Path B diff summary
  - Road/taxiway outcome (A source found / B chartered)
```
