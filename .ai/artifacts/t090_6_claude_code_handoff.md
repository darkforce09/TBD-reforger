Read CLAUDE.md first.

Implement **T-090.6** — Geometry-aware placement audit (simplified 3D bounds).

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-090

═══ READ (in order — spec wins on conflict) ═══
  1. docs/specs/Mission_Creator_Architecture/t090_6_geometry_placement_audit.md (this file)
  2. docs/specs/Mission_Creator_Architecture/t090_4_z_placement_audit.md (Phase A point audit context)

═══ PROBLEM ═══
For every exported map object (Eden-scale 1M+), we need to use center + rotation + simplified 3D bounds to compute which parts are above terrain, buried, or inside another object — fully automated, no manual eyeballing.

═══ DO ═══
  1. scripts/map-assets/run-geometry-audit.ts
  2. scripts/map-assets/obbSamples.ts — transform + sample helpers
  3. scripts/map-assets/spatialHash.ts — neighbor query
  4. Extend T-090.2 schema bounds block
  5. Vitest fixtures: tilted box, half-buried rock, floating building
  6. Write .ai/artifacts/t090_6_verify_log.md
  7. Tag **T-090.6** · commit prefix **T-090.6:**

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE status markers
  - Attempt full mesh CSG / boolean operations

═══ VERIFY (all exit 0) ═══
  npm run test -- --run geometryAudit
  make schema-validate
  npm run build
  npm run lint

═══ RETURN ═══
  - Commit SHA + tag T-090.6
  - .ai/artifacts/t090_6_verify_log.md
  - Vitest + build/lint (PASS)
  - **Ready for Cursor doc sync.**
