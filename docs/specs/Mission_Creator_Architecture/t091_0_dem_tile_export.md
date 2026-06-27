# T-091.0 — DEM + tile export (human)

**Ticket:** T-091 · **Slice:** T-091.0  
**Status:** Spec ready — **next human gate after T-090.0 docs**  
**Executor:** human (+ cursor-docs for manifest commit)  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)

---

## In one sentence

Export Everon 16-bit DEM PNG + aligned tile pyramid from Workbench, probe ≥10 anchors with `GetSurfaceY`, commit assets under `packages/map-assets/everon/`, and pass automated alignment verify.

---

## Prerequisites

| Gate | Evidence |
|------|----------|
| **T-090.0** | Manifest schema, verify scripts, DEV_RUNBOOK on disk |
| **Workbench** | Arma Reforger + Workbench; vanilla Everon world accessible |
| **Git LFS** | `git lfs install` + push succeeds for PNG/WebP |

---

## Problem

No DEM or tiles in repo — editor Z is always 0; horizontal alignment unverified; T-090.1 and T-091.1 blocked.

---

## Goal

1. Export **16-bit heightmap PNG** (Base **and** Modified for A/B test).
2. Export **tile pyramid** `tiles/{z}/{x}/{y}.webp`.
3. Update `manifest.json` with **measured** `widthPx`, `heightPx`, height range, `exportedAt`, `workbenchVersion`, chosen `dem.source`.
4. Log ≥10 `GetSurfaceY(x,z)` probes → `anchors/verification.json`.
5. Run verify scripts — all anchors within threshold.

---

## Out of scope

- Frontend DEM code (**T-091.1**)
- Z on place/move (**T-091.2**)
- Arland export (defer)

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| PNG encoding | uint16 linear: `elevM = min + (uint16/65535) * (max - min)` |
| Base vs Modified | Export **both**; pick lower max anchor error vs `GetSurfaceY` |
| Anchor threshold | Initial **±1.0 m** — tune in manifest `precision` note if needed |
| World path | **Record exact path** in ops log — do not guess `Eden.ent` |
| Pixel dimensions | From **Info & Diags** — not assumed 6400² |

---

## Step-by-step runbook

### 1 — Ops log (start before export)

Copy template from hub §T-091.0 ops log. Fill Workbench version + **exact** Everon world/project path.

### 2 — Info & Diags

Terrain Creation Tool → **Info & Diags**:

- Planar resolution (m) — expect ~**2.0**
- Height min/max (m)
- Heightmap width × height (px)
- World bounds (confirm 12800×12800)

### 3 — DEM export

1. **Manage → Height Map → Rebuild Height Map**
2. **Export Height Map** → 16-bit PNG — **Modified** (and separately **Base** for A/B)
3. Save winner to `packages/map-assets/everon/dem/everon-dem-16bit.png`
4. Update manifest `dem.*` fields

### 4 — Tile export

- **Option A:** World Editor satellite / 2D map ([Terrain Creation Tool wiki](https://community.bohemia.net/wiki/Arma_Reforger:World_Editor:_Terrain_Creation_Tool))
- **Option B:** [Enhanced Map Tool](https://github.com/Til-Weimann/tilw-terrain-tools) — same origin as DEM

Output: `packages/map-assets/everon/tiles/{z}/{x}/{y}.webp` (256 px tiles; z0–z5).

### 5 — Anchor probes (required)

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
# wb_play over dev scenario; log GetSurfaceY at each anchor
```

Minimum anchors (≥10):

| id | Suggested location |
|----|-------------------|
| `coast-sw` | Southwest coast |
| `valley-inland` | Low interior |
| `hill-north` | High north |
| `airfield` | Airfield apron |
| `bridgehead-1` … `bridgehead-3` | x/z from [`bridgehead-at-levie.json`](../../../packages/tbd-schema/golden-missions/bridgehead-at-levie.json) slots |

Write [`anchors/verification.json`](../../../packages/map-assets/everon/anchors/verification.json) — schema [`terrain-anchors.schema.json`](../../../packages/tbd-schema/schema/terrain-anchors.schema.json).

Optional raw log: `anchors/surface-y-log.txt`.

### 6 — Verify + commit

```bash
make verify-terrain
make verify-terrain-strict
cd packages/tbd-schema && npm run validate
git lfs push origin main   # if using remote
```

---

## Verification gate (mandatory)

**Advance T-091.0 → T-090.1 / T-091.1 only when ALL PASS.**

### Automated (exit 0)

```bash
make verify-terrain
make verify-terrain-strict
test -f packages/map-assets/everon/dem/everon-dem-16bit.png
test -d packages/map-assets/everon/tiles/0
jq -e '.dem.widthPx > 0 and .dem.heightPx > 0' packages/map-assets/everon/manifest.json
jq -e '.anchors | length >= 10' packages/map-assets/everon/anchors/verification.json
```

### Acceptance criteria (A1–A10)

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | DEM file | 16-bit PNG in repo; manifest dims **match** Info & Diags |
| A2 | Height range | Manifest min/max matches export (includes negatives) |
| A3 | Anchors | All entries ±1.0 m DEM sample vs `surfaceYM` (`--strict`) |
| A4 | Base vs Modified | Ops log documents both errors; winner in `dem.source` |
| A5 | Tiles | z0 directory exists; sample tile opens |
| A6 | H1 | Grid origin ↔ map corner documented in ops log |
| A7 | H2 | 3 zoom levels landmark check documented |
| A8 | LFS | PNG/WebP tracked; clone + `git lfs pull` restores files |
| A9 | Schema | `npm run validate` includes manifest + anchors |
| A10 | Git | Assets committed on `main` (no feature branch) |

---

## Ops log template

```text
Date:
Operator:
Workbench version:
Everon world path (exact):
Info & Diags — planar resolution (m):
Info & Diags — height min/max (m):
Info & Diags — heightmap widthPx × heightPx:
Export — Base PNG max anchor error (m):
Export — Modified PNG max anchor error (m):
Chosen dem.source:
Tile export tool (A/B):
H1 origin check (pass/fail + notes):
H2 landmark check (pass/fail + notes):
verify-terrain-alignment --strict: pass/fail
Git LFS push: yes/no
```

---

## Related

- [`t090_1_aligned_basemap.md`](t090_1_aligned_basemap.md)
- [`t091_1_dem_loader.md`](t091_1_dem_loader.md)
