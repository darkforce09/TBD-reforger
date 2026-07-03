# Parallel ticket worktrees

| Worktree | Branch | Ticket | Slices | Status |
|----------|--------|--------|--------|--------|
| *(repo root)* | `main` | **T-090** | **T-090.1.1** Map cartographic view | **active** |
| [`TBD-T-092/`](TBD-T-092/) | `ticket/T-092` | **T-092** | **T-092.1** → **T-092.2** (sequential) | **active** |

**Merged / cleaned:** T-127 · T-128 · T-090-2 worktrees removed.

---

## Active parallel pair (2026-07-03)

| Stream | CWD | Prompt |
|--------|-----|--------|
| **Map view** | repo root | [`t090_1_1_SEND_TO_CLAUDE.md`](../t090_1_1_SEND_TO_CLAUDE.md) |
| **Spawn + compile** | `TBD-T-092` | [`t092_SEND_TO_CLAUDE.md`](../t092_SEND_TO_CLAUDE.md) |

Playbooks: [`t090_1_1_parallel_setup.md`](../t090_1_1_parallel_setup.md) · [`t092_parallel_setup.md`](../t092_parallel_setup.md)

**Rebase worktree before merge:**

```bash
cd .ai/artifacts/worktrees/TBD-T-092
git rebase main
```

---

## Create T-092 worktree

```bash
git branch ticket/T-092 main
git worktree add .ai/artifacts/worktrees/TBD-T-092 ticket/T-092
```

**Cleanup after merge:**

```bash
git worktree remove .ai/artifacts/worktrees/TBD-T-092
git branch -d ticket/T-092
```

**List worktrees:** `git worktree list` (from repo root)
