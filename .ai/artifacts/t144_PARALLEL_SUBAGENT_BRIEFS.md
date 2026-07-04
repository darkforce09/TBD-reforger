# T-144.1 — Parallel subagent briefs (optional speed-up)

**Parent agent:** orchestrates, synthesizes, commits. **Subagents:** read-only partials only.

`ARMA=/home/Samuel/Projects/TBD_Arma_3_Remaster/Arma_3_SourceCode_Old`

---

## Shared rules (every subagent)

- Read-only on `$ARMA/**`
- Write **only** your assigned file under `.ai/artifacts/t144_parallel/`
- Every claim: `path:line` + symbol name
- Log **rejects** (briefing map, VBS-only, `#if 0`, wrong surface)
- Source = **Arma3_2012** Arcade editor — state provenance caveat
- Do **not** edit TBD-Reforger docs/apps/registry

---

## Agent A — UI / Editor spine

**Output:** `.ai/artifacts/t144_parallel/t144_parallel_A_ui_editor.md`

Trace how the **mission editor 2D map** opens and draws. Start broad under `$ARMA/lib/UI/` and related:

- `DisplayArcadeMap`, `DisplayMapEditor`, `CStaticMapArcadeViewer`
- `missionEditor.cpp`, `dispMissionEditor.cpp`, `uiArcade.cpp`
- `baseEditor.*`, `missionEditorCursor.*`
- Relationship to `lib/UI/uiMap.*` — primary, secondary, or shared base?

Deliver: call chain editor-open → map widget → first draw call; retail vs VBS; top 15 files.

---

## Agent B — Landscape / World data feed

**Output:** `.ai/artifacts/t144_parallel/t144_parallel_B_landscape_world.md`

How **world objects** reach the 2D map (not 3D view):

- `landscape.*`, `world.cpp`, land object DB
- Roads, forests, buildings, trees on map — vector vs icon vs polygon
- Cull / LOD / per-square visibility

Deliver: data flow diagram (text); key structs; top 15 files.

---

## Agent C — Raster / Draw pipeline

**Output:** `.ai/artifacts/t144_parallel/t144_parallel_C_raster_draw.md`

What **pixels** fill the map background:

- `DrawField`, `CStaticMap::Draw*`, texture binding
- `LandGrid`, `pictureMap`, satellite vs user chart / map chart
- Layer order bottom → top

Deliver: raster stack table; switch mechanism map vs satellite; top 15 files.

---

## Agent D — Config layer

**Output:** `.ai/artifacts/t144_parallel/t144_parallel_D_config.md`

Config-driven behavior under `$ARMA/cfg/` and param classes:

- `CfgWorlds`, `RscDisplayArcadeMap`, `RscMap`, map controls
- Map colors, zoom thresholds, feature toggles in config

Deliver: config key table; which keys affect 2D editor map; top 15 files.

---

## Agent E — Tools / Legacy paths

**Output:** `.ai/artifacts/t144_parallel/t144_parallel_E_tools_legacy.md`

Parallel or legacy pipelines:

- `VisitorExchange/`, `buldozer/`, `CfgEdit/`, `TerSynth/`, `binarize/`
- `mapViewer` / `gameStateExt.cpp` pipe
- Export paths (`uiMapExport.cpp`, visitor)

Deliver: alive vs dead; relationship to retail editor map; top 15 files.

---

## Parent merge checklist

1. Read all five partials
2. Cross-validate spine from **A + at least one of B/C**
3. Write §0 discovery + uiMap verdict
4. Merge §3–§8 from best evidence per section
5. §9–§10 gap vs T-090
6. Single `t144_arma3_map_architecture_report.md` — partials are inputs, not shippable alone
