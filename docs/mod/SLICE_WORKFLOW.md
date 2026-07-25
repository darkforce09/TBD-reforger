# Slice workflow — command center, worktrees, waves

**Operator-defined, binding. Read this before dispatching any slice agent.**
This is the process; [`t181_event_mod_program.md`](t181_event_mod_program.md) is the program;
[`TBD_MOD_DESIGN.md`](TBD_MOD_DESIGN.md) is the north star.

## The shape

The main chat is the **command center**. It never implements slices — it dispatches, integrates,
verifies and sequences. Each slice is implemented by a **full-capability Opus subagent** with a
self-contained prompt, in **its own git worktree**, so the main context stays clear and each slice
gets a whole context window to reason in.

```
main ──┬── wave N ──┬── worktree slice/T-181.7   → agent A
       │            ├── worktree slice/T-181.10  → agent B
       │            └── worktree slice/T-181.14  → agent C
       │                       ↓ all three complete
       ├──────────── merge all three to main
       ├──────────── DELETE the worktrees (disk!)
       ├──────────── aggressive VERIFY agent on merged main
       └──────────── verify green → dispatch wave N+1 automatically
```

## Rules

1. **One worktree per slice.** `T-181.7` gets one worktree. **Sub-slices stay in the parent's
   worktree** — `T-181.7.1`, `T-181.7.2` are the same slice's work, not new trees.
2. **Three worktrees at a time.** Not more. Disk is the constraint (~131 GB free, repo `.git`
   1.7 GB shared across worktrees).
3. **Merge only when all three complete**, then **delete the worktrees immediately**. Leftover
   worktrees fill the disk — this is an operator instruction, not a preference.
4. **Then run an aggressive verify agent** against merged `main`. It is adversarial: its job is to
   find what the slice agents got wrong, not to confirm they were right.
5. **Verify green → automatically dispatch the next wave.** Do not wait to be asked.
6. **Batch waves by file-disjointness.** The parallelism limit is file collisions, not Workbench.
   `TBD_SpawnManager.c` and `TBD_FrameworkManager.c` are the contended files — never give two
   concurrent agents write access to the same one. Worktrees make edits *safe* (no clobbering) but
   they do **not** prevent merge conflicts, so disjointness still matters.
7. **Agents never self-ship.** They implement, compile-verify, report. The command center owns
   `.ai/tickets/registry.json` and every status transition.
8. **Agents must leave their tree compiling green** and must put throwaway API probes in `/tmp`,
   never in the mod tree. (A dead agent once left `ZZ_Probe.c` in `Scripts/Game/TBD/UI/` and broke
   the build for everyone.)

## Deviation from CLAUDE.md, recorded

`CLAUDE.md` says *"commit directly to `main`; never create a branch (single-ticket mode)"*. This
program deviates: slices run on `slice/T-181.x` branches in worktrees and merge back to `main`.
That is an explicit operator decision for T-181 because parallel agents cannot share one tree.
Everything still lands on `main` — no long-lived branches, no PRs.

## Run it programmatically

The cycle is DATA + SCRIPT, not memory. A fresh session — or one resuming after a context
compaction — recovers full state with one command:

```bash
bash scripts/mod/wave.sh status    # which wave, which trees ready, what is blocking, what to do
bash scripts/mod/wave.sh prep 2    # create worktrees for wave 2
bash scripts/mod/wave.sh land      # merge all complete slices -> run gate -> reap trees
bash scripts/mod/wave.sh gate      # the full verification suite on its own
```

`land` is deliberately conservative: it refuses to merge a dirty worktree (uncommitted slice work
would be lost), runs the full gate AFTER merging so a bad slice is caught on `main` immediately, and
only reaps the trees once the gate is green.

Wave membership lives in [`wave_plan.tsv`](wave_plan.tsv) — waves 1–4 are already planned, batched
by file-disjointness with an explicit `owns` column so write-conflicts are visible before dispatch.
The post-merge adversarial reviewer is [`VERIFY_AGENT_PROMPT.md`](VERIFY_AGENT_PROMPT.md).

