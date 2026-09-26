# Handoff template

The skeleton of a handoff: the long-form context one slice needs when it does not fit in the spec,
such as the operator's report, the execution order and the file map. Most tickets need none; the
spec is the source of truth, and its prompt block (see
[`implementation_prompt.md`](/.ai/tickets/implementation_prompt.md)) points at the handoff rather
than repeating it.

- **Path:** `.ai/artifacts/<slug>_claude_code_handoff.md`, the slug being `t`, the slice id without
  `T-`, dots as underscores, lowercased (slice `T-<n>.<m>` → `t<n>_<m>`).
  `cargo xtask ticket prompt <id> --header` prints the path.
- **Where it sits:** `.ai/artifacts/` holds run records and working files, not documentation; the
  handoff carries no status line, and nothing in it is lasting knowledge. What outlasts the slice
  goes into the feature doc with the code.

Copy the skeleton below into the new file.

````markdown
# <slice id> — handoff ({short title})

**Slice:** <slice id> · **Executor:** claude-code · **Branch:** `main`
**Earlier slice shipped:** <slice id> @ `{sha}`
**Spec (authority):** [`documentation_v2/tickets/specs/<spec file>`](/documentation_v2/tickets/specs/<spec file>)

## Operator report

{What the operator saw — 2–5 bullets. Screenshots referenced by location, not embedded.}

## What you are building

{ASCII or bullet pipeline — 3–6 lines.}

## Do not

| Forbidden | Why |
|---|---|
| … | … |
| Change the ticket's status or ship it | the operator ships after verifying |

**Do not reopen:** {shipped slices @ sha}

## Execution order (strict)

1. {first gate or analysis step}
2. …
N. Update the documentation the change touches and commit with <slice id> in the subject.

## Preflight

```bash
git pull && git lfs pull  # Trunk and the API serve /map-assets straight from assets_v2/
cargo xtask ticket brief <ticket id>
```

## Key files

| File | Role |
|---|---|
| `path/to/file` | … |

## Verify commands

```bash
{copied from the spec's Verify block}
```

## Manual acceptance

| Id | What |
|---|---|
| **A1** | … |

## Return to the operator

1. Commit sha (subject names <slice id>)
2. The documentation files the commit updated
3. Verify output (PASS)
4. Manual notes for each acceptance id
````

## Handoff, spec and prompt

| Content | Handoff | Spec | Prompt (in the spec) |
|---|---|---|---|
| Locked decisions table | — | ✓ in full | bullets only |
| Verify commands | summary | ✓ in full | copied |
| File touch list | ✓ table | ✓ table | — |
| The operator's report and screenshots | ✓ | — | — |
| Analysis output shape | — | ✓ | the first DO step |
| Copy-paste block | — | — | ✓ |
