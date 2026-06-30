# Copy-paste prompt for Claude Code — T-090.1.2.5

---

Implement **T-090.1.2.5** — Satellite basemap water (ocean + inland).

Preflight: git pull && git lfs pull && make map-assets-link && ./scripts/ticket brief T-090

Read:
  .ai/artifacts/t090_1_2_5_claude_code_handoff.md
  docs/specs/Mission_Creator_Architecture/t090_1_2_5_satellite_water_composite.md

Problem: No readable water — grey SAP seabed at coast, dry lake beds inland. Interim raster had blue ocean only, no inland. Product needs real hydrology from engine/DEM data, not hand paint.

Do:
  1. P0 spike — pick water mask source → t090_1_2_5_water_source_spike.json
  2. composite-water-ortho.mjs — ocean + inland onto SAP ortho (north-up, aligned)
  3. verify-sap-ortho + lossless z0-6 pyramid rebuild
  4. Update map_export_everon.json; t090_1_2_5_verify_log.md
  5. Manual W1–W4 in spec

Run AFTER T-090.1.2.2 seam fix ortho when possible (one pyramid rebuild).

Do not: hand-painted water, AI rivers, docs/registry edits.

Tag T-090.1.2.5. Return "Ready for Cursor doc sync."
