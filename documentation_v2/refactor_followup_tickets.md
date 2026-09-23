**Status:** live — Documentation V2 follow-up tickets, filed by P1-2b

# Documentation V2 follow-up tickets (filed)

P1-2b files the four tickets below top-level with `cargo xtask ticket` verbs. The ticket id
pattern admits three or more digits (`.ai/tickets/schema.json:18`, the checkmark pattern in
`tools_v2/ticket-engine/src/sync/gap_analysis.rs`), so `ticket add` mints T-1000 onward, and
every id-ordered output puts a four-digit id after the three-digit ones through one shared key,
`ticket_id_order_key` in `tools_v2/ticket-engine/src/store.rs`.

## Filed tickets

| Section | Id | Title | Status | Executor |
|---|---|---|---|---|
| a | T-1000 | Rename scenario to mission across code, data and mod | deferred, order 9811 | none set (claude-code) |
| b | T-1001 | Mod wave gate calls make targets with no Makefile | idea | none set (claude-code) |
| c | T-1002 | Clean up stale local branches, worktrees and ticket statuses | idea | human |
| d | T-1003 | Retire the untyped ticket-tree load path | idea | none set (claude-code) |

- All four are top-level tickets (operator answer to CP1 question 17).
- T-1000 stays deferred until Documentation V2 ends (CP1 question 18).
- T-1002 also reviews the two unmerged branches, prunes the worktree outside the repository and
  reconciles the four ticket statuses (CP1 question 20).

### Filing sequence

```text
cargo xtask ticket add "<a title>" --summary "<a summary>"   # Added T-1000 (idea)
cargo xtask ticket set-status T-1000 deferred                # repacks wave.lock: wave 0 gains T-1000
# .ai/tickets/T-1000.toml: order = 9811, set by hand (see the tooling gap below)
cargo xtask ticket add "<b title>" --summary "<b summary>"   # Added T-1001 (idea)
cargo xtask ticket add "<c title>" --summary "<c summary>"   # Added T-1002 (idea)
cargo xtask ticket add "<d title>" --summary "<d summary>"   # Added T-1003 (idea)
# .ai/tickets/T-1002.toml: executor = "human", set by hand (no verb writes executor)
cargo xtask ticket sync
cargo xtask wave repack                                      # ticket check demands it: wave 0 gains T-1002
cargo xtask ticket check --strict                            # check OK
```

- `wave.lock` changes in one place: wave 0 gains `T-1000` and `T-1002` after `T-981`. The open
  waves, `pack_last`, `[owns]` and `[depends_on]` stay byte-identical.
- `queue.json`, the roadmap's next-work block and the gap-analysis ticket column do not change.
  None of them lists an idea or deferred ticket.

### Tooling gap: set-status deferred on a ticket without an order

- `ticket set-status <id> deferred` keeps the ticket's current order, and writes none when the
  ticket has none (`tools_v2/ticket-engine/src/ops/transitions.rs:79`).
- `ticket check` requires an order on every status except idea
  (`tools_v2/ticket-engine/src/validation/schema.rs:84-89`).
- Every mutating verb runs that check first (`require_check_ok`,
  `tools_v2/ticket-engine/src/validation/command.rs:99`). After the write, `reorder`, `remove`
  and `add` all refuse, so no verb can add the missing order.
- Running `reorder` before `set-status` does not help either. It flips an idea ticket to queued,
  and the post-image gate refuses a queued work ticket without `main_goal`, which no verb writes.

T-1000's `order = 9811` is the value `ticket reorder T-1000 T-981` mints. T-981 holds the
corpus's highest order, 9810.

### Reaching queued

`add` mints `idea`, with a title of at most 10 words and a summary of at most 40. Queuing one of
these tickets needs four things:

- an `order`. `ticket reorder <id> <anchor>` mints one.
- a `main_goal`. The post-image gate refuses a live work ticket without one.
- a non-empty `owns`. `ticket check` requires it on open work
  (`tools_v2/ticket-engine/src/validation/scope.rs:22`).
