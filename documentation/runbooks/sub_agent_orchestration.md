**Status:** live

# Sub-agent orchestration

The standard working method for any non-trivial task in the repository. One orchestrating session
researches, asks the operator, writes a plan that holds a shared brief and one prompt per agent,
launches implementing sub-agents as soon as their inputs exist, routes every finding they report,
and runs the gates and the live walkthrough itself. Each agent works on a
file-disjoint slice in a whole context of its own, so the orchestrating session stays lean enough
to carry a cross-crate milestone to the end. API v2 milestone B, planned as thirty agents in eight
waves, is the worked example.

```text
explorers (parallel, read-only) ─▶ planning agent ─▶ operator question rounds
   ─▶ plan: decisions, corrections, brief, prompts, waves, gates ─▶ operator approval
   ─▶ launch each agent when its inputs exist ─▶ report
         ─▶ review diff ─▶ execution record row ─▶ route each finding:
               amend a later prompt │ message the running agent
               follow-up agent <ID>b │ closing-fix batch G<n>
   ─▶ gate between waves ─▶ closing-fix batches
   ─▶ final sweep, live walkthrough
   ─▶ records agent ─▶ commit when the operator asks ─▶ (optional) ship tickets
```

## When to use it

- Any task that touches more than one file with a design decision in it, more than one crate, or
  more than one boundary layer: a milestone, a feature across the API and the single-page app, a
  refactor program, a documentation program.
- It stays in the chat when the work is a one-file fix, a typo, a single red test with an evident
  cause, or a question. The orchestrator's own edits are limited to mechanical fixes of that size
  (formatting, one import line) at any point of a program.
