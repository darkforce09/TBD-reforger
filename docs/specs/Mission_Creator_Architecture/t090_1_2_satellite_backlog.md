# T-090.1.2.x — Satellite basemap backlog (resume guide)

**Program hub:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Registry active slice:** `./scripts/ticket brief T-090` → **T-090.1.2.5**  
**Last shipped:** **T-090.1.2.8** @ `db9057ef` (tbd-sat v1 — unified GPU mips)

---

## What the editor shows today

| Aspect | State |
|--------|--------|
| **Source** | SAP supertexture stitch + T-090.1.2.2 apron-bridge — **locked** (engine ortho dead end) |
| **Delivery** | **Unified** `everon-sat.tbd-sat` (205.9 MB LFS) — one fetch + GPU mip chain; pyramid fallback via `delivery: "pyramid"` |
| **Detail @ max zoom** | Acceptable; residual soft ~256 m band is BI-baked (not fixable without new source) |
| **Pan / zoom feel** | **T-090.1.2.8 shipped** — no tile pop-in by construction (single BitmapLayer + trilinear mips) |
| **Water** | **T-090.1.2.5** (**active**) |
| **Hillshade** | **T-090.1.2.6** shipped @ `b958e3b4` (Mission Settings strength slider) |

**Format spike:** [`.ai/artifacts/t090_1_2_8_format_spike.json`](../../../.ai/artifacts/t090_1_2_8_format_spike.json) · **Verify:** [`.ai/artifacts/t090_1_2_8_verify_log.md`](../../../.ai/artifacts/t090_1_2_8_verify_log.md)

---

## Execution order (normative)

```text
1. T-090.1.2.8  Unified satellite texture     ✓ @ db9057ef
2. T-090.1.2.5  Satellite water composite     ← ACTIVE
3. T-090.1.2.6  Hillshade blend control       ✓ @ b958e3b4
4. T-090.1.1    Map cartographic view
—  T-090.1.2.3  Tile prefetch (legacy pyramid interim only)
```

**Shipped dead end:** T-090.1.2.4 @ `0d6fe485` — do not re-open engine ortho without new engine API evidence.

**Parallel (platform):** Fable audit remainder **T-130** — [`t130_fable_audit_remainder.md`](../../platform/t130_fable_audit_remainder.md)

---

## Slice index

| Slice | Status | Spec | Send-off |
|-------|--------|------|----------|
| **T-090.1.2.8** | shipped @ `db9057ef` | [`t090_1_2_8_unified_satellite_texture.md`](t090_1_2_8_unified_satellite_texture.md) | verify log |
| **T-090.1.2.5** | **active** | [`t090_1_2_5_satellite_water_composite.md`](t090_1_2_5_satellite_water_composite.md) | `t090_1_2_5_SEND_TO_CLAUDE.md` |
| **T-090.1.2.6** | shipped @ `b958e3b4` | [`t090_1_2_6_hillshade_blend_control.md`](t090_1_2_6_hillshade_blend_control.md) | verify log |

**Shipped:** T-090.1.2.6 @ `b958e3b4` · T-090.1.2.8 @ `db9057ef` · T-090.1.2.4 @ `0d6fe485` (FAIL) · T-090.1.2.2 @ `a3efdf6` · T-090.1.2.1 @ `19bc785`

---

## Operator manual (U1–U4)

**PASS** @ 2026-07-02 — operator: no flicker, no stutter; satellite delivery accepted.

See [`.ai/artifacts/t090_1_2_8_verify_log.md`](../../../.ai/artifacts/t090_1_2_8_verify_log.md).
