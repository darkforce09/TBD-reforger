# Parallel ticket worktrees

| Worktree | Branch | Ticket | Status |
|----------|--------|--------|--------|
| [`TBD-T-127/`](TBD-T-127/) | `ticket/T-127` | MC UX | merged → main |
| [`TBD-T-128/`](TBD-T-128/) | `ticket/T-128` | Doc links | merged → main |
| [`TBD-T-130/`](TBD-T-130/) | `ticket/T-130` | Fable audit remainder | **active** — merge `main` after T-130.0 setup |

**Active parallel pair:**

- **main** — T-090.1.2.8 → T-068 → T-092 ([`t090_1_2_8_SEND_TO_CLAUDE.md`](../t090_1_2_8_SEND_TO_CLAUDE.md))
- **TBD-T-130** — T-130.1 → T-130.6 ([`t130_SEND_TO_CLAUDE.md`](../t130_SEND_TO_CLAUDE.md))

**Create T-130 worktree:**

```bash
git branch ticket/T-130 main
git worktree add .ai/artifacts/worktrees/TBD-T-130 ticket/T-130
```

**Cleanup after merge:**

```bash
./scripts/ticket clean T-130
```

**List worktrees:** `git worktree list` (from repo root)

Operator card: [`.ai/artifacts/t130_audit_operator_resume.md`](../t130_audit_operator_resume.md)
