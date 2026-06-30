# T-125 — Claude Code handoff

**Status:** **in progress** · active slice **T-125.5** · **T-125.4 shipped** @ `cb508cf` (tag **T-125.4**)  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../docs/platform/t125_coding_standards_enforcement.md) §T-125.5  
**Authority:** [`CODING_STANDARDS.md`](../docs/platform/CODING_STANDARDS.md) §7 FMT-2/3, §10 matrix · [`DOCUMENTATION_STANDARDS.md`](../docs/platform/DOCUMENTATION_STANDARDS.md)

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · T-125.3 @ `e5fbf4b` (tag **T-125.3**) · **T-125.4 @ `cb508cf` (tag T-125.4)**

---

## T-125.4 — DONE ✓

All 110% upgrades shipped; zero deferrals. Post-ship report:

| Item | Result |
|------|--------|
| **GO-7** | 77 `@route` added / 82 handlers; Register route-match verifier |
| **M6** | 15/15 (8 bucket-A, 7 bucket-B); GetDashboard `//nolint:cyclop` |
| **GO-3** | 0 new — 15 WriteAudit sites already annotated (T-125.2) |
| **LOG-3** | 140 sites (75 5xx + 65 mutator 4xx) + telemetry 200-path log |
| **GO-9** | `services.RefreshLeaderboard`; 3 structural allowlist rows |
| **ERR-4** | Script live; fixed `field_tools.go` 422 `solution`→`details` |
| **ENF-4** | 10/10 `enfusion/*.sample.json` + data-driven `validate.mjs` |
| **Scripts** | 4 new + `make verify-coding-standards` → `ci-local` + `ci.yml` backend |
| **Verify** | `make ci-local` green @ ~22s; `make test-it` PASS |

---

## Next — T-125.5 (Claude Code)

Root **`.editorconfig`** + optional Prettier + `format`/`format:check` npm scripts (FMT-2/FMT-3). See spec §T-125.5.

Do **not** redo T-125.4. T-125.6 is **cursor-docs** (registry shipped + final hub sync).

---

## Slice order (remaining)

| # | Slice | Executor | Focus |
|---|-------|----------|-------|
| 5 | **T-125.5** | claude-code | `.editorconfig` / Prettier |
| 6 | **T-125.6** | cursor-docs | Registry shipped, final hub sync |

## Return to Cursor

After T-125.5 verify → paste post-ship report → Cursor advances to T-125.6 (mark T-125 shipped, `./scripts/ticket sync`).
