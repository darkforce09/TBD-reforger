**Status:** live

# Sub-agent orchestration

When an agent session in the repository hands work to sub-agents, and how. Work happens in the
chat by default: the session reads, edits, builds and tests itself. Sub-agents are a tool for two
cases only, and a multi-agent program runs only when the operator asks for one.

```text
default ─────────▶ the session does the work in the chat
broad search ────▶ one or more read-only explorer agents ─▶ conclusions with path:line citations
parallel pieces ─▶ one agent per file-disjoint piece ─▶ the session reviews each diff
program (asked) ─▶ plan ─▶ each agent's plan approved ─▶ launch ─▶ review ─▶ gates ─▶ commit when asked
```

## When to use it

- **Stay in the chat** for a fix, a feature, a refactor or a question the session can carry in its
  own context, whatever the number of files.
- **A broad search**: answering means sweeping many files, folders or naming conventions and only
  the conclusion matters. Launch read-only explorer agents and keep their conclusions, not the
  file dumps.
- **Parallel independent pieces**: two or more pieces of work that share no file and need no
  answer from each other. Each piece goes to its own agent, run in the background.
- **A multi-agent program**: only when the operator asks for one. Every sub-agent's plan is
  approved before that agent edits anything.

The [orchestrator](/documentation/glossary/n_to_z.md#orchestrator) is the session that launches the
agents. It reviews every report against the tree, runs the gates itself and commits only when the
operator asks.

## Prerequisites

- `CLAUDE.md` and the [documentation standards](/documentation/standards/documentation_standards.md)
  read.
- The log folder `.workstation/logs/`, gitignored; every gate run and agent log goes there, one
  file per command.
- The local stack the gates need, per [local development](/documentation/runbooks/local_development.md).

## Steps

1. Decide the mode from [When to use it](#when-to-use-it). For a broad search or parallel pieces,
   continue at step 3; for a program the operator asked for, start at step 2.

2. Write the program's plan and present it: the task, the verified findings, the operator's
   decisions, one prompt per agent with the files it owns, the order the agents run in, and the
   gates. Each agent's own plan is approved before it edits.

   Expected: the operator's approval in the chat. Nothing is edited before it.

3. Write each agent's prompt so it stands alone: its role, the files it reads, the files it owns
   (and may edit), the checks it runs, the log paths under `.workstation/logs/`, and a report
   limit. An explorer owns no file.

4. Launch the agents in one message when they are independent, in the background. Agents run in
   parallel only when no two own the same file.

   Expected: one task notification per agent when it reports.

5. Review every report against the tree before acting on it:

   ```bash
   git diff --stat
   ```

   Expected: the changed files match the report's list; an edit outside an agent's owned files is
   accepted only after review.

6. Route each finding: **FIX** it now when it turns a check red or is small in a file the work
   owns; **NOTE** it as a [ticket](/documentation/glossary/n_to_z.md#ticket) in the ticket manager
   (`ttm`) when it needs an operator decision or is work of its own; **CLOSE** it with the
   evidence when it is already fixed or not a defect. A FIX goes to the agent that owns the file
   while it still runs, or to a narrow follow-up agent with its own prompt, or is made in the chat
   when it is a line or two.

7. Run the gates once at the end, one command per shell call, each into its own log:

   ```bash
   cargo xtask mk rust-fmt > .workstation/logs/<task>-fmt.log 2>&1
   ```

   then `cargo xtask mk rust-clippy` and the tests of every crate the work touched, plus
   `cargo xtask db test-it` when the API changed.

   Expected: every gate green, or the red routed as a finding.

8. Commit only when the operator asks, staging only the session's own hunks.

## The agent prompt

```markdown
You are agent <ID>, <one-line role>.

Repository <path>, branch main. Never create branches, commit, stage, stash, reset, clean,
checkout or restore. Read CLAUDE.md §1 once.

Reads: <files and line ranges, and the one precedent to copy>.
Owned files: <every file and folder this agent may edit>.
Deliverables: <what each owned file holds when done>.
Checks: <the narrowest commands>, one per shell call, each logged to
.workstation/logs/<ID>-<step>.log.
Findings: FIX / NOTE / CLOSE. Nothing not run is reported as run.

Report in at most <n> words: files changed; commands, exit codes and log paths; findings; what
was not done and why.
```

Shared files (module lists, route tables, README Contents blocks) take only each agent's own
lines, re-read right before each edit and never reformatted. Lock files are never hand-edited.
Tests are never weakened, skipped or given a looser tolerance to make a report green.

## Verify

```bash
git status --short
```

Expected: only the files the work owns are changed, and the gate logs under `.workstation/logs/`
are green.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| a build fails in a file the agent does not own | a parallel agent or another session is mid-edit | the agent waits and retries; after a few tries it reports a blocker and the orchestrator sequences the two |
| a shared file loses another agent's line | an agent formatted or rewrote the file | restore the line; name the file own-lines-only in the prompts |
| the session's context grows large | reports or logs read in full | read logs through a `grep` of their result lines; keep agent reports short |
| an agent's report claims a green the log does not show | the agent summarised instead of running | rerun the named command yourself before accepting the report |

## Related

- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — the gates run at the end.
- [Commit checklist](/documentation/standards/commit_checklist.md) — what a commit carries.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — driving Workbench
  from a session.
