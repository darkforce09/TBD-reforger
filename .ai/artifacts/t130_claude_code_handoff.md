# T-130 — Claude Code handoff (Fable audit remainder)

**Ticket:** T-130 · **Executor:** claude-code · **Branch:** `ticket/T-130` (worktree)  
**After:** T-128 shipped (tag **T-128**) · **Parallel:** T-090.1.2.8 runs on **main** — do not touch map/satellite work here  
**Spec:** [`docs/platform/t130_fable_audit_remainder.md`](../../docs/platform/t130_fable_audit_remainder.md)  
**Tracker:** [`.ai/artifacts/fable_5_omni_audit_report.md`](fable_5_omni_audit_report.md) — all **OPEN** + **PARTIAL** rows

**Preflight (worktree):**

```bash
cd .ai/artifacts/worktrees/TBD-T-130
git merge main    # stay current with T-128 merge
./scripts/ticket brief T-130
```

---

## Execution order (worktree)

Run slices in order; `./scripts/ticket run` skips cursor-docs rows.

1. **T-130.1** — backend hygiene (F2B-07, 08, 09, 11)
2. **T-130.2** — Discord 429 + webhook limits + OAuth guard (F3-01…03)
3. **T-130.3** — CI scope (F2B-06)
4. **T-130.4** — mod loaders/exporters (F1-16…20)
5. **T-130.5** — MC conflict new-tab + non-UUID trap + admin Dialog (F4-03, F4-07, F2F-07)
6. **T-130.6** — mission archive/delete (F2B-05, F4-04) — largest slice; last

**Do not edit:** `docs/**`, `.ai/tickets/registry.json`, CLAUDE status markers.

---

## T-130.1 — Backend hygiene

| ID | File | Fix |
|----|------|-----|
| F2B-07 | `internal/handlers/missions.go` | Return 500 if `Count` fails; never `{ total: 0, data: [...] }` on count error |
| F2B-08 | `missions.go` `buildMissionDoc` | Propagate load failure — no silent empty payload |
| F2B-09 | `internal/handlers/auth.go` or boot | Purge old revoked refresh rows (document policy) |
| F2B-11 | `internal/middleware/ratelimit.go` | Replace substring prefix match with exact path or safe segment match |

Add/adjust tests mirroring T-126 patterns.

---

## T-130.2 — Discord

| ID | Fix |
|----|-----|
| F3-01 | `services/discord.go` `do()` + `webhook.go`: honor `Retry-After` on 429, bounded retry |
| F3-02 | Truncate embed title 256, footer 2048 runes before POST |
| F3-03 | OAuth authorize: reject/warn when `client_id` blank instead of broken redirect |

Extend `discord_test.go` / `webhook_test.go`.

---

## T-130.3 — CI

| ID | Fix |
|----|-----|
| F2B-06 | `.github/workflows/ci.yml` + `Makefile` `ci-local`: add `go test ./internal/services/... ./internal/middleware/... ./internal/realtime/...` |

---

## T-130.4 — Mod

| ID | File area | Fix |
|----|-----------|-----|
| F1-16 | `TBD_MissionLoader.c` | Explicit error if profile file exceeds cap |
| F1-17 | `TBD_MissionBrowser.c` | Bound list RPC or gate |
| F1-18 | `TBD_*ExportPlugin.c` | Check every `Write` |
| F1-19 | `TBD_RegistryItemsExportPlugin.c` | Fail if zero items resolved |
| F1-20 | Satellite/ortho meta JSON | Escape strings; optional env for Proton path |

---

## T-130.5 — Frontend MC + admin

| ID | Fix |
|----|-----|
| F4-03 | `useMissionEditor.ts` + session/IDB: new-tab cold boot skips conflict when server state already adopted (extend T-127 U1) |
| F4-07 | `MissionCreatorPage.tsx`: hard block or redirect for non-UUID mission ids — not yellow banner only |
| F2F-07 | `pages/admin.tsx`: Aegis `Dialog` instead of `window.confirm` |

---

## T-130.6 — Mission lifecycle

Mirror [`cms.go`](../../apps/website/internal/handlers/cms.go) archive pattern:

- Handler: archive status + soft delete
- FE: library actions for author/admin
- Types + mutations in snake_case

---

## Verify

```bash
go test ./internal/services/... ./internal/middleware/... ./internal/handlers/...
cd apps/website/frontend && npm run build && npm run lint
make test-it   # when db-up
```

**Deliver:** `.ai/artifacts/t130_verify_log.md` (sections per slice)

---

## Return

Commit on **`ticket/T-130`** · prefix **T-130.1:** … **T-130.6:** (or single **T-130:** if one commit)  
Tag **T-130** when merged to main · **Ready for Cursor doc sync** (tracker + registry + T-130.7)
