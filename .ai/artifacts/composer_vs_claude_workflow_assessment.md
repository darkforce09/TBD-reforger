# Assessment: Consolidating planning/docs from Cursor (Composer 2.5) onto Claude Code

## Context

You currently run a two-agent split on this repo: **Cursor/Composer 2.5** owns all planning,
ticket specs, and documentation writes; **Claude Code** is restricted to shipping code in
`apps/website/**` (and a few mod paths) and is explicitly forbidden from touching `docs/`. This
is codified, not informal — `.cursor/rules/*.mdc` hard-gates Cursor out of application code, and
`docs/specs/Mission_Creator_Architecture/agent_execution.md` states Claude Code "Does NOT: Edit
documentation files."

You're considering dropping Cursor and using Claude Code (Sonnet) for both planning/organizing
and execution. You asked for a fact-based case, not a sales pitch, so the findings below come
from three parallel investigations of the actual repo (docs, code scale, and git history) rather
than general opinions about AI coding tools.

## What the evidence actually shows

**1. The split has a measurable, recurring cost.** Across the last 100 commits, roughly a third
to a half carry **zero application code** — they exist purely to keep `registry.json`, spec docs,
and `CLAUDE.md` in sync with what Claude Code already shipped (e.g. `afc01ed`, `446c9c0`,
`cf7d8ad`). On top of that ceremony, there's **recurring rework fixing the sync itself**:
`23065ed` "Fix monorepo path drift in mod scripts and stale doc links", `56a5c74` "Doc sync: fix
stale T-091.2 active refs", `daa4a07` "Doc fix: dedupe T-090.1 row", `eaba893` "Correct
documentation inaccuracies (T-006)", `a47a7b5` fixing an "inverted premise." These are not
edge cases — they're a steady drumbeat across the history. The failure mode is structural: a
planner working in a separate tool, on a separate model's mental snapshot of the repo, has no way
to verify its doc/registry edits against the code's actual current state the way an agent with
live `Read`/`Bash`/`git` access in the same context can.

**2. The "independent planner" benefit isn't clearly showing up.** The premise of a split is
that a dedicated planner does upfront design before execution. But the codebase's own ticket specs
(`docs/specs/Mission_Creator_Architecture/`, 99 files, 71 ticket IDs) show architecture decisions
— the Y.Doc/Zustand binding model, worker-offload compile design, spatial-index/clustering
strategy, chunked IndexedDB persistence — made **inline inside ticket descriptions as they're
written**, not resolved in a separate clean planning pass first. In practice Composer's day-to-day
work is closer to bookkeeping (status flips, registry sync, advancing slices) than architecture
planning. The doc-drift fixes found in git history are corrections of Cursor's own sync output,
not corrections of Claude's code — the cross-check isn't catching the thing it's nominally for.

**3. The infrastructure to support single-agent ownership already exists.** `.ai/tickets/registry.json`
is the source of truth (101 tickets; 64 `executor: claude-code`, 32 `cursor-docs`, 3 `workbench`,
2 `human`). `scripts/ticket` already has `brief`, `sync`, `check`, `advance-slice`, and — notably
— a **working headless autonomous runner** (`run`): it builds a sparse worktree per ticket,
extracts the prompt from the spec, and invokes `claude -p ... --max-turns 40 --max-budget-usd 10
--permission-mode dontAsk`, then auto-commits and flips status to `review`. This is real,
already-shipped automation — it just currently only fires on rows whose specs Cursor already
wrote. The CI gates (4 parallel jobs: backend/frontend/schema/editorconfig, 38 coding-standards
rules, a custom `@contract` citation checker) are author-agnostic — they don't care who wrote the
commit, only whether it's correct. T-123 already proved the "same agent, same commit" pattern
works: Claude Code writes in-code `@contract`/`@route`/Godoc/TSDoc tags itself, as an explicit
carve-out from the "Cursor owns all docs" rule.

