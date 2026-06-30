# Copy-paste prompt for Claude Code — T-090.1.2.3

---

Implement **T-090.1.2.3** — basemap tile prefetch & pan stability.

Preflight: git pull && make map-assets-link && ./scripts/ticket brief T-090

Read:
  .ai/artifacts/t090_1_2_3_claude_code_handoff.md
  docs/specs/Mission_Creator_Architecture/t090_1_2_3_basemap_tile_prefetch.md

Problem: Panning Satellite basemap drops to ~40 fps with significant tile pop-in/flicker. Static view ~165 fps. Each BitmapLayer fetches VP8L on mount; crossing tile boundaries shows blank until decode.

Do:
  1. Tile texture cache keyed by z/x/y (ImageBitmap or equivalent)
  2. Prefetch 1-tile ring beyond viewport at current LOD
  3. Keep previous tiles visible until replacements ready
  4. Optional worker decode if main thread bound
  5. Maintain MAX_VISIBLE_BASEMAP_TILES=64; target ≥55 fps while panning

Do not: rebuild pyramid, change tileUrl/bounds, edit docs/registry.

Verify: npm run build && npm run lint && npm test && make ci-local-frontend

Manual P1: pan across seams — no pop-in.

Tag T-090.1.2.3. Return verify log + "Ready for Cursor doc sync."
