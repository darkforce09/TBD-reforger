Continue the TBD Reforger engine-split program. Phases 1 and 2 shipped. Phase 3 is in progress
and PARTIALLY done — read before doing anything.

READ FIRST, in this order:
  CLAUDE.md                                            Core Project Laws 1-9
  .ai/artifacts/engine_split_phase3a/README.md         the four remaining 3A briefs + run order
  docs/platform/engine_split_phase3_baseline.md        what was already red before Phase 3 began
  docs/platform/ENGINE_SPLIT_PROGRAM.md                §4 = Phase 3 (contains errors, see below)
  /home/Samuel/.claude/plans/phase-2-is-done-silly-iverson.md   the approved plan

The plan file OVERRIDES the program document wherever they disagree. The program doc was written
before Phase 2 and has three known errors, all corrected in the plan.

WHERE THINGS STAND — HEAD is 08b160dad, do not re-derive this:
  3A is ~60% done. Eight commits landed `map-engine/src/editing/{picking.rs, tools/, commands/,
  history/, host.rs, batch.rs}` plus gate rules 5 and 6. All green:
      cargo xtask verify engine-layers   ENGINE-LAYERS: PASS (all 8 rules)
      cargo test -p website-map-engine --all-features   1290 passed; 0 failed
      cargo test -p website-frontend                    1316 passed; 0 failed
  Remaining 3A work (~7,200 LOC) is split into four briefs A/B/C/D under
  .ai/artifacts/engine_split_phase3a/. Run order: A first, then B, then C, then D (C must precede
  D). Brief A is a regression fix — the first agent dropped three tests.

HOW TO EXECUTE:
  One general-purpose subagent per brief, ONE AT A TIME, never in parallel. Gate between them:
  run the brief's own verification yourself and confirm green before launching the next.
  Keep each agent's scope to exactly one brief — the first 3A agent was stopped for running too
  long on too wide a scope.

OPERATOR DECISIONS ALREADY MADE — do not re-ask:
  1. T-986 (v-suite oracle re-freeze) is explicitly DEFERRED out of Phase 3. Do not re-freeze any
     oracle. Acceptance is a diff against the committed baseline: set equality on failing route
     names, and the four clean routes (notfound, eventmgr, callback, login) must still pass.
  2. Leave the dirty working tree alone. ~69 files are modified/untracked that are NOT ours
     (apps/mod/** is out of scope per §7, plus CLAUDE.md, xtask/src/*, documentation_v2/,
     tools_v2/). Stage ONLY files you authored, with explicit `git add <path>`. Never `git add -A`.
  3. pages/operations/{orbat_manager,faction_manager}.rs → v2/apps/editor/ui/modals/ (they are
     unrouted editor modals, not pages). Keep distinct from v2/pages/operations/orbat_selection/.
  4. Law 7 ratchets repo-wide in 3C: two thresholds (500 production / 1000 test), SIZE-1 retired,
     ~132 dated grandfather rows, 29 obsolete rows deleted per-file.

  5. **STOP AFTER 3B AND NOTIFY THE OPERATOR.** Do not launch 3C or 3D. They are paused at
     operator instruction — not deferred, not descoped. Resuming needs only the operator's word.

THREE GATES WERE ALREADY RED BEFORE PHASE 3 TOUCHED ANYTHING — not yours, not regressions:
  gate v-suite verify            21 of 25 routes fail (T-986; the doc's "22" is wrong)
  cargo xtask verify file-length exit 1, 9 unallowlisted SIZE-3 — Phase 3C clears them
  cargo test -p xtask            8 flaky failures, all pinning a missing apps/mod/** script
  Because file-length sits inside verify-coding-standards inside ci-local, **ci-local is red
  until 3C lands**. Judge the xtask suite by NAME SET, never count — it drifts run to run.

BUILD AND DISK:
  Every cargo invocation: CARGO_TARGET_DIR=target-container cargo ...   (never bare cargo)
  Never run two cargo commands concurrently — they deadlock on the target-dir lock.
  ALWAYS run cargo from the repo root. Agents have twice created stray target-container/ dirs
  inside the source tree by cd-ing first; ~45G had to be reclaimed. Use ( cd sub && ... ) if a
  different cwd is needed. Check `df -h .`; under ~10G free, stop.
  target-container/debug is currently ~132G. A cargo clean between phases is worth proposing.

COMMITS: directly to main, never branch. Subject suffix (3A)/(3B). End with
  Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>

Start by reading the files above, then launch the agent for brief A.