**4. Scale is well within a single agent's working set, ticket by ticket.** Total source is
~36,000 LOC / ~244 files (Go backend ~9,900 LOC/72 files, frontend ~19,900 LOC/133 files, mod
~6,400 LOC/39 files). The heaviest area, Mission Creator (`tactical-map` + `mission-creator`), is
~10,200 LOC/74 files — but individual shipped slices are tightly scoped (commit diffs of 2-7
files is typical; `da78452`, `a85f16b`, `21ec91e`). This is a codebase that moves in small,
well-specified increments, which is exactly the shape a single agent with read+write+verify
access in one context handles cleanly — there's no evidence of tickets large enough to need a
separate dedicated planning pass to stay coherent.

**5. Process hygiene is already loose, which undercuts the "two-agent rigor" argument.**
Co-author trailers are inconsistent — four different Cursor trailer formats, three different
Claude formats, no enforced template — and commit cadence is bursty (2/33/3/35/27 commits across
5 days), consistent with batch ticket-cranking rather than steady, carefully sequenced planning
handoffs. The split isn't currently buying disciplined pacing either.

## Net assessment

The case for consolidating is real and specific to this repo, not generic: **the split's
main cost (sync ceremony + drift rework) is measurable in your own git history, and its main
claimed benefit (independent planning/cross-check) isn't visibly happening** — architecture
calls are made inline by whoever writes the ticket spec, and the doc errors being caught are
self-inflicted by the sync step, not caught-by-a-second-opinion code errors. A single agent that
plans, implements, and verifies inside one context — able to `grep`/`Read`/run `make ci-local`
against the real current state before writing a spec or flipping a status — removes the specific
failure mode you're paying for today.

This is **not** an argument that a second opinion has no value in general — only that *this
repo's current implementation of "second opinion"* (a separate tool with no execution/verification
access, syncing after the fact) isn't producing one. If you want a cross-check, the more useful
version interactively is to use a Plan-mode review pass for plans, or `/code-review` afterward — both have access to ground truth.

## If you decide to proceed (no code written yet — for your next turn)

These are the concrete things that would need to change; flagging them now so you know the shape
of the follow-up, not doing them yet:

- **Policy:** retire/rewrite `.cursor/rules/*.mdc` (the hard gates keeping Cursor out of app
  code become moot either way) and rewrite `docs/specs/Mission_Creator_Architecture/agent_execution.md`
  §Agent roles — flip "Claude Code does NOT edit documentation" to the same-commit pattern T-123
  already validated for in-code tags, extended to spec/registry docs.
- **`docs/website/AGENT_COMMIT_CHECKLIST.md`:** drop the "Cursor syncs later" exception; doc
  updates land in the same commit as the code that needed them, like in-code tags already do.
- **`.ai/tickets/registry.json`:** the 32 `cursor-docs`-executor rows need either reassignment to
  `claude-code` or a decision about who authors brand-new ticket specs going forward (this is the
  one genuinely open design question — see below).
- **`scripts/ticket`:** the `slice_executor` gate logic in `run` would need to either drop the
  `cursor-docs` distinction or repurpose it.
- **CLAUDE.md §Documentation / §Agent split note:** update the 2026-06 "Agent split" callout.

## Revision after pushback (your read of the evidence vs. mine)

You pointed out, correctly, that I conflated two different jobs. The git-history evidence
(doc-drift fixes, sync ceremony) measures **bookkeeping quality**, not **ideation/planning
quality** — and in this repo's actual workflow, Claude Code has never been handed the "babble an
idea → come up with the ticket → write a thorough plan" job. Every ticket arrives pre-decided.
So your impression that Composer is more thorough at that specific step isn't refuted by anything
I found — I just never measured it. I don't have evidence either way on that axis, and you've
used Composer extensively while you've used Claude Code's terminal app for about ten minutes, so
your read on relative planning depth is real data, not noise.