- The [factory waves](/documentation/runbooks/factory_waves/README.md) and the
  [mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) run queued
  [tickets](/documentation/glossary/n_to_z.md#ticket) in git worktrees through
  `cargo xtask platform wave` and `cargo xtask mod wave`. This method runs its agents in the one
  checkout on `main`, kept apart by file ownership instead of worktrees, for work the operator
  plans in the session.

The roles:

| Role | Does | Never does |
|---|---|---|
| operator | answers the question rounds, approves the plan, runs what needs a person (a [Workbench](/documentation/glossary/n_to_z.md#workbench) restart, a dialog), decides every criterion, asks for the commit | — |
| [orchestrator](/documentation/glossary/n_to_z.md#orchestrator) | researches through agents, plans, launches, reviews, routes findings, runs gates and the walkthrough, keeps the records | implements a slice; loosens a criterion; commits unasked |
| explorer agent | reads and reports conclusions with file and line citations | edits |
| planning agent | drafts the plan from the explorer reports and the task | edits the repository |
| implementing agent | builds, tests and documents the files its prompt owns | edits a file outside its list without saying so; runs full suites |
| records agent | writes the register, checkpoint, milestone and ticket records at the close | changes code |

## Prerequisites

- `CLAUDE.md` and the [documentation standards](/documentation/standards/documentation_standards.md)
  read; the operator's standing rules loaded (sub-agent model, token cap per agent, one command per
  shell call).
- A session scratchpad outside the repository, written `<scratchpad>` below, and a gitignored log
  folder for the program's gate runs, written `<logs>` (milestone B used
  `target/api-progress-checkpoint/<date>-<milestone>/`).
- The program's record document in the repository, which holds the execution record and the
  amendments table: a progress checkpoint, a ticket plan or a feature doc's evidence folder.
- The local stack the gates need, per [local development](/documentation/runbooks/local_development.md).

## Steps

### Phase 1: research and questions

1. Launch up to three read-only explorer agents in one message, in parallel: a **code map** (files,
   symbols, registration points, the tests that pin current behaviour), **domain data** (schemas,
   fixtures, game data, external facts) and **verification infrastructure** (gates, test binaries,
   the register, the CI lanes the change touches). Run targeted probes of your own meanwhile, such
   as a Workbench query through the [MCP bridge](/documentation/runbooks/enfusion_mcp_tooling.md).

   Expected: three reports of conclusions with `path:line` citations, no file dumps.

2. Give one planning agent the task, the explorer reports and your probe results.

   Expected: a draft plan in the shape of step 4, with its open questions listed.

3. Ask the operator exhaustive question rounds before planning further: up to four questions per
   round, each with the recommended option first and the evidence for it. Ask again whenever
   research, the planning agent or a later agent report changes the picture.

   Expected: every open design choice answered; the answers are the plan's binding decisions.

### Phase 2: the plan

4. Write the plan file with these sections, in order:

   | Section | Holds |
   |---|---|
   | Context | the task, the program it belongs to, what exists now |
   | Verified findings | the research results, each checked against the code |
   | Binding decisions | the operator's answers, numbered |
   | Double-check corrections | `C1…Cn`: what a second review of the first draft changed and why |
   | Shared brief | verbatim, as the agents will read it ([below](#the-shared-brief)) |
   | Prompt template and roster | one prompt per agent with an S, M or L budget, grouped into waves |
   | Waves and gates | which agents run together, and the gate after each wave |
   | Verification and register spec | each new requirement, its check command, case pattern and marker |
   | Orchestrator runbook | this procedure made concrete: baselines, gates, walkthrough |
   | Risks | what may break the plan, and what happens then |

   Expected: a plan an agent with no other context can execute.

5. Double-check the draft: reread it against the code (yourself or through a reviewing agent) for
   crate direction, host versus wasm builds, gitignored inputs, shared manifest hunks, body limits
   and timing between agents. Record each change as a correction `C<n>` and apply it.

   Expected: the corrections list is non-empty or says why the draft stood.

6. Present the plan for approval. Nothing in the repository is edited before the operator
   approves; an approved plan then runs to the end without pauses for permission on its named steps.

   Expected: the operator's approval in the chat.

### Phase 3: launch

7. Prepare the scratchpad: the brief as `<scratchpad>/brief.md`, the decisions and corrections as
   `decisions.md`, each agent's prompt as `prompt_<ID>.md` (extracted from the plan), and the
   folders `logs/`, `perturb/` and `probes/`. Snapshot every file another session has changed, so
   no agent edits it:

   ```bash
   git ls-files -m -o --exclude-standard -z | xargs -0 -r sha256sum > <scratchpad>/foreign_baseline.sha256
   ```

   Expected: one line per foreign file, or an empty file on a clean tree.

8. Take baselines of the suites the program touches, one command per shell call with its own log,
   and record the counts in the first execution record row. For example:

   ```bash
   cargo xtask db test-it > <logs>/p0-db-test-it.log 2>&1
   ```

   Expected: passed, failed and ignored counts per suite; any red baseline is named before launch.

9. Launch each agent with a one-line prompt, in the background, on the model the operator set for
   sub-agents:

   ```text
   Your full instructions are in <scratchpad>/prompt_<ID>.md. Read it and follow it exactly.
   ```

   Launch dependency-driven, not wave-locked: an agent starts the moment its inputs exist, even
   while its wave's neighbours still run. Agents run in parallel only when no two own the same
   file; the shared registration files of the brief are the one exception.

   Expected: the launch recorded as an execution record row, with any launch-time addition to a
   prompt logged in the amendments table.

### Phase 4: handling reports

10. Review every report against the tree: `git diff --stat`, then the changed files, then a
    spot-check of one claim (a count in a log). Check the foreign snapshot is
    intact:

    ```bash
    sha256sum -c <scratchpad>/foreign_baseline.sha256 --quiet
    ```

    Expected: no output; the diff matches the report's file list.

11. Append one execution record row: what the agent delivered, its counts and logs, and the
    orchestrator's decision on each finding.

12. Route each finding ([routing table](#routing-a-finding)); triage it first as FIX, NOTE or CLOSE.

13. When an agent nears its budget, message it to wrap up and report what is done and not done;
    hand the rest to a fresh narrow agent with its own prompt.

14. An edit outside an agent's owned list is accepted only after review, and the record row names
    it as reviewed and accepted.

### Phase 5: gates and close

15. Between waves, run the gate the plan names for that wave, one command per call, each into its
    own log under `<logs>`. Re-snapshot the foreign baseline when another session has moved.

    Expected: every gate green, or the red routed as a finding before the next wave launches.

16. Run the closing-fix batches near the end: each `closing_fixes*.md` list becomes one agent,
    `G1`, `G2` and on.

17. Optionally, when a green looks too easy, spot-check one new check by perturbation: save the
    target file's hash, plant one deliberate defect, run the narrowest check, record which cases go
    red, restore the file and prove it byte-equal. This is a judgment call, not a required proof:

    ```bash
    sha256sum -c <scratchpad>/perturb/<name>.sha256
    ```

    Expected: `<file>: OK`. A perturbation the session's permission classifier refuses to you is
    given to an agent with the same instructions.

18. Run the final sweep: `cargo xtask mk rust-fmt`, `cargo xtask mk rust-clippy` and the tests of
    every crate the program touched, plus `cargo xtask db test-it` when the API changed, each into
    its own log. `cargo xtask mk leptos-gates` runs only for a risky Mission Creator runtime change;
    no documentation gate is required. Before a large run, check the
    free disk and prune rebuildable caches (incremental folders, stale test binaries).

19. Walk the feature live in a browser, as a user would. Rebuild the single-page app first, since a
    stale dist has produced false differences:

    ```bash
    cargo xtask mk leptos-build
    ```

    Expected: `trunk build --release` finishes. Then walk it online (API and app up), and again
    with the API stopped behind the proxy, which answers 502 rather than a network error. Every
    defect found becomes a closing-fix agent, and the walkthrough is repeated after it lands.

20. Launch the records agent (see [Records](#records)). At each phase boundary, with no agent
    running, write the handoff into the record document, update the session memory, and tell the
    operator the session is safe to compact.

21. Commit and push only when the operator asks. After the commit, ship each delivered ticket in
    the central ticket manager with its landing commit:

    ```bash
    ttm --project reforger ship <ticket> --sha <landing sha>
    ```

    Expected: `<ticket> shipped at <landing sha>`; then `ttm --project reforger check` prints
    `reforger: 0 error(s), <n> warning(s)`.

## The shared brief

The brief is written once, before the first launch, and every prompt points to it. It holds four
parts.

**Rules**

- The repository and branch; never create branches, commit, stage, stash, reset, clean, checkout
  or restore; read the laws of `CLAUDE.md` once.
- Owned files: an agent edits only the files its prompt lists. Files in the foreign snapshot
  belong to another session and are never edited, formatted or staged; a build failing in a file
  the agent does not own is retried a few times, then reported as a blocker.
- Shared registration files (module lists, route tables, README Contents blocks, DTO indexes): an
  agent adds only its own lines, re-reads the file right before each edit, and never reorders or
  reformats it. Lock files are never hand-edited; each manifest hunk has one owner.
- Keep every crate compiling at each save point: write complete files first, add the `mod` line
  last.
- The file-size and documentation guidance: production files around 500 lines, tests around
  1000, sibling test files, present-tense comments without ticket ids, glossary terms, and a
  truthful README for every folder whose surface changes. Tests follow the pre-alpha test policy
  (`CLAUDE.md` law 11).
- Tests are never weakened, skipped, ignored, deleted or given a looser tolerance; missing
  infrastructure is a failure with its cause, never a pass. New suites run twice before the report.
- One build, test or gate per shell call, each to its own log `<scratchpad>/logs/<ID>-<step>.log`,
  read through a `grep` of the result lines; anything longer than 30 seconds runs in the
  background and is waited on, not polled.
- Formatting: never `cargo fmt` without `--check`; format only one's own leaf files; never
  format a file that declares child modules, since that rewrites its neighbours' lines.
- Throwaway probes live in `<scratchpad>/probes/`, never in the repository.
- Before the report: the foreign snapshot check, `git status --short` with every file listed,
  `editorconfig-checker` on the agent's files.
- Findings are triaged FIX, NOTE or CLOSE ([below](#routing-a-finding)); nothing not run is
  reported as run.

**Efficiency**

- Read only what the prompt names, and only the line ranges needed: search first, then read with
  an offset and a limit; never reread a file just edited. The spec is verified; do not re-derive
  it.
- Write whole files in one write; batch independent edits in one message.
- Develop against the narrowest test loop (a name-filtered test of the module); run the crate's
  suite and its linter once at the end. Never run the full-repository suites; those are the
  orchestrator's gates. A build waiting on the cargo lock is normal; never start a second one.
- Stop at the budget line; report in at most 250 words, with tables.

**Spec** — the authoritative facts every agent shares: units, frames, algorithms, contract shapes,
migrations, routes and test-name prefixes, verified during planning.

**Report format** — the numbered sections every report fills, so the orchestrator reviews them in
the same order.

```markdown
# <Program> brief

## Rules
- Repository <path>, branch main. Never create branches, commit, stage, stash, reset, clean,
  checkout or restore. Read CLAUDE.md §1 once.
- Edit only the files your prompt owns. Files in <scratchpad>/foreign_baseline.sha256 are never
  edited, formatted or staged.
- Shared registration files (own lines only, re-read before each edit): <list>.
- <compile-at-every-save-point, file-size and documentation laws, migrations, test rules>
- One command per shell call, output to <scratchpad>/logs/<ID>-<step>.log.
- Formatting: <the formatter and its edition per crate>; never on a file that declares modules.
- Before reporting: sha256sum -c <scratchpad>/foreign_baseline.sha256 --quiet; git status --short;
  editorconfig-checker <your files>.
- Findings: FIX / NOTE / CLOSE, as defined here: <definitions>.

## Efficiency (mandatory)
- <read ranges only; whole-file writes; narrowest test loop; no full-suite runs; stop at budget>

## Spec (authoritative)
<units, algorithms, contracts, migrations, routes, test prefixes>

## Report format (≤ 250 words)
1 files created or changed; 2 commands, exit codes, counts, log paths; 3 tests added per prefix;
4 findings (id, file:line, class, action); 5 not run or deviations, with the reason.
```

An agent prompt is one role line, the pointer to the brief, the body and the budget line. The
budgets are stop lines: S = 150k tokens, M = 250k, L = 350k, all under the per-agent cap.

```markdown
You are agent <ID>, <one-line role> for <program>.

Read <scratchpad>/brief.md first (Rules, Efficiency, Spec, Report format).

Reads: <files and line ranges, and the one precedent to copy>.
Deliverables: <files to create or rewrite, and what each holds>.
Tests: <cases with the name prefix, run twice; the red-first cases>.
Owned files: <every file and folder this agent may edit, the shared registration lines included>.
Checks: <the narrowest commands, then the crate suite and its linter once>.

Budget: <S|M|L> (<tokens>). Stop there and report done and not done.
```

## Routing a finding

Each finding is triaged first:

- **FIX** now: it turns one of the program's checks red, makes a test or gate fail open on the
  program's surface, or is small (up to about 50 lines) in a file the program owns, with a
  red-first test.
- **NOTE**: it needs an operator decision, is a feature or refactor of its own, sits in a foreign
  file, or is large. It becomes a ticket, written by the records agent.
- **CLOSE**: it is already fixed or not a defect; the evidence is cited.

A FIX then takes the narrowest of four routes:

| Route | When | How |
|---|---|---|
| Amend a later prompt | the agent that should carry it has not launched yet | append to its `prompt_<ID>.md`; add a row to the amendments table |
| Message the running agent | the agent that owns the file is still running | send it the finding; log the message as an amendment |
| Follow-up agent `<ID>b`, `<ID>c` | the finding is a narrow slice of its own, its owner has reported | write a new prompt with the finding, the owned files and the checks |
| [Closing-fix batch](/documentation/glossary/a_to_f.md#closing-fix-batch) `G<n>` | small independent items that can wait for the end | queue them in `<scratchpad>/closing_fixes.md`; one agent runs the batch near the close |

## Operator checkpoints and criterion questions

- A step only the operator can do (restarting Workbench on another addon, typing an id into a
  plugin dialog) is an **operator checkpoint** in the plan. The orchestrator asks the moment its
  input exists, then drives the rest itself, such as a Workbench run through the MCP bridge.
- When a check shows a gap between the model and the criterion (residuals over a tolerance, a
  golden one unit off), the orchestrator investigates through an agent and brings the evidence to
  the operator as a criterion question. No agent and no orchestrator loosens a tolerance, weakens
  a test or re-captures a golden to hide a difference; the fix follows the operator's answer.

## Records

- **Execution record**: a dated table in the program's record document, one row per launch,
  report, gate run, operator checkpoint, walkthrough and perturbation set, each naming its logs.
- **Amendments table**: every addition to a pre-written prompt, beyond concrete values filled in
  at launch, as one row per agent: the brief, a prompt amendment, a message, a new follow-up or
  closing-fix agent.
- **Measured counts**: the passed-case counts of the final sweep, kept in
  `<scratchpad>/measured_counts.md`. The register's minimums are set to these counts and are never
  lowered.
- **Records agent**: the last agent writes the register rows, updates the checkpoint, milestone
  and roadmap marks, and files the NOTE findings as tickets. It runs before the commit.

## Verify

The program is closed when the final sweep is green in its logs and the walkthrough passed online
and with the API stopped:

```bash
cargo xtask mk rust-clippy
```

Expected: exit 0; the execution record ends with the commit row.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| a build fails in a file the agent does not own | a parallel agent is mid-edit | the agent waits and retries; after a few tries it reports a blocker and the orchestrator sequences the two |
| a shared registration file loses another agent's line | an agent formatted or rewrote the file | restore the line; add the file to the brief's own-lines-only list |
| the page shows an old answer after a fix | the single-page app dist predates the edit | `cargo xtask mk leptos-build`, then walk again |
| an offline path passes the gate but fails by hand | the gate stops the whole listener, while a proxy in front of a stopped API answers 502 | walk it with the API stopped behind the proxy; add a 502 step to the gate |
| the permission classifier refuses a perturbation or a process stop | the session's guard | hand the step to an agent with the same restore proof |
| a large run fails on space | build caches fill the disk | prune incremental folders and stale test binaries, then rerun |
| the orchestrator's context grows past a comfortable size | reports read in full, gates read unfiltered | read logs through the result `grep`; write the handoff and compact at the next boundary |

## Worked example

API v2 milestone B (game ballistics) ran this method end to end. Its
[execution record](/documentation/archive/api_v2_completion/milestone_b_execution_record.md#milestone-b-execution-record)
shows the phase handoffs, and its
[launch amendments](/documentation/archive/api_v2_completion/milestone_b_execution_record.md#milestone-b-launch-amendments)
the routed findings:

- Prompt amendments: the design note's NOTE findings amended the prompts of three later agents
  before they launched.
- Follow-up agents: B08b took the wind-corrected aim out of the solver agent's findings; B09b
  identified the engine's integrator from the oracle samples when calibration came back red, with
  no tolerance changed.
- Closing-fix batches: G1 to G11, from a queued batch of small items to single defects found by
  the final gates.
- Perturbations: each agent's red cases in its row, and the orchestrator's own set (a drag term
  zeroed, a wind sign flipped), each restored sha256-equal.
- Walkthrough defects: an optional font failure that marked the whole offline pack failed (G5),
  an illumination fuze that tried only the recommended charge (G6), and an offline page that did
  not fall back when the proxy answered 502 for a stopped API (G7).

The plan itself, with the brief in its section 5, the prompt template in section 6, the roster in
section 7, the register spec in section 8 and the orchestrator runbook in section 9, lives outside
the repository in the operator's plan store as
`pasted-content-id-1072-resume-the-radiant-conway.md`.

## Related

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — ticket waves in worktrees
  through `cargo xtask platform wave`.
- [Mod slice workflow](/documentation/runbooks/mod_slice_workflow.md) — the mod's wave of slice
  agents.
- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — the gates of the final sweep.
- [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — driving Workbench at
  an operator checkpoint.
- [Ticket manager client](/tools/foundation/ticket_manager_client/README.md) — the central
  ticket manager, `ttm`, that ships tickets after the commit.
