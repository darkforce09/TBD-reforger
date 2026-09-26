# Implementation prompt standard

The shape of the copy-paste prompt that hands one ticket or slice to an AI coding agent, whichever
product runs it, and where that prompt lives. One fixed shape keeps prompts scannable and
comparable across tickets, instead of each chat inventing its own send-off.

## Where the prompt lives

| Layer | Path | Role | Written |
|---|---|---|---|
| **Spec** | `documentation_v2/tickets/specs/t<id>_<subject>.md` | source of truth: problem, locked decisions, verify commands, acceptance, and the prompt block | always, from [`spec_template.md`](/.ai/tickets/spec_template.md) |
| **Handoff** | `.ai/artifacts/<slug>_claude_code_handoff.md` | long-form context: the operator's report, the execution order, the file map | only when the context does not fit in the spec, from [`handoff_template.md`](/.ai/tickets/handoff_template.md) |

**The canonical prompt text is a fenced block in the spec**, under a heading that starts
`## Claude Code prompt` (for example `## Claude Code prompt — <slice id>`).
`cargo xtask ticket prompt <id> [--slice <slice id>]` prints the first fenced block after that
heading; the heading text is fixed by the extractor in `tools_v2/ticket-engine/src/cli/prompt.rs`,
so keep it even when another agent runs the prompt. `--header` also prints the handoff path.

`cargo xtask ticket run` does not read the block: `cargo xtask platform slice-run` gives the agent a
fixed prompt that names the ticket and its spec and tells it to read `CLAUDE.md` and follow
`cargo xtask ticket brief <id>`. A spec therefore stands on its own; the prompt block is for an
agent started by hand in a chat.

The handoff slug is `t`, the slice id without `T-`, dots as underscores, lowercased: slice
`T-<n>.<m>` → `.ai/artifacts/t<n>_<m>_claude_code_handoff.md` (`slice_id_to_artifact_slug` in
`tools_v2/ticket-engine/src/registry/mod.rs`).

## Prompt skeleton

Use the **exact section headers** (`═══ … ═══`), in this order.

````markdown
## Claude Code prompt — <slice id> (copy-paste)

Authority: this spec (and the handoff, when there is one).

```text
Read CLAUDE.md first.

Implement <slice id> — <one-line title>.

═══ PREFLIGHT ═══
  git pull && git lfs pull  # Trunk and the API serve /map-assets straight from assets_v2/
  cargo xtask ticket brief <ticket id>

═══ READ (in order — the spec wins on conflict) ═══
  1. documentation_v2/tickets/specs/<spec file>
  2. .ai/artifacts/<slug>_claude_code_handoff.md   (only when it exists)
  {3. a key source file — only if the spec or handoff lists it}

═══ PROBLEM ═══
  {2–4 sentences: what is broken, in which layer (map engine / app / API / mod).}

═══ SHIPPED (do not reopen) ═══
  {earlier slices @ commit — one line each}

═══ LAYER GATE (engine crates vs the Leptos view — MANDATORY on engine and editor work) ═══
  website-graphics-engine OWNS: pipelines, shaders, bind groups, draw batching. Zero map concepts.
  website-map-engine OWNS: geometry, LOD, residency, SoA→GPU sync, selection/drag/cluster policy,
  camera math, spatial indexes, terrain formats, the mission document model. Zero Leptos.
  website-frontend ONLY: view components, routing, pointer/keyboard events translated into engine
  commands, and the canvas mount.
  STOP IF: about to add engine policy / streaming / LOD / camera math under frontend/src/v2/
  → put it in apps/website/map-engine instead. Do not "just finish it in the view layer".
  LOC budget: {the frontend files and their maximum line counts}

═══ LOCKED ═══
  {at most 8 bullets — the full table is the spec's Locked decisions}

═══ DO ═══
  1. {the first gate or analysis step — prefer Rust}
  2. {implementation step}
  …
  N. Update the documentation the change touches, in the same commit: comments, the README.md
     of every folder whose contents or surface change, the feature docs whose behaviour changes.
     Commit subject names <slice id>.

═══ DO NOT ═══
  - Change the ticket's status or ship it — the operator ships
  - Grow engine policy inside a frontend bridge or controller module
  - Defer / "fold forward" / invent out-of-scope for in-scope work unless the operator
    explicitly said "defer X" / "skip X" (.cursor/rules/no-silent-deferrals.mdc)
  - {slice-specific forbidden items}

═══ VERIFY (all exit 0) ═══
  {the spec's Verify block, verbatim}
  {wc -l on budgeted frontend files when the LAYER GATE applies}

═══ MANUAL ═══
  {acceptance ids — one line each}

═══ RETURN ═══
  - Commit sha (subject names <slice id>)
  - The documentation files the commit updated
  - Automated verify output (PASS)
  - Manual notes for each acceptance id
```
````