- a `wave repack`. A queued claude-code work ticket is dispatchable
  (`tools_v2/ticket-engine/src/wave_lock/model.rs:83-88`), and `ticket check` requires
  `wave.lock` to match the tickets.

No ticket verb writes `main_goal`, `owns` or `executor` (verb list:
`tools_v2/xtask/src/commands/ticket/cli.rs`). The owner of `.ai/tickets/*.toml` sets them by hand
from the proposals below.

## a) Rename scenario to mission across code, data and mod

- **Id and status:** T-1000, deferred, order 9811. The operator defers this rename until
  Documentation V2 completes; `deferred` parks it in wave 0, where the wave packer never
  dispatches it.
- **Summary (39 words, as filed):** Rename scenario to mission in our code: identifiers, file
  names, UI strings, /fleet/scenarios routes, contract fields, scenario_id columns via a new
  migration, mod classes, fleet host agent, comments; the engine concept becomes mission header.
  Engine-owned scenarioId and SCR_EScenario* stay.
- **Relations:**
  - A program by nature: the operator decision calls it "a separate program after this one".
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
- **Measured scope:** 2,296 case-insensitive `scenario` lines across `apps/`, `tools_v2/` and
  `contracts_v2/` (excluding `apps/mod/vanilla_reference` and `apps/mod/crf_framework`). This
  includes:
  - `TBD_ScenarioBrowserPanel.c`, `TBD_ScenarioBrowser.layout` and its `.meta`
  - the `/fleet/scenarios` routes (`apps/website/api_v2/src/server_infrastructure/routes.rs:85`)
  - `scenario_id` columns (`apps/website/api_v2/migrations/0053_mission_deployments.sql:7,32`).
    The column value is the engine's `{GUID}path.conf` scenarioId, so only the column name is
    ours to rename.
  - 3 contract schemas: `mission-deployment`, `mission-editor-payload`, `mission`
  - `apps/fleet_host_agent/` (177 lines)
  - `scenarioId` appears in 22 files. It stays: it is the dedicated-server config key.

## b) Mod wave gate calls make targets with no Makefile

- **Id and status:** T-1001, idea.
- **Summary (37 words, as filed):** cargo xtask mod wave gate runs four make targets through
  distrobox-host-exec (mod_ops/wave_execution/execution.rs), but no Makefile is tracked, so those
  steps always fail. Call cargo xtask ci schema-validate, enf citations, enf capability and cargo
  xtask verify no-crf-leak directly.
- **Relations:**
  - T-181's slices T-181.4 and T-181.2.1 introduce the oracle-citation, CRF-leak and capability
    gates; the recipe comments in the deleted Makefile name both slices.
  - Related: T-853 and T-897. Commit `aa67ec6d8` ("T-853 Phase 3 — delete the root Makefile")
    removes the targets and leaves these four callers.
  - No spec yet.
- **Evidence:**
  - `cmd_gate` in `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs` runs
    `make schema-validate`, `make verify-capability`, `make verify-oracle` and
    `make verify-no-crf-leak` through `distrobox-host-exec` (lines 349-368), marks any non-zero
    step FAIL and returns 1 ("GATE: FAIL"). `git ls-files` lists no Makefile anywhere.
  - The deleted recipes (`git show aa67ec6d8^:Makefile`, lines 227-267):
    - `schema-validate` runs the xtask schema gates, now `cargo xtask ci schema-validate`.
    - `verify-oracle` runs `enf -- citations`.
    - `verify-no-crf-leak` runs `cargo xtask verify no-crf-leak`.
    - `verify-capability` runs `enf -- capability`.
  - Nothing in code or CI calls `enf citations` or `enf capability` except these `make` lines
    (`git grep`).
- **main_goal proposal:** `cargo xtask mod wave gate` runs every check through cargo xtask or the
  enf binary and can pass.
- **owns proposal:** `tools_v2/xtask/src/commands/mod_ops/wave_execution/execution.rs` and the
  wave_execution tests.

## c) Clean up stale local branches, worktrees and ticket statuses

- **Id, status and executor:** T-1002, idea, executor `human`. Two of the branches carry unmerged
  commits, and the removals are destructive.
