# Copy-paste prompt for Claude Code — T-090.1.2.2

---

Implement **T-090.1.2.2** — SAP supertexture cell seam repair.

Preflight: git pull && git lfs pull && make map-assets-link && ./scripts/ticket brief T-090

Read:
  .ai/artifacts/t090_1_2_2_claude_code_handoff.md
  docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md

Problem: Visible ~256 m grid seams where 50×50 SAP cells meet in stitch-sap-ortho.mjs (hard paste, no feather). Operator screenshot shows vertical line + cross at cell corners. NOT pyramid tile seams. T-090.1.2.1 lossless VP8L preserved the artifact.

Do:
  1. Create analyze-sap-seams.mjs — quantify edge ΔRGB on 256 px grid → t090_1_2_2_seam_analysis.json
  2. Fix stitch (feather 2–8 px and/or per-cell exposure match on overlap — smallest fix that passes)
  3. Create verify-sap-seams.mjs gate (threshold from P0 baseline)
  4. Re-stitch + verify-sap-ortho (orientation must stay PASS)
  5. Rebuild lossless z0–6 pyramid (build-tile-pyramid.sh --lossless --maxzoom 6)
  6. All ship gates in spec; tag T-090.1.2.2

Do not: z7 pyramid, AI upscale, BC7 “fix” fantasies, docs/registry edits, frontend unless needed.

Manual S1: operator seam location invisible at max zoom.

Return: SHA, tag, analysis summary, verify output, "Ready for Cursor doc sync."
