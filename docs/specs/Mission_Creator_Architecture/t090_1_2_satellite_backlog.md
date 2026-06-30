# T-090.1.2.x — Satellite basemap backlog (resume guide)

**Program hub:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Registry active slice:** check `./scripts/ticket brief T-090`  
**Last shipped:** **T-090.1.2.1** @ `19bc785` (lossless VP8L z0–6, 299M LFS)

---

## What the editor shows today

| Aspect | State |
|--------|--------|
| **Source** | SAP supertexture stitch → lossless WebP pyramid |
| **Detail @ max zoom** | Acceptable (native ~1 m/px band via z6) |
| **Seams** | Visible **256 m grid lines** at cell boundaries — **T-090.1.2.2** |
| **Pan** | **~40 fps** + tile pop-in / flicker — **T-090.1.2.3** |
| **Water** | **None readable** — grey seabed at coast, dry lake beds inland — **T-090.1.2.5** |
| **Darkness** | SAP exposure — defer (tone pass later) |
| **Resolution R&D** | **T-090.1.2.4** idea only — not started |

**Staging ortho (gitignored, must exist locally):**  
`packages/map-assets/everon/staging/sap/everon-sap-ortho.png` (12800²)

---

## Execution order (normative)

Orthography changes (**seams**, **water**) each require a **full lossless pyramid rebuild** (~299M LFS). Batch ortho fixes before rebuilding when possible.

```text
1. T-090.1.2.2  SAP cell seam repair     ← ACTIVE (ortho + pyramid rebuild)
2. T-090.1.2.3  Basemap tile prefetch    (frontend only — CAN run parallel to #1)
3. T-090.1.2.5  Satellite water composite (ortho + pyramid rebuild — after #1)
4. T-090.1.1    Map cartographic view
—  T-090.1.2.4  Engine render ortho spike (idea — do not start unless promoted)
```

---

## Slice index

| Slice | Status | Spec | Handoff | Claude send-off |
|-------|--------|------|---------|-----------------|
| **T-090.1.2.2** | **active** | [`t090_1_2_2_sap_cell_seam_repair.md`](t090_1_2_2_sap_cell_seam_repair.md) | [`.ai/artifacts/t090_1_2_2_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_2_claude_code_handoff.md) | [`.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_2_SEND_TO_CLAUDE.md) |
| **T-090.1.2.3** | queued | [`t090_1_2_3_basemap_tile_prefetch.md`](t090_1_2_3_basemap_tile_prefetch.md) | [`.ai/artifacts/t090_1_2_3_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_3_claude_code_handoff.md) | [`.ai/artifacts/t090_1_2_3_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_3_SEND_TO_CLAUDE.md) |
| **T-090.1.2.5** | queued | [`t090_1_2_5_satellite_water_composite.md`](t090_1_2_5_satellite_water_composite.md) | [`.ai/artifacts/t090_1_2_5_claude_code_handoff.md`](../../../.ai/artifacts/t090_1_2_5_claude_code_handoff.md) | [`.ai/artifacts/t090_1_2_5_SEND_TO_CLAUDE.md`](../../../.ai/artifacts/t090_1_2_5_SEND_TO_CLAUDE.md) |
| **T-090.1.2.4** | idea | [`t090_1_2_4_engine_render_ortho_spike.md`](t090_1_2_4_engine_render_ortho_spike.md) | — (when promoted) | — |

**Shipped:** T-090.1.2 @ `c2730a3` · T-090.1.2.1 @ `19bc785` — verify logs under `.ai/artifacts/t090_1_2*_verify_log.md`

---

## Operator preflight (every session)

```bash
git pull && git lfs pull
make map-assets-link
./scripts/ticket brief T-090
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png && magick identify $_
```

**Dev login:** `http://localhost:8080/api/v1/auth/dev-login?role=mission_maker` → Mission Creator → Satellite view.

---

## After each Claude ship

1. Hard refresh MC → manual acceptance IDs in slice verify log  
2. Tell Cursor **"doc sync"** → registry `shipped_at`, hub, CLAUDE.md  
3. `./scripts/ticket advance-slice T-090` (or edit registry `active_slice`)

---

## Do not (program-wide)

- Re-decode/re-stitch SAP unless verify-sap-ortho fails or slice explicitly requires it  
- Ship `maxZoom: 6` without 4096 z6 tiles  
- Hand-edit `docs/TICKET_*.md` or CLAUDE status markers — registry + `./scripts/ticket sync`  
- Start **T-090.1.2.4** without promoting `idea` → `ready`  
- AI upscale / z7 pyramid for “more detail”