### Section rules

| Section | Limit | Notes |
|---|---|---|
| PROBLEM | 4 sentences | no background essay — the handoff or the spec holds context |
| LAYER GATE | required on engine and editor tickets | omit only for work outside the engine crates and the editor |
| LOCKED | 8 bullets | the rest stays in the spec's table |
| DO | 3–12 numbered steps | analysis gates first where they apply; Rust first; the last step is the documentation and the commit |
| DO NOT | always the status and deferral bans | plus **no engine policy in the view layer** on engine tickets |
| VERIFY | the spec's block, verbatim | the commands CI and the operator run; LOC budgets when gated |
| MANUAL | one line per acceptance id | ids match the spec's table exactly |
| RETURN | fixed | sha, updated documentation, verify output, manual notes |

## The layer gate

Agents edit the file they already have open. On an engine ticket that is the Leptos component, so
streaming, LOD and camera policy grows a second home in `apps/website/frontend/src/v2/` and the two
copies disagree. The boundary is `CLAUDE.md` law 6, detailed in the
[engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) and enforced by
`cargo xtask verify engine-layers`.

Whoever writes the prompt:

1. puts `═══ LAYER GATE ═══` in **every** engine or editor prompt;
2. lists **explicit frontend line budgets** in VERIFY (`wc -l … ≤ N`);
3. writes DO steps that name **the engine crate first**, then the thin view adapter;
4. bans, in DO NOT, growing streaming, LOD or camera policy under `apps/website/frontend/src/v2/`;
5. fixes a hotfix **in the engine crate**, never with a second policy layer.

**The executing agent stops and asks** when the only way it sees to ship is 100 or more lines of
view-layer policy.

## Operator workflow

```bash
# 1. Spec with its prompt block, plan, (handoff); ticket ready — see agent_playbook.md
cargo xtask ticket mark-ready <ticket id> <spec path>

# 2. Print the prompt for an agent in a chat
cargo xtask ticket prompt <ticket id>                         # the active slice's spec
cargo xtask ticket prompt <ticket id> --slice <slice id>      # an explicit slice

# 3. Or run the ready queue through the agent command
cargo xtask ticket run --dry-run
cargo xtask ticket run
```

## Anti-patterns

| Bad | Good |
|---|---|
| The prompt only in a chat or a side file | the prompt in the spec, where `ticket prompt` finds it |
| Different headers every slice (`Problem:` vs `PROBLEM:` vs prose) | the fixed `═══` sections |
| The whole spec pasted into the prompt | the prompt summarizes; the spec and handoff are read first |
| Code written into the prompt | DO steps point at the spec's tasks |
| A RETURN without the documentation it updated | sha, updated documentation, verify output, manual notes |
| Parallel streams in prose | one full fenced block per stream ([`claude-prompt-delivery.mdc`](/.cursor/rules/claude-prompt-delivery.mdc)) |
| Engine logic in the Leptos view layer | LAYER GATE, the engine crate first, a frontend line budget in VERIFY |

## Parallel sessions

When the operator runs two streams at once, whoever writes the prompts delivers **two complete
copy-paste prompts** in the chat, one fenced block each. The spec's prompt block and the handoff
stay the source; the chat blocks are the copy the operator pastes.

## Related

- [`spec_template.md`](/.ai/tickets/spec_template.md) — a new spec.
- [`handoff_template.md`](/.ai/tickets/handoff_template.md) — a handoff.
- [`agent_playbook.md`](/.ai/tickets/agent_playbook.md) — the recipe around them.
- [`cursor-agent-workflow.mdc`](/.cursor/rules/cursor-agent-workflow.mdc) — which Cursor mode
  writes the prompt.
