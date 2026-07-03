# T-092 parallel setup — spawn policy (.1) + mod compile (.2)

**Status:** **OPEN** — worktree ready for Claude Code  
**Active slice:** **T-092.1** → **T-092.2** (sequential on one branch)

---

## Parallel streams

| Stream | CWD | Branch | Slices |
|--------|-----|--------|--------|
| **A** Map cartographic view | repo root (`main`) | `main` | **T-090.1.1** |
| **B** Spawn + mod compile | `.ai/artifacts/worktrees/TBD-T-092` | `ticket/T-092` | **T-092.1** → **T-092.2** |

**No file overlap** — safe for two Claude Code sessions simultaneously.

**Rebase B onto A** before starting or before merge:

```bash
cd .ai/artifacts/worktrees/TBD-T-092
git fetch origin 2>/dev/null; git rebase main
```

---

## Worktree (create / recreate)

```bash
# From repo root — branch tracks main @ setup time
git branch ticket/T-092 main
git worktree add .ai/artifacts/worktrees/TBD-T-092 ticket/T-092
git worktree list
```

Cleanup after merge:

```bash
git worktree remove .ai/artifacts/worktrees/TBD-T-092
git branch -d ticket/T-092
```

---

## Prompts

```bash
# Stream B — start .1
cd .ai/artifacts/worktrees/TBD-T-092
./scripts/ticket prompt T-092 --slice T-092.1

# After tag T-092.1 — continue .2 on SAME branch (no new worktree)
./scripts/ticket prompt T-092 --slice T-092.2
```

Send-off: [`.ai/artifacts/t092_SEND_TO_CLAUDE.md`](t092_SEND_TO_CLAUDE.md)

---

## Verify (per slice)

**T-092.1:**

```bash
bash scripts/mod/tbd-dev-bootstrap.sh
cd packages/tbd-schema && npm run validate
# wb_play M1–M4 (3 elevations + headingDeg)
```

**T-092.2:**

```bash
make test-it
cd packages/tbd-schema && npm run validate
cd apps/website/frontend && npm run build && npm run lint
curl -H "X-Service-Token: …" http://localhost:8080/api/v1/missions/{id}/compiled
```

---

## Doc sync (Cursor, after merge to main)

1. Registry: T-092.1 + T-092.2 → `shipped` + tags
2. Hub + CLAUDE Done bullets
3. `./scripts/ticket sync && ./scripts/ticket check`
4. Update this doc → **CLOSED**

---

## Unblocks

| After | Ticket |
|-------|--------|
| T-092.2 ship | **T-071** ORBAT Manager |
| T-092.2 + T-071.2 + T-068.13 | **T-068.7+** Virtual Arsenal Phase 2 |
