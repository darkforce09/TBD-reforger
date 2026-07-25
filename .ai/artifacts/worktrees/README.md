# Ticket worktrees

| Worktree | Branch | Ticket | Status |
|----------|--------|--------|--------|
| *(repo root)* | `main` | docs / registry | — |

**No active worktrees.** T-159 (Leptos UI rewrite incl. React deletion) and T-161/T-162
(ticket CLI → Rust xtask + Python eradication) merged to `main` 2026-07-17 (T-163) and their
worktrees + branches were removed.

**Merged / cleaned:** T-127 · T-128 · T-090-2 · T-092 · **T-159** · **T-161/T-162**.

---

## When you need a worktree again

```bash
git branch t-0xx-slug main
git worktree add .ai/artifacts/worktrees/TBD-T-0xx t-0xx-slug
```

**Cleanup after merge:**

```bash
git worktree remove .ai/artifacts/worktrees/TBD-T-0xx
git branch -d t-0xx-slug
```

**List worktrees:** `git worktree list` (from repo root)
