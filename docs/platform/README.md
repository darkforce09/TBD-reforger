# Platform documentation

Cross-cutting docs for the TBD Reforger monorepo (not tied to a single app).

| Doc | Purpose |
|-----|---------|
| [`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md) | **T-122** audit findings + shipped/deferred log |
| [`FABLE_5_AUDIT_PROGRAM.md`](FABLE_5_AUDIT_PROGRAM.md) | **T-126→128** Fable 5 remediation (T-127 active) |
| [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) | How to document Go / TS / Enfusion boundaries |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | **How code is written** — style, errors, CI gates (distinct from doc standards) |
| [`MONOREPO_MIGRATION.md`](MONOREPO_MIGRATION.md) | M0–M3 migration runbook (`apps/` + `packages/` + `.ai/`) |
| [`context_handoff.md`](context_handoff.md) | Redirect stub → [`docs/website/platform/context_handoff.md`](../website/platform/context_handoff.md) |
| [`audit/t122_codebase_audit_hotfix.md`](audit/t122_codebase_audit_hotfix.md) | T-122 slice spec (shipped) |
| [`t123_documentation_standards_rollout.md`](t123_documentation_standards_rollout.md) | **T-123 (shipped @ `169e47d`)** — full program: tags + codegen + Go JSON validation + CI (slices .0–.6) |
| [`t124_dependency_upgrade.md`](t124_dependency_upgrade.md) | **T-124 (shipped @ `cd11db0`)** — deps + toolchain: vitest 4, gin/gorm/pgx latest, Go 1.26, Node 26, Postgres 18 |
| [`t125_coding_standards_enforcement.md`](t125_coding_standards_enforcement.md) | **T-125 (shipped @ `e21dac3`, tag T-125.5)** — [`CODING_STANDARDS.md`](CODING_STANDARDS.md): 38 rules, all CI gates live (golangci, strict TS, GO-7, verify-* scripts, ENF-4, editorconfig + Prettier) |

**Hub:** [`docs/website/README.md`](../website/README.md) · **Tickets:** [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md)
