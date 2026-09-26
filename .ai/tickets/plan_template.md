**Status:** live

# T-<id> — Plan

## Context

{Why this ticket, now: the one or two facts, with file:line evidence, that make it next. The plan
is this ticket's own execution document; the shared design stays in its spec. Copy this file to
`documentation_v2/tickets/plans/t-<id>_plan.md` (the id lowercased, dots as underscores) before
`cargo xtask ticket mark-ready`, which refuses while the file is missing. The plan is live while
the ticket is `idea`, `queued` or `ready`; the landing commit sets its status line to
`**Status:** frozen record`. Delete every `{…}` hint.}

## Approach

{The intended steps, in order, naming the files and modules they touch, and the documentation
each step updates in the same commit.}

## Risks

{What could go wrong or invalidate the approach, and the fallback.}

## Verification

{The commands and checks that prove it landed, matching the ticket's `verify` and `acceptance`
fields.}
