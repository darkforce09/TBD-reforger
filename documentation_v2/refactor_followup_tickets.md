**Status:** live — Documentation V2 follow-up ticket texts (filed by P1-2b)

# P0-4 follow-up tickets (not filed)

Step 3 is blocked: xtask does not compile, so the three tickets below are not filed. Each one
lists its title, target status, summary and relations, and what it needs to reach `queued`.

## Why nothing was filed

1. **xtask does not compile.** `cd <repo> && hcargo build -p xtask` exited 101
   (log: `<scratchpad>/logs/P0-4/build_xtask.log`). All four errors are in the other session's
   untracked `tools_v2/xtask/src/commands/mod_ops/equipment_gameplay/`:

   ```text
   error[E0364]: `validate` is private, and cannot be re-exported
     --> tools_v2/xtask/src/commands/mod_ops/equipment_gameplay/mod.rs:10:16
   error[E0603]: function import `validate` is private
   error[E0603]: function `read` is private   (equipment_gameplay/validation.rs:67 and one more call site)
   error: could not compile `xtask` (bin "xtask") due to 4 previous errors
   ```

2. **`ticket add` would break the registry even when xtask compiles.** It mints `max parent id + 1`
   (`tools_v2/ticket-engine/src/store.rs:142-149`, formatted `T-{:03}` at
   `tools_v2/ticket-engine/src/ops/creation.rs:16`). The highest parent id is `T-999`, so the next
   id is `T-1000`. But `.ai/tickets/schema.json:16-19` allows only three digits
   (`^T-[0-9]{3}(\.[0-9]+)*$`). `ticket check` validates that schema first
   (`tools_v2/ticket-engine/src/validation/runner.rs:8`), and `cmd_add` runs its check preflight
   before it writes (`tools_v2/ticket-engine/src/cli/mutations.rs:17`). So one `add` succeeds and
   then turns every later `ticket check` red, which blocks every mutating verb.
   `tools_v2/ticket-engine/src/sync/gap_analysis.rs:12` (`✅\s*(T-\d{3})`) has the same three-digit
   limit. There are two ways out:
   - widen both patterns to `[0-9]{3,}` before filing. This is a tooling change and is outside P0-4.
   - use `ticket add-child <program> …` under an existing program. A child can never become a
     program itself (`ops/creation.rs:121-123`: a program cannot carry a parent).

3. **The verbs cannot produce `queued`.** `add` and `add-child` write only the title and summary,
   and they mint `idea` (`ops/creation.rs:49`). The summary must be ≤40 words
   (`ops/validation.rs`, post-image summary cap). Reaching `queued` needs four things:
   - an `order`. `ticket reorder <id> <anchor>` mints one (`ops/transitions.rs`, `Queued` arm).
   - a `main_goal`. The post-image gate refuses a live work ticket without one
     (`ops/validation.rs`, queued-tier main_goal).
   - a non-empty `owns`. `ticket check` requires it (`validation/scope.rs:22`).
   - a `wave repack`. A queued claude-code work ticket is dispatchable
     (`wave_lock/model.rs:83-88`), and `ticket check` requires `wave.lock` to match
     (`validation/runner.rs:25-28`).

   No ticket verb writes `main_goal` or `owns` (verb list: `tools_v2/xtask/src/commands/ticket/cli.rs`).
   The owner of `.ai/tickets/*.toml` (P2-3) has to hand-set both fields, and the owner of
   `wave.lock` has to run the repack. `set-status <id> deferred` has no prerequisites
   (`ops/transitions.rs`, `Deferred` arm) and parks the ticket in wave 0 (`wave_lock/model.rs:91-96`).

Filing sequence once xtask compiles. Run one command per call, each with its own log:

```text
hcargo xtask ticket add-child <PARENT> "<title>" --summary "<summary>"   # prints "Added <id>: <title>" (status idea)
# P2-3 (owner of .ai/tickets/*.toml): add main_goal + owns from the proposals below
hcargo xtask ticket reorder <id> <anchor>
hcargo xtask ticket set-status <id> queued
hcargo xtask wave repack                                                # wave.lock owner
hcargo xtask ticket check --strict
```

## a) Rename scenario to mission across code, data and mod

- **Title:** Rename scenario to mission across code, data and mod
- **Status:** queued, as the brief asks. The operator deferred this rename until Documentation V2
  completes. `queued` makes it dispatchable for wave packing, so `deferred` matches that decision
  better. This is a question for the operator.
- **Summary (39 words, the `--summary` value):** Rename scenario to mission in our code:
  identifiers, file names, UI strings, /fleet/scenarios routes, contract fields, scenario_id
  columns via a new migration, mod classes, fleet host agent, comments; the engine concept becomes
  mission header. Engine-owned scenarioId and SCR_EScenario* stay.
- **Relations:**
  - This is a program by nature: the operator decision calls it "a separate program after this
    one". It needs its own top-level id, so it waits on the `[0-9]{3,}` id-pattern fix
    (blocker 2).
  - No existing program fits it as a parent. As a stop-gap, the closest is T-934, the shipped
    website reorganization program.
  - Blocked by Documentation V2, which has no ticket: `documentation_v2/refactor_program_plan.md`.
  - No spec yet.
