# Send-off — T-090.1.2.8 (main queue) ← **primary track**

**Checkout:** repo root · **Branch:** `main` (or `ticket/T-090` if you prefer branch-per-ticket — brief says `ticket/T-090`)

```bash
cd /home/Samuel/Projects/TBD-Reforger
git checkout main && git pull
./scripts/ticket brief T-090
./scripts/ticket prompt T-090
```

| Doc | Path |
|-----|------|
| Handoff | [`.ai/artifacts/t090_1_2_8_claude_code_handoff.md`](t090_1_2_8_claude_code_handoff.md) |
| Spec | [`docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md) |
| Program hub | [`docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md) |

**Parallel:** T-130 runs in `.ai/artifacts/worktrees/TBD-T-130` — do not mix commits.

---

## Copy-paste prompt

```
Read CLAUDE.md first.

Implement **T-090.1.2.8** — unified GPU satellite texture (no tile flicker).

═══ PREFLIGHT ═══
  git checkout main && git pull
  ./scripts/ticket brief T-090
  make map-assets-link

═══ READ ═══
  1. .ai/artifacts/t090_1_2_8_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_1_2_8_unified_satellite_texture.md
  3. docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md
  4. features/tactical-map/layers/useTerrainBasemapLayer.ts
  5. packages/map-assets/everon/manifest.json
  6. packages/map-assets/everon/staging/sap/everon-sap-ortho.png

═══ CONTEXT ═══
  T-090.1.2.4 @ 0d6fe485 P0 FAIL — no engine ortho API. SAP ortho is the source.
  Goal: one binary + GPU mips — NOT 5461 WebP tile fetches on pan.

═══ DO NOT REOPEN ═══
  T-090.1.2.1–.4 shipped · SAP stitch path locked

═══ DO ═══
  1. P0 format spike → .ai/artifacts/t090_1_2_8_format_spike.json
  2. scripts/map-assets/build-unified-satellite.mjs + verify script
  3. Frontend unified loader branch in useTerrainBasemapLayer
  4. manifest.tiles.satellite.delivery: "unified"
  5. .ai/artifacts/t090_1_2_8_verify_log.md
  6. Tag **T-090.1.2.8** · prefix T-090.1.2.8:

═══ VERIFY ═══
  cd apps/website/frontend && npm run build && npm run lint
  Manual: pan/zoom MC @ max zoom — no tile pop-in

═══ RETURN ═══
  SHA + tag · verify log · Ready for Cursor doc sync.
  No docs/registry edits.
```

**After ship:** `./scripts/ticket advance-slice T-090` · Cursor doc sync · then T-068 / T-092 per queue.
