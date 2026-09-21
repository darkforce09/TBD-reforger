# Claude Code handoff template

**Path:** `.ai/artifacts/{slug}_claude_code_handoff.md`  
**Slug:** `T-090.1.2.2` → `t090_1_2_2`  
**Prompt:** lives in the **slice spec** §Claude Code prompt — not here. See [`CLAUDE_CODE_PROMPT.md`](CLAUDE_CODE_PROMPT.md).

Copy this skeleton when Cursor creates a new handoff (Mode B).

---

```markdown
# T-0xx.Y — Claude Code handoff ({short title})

**Slice:** T-0xx.Y · **Executor:** claude-code · **Branch:** `main`  
**Parent shipped:** T-0xx.Y-1 @ `{sha}`  
**Spec (authority):** [`docs/specs/.../t0xx_slice.md`](...)

---

## Operator report

{What the human saw — 2–5 bullets. Screenshots referenced by location, not embedded.}

---

## What you are building

{ASCII or bullet pipeline — 3–6 lines max.}

---

## Do not

| Forbidden | Why |
|-----------|-----|
| … | … |
| Edit docs or ticket files | Cursor sync after merge |

**Do not reopen:** {shipped slices @ sha}

---

## Execution order (strict)

1. P0 — …
2. …
N. Tag **T-0xx.Y**

---

## Preflight

\`\`\`bash
git pull && git lfs pull  # Trunk and the API serve /map-assets straight from assets_v2/
cargo xtask ticket brief T-0xx
\`\`\`

---

## Key files

| File | Role |
|------|------|
| `path/to/file` | … |

---

## Verify commands

\`\`\`bash
{copy from spec}
\`\`\`

---

## Manual acceptance

| ID | What |
|----|------|
| **A1** | … |

---

## Return to operator / Cursor

1. Commit SHA + tag T-0xx.Y
2. …
N. **Ready for Cursor doc sync.**
```

---

## Handoff vs spec vs prompt

| Content | Handoff | Spec | Prompt (in spec) |
|---------|---------|------|------------------|
| Locked decisions table | — | ✓ full | bullets only |
| Verify bash | summary | ✓ full | copy |
| File touch list | ✓ table | ✓ table | — |
| Operator screenshot context | ✓ | — | — |
| P0 analysis JSON shape | — | ✓ | step 1 in DO |
| Copy-paste block | — | — | ✓ |

