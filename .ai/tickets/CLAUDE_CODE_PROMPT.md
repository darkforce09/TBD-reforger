# Claude Code prompt standard

**Audience:** Cursor (docs/handoffs) · **Consumer:** Claude Code · **`cargo xtask ticket run`**

One contract, three files. Prompts drift when each chat reinvents the send-off — this doc fixes that.

---

## Three layers (never merge)

| Layer | Path | Role | Who writes |
|-------|------|------|------------|
| **1. Spec** | `docs/specs/**/t0xx_*.md` | Source of truth — problem, locked decisions, verify gates, acceptance | Cursor |
| **2. Handoff** | `.ai/artifacts/t0xx_claude_code_handoff.md` | Context, preflight, execution phases, file map, return contract | Cursor |
| **3. Send-off** | `.ai/artifacts/t0xx_SEND_TO_CLAUDE.md` | **Optional** one-liner for humans — points at `cargo xtask ticket prompt` | Cursor |

**Canonical prompt text lives in the spec** under `## Claude Code prompt — {SLICE_ID}` (fenced block).  
`cargo xtask ticket run` and `cargo xtask ticket prompt ID` extract that block from the spec.

**Do not** put implementation detail only in SEND_TO_CLAUDE — it will rot. SEND_TO_CLAUDE is a bookmark, not a second spec.

---

## Prompt skeleton (copy into every claude-code slice spec)

Use **exact section headers** (`═══ … ═══`) so prompts are scannable and comparable across tickets.

````markdown
## Claude Code prompt — T-0xx.Y (copy-paste)

Authority: this spec + handoff. **Do not edit docs or ticket files.**

```
Read CLAUDE.md first.

Implement **T-0xx.Y** — {one-line title}.

═══ PREFLIGHT ═══
  git pull && git lfs pull  # Trunk and the API serve /map-assets straight from assets_v2/
  cargo xtask ticket brief T-0xx

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t0xx_claude_code_handoff.md
  2. docs/specs/.../t0xx_slice_spec.md
  {optional: 3. path/to/key/source.rs — only if handoff lists it}

═══ PROBLEM ═══
  {2–4 sentences. What's broken. What layer (ortho / FE / API).}

═══ SHIPPED (do not reopen) ═══
  {Parent slices @ commit — one line each}

═══ LAYER GATE (engine crates vs Leptos view — MANDATORY) ═══
  website-graphics-engine OWNS: pipelines, shaders, bind groups, draw batching. Zero map concepts.
  website-map-engine OWNS: geometry, LOD, residency, SoA→GPU sync, selection/drag/cluster policy,
  camera math, spatial indexes, terrain formats, the mission document model. Zero Leptos.
  website-frontend ONLY: view components, routing, pointer/keyboard events translated into engine
  commands, and the canvas mount.
  STOP IF: about to add engine policy / streaming / LOD / camera math under frontend/src/v2/
  → put it in apps/website/map-engine instead. Do not "just finish it in the view layer".
  LOC budget: {list frontend files + max lines, e.g. canvas_mount.rs ≤ 60}

═══ LOCKED ═══
  {Max 8 bullets — pointer: full table in spec §Locked decisions}
  - …

═══ DO ═══
  1. {P0 / first gate — prefer Rust}
  2. {Implementation step}
  3. …
  N. Tag **T-0xx.Y** · commit prefix **T-0xx.Y:**

═══ DO NOT ═══
  - Edit docs/**, the `.ai/tickets/T-*.toml` files
  - Grow engine policy inside a frontend bridge or controller module
  - Defer / "fold forward" / invent Out-of-scope for in-scope work unless the operator
    explicitly said "defer X" / "skip X" (see .cursor/rules/no-silent-deferrals.mdc)
  - {slice-specific forbidden items}

═══ VERIFY (all exit 0) ═══
  {bash block — copy from spec §Verify / Rebuild}
  {include wc -l on budgeted frontend files when the LAYER GATE applies}

═══ MANUAL ═══
  {Acceptance IDs — one line each, e.g. S1: …}

═══ RETURN ═══
  - Commit SHA + tag T-0xx.Y
  - {slice artifacts, e.g. verify log path}
  - Automated verify output (PASS)
  - Manual notes for acceptance IDs
  - Ready for Cursor doc sync.
```
````

### Section rules

