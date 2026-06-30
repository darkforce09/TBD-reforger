# T-090.1.2.2 — Claude Code handoff (SAP cell seam repair)

**Slice:** T-090.1.2.2 · **Executor:** claude-code · **Parent:** T-090.1.2.1 @ `19bc785`  
**Spec:** [`docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md`](../docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md)

## Operator report

Visible **256 m grid seams** at max zoom (vertical lines + cross at cell corners). Screenshot documented. Lossless pyramid is fine — seams are in the **stitched ortho**, not WebP tiles.

## Do not

- Add z7 pyramid or AI upscale
- Re-decode format / change BC7 contract unless analysis proves decode bug
- Edit docs/registry (Cursor sync after ship)

## Execution order

1. **P0** — `analyze-sap-seams.mjs` → `.ai/artifacts/t090_1_2_2_seam_analysis.json`
2. **Fix** — `stitch-sap-ortho.mjs` (feather / exposure match / placement — see spec strategies A–D)
3. **Gate** — `verify-sap-seams.mjs` + existing `verify-sap-ortho.mjs`
4. **Rebuild** lossless pyramid (same as T-090.1.2.1 `--lossless --maxzoom 6`)
5. **Verify** — full automated gate list in spec
6. Tag **`T-090.1.2.2`**

## Preflight

```bash
git pull && git lfs pull && make map-assets-link
./scripts/ticket brief T-090
test -f packages/map-assets/everon/staging/sap/everon-sap-ortho.png
```

## Return

Commit SHA, tag, seam analysis JSON summary, verify output, operator S1 location before/after note. **"Ready for Cursor doc sync."**
