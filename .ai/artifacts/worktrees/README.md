# Ticket worktrees

| Worktree | Branch | Ticket | Status |
|----------|--------|--------|--------|
| *(repo root)* | `main` | docs / registry | Cursor docs on main |
| **TBD-T-159** | `t-159-leptos-ui` | **T-159** Leptos UI rewrite | **Active** |
| **TBD-T-161** | `t-161-ticket-xtask` | **T-161** Ticket CLI → Rust xtask | **Active** — T-161.1 ready |

**Merged / cleaned:** T-127 · T-128 · T-090-2 · **T-092** (`TBD-T-092` removed 2026-07-04).
README still mentions TBD-T-152 historically; use `git worktree list` for live set.

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
