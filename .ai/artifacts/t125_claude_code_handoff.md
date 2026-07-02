# T-125 — Claude Code handoff

**Status:** **shipped** · program **T-125.0–.6 complete** · code tag **T-125.5** @ `e21dac3`  
**Spec:** [`docs/platform/t125_coding_standards_enforcement.md`](../../docs/platform/t125_coding_standards_enforcement.md)  
**Authority:** [`CODING_STANDARDS.md`](../../docs/platform/CODING_STANDARDS.md) — all **38** §10 rules **live**

**Shipped:** T-125.0 @ `a54f491` · T-125.1 @ `9792182` · T-125.2/.2.1 @ `80c7f07` · T-125.3 @ `e5fbf4b` · T-125.4 @ `cb508cf` · **T-125.5 @ `e21dac3` (tag T-125.5)**

---

## T-125.5 — DONE ✓

| Item | Result |
|------|--------|
| **FMT-2** | `.editorconfig` + `editorconfig-checker` v3.8.0; `make verify-editorconfig` |
| **FMT-3** | Prettier 3.9.4 + eslint-config-prettier 10.1.8; 58 files reformatted |
| **CI** | `format:check` in frontend job; dedicated `editorconfig` job in `ci.yml` |
| **Verify** | `make ci-local` @ 22.7s |

---

## T-125.6 — DONE ✓ (Cursor)

Registry shipped; CODING_STANDARDS FMT-2/3 live; CLAUDE §Done; DOCUMENTATION_STANDARDS §0/§10 drift fixed; DEV_RUNBOOK updated.

No further Claude Code slices on T-125.
