# Claude Code prompt standard

**Audience:** Cursor (docs/handoffs) · **Consumer:** Claude Code · **`./scripts/ticket run`**

One contract, three files. Prompts drift when each chat reinvents the send-off — this doc fixes that.

---

## Three layers (never merge)

| Layer | Path | Role | Who writes |
|-------|------|------|------------|
| **1. Spec** | `docs/specs/**/t0xx_*.md` | Source of truth — problem, locked decisions, verify gates, acceptance | Cursor |
| **2. Handoff** | `.ai/artifacts/t0xx_claude_code_handoff.md` | Context, preflight, execution phases, file map, return contract | Cursor |
| **3. Send-off** | `.ai/artifacts/t0xx_SEND_TO_CLAUDE.md` | **Optional** one-liner for humans — points at `./scripts/ticket prompt` | Cursor |

**Canonical prompt text lives in the spec** under `## Claude Code prompt — {SLICE_ID}` (fenced block).  
`./scripts/ticket run` and `./scripts/ticket prompt ID` extract that block via `scripts/lib/extract_claude_prompt.py`.

**Do not** put implementation detail only in SEND_TO_CLAUDE — it will rot. SEND_TO_CLAUDE is a bookmark, not a second spec.

---

## Prompt skeleton (copy into every claude-code slice spec)

Use **exact section headers** (`═══ … ═══`) so prompts are scannable and comparable across tickets.

````markdown
## Claude Code prompt — T-0xx.Y (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first.

Implement **T-0xx.Y** — {one-line title}.

═══ PREFLIGHT ═══
  git pull && git lfs pull && make map-assets-link
  ./scripts/ticket brief T-0xx

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t0xx_claude_code_handoff.md
  2. docs/specs/.../t0xx_slice_spec.md
  {optional: 3. path/to/key/source.ts — only if handoff lists it}

═══ PROBLEM ═══
  {2–4 sentences. What's broken. What layer (ortho / FE / API).}

═══ SHIPPED (do not reopen) ═══
  {Parent slices @ commit — one line each}

═══ LOCKED ═══
  {Max 8 bullets — pointer: full table in spec §Locked decisions}
  - …

═══ DO ═══
  1. {P0 / first gate}
  2. {Implementation step}
  3. …
  N. Tag **T-0xx.Y** · commit prefix **T-0xx.Y:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, `docs/TICKET_*.md`, CLAUDE status markers
  - {slice-specific forbidden items}

═══ VERIFY (all exit 0) ═══
  {bash block — copy from spec §Verify / Rebuild}

═══ MANUAL ═══
  {Acceptance IDs — one line each, e.g. S1: …}

═══ RETURN ═══
  - Commit SHA + tag T-0xx.Y
  - {slice artifacts, e.g. verify log path}
  - Automated verify output (PASS)
  - Manual notes for acceptance IDs
  - **Ready for Cursor doc sync.**
```
````

### Section rules

| Section | Max length | Notes |
|---------|------------|-------|
| PROBLEM | 4 sentences | No history essays — handoff has context |
| LOCKED | 8 bullets | Rest stays in spec table |
| DO | 3–12 numbered steps | P0 analysis gates first when applicable |
| DO NOT | Always include doc ban | Plus slice forbidden list |
| VERIFY | Copy spec verbatim | Same commands CI/human will run |
| MANUAL | One line per acceptance ID | Match spec table IDs exactly |
| RETURN | Fixed boilerplate | Always end with **Ready for Cursor doc sync.** |

---

## Handoff artifact standard

See [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md). Handoff is **long-form**; the prompt **references** it, never duplicates it.

Naming: slice `T-090.1.2.2` → `.ai/artifacts/t090_1_2_2_claude_code_handoff.md`  
(slug = `t` + slice id without `T-`, dots → underscores, lowercased)

---

## Operator workflow

```bash
# 1. Cursor: spec + handoff + prompt section in spec; registry active_slice set; ticket sync

# 2. Human: print prompt
./scripts/ticket prompt T-090              # uses active_slice spec
./scripts/ticket prompt T-090 --slice T-090.1.2.3   # explicit slice

# 3. Paste into Claude Code OR
./scripts/ticket run                       # extracts prompt from slice spec (ready tickets)

# 4. After merge: tell Cursor "doc sync for T-0xx.Y"
```

---

## Anti-patterns (why prompts drifted)

| Bad | Good |
|-----|------|
| Prompt only in SEND_TO_CLAUDE.md | Prompt in spec; SEND_TO_CLAUDE points to `ticket prompt` |
| Different headers every slice (`Problem:` vs `PROBLEM:` vs prose) | Fixed `═══` sections |
| Entire spec pasted into prompt | Prompt summarizes; spec + handoff are attachments |
| Cursor writes app code in the prompt | DO section references spec tasks |
| Missing RETURN contract | Always SHA + tag + **Ready for Cursor doc sync.** |

---

## Reference implementation

**T-090.1.2.2** — [`docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md`](../../docs/specs/Mission_Creator_Architecture/t090_1_2_2_sap_cell_seam_repair.md) §Claude Code prompt.

---

## Related

- [`SPEC_TEMPLATE.md`](SPEC_TEMPLATE.md) — new slice specs
- [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md) — handoff artifacts
- [`AI_PLAYBOOK.md`](AI_PLAYBOOK.md) — Mode B recipe
- [`.cursor/rules/cursor-agent-workflow.mdc`](../../.cursor/rules/cursor-agent-workflow.mdc) — Cursor vs Claude split