- **Summary (37 words, as filed):** Review slice/T-939.4 (6 commits ahead) and
  scratch/track-b-draft (1 ahead) before deleting, delete the other 42 merged local branches,
  remove the five .ai/artifacts/worktrees slices and the detached
  $HOME/.cache/tbd-engine-reorg/baseline worktree, and reconcile T-212, T-939.2, T-946.55,
  T-946.86 (ready, branches merged).
- **Relations:**
  - Two of the worktrees are slices of T-946, the wave-close tooling program.
  - Related: `CLAUDE.md` Law 2, plus its `slice/<id>` tooling exception (writing brief,
    "Law 2 and its tooling exception").
  - No spec yet.
- **Measured state:**
  - 45 local branches including `main`, so 44 are stale.
  - 42 branches are merged into `main` with no commits ahead: 41 `slice/*` plus
    `probe/T-380-verify`.
  - 2 branches are unmerged: `slice/T-939.4` (6 commits ahead) and `scratch/track-b-draft`
    (1 commit ahead).
  - `git worktree list` shows five clean slice worktrees under `.ai/artifacts/worktrees/`:
    T-212, T-939.2, T-939.4, T-946.55 and T-946.86. A sixth, clean and on a detached HEAD, lives
    outside the repository at `$HOME/.cache/tbd-engine-reorg/baseline`.
  - T-212, T-939.2, T-946.55 and T-946.86 are still `ready` although their slice branches are
    merged. T-939.4 is `ready` with its branch unmerged.
- **Tools:**
  - `cargo xtask platform slice-worktree -- reap`. It removes merged, clean, started slice
    worktrees without `--force`
    (`tools_v2/xtask/src/commands/platform/slice_worktree/drop.rs:89-103`).
  - `-- drop <id>` removes one slice.
  - `git branch -d` for merged branches that have no worktree.
  - `git worktree prune` for the stray registration.
- **main_goal proposal:** Only main and tooling-managed live slice branches remain, every linked
  worktree belongs to an in-flight slice, and no ticket stays `ready` after its slice merges.
- **owns proposal:** `.ai/artifacts/worktrees`, plus the four reconciled ticket files.

## d) Retire the untyped ticket-tree load path

- **Id and status:** T-1003, idea.
- **Summary (38 words, as filed):** Every ticket file is typed, so load_registry's untyped branch
  (registry/mod.rs) never runs on the live tree. Delete it with
  ticket_file_storage::load_toml_tree, FROZEN_27, union_ticket_keys and
  frozen_27_matches_live_corpus, which returns early on typed trees; move or drop the two other
  load_toml_tree tests.
- **Relations:** none; no spec yet.
- **Evidence:**
  - `load_registry` (`tools_v2/ticket-engine/src/registry/mod.rs:17-37`) sends a typed tree to
    `typed_projection::load_phase2_tree` and anything else with `T-*.toml` files to
    `ticket_file_storage::load_toml_tree`
    (`tools_v2/ticket-engine/src/registry/ticket_file_storage/storage.rs:106`).
  - Every one of the 1,462 ticket files carries a `kind =` line, so `tree_is_phase2`
    (`tools_v2/ticket-engine/src/registry/typed_projection.rs:23-39`) is true for the live tree
    and the untyped branch is unreachable there.
  - `FROZEN_27` (`tools_v2/ticket-engine/src/registry/ticket_file_storage/key_contract.rs:5-34`)
    and `union_ticket_keys` (the same file, lines 100-113) have one reader:
    `frozen_27_matches_live_corpus`
    (`tools_v2/ticket-engine/src/registry/ticket_file_storage/tests/ticket_file_storage_tests.rs:3-13`),
    which returns early when `tree_is_phase2` holds.
  - Two more tests call `load_toml_tree`: `toml_roundtrip_is_byte_identical_to_the_registry_document`
    (the same test file, line 178, over a scratch tree `save_toml_tree` writes) and
    `no_ticket_lost_set_equality` (line 205, over the live tree).
- **main_goal proposal:** The registry has one load path, the typed one.
- **owns proposal:** `tools_v2/ticket-engine/src/registry`.
