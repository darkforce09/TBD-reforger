# Ticket worktrees

| Worktree | Branch | Ticket | Status |
|----------|--------|--------|--------|
| *(repo root)* | `main` | **T-071** or **T-090.1.1.1** | **single lane — pick one** |

**Merged / cleaned:** T-127 · T-128 · T-090-2 · **T-092** (`TBD-T-092` removed 2026-07-04).

---

## When you need a worktree again

```bash
git branch ticket/T-0xx main
git worktree add .ai/artifacts/worktrees/TBD-T-0xx ticket/T-0xx
```

**Cleanup after merge:**

```bash
git worktree remove .ai/artifacts/worktrees/TBD-T-0xx
git branch -d ticket/T-0xx
```

**List worktrees:** `git worktree list` (from repo root)