**The loop, end to end:**
`wave.sh prep N` → dispatch 3 slice agents → `wave.sh status` until all READY → `wave.sh land`
(merge + gate + reap) → dispatch the verify agent → fix any BLOCKER → `wave.sh prep N+1`.

## Worktree mechanics

```bash
bash scripts/mod/slice-worktree.sh new  T-181.7      # create  .ai/artifacts/worktrees/T-181.7
bash scripts/mod/slice-worktree.sh list
bash scripts/mod/slice-worktree.sh merge T-181.7     # merge to main (verifies first)
bash scripts/mod/slice-worktree.sh drop  T-181.7     # delete worktree + branch
bash scripts/mod/slice-worktree.sh reap             # delete every merged slice worktree
```

**Worktrees branch from a commit.** Anything uncommitted on `main` is invisible inside a worktree,
so the factory must be committed before dispatching — otherwise agents get a tree with no
`compile.sh`, no `enf`, no docs, and will flounder.

## Sources agents must use (and only these)

Every slice prompt must point at these. They exist so an agent proves rather than guesses:

| Need | Command / path |
|---|---|
| Does this Enfusion API exist? | `enf lookup <Symbol>` (CRF) · `--index …/vanilla_symbols.tsv` · `…/vanilla_api_classes.tsv` |
| What does vanilla actually DO? | `rg <pat> apps/mod/vanilla_reference/Source/` — real source **with bodies** |
| Fetch more vanilla source | `bash scripts/mod/fetch-vanilla-source.sh <File.c>` — **polite: one person's site, never `--all`** |
| How does a working framework do it? | `rg <pat> apps/mod/crf_framework/` — CRF, Arma Public License, **reference only** |
| Does my change compile? | `bash scripts/mod/compile.sh` — ~1.3 s, native server, **no Workbench** |
| Does this API EXIST? (definitive) | `bash scripts/mod/compile.sh --probe=/tmp/p` — call it in a throwaway `.c` under `/tmp`; compiles clean = exists, errors = does not. Never put probes in the mod tree. |

**Do NOT** let an agent rely on training-data knowledge of Enfusion. It is a niche language and the
model's priors are wrong. An agent asked to summarise one CRF file invented four APIs that do not
exist (`RequestSlotChange`, `ReleaseSlot`, `GetInstance`, a wrong base class). That incident is why
every index is mechanically generated and `make verify-oracle` gates prose citations.

## The environment fact every prompt must carry

Agent shells run inside a **`debian:12` podman container**: glibc 2.36, **no C toolchain**. The real
machine is Bazzite/Fedora (glibc 2.43, gcc). Prefix builds/game binaries with `distrobox-host-exec`
(or `source scripts/lib/hostrun.sh` then `hostrun`).

- in-container `cargo build` → `linker cc not found`
- host-built binary in-container → `GLIBC_2.39 not found`

**Neither means the repo is broken.** State this in every prompt with the *why* — a session that
misread it "fixed" a working toolchain and destroyed 2.6 GB of build artifacts.

## Wave gate (all must pass before the next wave)

```bash
bash scripts/mod/compile.sh                   # 0 clean
distrobox-host-exec make mod-compile-selftest # gate still catches a broken .c
distrobox-host-exec make verify-capability    # 0 UNTRIAGED
distrobox-host-exec make verify-oracle        # every @idx resolves
distrobox-host-exec make verify-no-crf-leak   # no APL code in prod
distrobox-host-exec ./scripts/ticket check    # registry valid
distrobox-host-exec cargo test -p tbd-tools --lib enf::
```

## What agents cannot do

Nothing here returns a framebuffer. Agents can prove **compilation**, not **appearance** or
**runtime behaviour**. Anything visual — the lobby, the briefing, the spectator view — needs the
operator's eyes. Batch those into one review session rather than interrupting per slice, and never
report a UI slice as "done" when it is only "compiles".
