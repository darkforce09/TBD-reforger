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
       ├──────────── PUSH main to GitHub
       ├──────────── aggressive VERIFY agent on merged main
       └──────────── verify green → dispatch wave N+1 automatically
```

## Rules

1. **One worktree per slice.** `T-181.7` gets one worktree. **Sub-slices stay in the parent's
   worktree** — `T-181.7.1`, `T-181.7.2` are the same slice's work, not new trees.
2. **As many concurrent slices as are FILE-DISJOINT — computed, not guessed.**
   ```bash
   python3 scripts/mod/slice-collisions.py                        # max concurrent set
   python3 scripts/mod/slice-collisions.py T-181.32 T-181.27      # what can join those in flight
   ```
   This rule used to say "three, not more, disk is the constraint". **That was wrong on the
   measurement.** A worktree is ~81 MB fresh and ~500 MB warm; six cost 3.0 GB against 129 GB free,
   so twenty would cost ~10 GB. Disk was never close to binding, and capping at three left the
   program running at roughly half its available width.
   The real limit is the one rule 7 always stated: **file collisions**. Worktrees make concurrent
   edits *safe* (no clobbering) but do not prevent merge conflicts, so two agents must never own
   overlapping paths. That is a mechanical property of the `owns` column, so it is computed.
   Two costs that DO scale with width, and neither justifies three:
   - **Shared-file merges.** Every slice adding a component touches `TBD_GameMode.et` and the
     roll-call in `TBD_FrameworkManager`. Wave 5 hit that at N=3. The conflicts are additive and
     trivial to resolve, but they are O(N).
   - **Integration attention.** Each agent returns a dense report the command center must actually
     read and act on. That is the real ceiling, and it is a lot higher than three.
3. **Merge only when all three complete**, then **delete the worktrees immediately**. Leftover
   worktrees fill the disk — this is an operator instruction, not a preference.
4. **Then run an aggressive verify agent** against merged `main`. It is adversarial: its job is to
   find what the slice agents got wrong, not to confirm they were right.
5. **Push to GitHub after every wave** (`wave.sh land` does it). Work must not be trapped on one
   machine. `git-lfs` is installed on neither the container nor the host and the `pre-push` hook
   exits 2 without it, so the push uses `--no-verify` — but ONLY after confirming no commit
   touches `packages/map-assets/**` (the only LFS-tracked path). If one does, `wave.sh push`
   refuses rather than leaving the remote pointing at LFS objects that were never uploaded.
6. **Verify green → automatically dispatch the next wave.** Do not wait to be asked.
7. **Batch waves by file-disjointness.** The parallelism limit is file collisions, not Workbench.
   `TBD_SpawnManager.c` and `TBD_FrameworkManager.c` are the contended files — never give two
   concurrent agents write access to the same one. Worktrees make edits *safe* (no clobbering) but
   they do **not** prevent merge conflicts, so disjointness still matters.
8. **Agents never self-ship.** They implement, compile-verify, report. The command center owns
   `.ai/tickets/registry.json` and every status transition.
9. **Agents must leave their tree compiling green** and must put throwaway API probes in `/tmp`,
   never in the mod tree. (A dead agent once left `ZZ_Probe.c` in `Scripts/Game/TBD/UI/` and broke
   the build for everyone.)
10. **EVERY agent runs on Opus 5 — slice agents, verifiers, throwaway research agents alike.**
    Operator instruction, verbatim: *"I don't want you to use Sonnet for any of the agents… I don't
    care what it is. Always use Opus five for the agents."* Pass `model: "opus"` explicitly on every
    dispatch. **Do not downgrade to work around API 529s, rate limits, latency or cost** — retry
    Opus, or do the work inline in the command center and say so. This rule exists because the
    command center once moved a blocked adversarial verifier to Sonnet to keep the loop moving:
    reasonable-sounding, but the tradeoff (weaker review of unattended work that then gets merged)
    was the operator's to make, not the agent's. If a tier change ever seems warranted, **ask**.

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
(merge → gate → reap → **push**) → dispatch the verify agent → fix any BLOCKER → `wave.sh prep N+1`.

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
| How is a lobby / slot picker SHAPED? | `rg <pat> apps/mod/playable_selector/` — PlayableSelector, **NO LICENCE, design-mirror only** (§Oracle lanes) |
| Does my change compile? | `bash scripts/mod/compile.sh` — ~1.3 s, native server, **no Workbench** |
| Probe dir hygiene | **Use a FRESH, uniquely-named dir per probe** (`/tmp/probe-$$`). Probe dirs are shared and sticky — a leftover file from another agent silently polluted a run. `compile.sh` now lists what it stages so contamination is visible. |
| Does this API EXIST? (definitive) | `bash scripts/mod/compile.sh --probe=/tmp/p` — call it in a throwaway `.c` under `/tmp`; compiles clean = exists, errors = does not. Never put probes in the mod tree. |

**A GREEN PROBE IS MEANINGLESS WITHOUT A NEGATIVE CONTROL.** Always compile a variant that MUST
fail, and confirm it does. Measured: duplicate `switch` case labels compile clean in Enfusion, so a
T-181.9.1 probe testing enum distinctness passed — **and so did its negative control**. The probe
proved nothing, and only running the control revealed that. If your control passes, your probe
cannot support any conclusion.

**PROBE BY ASSIGNING TO THE EXPECTED TYPE, never by printing — AND ASSIGN TO `string`, NOT `int`.**
A T-181.9.2 probe "proved" `s = s.Replace(a,b)` worked because `Print(s.Replace(...))` compiles —
`Print` accepts an int, and `string.Replace` mutates in place and returns a COUNT.

**The direction matters, and this doc had it wrong until T-181.27 measured it.** Enfusion coerces
**string → int implicitly** (but NOT int → string). So `int n = s.Foo();` compiles whether `Foo`
returns an int or a string — it discriminates nothing, and it was recommended here and used in real
probes. The test that actually discriminates is the other way round:

```c
string out = s.Replace(a, b);   // FAILS: "Incompatible parameter" — so Replace returns an int
string out = s.Trim();          // compiles — so Trim really does return a string
```

Always probe toward the **narrower** type. If a probe direction would compile under both answers, it
is not a probe.

**Two kinds of Enfusion class, two different oracles** — know which you are asking about:

| Kind | Example | Where it lives | How to check it |
|---|---|---|---|
| **Scripted** | `SCR_BaseGameMode`, `SCR_PlayerController` | shipped `.c` source | `enf lookup` / `rg apps/mod/vanilla_reference/Source/` |
| **Native engine** | `BaseWorld`, `Widget`, `IEntity` | compiled into the engine — **no source, not in any index** | `compile.sh --probe=/tmp/p` — the ONLY way |

A native symbol returning `NOT FOUND` from `enf lookup` does **not** mean it doesn't exist. That
confused a slice agent into thinking `BaseWorld.GetBoundBox` was unavailable; a probe proved it is
real. When the index is silent about something that looks engine-level, probe before concluding.

**Do NOT** let an agent rely on training-data knowledge of Enfusion. It is a niche language and the
model's priors are wrong. An agent asked to summarise one CRF file invented four APIs that do not
exist (`RequestSlotChange`, `ReleaseSlot`, `GetInstance`, a wrong base class). That incident is why
every index is mechanically generated and `make verify-oracle` gates prose citations.

## Oracle lanes — what each one is licensed for

`scripts/mod/slice-worktree.sh new` symlinks these into every worktree; they are all gitignored, so
a fresh tree has none of them until that step runs. **They are not equivalent, and the difference is
legal, not stylistic.** Read the row before you read the code.

| Lane | Licence | You may | You may NOT | Missing? |
|---|---|---|---|---|
| `apps/mod/vanilla_reference` | Bohemia game source, carved by `enf carve` | read for behaviour, cite | commit it, ship it | **REFUSE** — tree not handed over |
| `apps/mod/crf_framework` | **Arma Public License** | read, cite (`@idx crf#OnPlayerAuditSuccess`), design-mirror | copy code, reuse asset GUIDs, vendor it | **REFUSE** — tree not handed over |
| `apps/mod/playable_selector` | **NONE AT ALL** | read to understand *design* | copy **a single line**, adapt, redistribute | **WARN** — tree still handed over |

**"No licence" is worse than APL, not better.** APL at least grants terms. PlayableSelector ships
with no licence file, so **default copyright applies and there is no permission to copy, adapt or
redistribute any of it.** It is a **design-mirror only** lane: read it to learn how a lobby /
slot-picker is *shaped*, close it, then write ours. If you find yourself with a PS file open and our
file open beside it, you are already doing it wrong.

**Why PlayableSelector WARNs while the other two REFUSE** (T-181.52). The refusal rule exists for
exactly one failure mode: an agent with no way to *check* an Enfusion API fact will invent one. The
vanilla and CRF lanes are what answer that question, they live inside the repo, and repo tooling
provisions them — so their absence means a broken setup and the tree is withheld. PlayableSelector
answers *design* questions, not API questions; it proves no Enfusion fact, cannot be compiled
against, and lives **outside the repo** on one operator's disk, so on CI or any other machine it is
legitimately absent. Refusing there would break `new` everywhere but that one machine over something
that is not a correctness problem. Absence is loud, and design work that would have cited it must
**stop and ask, not guess**. Point the lane elsewhere with `TBD_PS_ORACLE=/path/to/PlayableSelector-main`.

**The gate.** `make verify-no-crf-leak` (name kept for the wave runner; it now covers every lane)
fails the build on a `CRF_` **or** `PS_` identifier in `apps/mod/tbd-framework/**`, and on any
oracle-only asset GUID reused in ours. Comments naming an oracle are allowed and encouraged —
citing what you design-mirrored is the practice we want; it is the prefix in *code* that fails.

**The other half of the gate is the deploy.** `scripts/mod/deploy-staging.sh` `--exclude`s every
lane from the rsync. The staging server only ever runs `apps/mod/tbd-framework`, so an oracle on
that box is pure licence exposure for zero benefit — and unlike a worktree, the main checkout holds
these as *real directories*, so a missing exclude ships the whole tree. Measured at T-181.52: only
`crf_framework` was excluded, and every deploy was rsyncing **3,797** carved Bohemia source files
to staging.

**Adding an oracle lane means three edits, not one:** the link step in `slice-worktree.sh`, the
prefix in `verify-no-crf-leak.sh`, and the `--exclude` in `deploy-staging.sh`. A lane missing any
of the three is a liability, not a convenience.

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
distrobox-host-exec make verify-no-crf-leak   # no oracle code in prod (CRF + PlayableSelector)
distrobox-host-exec ./scripts/ticket check    # registry valid
distrobox-host-exec cargo test -p tbd-tools --lib enf::
```

## Known-broken, unrelated to slices

`make schema-validate` exits 2 on `PNG decode: Invalid PNG signature`. **Pre-existing and not a
slice regression** — proved by stashing slice edits and re-running on a clean tree. Cause:
`git-lfs` is not installed, so `packages/map-assets/everon/dem/everon-dem-16bit.png` is a 133-byte
LFS pointer rather than an image. The golden-mission checks inside that target still PASS; it is the
later DEM step that dies. Not in the wave gate, so it does not block a wave — but do not mistake it
for something a slice broke.

## What agents cannot do

Nothing here returns a framebuffer. Agents can prove **compilation**, not **appearance** or
**runtime behaviour**. Anything visual — the lobby, the briefing, the spectator view — needs the
operator's eyes. Batch those into one review session rather than interrupting per slice, and never
report a UI slice as "done" when it is only "compiles".
