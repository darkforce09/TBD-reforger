# T-090 parallel setup — water (.2.5) + hillshade (.2.6)

**Created:** 2026-07-03 · **Status:** ready to run

## Streams

| Stream | Slice | CWD | Branch | Touches |
|--------|-------|-----|--------|---------|
| **A** | T-090.1.2.5 water | repo root (`main`) | `main` | `scripts/map-assets/`, `packages/map-assets/` |
| **B** | T-090.1.2.6 hillshade | **shipped** @ `b958e3b4` (merged to `main`) | — | FE only |

No file overlap — safe to run two Claude Code sessions simultaneously.

## Cleanup done

- Removed obsolete `.ai/artifacts/worktrees/TBD-T-130` + branch `ticket/T-130` (T-130 shipped)

## Prompts

```bash
# Stream A — main
./scripts/ticket prompt T-090 --slice T-090.1.2.5

# Stream B — worktree
cd .ai/artifacts/worktrees/TBD-T-090
./scripts/ticket prompt T-090 --slice T-090.1.2.6
```

## After both green

```bash
# Merge hillshade first (FE-only, low conflict risk)
git checkout main
git merge ticket/T-090

# Water may already be on main from stream A
# If water landed on main directly, only merge B

./scripts/ticket done T-090   # optional — only when entire T-090 ticket ships
./scripts/ticket clean T-090  # removes worktree after merge

# Per-slice doc sync (cursor-docs): advance-slice + registry shipped_at
```

## Send-off bookmarks

- A: [`.ai/artifacts/t090_1_2_5_SEND_TO_CLAUDE.md`](t090_1_2_5_SEND_TO_CLAUDE.md)
- B: [`.ai/artifacts/t090_1_2_6_SEND_TO_CLAUDE.md`](t090_1_2_6_SEND_TO_CLAUDE.md)
