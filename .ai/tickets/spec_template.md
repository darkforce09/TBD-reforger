**Status:** live

# T-<id> — {title}

{Copy this file to `documentation_v2/tickets/specs/t<id>_<subject>.md`: the ticket id without
`T-`, dots as underscores, then a snake_case subject. Name that path in the ticket's `spec` field
with `cargo xtask ticket mark-ready <id> <spec path>`. The spec stays live, and is corrected as the
design settles, while the ticket is `idea`, `queued` or `ready`. When the ticket ships or is
cancelled, the landing commit sets the status line to `**Status:** frozen record`, and from then
on only the spec's links change; the knowledge that outlasts the ticket moves into the feature doc
of the code it describes. Links are repository-root (`/documentation_v2/…`, `/apps/…`); no
personal absolute path and no host address. Delete this paragraph and every `{…}` hint.}

## In one sentence

{One sentence: the goal.}

## Problem

{What is broken or missing today, with file:line evidence from this checkout.}

## Goal

{Numbered acceptance criteria; each one names how it is checked.}

## Out of scope

- {Only another ticket that owns the surface, named with its id and the spec section that says
  so, or a deferral the operator gave in words, quoted. Nothing else is out of scope.}

## Locked decisions

| Decision | Choice |
|---|---|
| {decision} | {choice, with the reason} |

## Tasks

1. {file or module} — {change}
2. …

## Verify

```bash
cargo xtask mk ci-local-leptos
cargo xtask ticket check --strict
```

{Keep the commands that apply: `cargo xtask mk ci-local-leptos` for the app,
`cargo xtask db test-it` for the API or the database, `cargo xtask mk leptos-gates` for the Mission
Creator, `cargo xtask mod compile` for the mod, `cargo xtask verify link-check --path <folder>` for
documentation. The ticket's `verify` field lists the same commands.}

**Manual:**

- {checklist item, with its acceptance id}

## Documentation

{The documents this ticket changes, updated in the same commit as the code they describe: the
comments of the code it alters, the README.md of every folder whose contents, surface, commands or
boundaries change, the feature docs whose behaviour changes, and the `## Open work` line that
links this ticket. The [commit checklist](/documentation_v2/standards/commit_checklist.md) lists
them by kind of change.}

## Claude Code prompt — T-<id> (copy-paste)

{Optional: the prompt an agent in a chat receives, per
[`implementation_prompt.md`](/.ai/tickets/implementation_prompt.md).
`cargo xtask ticket prompt <id>` prints the first fenced block after this heading; keep the heading
text. Delete the section when no chat prompt is needed.}

```text
Read CLAUDE.md first.

Implement T-<id> — {one-line title}.

═══ PREFLIGHT ═══
  git pull
  cargo xtask ticket brief T-<id>

═══ READ (in order — the spec wins on conflict) ═══
  1. documentation_v2/tickets/specs/t<id>_<subject>.md

═══ PROBLEM ═══
  {2–4 sentences}

═══ SHIPPED (do not reopen) ═══
  - …

═══ LOCKED ═══
  - See the spec's Locked decisions

═══ DO ═══
  1. …
  N. Update the documentation the change touches, in the same commit; the subject names T-<id>.

═══ DO NOT ═══
  - Change the ticket's status or ship it — the operator ships
  - Defer or fold forward in-scope work without the operator's explicit "defer X"

═══ VERIFY (all exit 0) ═══
  {the Verify block above}

═══ MANUAL ═══
  - …

═══ RETURN ═══
  - Commit sha
  - The documentation files the commit updated
  - Verify output (PASS)
  - Manual notes for each acceptance id
```
