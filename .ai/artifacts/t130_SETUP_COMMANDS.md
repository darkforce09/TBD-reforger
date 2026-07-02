# T-130.0 setup — commands (COMPLETE)

Setup landed on **main** @ T-130.0 tag. Worktree: `.ai/artifacts/worktrees/TBD-T-130`.

**Operator card:** [t130_audit_operator_resume.md](t130_audit_operator_resume.md)

---

## Step 1 — Insert T-130 into `.ai/tickets/registry.json`

Insert **before** the `"id": "T-129"` block (after T-128 closing `},`):

```json
    {
      "id": "T-130",
      "order": 1300,
      "program": "platform",
      "stream": "web-platform",
      "targets": [
        "root",
        "website",
        "mod"
      ],
      "executor": "claude-code",
      "status": "ready",
      "active_slice": "T-130.1",
      "surfaces": [
        "DATA",
        "SHELL",
        "MAP",
        "UI"
      ],
      "impact": [
        "api",
        "ui",
        "security",
        "docs"
      ],
      "title": "Fable audit — remainder (OPEN + PARTIAL)",
      "summary": "Parallel to T-090 queue: drain ~21 OPEN + F4-03 PARTIAL from fable_5_omni_audit_report.md. Slices T-130.1–.6 claude-code on ticket/T-130 worktree; T-130.7 cursor-docs. Hub: docs/platform/t130_fable_audit_remainder.md.",
      "spec": "docs/platform/t130_fable_audit_remainder.md",
      "depends_on": [
        "T-128"
      ],
      "unblocks": [],
      "branch": "ticket/T-130",
      "slice_plan": {
        "T-130.0": {
          "targets": ["root"],
          "executor": "cursor-docs",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "shipped"
        },
        "T-130.1": {
          "targets": ["website"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "ready"
        },
        "T-130.2": {
          "targets": ["website"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        },
        "T-130.3": {
          "targets": ["root"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        },
        "T-130.4": {
          "targets": ["mod"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        },
        "T-130.5": {
          "targets": ["website"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        },
        "T-130.6": {
          "targets": ["website"],
          "executor": "claude-code",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        },
        "T-130.7": {
          "targets": ["root", "website"],
          "executor": "cursor-docs",
          "spec": "docs/platform/t130_fable_audit_remainder.md",
          "status": "queued"
        }
      }
    },
```

Mark **T-130.0** `status: "shipped"` in slice_plan after this setup commit lands (already `"shipped"` above).

---

## Step 2 — Sync + validate

```bash
./scripts/ticket sync
./scripts/ticket check
make ticket-check-strict
```

---

## Step 3 — Worktree

```bash
git branch ticket/T-130 main
git worktree add .ai/artifacts/worktrees/TBD-T-130 ticket/T-130
git worktree list
```

---

## Step 4 — Verify prompts

```bash
./scripts/ticket brief T-130 | head -20
./scripts/ticket brief T-090 | head -20
./scripts/ticket prompt T-130
```

---

## Step 5 — Commit (main)

```bash
git add .ai/tickets/registry.json docs/platform/t130_fable_audit_remainder.md \
  .ai/artifacts/t130_*.md docs/platform/FABLE_5_AUDIT_PROGRAM.md \
  .ai/artifacts/worktrees/README.md .ai/artifacts/fable_audit_operator_resume.md
git commit -m "$(cat <<'EOF'
T-130.0: register Fable audit remainder program + handoffs.

Parallel track to T-090.1.2.8; worktree ticket/T-130 for OPEN findings.

Co-Authored-By: Claude Code <noreply@anthropic.com>
EOF
)"
git tag T-130.0
```

---

## Handoff files (already on disk)

| File | Purpose |
|------|---------|
| `docs/platform/t130_fable_audit_remainder.md` | Spec hub |
| `.ai/artifacts/t130_claude_code_handoff.md` | Claude slice detail |
| `.ai/artifacts/t130_SEND_TO_CLAUDE.md` | Copy-paste prompts batch 1+2 |
| `.ai/artifacts/t090_1_2_8_SEND_TO_CLAUDE.md` | Main queue prompt |
| `.ai/artifacts/t130_audit_operator_resume.md` | Two-track operator card |
