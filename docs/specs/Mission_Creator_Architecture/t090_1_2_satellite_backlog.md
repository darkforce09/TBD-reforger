# T-090.1.2.x — Satellite basemap backlog (resume guide)

**Program hub:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Registry active slice:** `./scripts/ticket brief T-090` → **T-090.1.2.8**  
**Last shipped:** **T-090.1.2.4** @ `0d6fe485` (P0 FAIL — SAP locked as source)

---

## What the editor shows today

| Aspect | State |
|--------|--------|
| **Source** | SAP supertexture stitch + T-090.1.2.2 apron-bridge — **locked** (engine ortho dead end) |
| **Detail @ max zoom** | Acceptable; residual soft ~256 m band is BI-baked (not fixable without new source) |
| **Pan / zoom feel** | Tile pop-in / flicker — **5461 WebP tiles** → **T-090.1.2.8** (ACTIVE) |
| **Water** | **T-090.1.2.5** (queued) |
| **Hillshade** | **T-090.1.2.6** (queued) |

**Spike verdict:** [`.ai/artifacts/t090_1_2_4_engine_render_spike.json`](../../../.ai/artifacts/t090_1_2_4_engine_render_spike.json)

---

## Execution order (normative)

```text
1. T-090.1.2.8  Unified satellite texture     ← ACTIVE (GPU mips, no tile flicker)
2. T-090.1.2.5  Satellite water composite
3. T-090.1.2.6  Hillshade blend control       (parallel OK)
4. T-090.1.1    Map cartographic view
—  T-090.1.2.3  Tile prefetch (legacy pyramid interim only)
```

**Shipped dead end:** T-090.1.2.4 @ `0d6fe485` — do not re-open engine ortho without new engine API evidence.

**Parallel (platform):** Fable audit **T-126 → T-127 → T-128** — [`FABLE_5_AUDIT_PROGRAM.md`](../../platform/FABLE_5_AUDIT_PROGRAM.md)

---

## Slice index

| Slice | Status | Spec | Send-off |
|-------|--------|------|----------|
| **T-090.1.2.8** | **active** | [`t090_1_2_8_unified_satellite_texture.md`](t090_1_2_8_unified_satellite_texture.md) | `./scripts/ticket prompt T-090` |
| **T-090.1.2.5** | queued | [`t090_1_2_5_satellite_water_composite.md`](t090_1_2_5_satellite_water_composite.md) | `t090_1_2_5_SEND_TO_CLAUDE.md` |
| **T-090.1.2.6** | queued | [`t090_1_2_6_hillshade_blend_control.md`](t090_1_2_6_hillshade_blend_control.md) | — |

**Shipped:** T-090.1.2.4 @ `0d6fe485` (FAIL) · T-090.1.2.2 @ `a3efdf6` · T-090.1.2.1 @ `19bc785`

---

## Operator preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
```

**Dev login:** Mission Creator → Satellite view · hard refresh after `.2.8` ship.
