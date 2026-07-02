# T-128 — doc link repair log

**Ticket:** T-128 (tag **T-128**, branch `ticket/T-128`, base `a6f54ac0`) · **Date:** 2026-07-02
**Spec:** [`docs/platform/t128_doc_link_repair.md`](../../docs/platform/t128_doc_link_repair.md) · **Tracker:** [`fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md)

## Result

| Metric | Count |
|---|---:|
| Broken relative markdown links **before** (worktree scan) | **158** (157 excl. untracked `WORK_HERE.md`) |
| Broken **after** | **2 benign** (see below) |
| Fixed / removed | **156** |

Audit F5-07 counted **155** @ `a3efdf68` on the main checkout; the worktree baseline differs slightly (no gitignored `crf_framework` file, plus Fable-program artifacts added since, plus scanner-method deltas). Both counts are honest under their own method — this log's numbers are reproducible with the scanner below.

**Remaining 2 (deliberate, not broken in effect):**
1. `WORK_HERE.md` — untracked worktree scaffolding; never merges.
2. `.ai/tickets/HANDOFF_TEMPLATE.md` — a literal bracket-paren ellipsis placeholder in template prose; not a real link.

## Scanner (method)

```bash
find . -name '*.md' -not -path '*/node_modules/*' -not -path './.git/*' -print0 |
while IFS= read -r -d '' f; do
  dir=$(dirname "$f")
  grep -oE '\]\([^)]+\)' "$f" | sed -E 's/^\]\(//; s/\)$//; s/[#?].*$//' |
  while IFS= read -r t; do
    case "$t" in http://*|https://*|mailto:*|/*|\<*|'') continue;; esac
    [ ! -e "$dir/$t" ] && printf '%s\t%s\n' "$f" "$t"
  done
done
```

Inline links only (no reference-style); fragments stripped; code-span appearances of the raw link pattern count as hits (one such false positive documented above; two others in the audit report were reworded to code-span-free phrasing).

## Batches

| Batch | Files | Links | Fix |
|---|---|---:|---|
| **P0** `.ai/tickets/` | AI_PLAYBOOK, README, SPEC_TEMPLATE, CLAUDE_CODE_PROMPT | 13 | `../docs/` → `../../docs/`; `AGENT_COMMIT_CHECKLIST.md` retargeted to `docs/website/` |
| **P0** `.ai/artifacts/` | t090 handoff family (11 files), t122–t125 handoffs, README, operator resume, audit report | 34 | depth fixes; dead specs `t090_1_aligned_basemap.md` / `t090_basemap_dual_view.md` → program hub with supersession note; t122's never-created `audit/…` path → `CODEBASE_AUDIT_2026.md`; `scrape-eden-wiki.mjs` → real `scripts/website/tools/` path; audit's quoted broken-link example reworded to plain code |
| **P1** staging honesty | STAGING-SERVER.md, MILESTONES.md, build-plan, `deploy-staging.sh` | — (content) | see §P1 below |
| **P2** READMEs | `apps/mod/README.md` (25), `apps/website/README.md` (8), `apps/website/CLAUDE.md` (1), `apps/website/frontend/README.md` (1), `apps/mod/tbd-framework/README.md` (3) | 38 | monorepo paths (`docs/mod/`, `scripts/mod/`, `packages/tbd-schema/`, `../website/`); gitignored `Tbd_framework/` targets → plain text; mod README fully rewritten (commands, repo URL → `darkforce09/TBD-reforger`, status honesty); backend ROADMAP → real `docs/website/backend/ROADMAP.md` |
| **P3** orphans | `apps/website/frontend/docs/` (INDEX + pages/mission-editor) | 28 | **deleted** (`git rm`) — DOCUMENTATION_STANDARDS.md forbids `apps/**/docs/**`; nothing live links in |
| **P3** renumber | 8 MC docs | — | floor picker **T-126 → T-129** (ROADMAP ×2, t090_2 ×2, t090_eden_map_reference ×2, t090_4, t090_5, t090_6, t090_9); Fable-audit T-126 mentions untouched |
| **P4** | registry, CLAUDE.md, `MissionCreatorPage.tsx:44`, t128 spec | — | T-128 shipped + T-090 resumed in registry → `ticket sync` regenerated views/marker block; CLAUDE.md Fable block → COMPLETE, T-090 → RESUMED, Arland 10240 → **4096**; MissionCreatorPage comment 10.24km → **4.096km** |
| **Extended sweep** (user-approved) | AGENT_COMMIT_CHECKLIST (6), MC ROADMAP (3), macos_ux (3), t048 (6), t049–t060_1 specs (8), t068_3 (1), t090_0 (2), _template (2), DEV_RUNBOOK (1), CURSOR_SETUP (1), CLAUDE-CODE-START (1), stitch-exports README (1), FABLE hub (1), rest-spike (7) | 43 | depth fixes; `not-found.tsx` → `utility.tsx`; verify-terrain `.ts` → real `packages/tbd-schema/scripts/*.mjs`; gitignored `.cursor/…` targets → plain text; deleted `assetCatalogMock.ts` → plain text; rest-spike's 7 links to removed spike code → plain text + historical note (closes **F1-05**) |

Notes: `t126_*` / `t127_*` artifacts were verified already-correct (created post-reorg) — no edits. `docs/website/frontend/pages/mission-creator.md` exists now (audit predated it) — plain depth fix sufficed.

## P1 — staging honesty (F5-04 / F2C-02 docs-half)

- **`docs/mod/STAGING-SERVER.md`** — top callout: V2–V4 **BLOCKED on T-092** (routes existed only in the removed Phase-0 REST spike; current backend = `/api/v1` only → 404); §6 smoke annotated; verification-matrix rows V2/V3/V4 now state BLOCKED + current 404 + target status.
- **`docs/mod/MILESTONES.md`** — "mission loads from backend REST" checkbox flipped `[x]` → `[ ]` with "verified 2026-06-14 against the spike, since removed — re-verify blocked on T-092"; file-fallback / deploy / Direct-Join checkmarks kept (still true).
- **`docs/mod/tbd-reforger-platform-build-plan.md`** — "Current status" callout under the header + inline **(T-092 — not yet live)** markers on the mission-publish/roster flows, loader read-order, and §B7 API table.
- **`scripts/mod/deploy-staging.sh`** — V2–V4 smoke block now **skipped by default** with a loud `[SKIP] … BLOCKED on T-092` message; `TBD_RUN_T092_SMOKE=1` opt-in restores the original gate (dry-run branch preserved; `bash -n` clean). Previously the `|| exit 1` curls aborted every deploy at the smoke step.
- `apps/mod/README.md` + `apps/mod/tbd-framework/README.md` REST rows carry the same one-line T-092 note.

## P3 — `handlers/missions/` empty dir (F2B-10)

`apps/website/internal/handlers/missions/` is **not in git** (git cannot track empty directories) — it exists only as untracked cruft in the main checkout, so no commit can remove it. **Operator action on main checkout:**

```bash
rmdir apps/website/internal/handlers/missions
```

## Tracker + program docs updated

- [`fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md): F1-05, F2B-10, F2C-05, F5-01…F5-07 → **RESOLVED (T-128)**; F2C-02 split (docs-half RESOLVED, producer stays T-092 DEFERRED); F2C-04 → **OPEN** with note (branch policy now hybrid — parallel tickets use `ticket/T-0xx` worktrees; `scripts/ticket` text edit was out of T-128 scope); summary counts + program order + last-update line refreshed.
- [`docs/platform/FABLE_5_AUDIT_PROGRAM.md`](../../docs/platform/FABLE_5_AUDIT_PROGRAM.md) → program **COMPLETE**, merge-order quick start, resume at T-090.1.2.8.
- [`fable_audit_operator_resume.md`](fable_audit_operator_resume.md) → post-program one-pager.
- [`t090_091_map_terrain_program.md`](../../docs/specs/Mission_Creator_Architecture/t090_091_map_terrain_program.md) → **RESUMED**, T-090.1.2.8 **NEXT**.
- Registry: **T-128 shipped** (`shipped_at: "T-128"` — tag; post-merge doc sync may swap in the final main SHA per T-126 precedent), **T-090 summary unpaused**. `./scripts/ticket sync && ./scripts/ticket check` clean.

## Merge notes (operator)

1. Merge order: **`ticket/T-127` first**, then `ticket/T-128`.
2. **`registry.json` conflict expected** (both branches edit it): keep T-127's row from main **and** T-128 + T-90 rows from this branch; then on main run `./scripts/ticket sync && ./scripts/ticket check` (regenerates `docs/TICKET_*.md` + CLAUDE marker block into a consistent post-program state — in this branch the generated block intentionally still shows T-127 active).
3. `CLAUDE.md` may also conflict trivially (marker block + narrative); the post-merge sync + this branch's narrative ("Fable COMPLETE / T-090 RESUMED") is the intended final state.
4. `MissionCreatorPage.tsx` — one comment line changed here (Arland 4.096km); trivial conflict possible if T-127 touched the file.
5. `rmdir apps/website/internal/handlers/missions` on the main checkout (see above).

## Known leftovers (logged, not broken links)

- Root `CLAUDE.md` intro prose mentions `docs/backend/architecture.md` (actual: `docs/website/backend/architecture.md`) — code-span mention, not a markdown link; CLAUDE.md §Status narrative is Cursor-owned — flag for next doc sync.
- `docs/specs/Mission_Creator_Architecture/t090_1_2_satellite_backlog.md:35` still describes the Fable program as parallel-active — historical context line, links resolve.
- Eden-wiki scrape typos untouched (verbatim external — audit F5-12 "OK (no fix)").