You also flagged that Cursor/Composer renders diagrams inline and this environment "only has the
text for one" — a fair, concrete gap. That turned out to be wrong for *this* surface (a rendered
SVG flowchart was shown live in this conversation comparing the current vs. proposed workflow),
but it's a fair thing to have assumed from ten minutes in a bare terminal, and it's worth
confirming whether the terminal app you've been using renders Mermaid blocks in spec docs at all
— if not, that's a real ergonomics gap worth raising with you separately from the planning-quality
question.

**Proposed resolution — a bake-off, not more argument:** next time you have an idea, babble it to
me directly instead of Composer, and let me run the same kind of process used to produce this
assessment (parallel Explore agents over the real code, then a Plan pass) to turn it into a ticket
spec + implementation plan. Compare that output's thoroughness against what Composer would have
produced for the same idea. That settles the actual open question with a real artifact instead of
a position paper.

## Evidence from your exported Composer chat history

You exported a Cursor chat (`cursor_chat_cursor_project_setup.json`, 112MB, base64-encoded
protobuf blobs) and asked me to read it to understand your workflow. Findings:

1. **The T-092 spec is genuinely strong work** — [`t092_spawn_transform_program.md`](../../docs/specs/Mission_Creator_Architecture/t092_spawn_transform_program.md)
   lines 24-39: a clean 3-subgraph `flowchart LR` plus a table mapping each artifact to its
   consumer/schema, next to a "Why this ticket exists" table citing exact file:line evidence per
   gap. Dense, evidence-grounded, not generic. Credit where due.
2. **The diagram-speed gap is a configured rule, not a Claude Code capability gap.** Composer is
   running one of your own custom rules verbatim: *"Use mermaid and ascii diagrams to explain
   complex logic flows and architecture when appropriate but not for simple changes."* Cursor's
   webview renders mermaid natively/instantly; my SVG widget does real coordinate math first, so
   it's slower but more flexible. Trade-off, not a gap — fixable by giving me the same standing
   instruction.
3. **The workflow is already more bidirectional than "Composer plans, Claude builds."** Found
   multiple turns where you pasted *Claude's* plan output back into Composer for review
   (`"Ready to code? Here is Claude's plan: ..."`, `"Give me a prompt to add those fixes"`) —
   Composer's job there was critiquing a Claude-authored plan. Some of what reads as "Composer
   plans better" may be **the value of a second independent pass**, not Composer's exclusive
   first-pass superiority. A single-agent consolidation loses that cross-check regardless of which
   agent remains.
4. **Your babbling style is high-entropy, voice-dictated, emotionally invested** — long
   stream-of-consciousness turns, repeated insistence on "110%"/"not half-arsing anything," real
   frustration when blocked. Composer's job is partly therapeutic-structuring: turning that into
   the calm, tabular t092-style output. Whoever plans for you needs to handle that input style
   well, not just produce good docs.

**Net effect:** unchanged on the bookkeeping-overhead argument; revised on planning quality — now
grounded against one concrete artifact (t092) as the bar to match, and a found pattern suggesting
the value may be in the cross-check itself rather than Composer specifically.

## One open question — deferred

Composer's one function that doesn't trivially fold into Claude Code's existing same-commit
pattern is **authoring brand-new ticket specs from scratch** (vs. syncing status on existing
ones) — i.e., who decides what T-126+ even is and writes the initial spec doc. You've said you're
not ready to lock this in — fine to leave open; revisit once full consolidation has been tried for
a bit and you have a feel for whether spec-authoring-in-conversation with Claude Code works well
on its own, or whether a dedicated drafting step is still worth keeping.

## Verification (if you proceed)

No code/doc changes are proposed in this turn. If you approve moving forward, the next session's
verification would be: `./scripts/ticket check --strict` after registry edits, `make ci-local`
to confirm nothing in the doc/citation gates breaks, and a spot-check that `scripts/ticket run`
still resolves prompts correctly for rows whose `executor` field changed.
