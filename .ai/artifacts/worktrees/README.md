# Parallel ticket worktrees

Created from `main` @ `a6f54ac0` for Fable audit finish (**T-127** + **T-128**).

| Worktree | Branch | Ticket |
|----------|--------|--------|
| [`TBD-T-127/`](TBD-T-127/) | `ticket/T-127` | MC UX — frontend code |
| [`TBD-T-128/`](TBD-T-128/) | `ticket/T-128` | Doc links + staging honesty |

**Merge order:** `ticket/T-127` → `ticket/T-128` → `main`

**Cleanup after merge:**

```bash
./scripts/ticket clean T-127
./scripts/ticket clean T-128
```

**List worktrees:** `git worktree list` (from repo root)
