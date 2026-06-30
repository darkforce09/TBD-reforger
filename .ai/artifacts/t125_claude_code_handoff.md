# T-125 — Claude Code handoff

**Status:** **in progress** · active slice **T-125.4** · registry synced post-T-125.3  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md)  
**Authority:** [`CODING_STANDARDS.md`](../docs/platform/CODING_STANDARDS.md) + [`DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md)

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · **T-125.3 @ `e5fbf4b`** (tag **T-125.3**)

---

## T-125.3 — DONE ✓

Strict TS + eslint gates + TS-6 `@model`/`@contract` on FE contract exports. See spec §T-125.3 Shipped note.

---

## Copy this into a **new** Claude Code chat — T-125.4 ONLY

```
Read CLAUDE.md §Status + §Conventions, then implement ONLY slice T-125.4 from:
  docs/platform/t125_coding_standards_enforcement.md   # §T-125.4
  docs/platform/CODING_STANDARDS.md                    # GO-1, GO-7, GO-9, ERR-4, LOG-3, SIZE-1/3, ENF-4, §10 matrix
  docs/platform/DOCUMENTATION_STANDARDS.md             # §3 @route grammar
  .ai/artifacts/t125_claude_code_handoff.md

Slice T-125.4 — @route on handlers, M6 db.First fixes, verify-* scripts, Enfusion DTO gate.
Do NOT start T-125.5–.6. Do NOT redo T-125.3 (strict/eslint/TS-6 are live @ e5fbf4b).

═══ PREFLIGHT ═══
  ./scripts/ticket brief T-125              # SLICE: T-125.4
  git log -1 --oneline                      # expect e5fbf4b T-125.3
  make db-up && nvm use

═══ T-125.3 DONE — DO NOT REDO ═══
  strict:true (app + node tsconfigs), eslint TS-2..7/LOG-2/COMP-1(TS)
  import-x/no-restricted-paths + no-restricted-imports @/pages
  verify-contract-citations: @model/@contract on FE exports (36 checked)

═══ EXECUTION MODEL ═══
  One commit on main, tag T-125.4, Co-Authored-By trailer
  MAY edit: t125_coding_standards_enforcement.md §T-125.4 Shipped note ONLY
  DO NOT edit: registry.json, CLAUDE.md, CODING_STANDARDS body (Cursor after slice)

═══ TASKS (§T-125.4 + CODING_STANDARDS §10) ═══

  1. GO-7 — @route on every exported handler func in apps/website/internal/handlers/
     Extend verify-contract-citations.mjs: require @route METHOD /api/v1/... on ^func [A-Z]

  2. M6 — Fix _ = db.First(...).Error and similar silent DB reads (15 sites documented T-125.2)
     Real error handling per GO-2; no blanket nolint

  3. GO-1/GO-9 — scripts/website/verify-handler-imports.sh (handlers import allowlist)

  4. ERR-4 — scripts/website/verify-error-envelope.sh (gin.H keys ⊆ {error, details})

  5. LOG-3 — scripts/website/verify-handler-logging.sh

  6. SIZE-1/3 — scripts/website/verify-file-length.mjs (600 warn / 1000 fail + allowlist)

  7. ENF-4 — validate.mjs Enfusion DTO fixture branch

  8. Makefile — make verify-coding-standards meta target wiring scripts 3–6

═══ VERIFY ═══
  make verify-citations && make verify-coding-standards
  make test-it && make ci-local

Return: commit hash, @route count, M6 sites fixed, scripts added, ci-local wall-clock.
```

---

## Slice order (remaining)

| # | Slice | Focus |
|---|-------|-------|
| 4 | **T-125.4** | `@route`, M6, verify-* scripts, Enfusion DTO ← **ACTIVE** |
| 5 | **T-125.5** | `.editorconfig` / Prettier |
| 6 | **T-125.6** | **cursor-docs** — registry shipped, final hub sync |