- **main_goal proposal:** Our code says mission wherever it now says scenario; only engine-owned
  names keep Enfusion's spelling.
- **owns proposal (per phase child):**
  - `apps/website/api_v2/src`, `apps/website/api_v2/migrations`
  - `apps/website/frontend/src`, `apps/website/map-engine/src`
  - `contracts_v2/definitions`
  - `apps/mod/tbd-framework`
  - `apps/fleet_host_agent/src`
- **Measured scope, today:** 2,310 case-insensitive `scenario` lines across `apps/`, `tools_v2/`
  and `contracts_v2/` (excluding `apps/mod/vanilla_reference`, `apps/mod/crf_framework`). This
  includes:
  - `TBD_ScenarioBrowserPanel.c`, `TBD_ScenarioBrowser.layout` and its `.meta`
  - the `/fleet/scenarios` routes (`apps/website/api_v2/src/server_infrastructure/routes.rs:85`)
  - `scenario_id` columns (`apps/website/api_v2/migrations/0053_mission_deployments.sql:7,32`).
    The column value is the engine's `{GUID}path.conf` scenarioId, so only the column name is
    ours to rename.
  - 3 contract schemas: `mission-deployment`, `mission-editor-payload`, `mission`
  - `apps/fleet_host_agent/`
  - `scenarioId` appears in 22 files. It stays: it is the dedicated-server config key.

## b) Mod wave gate calls make targets with no Makefile

- **Title:** Mod wave gate calls make targets with no Makefile
- **Status:** queued
- **Summary (36 words):** The mod wave gate runs four make targets through distrobox-host-exec
  (execution.rs:354-363), but no Makefile is tracked, so those steps always fail. Call cargo xtask
  ci schema-validate, enf citations, enf capability and cargo xtask verify no-crf-leak directly.
- **Relations:**
  - Parent if filed now: T-181. Its slices T-181.4 and T-181.2.1 introduced the oracle-citation,
    CRF-leak and capability gates. The recipe comments in the deleted Makefile name both slices.
  - Related: T-853 and T-897. `aa67ec6d8` ("T-853 Phase 3 — delete the root Makefile") removed
    the targets and missed these four callers.
  - No spec yet.
- **Evidence:**
  - `cmd_gate` in `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs` marks any
    non-zero step FAIL and returns 1 ("GATE: FAIL"). `git ls-files` has no Makefile anywhere.
  - The deleted recipes (`git show aa67ec6d8^:Makefile`, lines 227-267):
    - `schema-validate` ran the xtask schema gates, which are now `cargo xtask ci schema-validate`.
    - `verify-oracle` ran `enf -- citations`.
    - `verify-no-crf-leak` ran `cargo xtask verify no-crf-leak`.
    - `verify-capability` ran `enf -- capability`.
  - Nothing in code or CI calls `enf citations` or `enf capability` except these `make` lines
    (`git grep`).
- **main_goal proposal:** `cargo xtask mod wave gate` runs every check through cargo xtask or the
  enf binary and can pass.
- **owns proposal:** `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs` and the
  wave_execution tests.

## c) Remove stale local branches and slice worktrees

- **Title:** Remove stale local branches and slice worktrees
- **Status:** queued. `executor = human` is suggested: two of the branches carry unmerged commits,
  and the removals are destructive.
- **Summary (35 words):** Law 2 cleanup: 44 local branches besides main (42 slice/*,
  probe/T-380-verify, scratch/track-b-draft) and five linked worktrees under
  .ai/artifacts/worktrees remain. Reap merged slices, review the two unmerged branches, drop the
  worktrees, prune the stray baseline worktree.
- **Relations:**
  - Parent if filed now: T-946, the wave-close tooling program. Two of the worktrees are T-946
    slices.
  - Related: `CLAUDE.md` Law 2, plus its `slice/<id>` tooling exception (writing brief,
    "Law 2 and its tooling exception").
  - No spec yet.
- **Measured state:**
  - `git branch --list` shows 45 branches including main, so 44 are stale. The brief says 45.
  - 42 branches are merged into main with zero commits ahead: 41 `slice/*` plus
    `probe/T-380-verify`.
  - 2 branches are unmerged: `slice/T-939.4` (6 commits ahead) and `scratch/track-b-draft`
    (1 commit ahead).
  - `git worktree list` shows the five slice worktrees under `.ai/artifacts/worktrees/`: T-212,
    T-939.2, T-939.4, T-946.55, T-946.86. All are clean. Their tickets are still `ready`, and 4 of
    the 5 branches are already merged.
  - A sixth linked worktree lives outside the repo: `~/.cache/tbd-engine-reorg/baseline`
    (detached HEAD).
- **Tools:**
  - `cargo xtask platform slice-worktree -- reap`. It removes merged, clean, started slice
    worktrees without `--force`
    (`tools_v2/xtask/src/commands/platform/slice_worktree/drop.rs:89-103`).
  - `-- drop <id>` removes one slice.
  - `git branch -d` for merged branches that have no worktree.
  - `git worktree prune` for the stray registration.
- **main_goal proposal:** Only main and tooling-managed live slice branches remain, and every
  linked worktree belongs to an in-flight slice.
- **owns proposal:** `.ai/artifacts/worktrees`