| Section | Max length | Notes |
|---------|------------|-------|
| PROBLEM | 4 sentences | No history essays — handoff has context |
| **LAYER GATE** | Required on any engine or editor ticket | Omit only for pure docs / non-engine tickets |
| LOCKED | 8 bullets | Rest stays in spec table |
| DO | 3–12 numbered steps | P0 analysis gates first when applicable; **Rust first** |
| DO NOT | Always include doc ban | Plus **no engine policy in the view layer** on engine tickets |
| VERIFY | Copy spec verbatim | Same commands CI/human will run; **LOC budgets** when gated |
| MANUAL | One line per acceptance ID | Match spec table IDs exactly |
| RETURN | Fixed boilerplate | Always end with **Ready for Cursor doc sync.** |

---

## Layer gate (why engine work drifts into the view layer)

Models edit the file they already have open. On an engine ticket that is the Leptos component,
so streaming, LOD and camera policy grows a second home in `frontend/src/v2/` and the two copies
disagree. The boundary is a repo law (CLAUDE.md §1.6), not a preference.

**Cursor (Mode B) must:**

1. Put `═══ LAYER GATE ═══` in **every** engine or editor copy-paste prompt (no exceptions).
2. List **explicit frontend LOC budgets** in VERIFY (`wc -l … ≤ N`).
3. Prefer DO steps that name **the engine crate first**, then the thin view adapter.
4. In DO NOT: ban growing streaming / LOD / camera policy under `frontend/src/v2/`.
5. If a hotfix is needed: **fix it in the engine crate** — do not add a second policy layer.

**Executor must STOP and ask** if the only way they see to ship is 100+ lines of view-layer policy.

---

## Handoff artifact standard

See [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md). Handoff is **long-form**; the prompt **references** it, never duplicates it.

Naming: slice `T-090.1.2.2` → `.ai/artifacts/t090_1_2_2_claude_code_handoff.md`  
(slug = `t` + slice id without `T-`, dots → underscores, lowercased)

---

## Operator workflow

```bash
# 1. Cursor: spec + handoff + prompt section in spec; ticket active_slice set; ticket sync

# 2. Human: print prompt
cargo xtask ticket prompt T-090              # uses active_slice spec
cargo xtask ticket prompt T-090 --slice T-090.1.2.3   # explicit slice

# 3. Paste into Claude Code OR
cargo xtask ticket run                     # extracts prompt from slice spec (ready tickets)

# 4. After merge: tell Cursor "doc sync for T-0xx.Y"
```

---

## Anti-patterns

| Bad | Good |
|-----|------|
| Prompt only in SEND_TO_CLAUDE.md | Prompt in spec; SEND_TO_CLAUDE points to `ticket prompt` |
| Different headers every slice (`Problem:` vs `PROBLEM:` vs prose) | Fixed `═══` sections |
| Entire spec pasted into prompt | Prompt summarizes; spec + handoff are attachments |
| Cursor writes app code in the prompt | DO section references spec tasks |
| Missing RETURN contract | Always SHA + tag + **Ready for Cursor doc sync.** |
| Parallel streams in prose | **Two (or N) full fenced blocks** — see [`.cursor/rules/claude-prompt-delivery.mdc`](../../.cursor/rules/claude-prompt-delivery.mdc) |
| Engine logic in the Leptos view layer | **LAYER GATE** + engine crate first; frontend LOC budget in VERIFY |

---

## Parallel Claude Code sessions

When operator runs **Stream A + Stream B** (e.g. `main` + worktree), Cursor delivers **two complete copy-paste prompts** in chat — one fenced block each. Spec §Claude Code prompt + handoff remain source of truth; chat blocks are the operator-facing copy layer.

---

## Reference implementation

**T-090.1.2.2** — [`documentation_v2/tickets/specs/t090_1_2_2_sap_cell_seam_repair.md`](/documentation_v2/tickets/specs/t090_1_2_2_sap_cell_seam_repair.md) §Claude Code prompt.

---

## Related

- [`SPEC_TEMPLATE.md`](SPEC_TEMPLATE.md) — new slice specs
- [`HANDOFF_TEMPLATE.md`](HANDOFF_TEMPLATE.md) — handoff artifacts
- [`AI_PLAYBOOK.md`](AI_PLAYBOOK.md) — Mode B recipe
- [`.cursor/rules/cursor-agent-workflow.mdc`](../../.cursor/rules/cursor-agent-workflow.mdc) — Cursor vs Claude split
