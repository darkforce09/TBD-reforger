# Platform documentation

Cross-cutting docs for the TBD Reforger monorepo (not tied to a single app).

| Doc | Purpose |
|-----|---------|
| [`CODEBASE_AUDIT_2026.md`](CODEBASE_AUDIT_2026.md) | **T-122** audit findings + shipped/deferred log; **Fable S1–S6** @ T-126 |
| [`known-bugs/`](known-bugs/README.md) | **Known bugs** — recorded/triaged defects not currently actioned (why + how to fix later). KB-001: MC selection-at-scale |
| [`../../.ai/artifacts/fable_5_omni_audit_report.md`](../../.ai/artifacts/fable_5_omni_audit_report.md) | **Fable 5 living tracker** — all findings, status, ticket mapping |
| [`FABLE_5_AUDIT_PROGRAM.md`](FABLE_5_AUDIT_PROGRAM.md) | **T-126→128** Fable 5 remediation (T-127 active) |
| [`DOCUMENTATION_STANDARDS.md`](DOCUMENTATION_STANDARDS.md) | How to document Go / TS / Enfusion boundaries |
| [`CODING_STANDARDS.md`](CODING_STANDARDS.md) | **How code is written** — style, errors, CI gates (distinct from doc standards) |
| [`MONOREPO_MIGRATION.md`](MONOREPO_MIGRATION.md) | M0–M3 migration runbook (`apps/` + `packages/` + `.ai/`) |
| [`context_handoff.md`](context_handoff.md) | Redirect stub → [`docs/website/platform/context_handoff.md`](../website/platform/context_handoff.md) |
| [`audit/t122_codebase_audit_hotfix.md`](audit/t122_codebase_audit_hotfix.md) | T-122 slice spec (shipped) |
| [`t123_documentation_standards_rollout.md`](t123_documentation_standards_rollout.md) | **T-123 (shipped @ `169e47d`)** — full program: tags + codegen + Go JSON validation + CI (slices .0–.6) |
| [`t124_dependency_upgrade.md`](t124_dependency_upgrade.md) | **T-124 (shipped @ `cd11db0`)** — deps + toolchain: vitest 4, gin/gorm/pgx latest, Go 1.26, Node 26, Postgres 18 |
| [`t125_coding_standards_enforcement.md`](t125_coding_standards_enforcement.md) | **T-125 (shipped @ `e21dac3`, tag T-125.5)** — [`CODING_STANDARDS.md`](CODING_STANDARDS.md): 38 rules, all CI gates live (golangci, strict TS, GO-7, verify-* scripts, ENF-4, editorconfig + Prettier) |
| [`t130_fable_audit_remainder.md`](t130_fable_audit_remainder.md) | **T-130 (shipped @ `90c9f261`)** — Fable OPEN/PARTIAL remainder |
| [`tbd_north_star_backlog.md`](tbd_north_star_backlog.md) | **North Star gaps** — brain-dump items captured as registry `idea` **T-131…T-142** |

**Hub:** [`docs/website/README.md`](../website/README.md) · **Tickets:** [`docs/TICKET_LEAD.md`](../TICKET_LEAD.md) · **Ideas:** [`docs/TICKET_BRAINSTORM.md`](../TICKET_BRAINSTORM.md)
