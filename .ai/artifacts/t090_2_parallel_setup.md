# T-090 parallel setup — water refine (.2.5.1) + taxonomy (.2)

**Status:** **CLOSED** — both streams shipped (`.2.5.1` @ `82488c6f`, `.2` @ `691d9b26` merged to `main`).

Worktree cleanup (optional):

```bash
git worktree remove .ai/artifacts/worktrees/TBD-T-090-2
git branch -d ticket/T-090-2
```

Historical playbook below.

---

## Streams

| Stream | Slice | CWD | Branch | Touches |
|--------|-------|-----|--------|---------|
| **A** | T-090.1.2.5.1 inland water refine | repo root (`main`) | `main` | `scripts/map-assets/`, `packages/map-assets/everon/staging/sap/`, satellite bundle |
| **B** | T-090.2 map object taxonomy | `.ai/artifacts/worktrees/TBD-T-090-2` | `ticket/T-090-2` | `packages/tbd-schema/**`, `scripts/map-assets/census-types.mjs` |

**No file overlap** — safe to run two Claude Code sessions simultaneously.

**Low merge-conflict risk:** `packages/map-assets/everon/manifest.json` if both slices add fields — rebase B onto A before merge.

---

## Worktree (already created)

```bash
# Created @ 0418d952
git worktree list
# → .ai/artifacts/worktrees/TBD-T-090-2  [ticket/T-090-2]

# Recreate if removed:
git worktree add .ai/artifacts/worktrees/TBD-T-090-2 ticket/T-090-2
```

Before starting stream B after A lands commits on `main`:

```bash
cd .ai/artifacts/worktrees/TBD-T-090-2
git fetch origin && git rebase main
```

---

## Prompts

```bash
# Stream A — main (active_slice in registry)
./scripts/ticket prompt T-090 --slice T-090.1.2.5.1

# Stream B — worktree
cd .ai/artifacts/worktrees/TBD-T-090-2
./scripts/ticket prompt T-090 --slice T-090.2
```

---

## Verify (per stream)

**A — water refine:**

```bash
node scripts/map-assets/verify-sap-ortho.mjs TERRAIN=everon
node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
make verify-terrain
cd apps/website/frontend && npm run build && npm run lint
```

**B — taxonomy:**

```bash
make schema-validate
make map-object-enums-verify
make map-census TERRAIN=everon
```

---

## After both green

```bash
# 1. Merge stream A on main (if not already committed there)
git checkout main
# ... merge / cherry-pick T-090.1.2.5.1 tag

# 2. Rebase taxonomy branch onto updated main
cd .ai/artifacts/worktrees/TBD-T-090-2
git rebase main

# 3. Merge taxonomy (schema-only — low conflict vs A)
git checkout main
git merge ticket/T-090-2

# 4. Cursor doc sync per slice (registry shipped_at + CLAUDE §Status)
# ./scripts/ticket advance-slice T-090   # only when appropriate

# 5. Optional cleanup
git worktree remove .ai/artifacts/worktrees/TBD-T-090-2
git branch -d ticket/T-090-2
```

---

## Send-off bookmarks

| Stream | Send-off | Handoff |
|--------|----------|---------|
| A | [`t090_1_2_5_1_SEND_TO_CLAUDE.md`](t090_1_2_5_1_SEND_TO_CLAUDE.md) | [`t090_1_2_5_1_claude_code_handoff.md`](t090_1_2_5_1_claude_code_handoff.md) |
| B | [`t090_2_SEND_TO_CLAUDE.md`](t090_2_SEND_TO_CLAUDE.md) | [`t090_2_claude_code_handoff.md`](t090_2_claude_code_handoff.md) |

---

## Queue after this parallel pair

1. **T-090.1.1** — Map (.topo) cartographic view (after `.2.5.1` ships)
2. **T-090.3** — Workbench export (needs T-090.2 goldens + classify rules)
3. **T-068 Phase 2** / **T-092** — still gated per program hub
