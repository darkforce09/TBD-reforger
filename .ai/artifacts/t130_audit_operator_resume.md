# Audit zero-backlog — operator resume

**Two parallel tracks** (confirmed order: registry queue on main + T-130 worktree).

| Track | Checkout | Branch | Next slice | Send-off |
|-------|----------|--------|------------|----------|
| **A — Map queue** | repo root | `main` | **T-090.1.2.5** (after **T-090.1.2.8** @ `db9057ef`) | [t090_1_2_5_SEND_TO_CLAUDE.md](t090_1_2_5_SEND_TO_CLAUDE.md) |
| **B — Audit remainder** | `.ai/artifacts/worktrees/TBD-T-130` | `ticket/T-130` | **T-130.1** | [t130_SEND_TO_CLAUDE.md](t130_SEND_TO_CLAUDE.md) |

**Living tracker:** [fable_5_omni_audit_report.md](fable_5_omni_audit_report.md) · **Spec:** [`docs/platform/t130_fable_audit_remainder.md`](../../docs/platform/t130_fable_audit_remainder.md)

---

## One-time setup (Cursor — run on main)

**Status:** pending until registry + worktree land (see below).

```bash
cd /home/Samuel/Projects/TBD-Reforger

# 1) Apply T-130 registry block (see t130_SETUP_COMMANDS.md or agent commit T-130.0)

# 2) Sync views
./scripts/ticket sync && ./scripts/ticket check && make ticket-check-strict

# 3) Branch + worktree
git branch ticket/T-130 main
git worktree add .ai/artifacts/worktrees/TBD-T-130 ticket/T-130

# 4) Optional cleanup (T-127/T-128 already merged)
./scripts/ticket clean T-127
./scripts/ticket clean T-128
```

---

## Claude Code sessions

### Session 1 — main (Track A)

```bash
cd /home/Samuel/Projects/TBD-Reforger
./scripts/ticket prompt T-090
```

Paste from [t090_1_2_8_SEND_TO_CLAUDE.md](t090_1_2_8_SEND_TO_CLAUDE.md).

### Session 2 — worktree (Track B, batch 1)

```bash
cd .ai/artifacts/worktrees/TBD-T-130
./scripts/ticket prompt T-130
```

Paste **Batch 1** from [t130_SEND_TO_CLAUDE.md](t130_SEND_TO_CLAUDE.md) (T-130.1–.3).

### Session 3 — worktree (Track B, batch 2)

Same checkout — paste **Batch 2** (T-130.4–.6).

---

## After T-130 code merges

```bash
git checkout main
git merge ticket/T-130
./scripts/ticket sync && ./scripts/ticket check
```

Tell Cursor: **"doc sync for T-130"** → T-130.7 (manifest/schema/docs) + flip tracker OPEN → RESOLVED.

---

## What closes when

| Bucket | Closes via |
|--------|------------|
| OPEN (~21) | **T-130** |
| PARTIAL F4-03 | **T-130.5** |
| DEFERRED (~15) | **T-090** + **T-092** + **T-122 T15** on main queue after .2.8 |

**End state:** living tracker OPEN 0 · PARTIAL 0 · DEFERRED only intentional future ideas.
