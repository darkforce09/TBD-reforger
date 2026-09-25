**Status:** live

# Mod slice workflow

How work on the TBD [mod](/documentation_v2/glossary.md#mod) runs: an orchestrating session splits a
[wave](/documentation_v2/glossary.md#wave) of the mod program into slices, one agent builds each
slice in its own git worktree, and `cargo xtask mod wave` merges, gates, reaps and pushes them.
The rules below are operator-defined and binding; read them before dispatching any slice agent.
The program is the [mod program spec](/documentation_v2/tickets/specs/t181_event_mod_program.md),
and the [mod design](/documentation_v2/mod/tbd-framework/mod_design.md) is the standard every slice
meets.

```text
main ──┬── wave N ──┬── worktree slice/<slice A>  ─▶ agent A ─┐
       │            ├── worktree slice/<slice B>  ─▶ agent B ─┼─ each: commit, compile, slice gate
       │            └── worktree slice/<slice C>  ─▶ agent C ─┘
       ├── mod wave land: merge every READY slice ─▶ mod wave gate ─▶ reap the worktrees ─▶ push
       ├── adversarial verify agent on merged main
       └── verify green ─▶ ship the slices ─▶ dispatch wave N+1
```

The orchestrating session (the orchestrator) never implements a slice: it dispatches, integrates,
verifies and sequences, so its context stays clear and each slice agent has a whole context to
reason in.

## Prerequisites

- `CLAUDE.md`, the mod design and the program spec, read.
- Everything the agents need is committed on `main`. A worktree branches from a commit, so
  anything uncommitted on `main` is missing inside it.
- The oracle lanes in the main checkout (see [Oracle lanes](#oracle-lanes)):
  `apps/mod/crf_framework/` and `apps/mod/vanilla_reference/` (both gitignored), and optionally a
  PlayableSelector checkout named by `TBD_PS_ORACLE`.
- The Arma Reforger dedicated server under Steam and a host bridge (`distrobox-host-exec` or
  `host-spawn`), which `mod compile` and the world boot need; the
  [mod commands](/tools_v2/xtask/src/commands/mod_ops/README.md) list the exit codes.
- The wave plan: the program's dotted child [tickets](/documentation_v2/glossary.md#ticket) in
  `.ai/tickets/wave.lock`, each with an `owns` list. `cargo xtask wave repack` is the only writer
  of the lock; the corpus pin `game_mod_programme_ticket` in `.ai/tickets/corpus-pins.toml` names
  the program.
- [Workbench](/documentation_v2/glossary.md#workbench) only for world, prefab and play-in-editor
  work: `cargo xtask mod dev-bootstrap` brings it and the MCP bridge up, as the
  [Workbench MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md#bringing-the-bridge-up)
  describes.

### The environment every prompt carries

Agent shells run in a `debian:12` container: glibc 2.36 and no C toolchain. The host is Fedora
(glibc 2.43, gcc). Builds and game binaries reach the host through `distrobox-host-exec`, through
`cargo xtask`, or through `tools_v2/xtask/src/core/host_execution.rs`. An in-container
`cargo build` fails with `linker cc not found`, and a host-built binary run in the container fails
with `GLIBC_2.39 not found`. Neither means the repository is broken; state this, with the reason,
in every prompt, because an agent that reads either as a broken toolchain destroys working build
output trying to fix it.

## Rules

1. **One worktree per slice.** A sub-slice (a two-dot id such as `T-<n>.<m>.<k>`) is the same
   slice's work and stays in its parent's worktree. `slice-worktree new` maps a sub-slice id to its
   parent and says so; `merge` does the same silently.
2. **Run as many slices at once as are file-disjoint, computed rather than guessed.** Worktrees
   make concurrent edits safe but do not prevent merge conflicts, so two concurrent agents never
   own overlapping paths. `cargo xtask slice-collisions` computes the largest disjoint set from
   the `owns` lists in the wave lock (prefix containment, which errs toward reporting a
   collision), capped at `TBD_MAX_CONCURRENT` (default 8). Disk is not the limit: a worktree costs
   about 81 MB fresh and about 500 MB warm. Two costs grow with width: shared-file merges (each
   slice that adds a component touches `Prefabs/Systems/TBD_GameMode.et` and the roll-call in
   `TBD_FrameworkManager.c`; the conflicts are additive and easy, but one per slice), and the
   orchestrator's attention, since every agent returns a report that must be read and acted on.
3. **Land when every slice of the wave is READY, then delete the worktrees at once.** Leftover
   worktrees fill the disk; `mod wave land` reaps them after a green gate.
4. **Then run an adversarial verify agent** against merged `main`
   ([the prompt](#verify-agent-prompt)). Its job is to find what the slice agents got wrong, not
   to confirm they were right.
5. **Push after every wave.** `mod wave land` ends with the push, so work is never trapped on one
   machine. The push bypasses the git-lfs `pre-push` hook (`--no-verify`) only when no commit in
   the range touches `assets_v2/terrains/`, the only path `.gitattributes` sends to LFS;
   otherwise it refuses.
6. **Verify green: dispatch the next wave** without waiting to be asked.
7. **Never give two concurrent agents write access to the same contended file.**
   `Systems/Spawning/TBD_SpawnManager.c` and `Gamemode/Orchestrator/TBD_FrameworkManager.c` (under
   `apps/mod/tbd-framework/Scripts/Game/TBD/`) are the usual ones. The limit is file collisions,
   not Workbench.
8. **Agents never ship.** They implement, compile, gate their slice and report. The orchestrator
   owns every ticket status change, through `cargo xtask ticket`.
9. **Agents leave their tree compiling green**, and put throwaway API probes under `/tmp`, never in
   the mod tree: a stray probe file in `Scripts/` breaks the compile for every agent.
10. **Every agent in the workflow runs on the operator's chosen model tier** — slice agents,
    verifiers and throwaway research agents alike. Never downgrade to get past rate limits,
    overload errors, latency or cost: retry, or do the work in the orchestrating session and say
    so. The tradeoff of a weaker review of unattended work that then merges is the operator's to
    make; if a tier change seems warranted, ask.

**Branches.** `CLAUDE.md` Law 2 puts every commit on `main` and forbids creating branches. Its one
exception is the `slice/<id>` branch that the xtask slice and wave tooling creates and deletes
itself: `slice-worktree new` creates it from `main`
(`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`), `merge` merges it back with
`--no-ff`, and `drop` and `reap` delete it
(`tools_v2/xtask/src/commands/platform/slice_worktree/drop.rs`). Nothing else creates a branch;
there are no long-lived branches and no pull requests.

## Steps

Run every step from the repository root of the main checkout unless it says the worktree.

1. Find where the program stands. The driver reads the wave lock, the program's slice plan and
   the live worktrees, so a fresh or resumed session needs nothing else.

   ```bash
   cargo xtask mod wave status
   ```

   Expected: `═══ mod wave status ═══`, then `current wave: <N>`, one line per slice with its
   worktree state (`absent`, `dirty` or `committed`) and mark (`READY`,
   `empty (no commits yet)`, `DIRTY — agent must commit in its worktree` or
   `no worktree — run: …`), its title, `ready to merge: <r>/<t>` and an `ACTION:` line. With every
   planned wave shipped it prints only ``ALL PLANNED WAVES SHIPPED. Next: queue mod tickets and
   `cargo xtask wave repack`, or close the program.`` and exits 0. A missing lock or pin file
   exits 2.

2. Check that the wave's slices are disjoint from each other and from anything else in flight.

   ```bash
   cargo xtask slice-collisions
   ```

   Expected: `next wave is <N>. Max disjoint dispatch set (<k>, cap 8):`, then each ticket with its
   `owns:` line. Naming running tickets as arguments prints `already in flight` and
   `may join them` instead; `--check <id>` prints one ticket's `owns` and what it collides with.

3. Create the wave's worktrees.

   ```bash
   cargo xtask mod wave prep <N>
   ```

   Expected: for each slice, `  oracle ok: apps/mod/<lane> -> <source>` for each linked lane, the
   note that `assets_v2/terrains` holds LFS pointers in the worktree, and
   `worktree: <root>/.ai/artifacts/worktrees/<slice>   branch: slice/<slice>`. Without a
   PlayableSelector checkout, a `WARNING: no playable_selector oracle at …` block. `prep` exits 0
   even when `new` refused a slice, so read the output for `REFUSING`.

4. Dispatch one agent per slice, in parallel, each with a self-contained prompt: the slice id and
   worktree path, the ticket and its spec, the [rules](#rules), the [sources](#sources) and the
   environment note above. The agent works and commits inside its worktree and compiles there.

   ```bash
   cargo xtask mod compile
   ```

   Expected (in the worktree): `OK: compiled clean` and exit 0. Exit 1 lists each error as
   `file:line`; exit 3 is the machine, not the mod.

5. Before reporting done, each agent runs the slice gate from its worktree. `merge` refuses a
   slice that has no green gate verdict for its branch tip, so this step is what makes a slice
   landable.

   ```bash
   cargo xtask platform wave gate --slice <slice id>
   ```

   Expected: one `PASS` or `FAIL` line per step, then a verdict receipt at
   `.ai/artifacts/verdicts/<slice id>.json` in the main checkout, written on pass and on fail. Run
   from `main`, the range is empty and the gate refuses with exit 2. A commit after the gate makes
   the receipt stale, so the agent gates again after its last commit. The step list is in the
   [wave gate README](/tools_v2/xtask/src/commands/platform/wave_execution/gate/README.md).

6. Repeat step 1 until every slice is READY (committed, clean, with commits ahead of `main`).

7. Land the wave.

   ```bash
   cargo xtask mod wave land
   ```

   Expected: in order, (1) a refusal and exit 1 if any worktree is dirty, before anything merges;
   (2) `── merging <slice>` for each slice with commits (`── skipping <slice> (no commits)`
   otherwise), then `merged <m>, skipped <s>`; (3) the mod wave gate (below) on merged `main`;
   (4) the reap; (5) `═══ push ═══` and either `nothing to push` or
   `pushing <n> commit(s) (no LFS content — hook bypass is safe)`. A failed merge prints
   `MERGE FAILED for <slice> — resolve manually, then re-run land` and exits 1. A failed gate
   prints `Gate FAILED after merge. Worktrees kept for inspection.` and exits 1, with the merges
   already on `main` and nothing reaped or pushed.

8. Dispatch the verify agent with the [prompt](#verify-agent-prompt), and fix every BLOCKER it
   reports before going on.

9. Ship each landed slice, as the orchestrator. `mod wave status` reads shipped slices from the
   program's slice plan, which the child ticket files feed.

   ```bash
   cargo xtask ticket ship <slice id>
   ```

   Expected: the ticket file's status becomes `shipped` and the wave lock is refreshed
   (`--no-repack` skips that, for one `cargo xtask wave repack` after the last slice). Once every
   slice of wave N is shipped, step 1 reports wave N+1; go back to step 2.

### The mod wave gate

`cargo xtask mod wave gate` runs the gate that `land` runs, on its own. It runs twelve steps,
prints `PASS` or `FAIL` for each with the last 12 lines of a failure, and ends `GATE: PASS` (exit
0) or `GATE: FAIL` (exit 1):

| Step | Command it runs |
|---|---|
| compile, compile-selftest | `cargo run -q -p xtask -- mod compile`, then `mod compile-selftest` |
| world boot, world-boot selftest, world boot +mission | `mod world-boot`, `mod world-boot --selftest`, `mod world-boot --mission=bridgehead-at-levie` |
| ui layouts | `cargo run -q -p xtask -- verify ui-layouts` |
| schema validate, capability, oracle citations, no-crf-leak | `distrobox-host-exec make schema-validate`, `make verify-capability`, `make verify-oracle`, `make verify-no-crf-leak` |
| ticket registry | `cargo run -q -p xtask -- ticket check`, through `distrobox-host-exec` |
| enf unit tests | `cargo test -q -p developer-tools --lib enf::`, through `distrobox-host-exec` |

On the current tree the gate cannot pass: the repository tracks no Makefile, so the four `make`
steps always fail, and `verify ui-layouts` finds no layout because it does not walk the
subfolders of `apps/mod/tbd-framework/UI/layouts/`. The `enf::` filter matches no test, so that
step passes without testing anything. Until the gate is fixed, run the checks the `make` steps
stand for by hand, each on its own:

```bash
cargo xtask ci schema-validate
cargo run -q -p developer-tools --bin enf -- capability
cargo run -q -p developer-tools --bin enf -- citations
cargo xtask verify no-crf-leak
```

`enf capability` exits 1 when a framework file matches no verdict rule (UNTRIAGED); `enf citations`
exits 1 on any `@idx` citation that does not resolve. `verify no-crf-leak` also exits 1 on the
committed tree today; its [README](/tools_v2/xtask/src/verifications/licensing/README.md) lists the
hits.

## Verify

```bash
cargo xtask platform slice-worktree -- list
```

Expected: `git worktree list` output with the main checkout on `[main]` and no
`.ai/artifacts/worktrees/<slice>` row for a slice of the landed wave. `cargo xtask mod wave status`
then names the next wave, or prints `ALL PLANNED WAVES SHIPPED`.

## Worktree mechanics

`mod wave prep`, `land` and the reap call these in-process; run them by hand to repair one slice.
The [slice worktree README](/tools_v2/xtask/src/commands/platform/slice_worktree/README.md) has
every guard.

```bash
cargo xtask platform slice-worktree -- new <slice id>
cargo xtask platform slice-worktree -- list
cargo xtask platform slice-worktree -- merge <slice id>
cargo xtask platform slice-worktree -- drop <slice id>
cargo xtask platform slice-worktree -- reap
```

- `new` creates `.ai/artifacts/worktrees/<slice>/` on `slice/<slice>` from `main`, with the
  git-lfs filters and hooks neutralised, and links the oracle lanes. Re-running it on an existing
  tree re-checks and repairs the links.
- `merge` refuses a missing, dirty or unreadable tree (exit 1) and a slice whose gate verdict is
  missing, red or stamped for another commit (exit 2), then merges `slice/<slice>` with `--no-ff`
  into the checked-out branch, so run it on `main`.
- `drop` refuses a branch with commits not on `main`, or a dirty tree, unless `--force` follows the
  id; it then removes the worktree and force-deletes the branch.
- `reap` keeps every tree that has no branch, is dirty or unreadable, has no commits and no merge,
  or is not merged, and deletes the rest.

In a worktree `assets_v2/terrains/` holds LFS pointers, not images, so `cargo xtask ci
schema-validate` fails there on the DEM step with `PNG decode: Invalid PNG signature`; run
`cargo xtask schema validate` in a worktree instead. The main checkout, with git-lfs installed,
holds the real files.

## Sources

Every slice prompt points at these; an agent proves an Enfusion fact rather than guessing it. Do
not let an agent rely on what it believes about [Enfusion](/documentation_v2/glossary.md#enfusion):
[EnfScript](/documentation_v2/glossary.md#enfscript) is a niche language and a model's priors are
wrong. An agent asked to summarise one CRF file once invented four APIs that do not exist
(`RequestSlotChange`, `ReleaseSlot`, `GetInstance` and a wrong base class), which is why every index
is generated and `enf citations` gates prose citations.

| Need | Command or path |
|---|---|
| Does a CRF or vanilla scripted symbol exist? | `cargo run -q -p developer-tools --bin enf -- lookup <Symbol>` (the CRF index by default); add `--index .ai/artifacts/enf-index/vanilla_symbols.tsv` for vanilla |
| Is a class in the official Script API? | `rg '^<Class>\t' .ai/artifacts/enf-index/vanilla_api_classes.tsv` |
| What does vanilla actually do? | `rg <pattern> apps/mod/vanilla_reference/Source/`: real source with method bodies |
| More vanilla source | `cargo xtask fetch vanilla-source <ClassName>…`, then `enf source`; the site is one person's, so never `--all` |
| How does a working framework do it? | `rg <pattern> apps/mod/crf_framework/`: CRF, Arma Public License, reference only |
| How is a lobby or slot picker shaped? | `rg <pattern> apps/mod/playable_selector/` in a worktree: no licence, design only ([Oracle lanes](#oracle-lanes)) |
| Where does a subsystem live? | `enf dirs`, and `.ai/artifacts/enf-index/capability_matrix.tsv` |
| Does my change compile? | `cargo xtask mod compile`: about 1.3 s on the native server, no Workbench |
| Does an API exist, definitively? | `cargo xtask mod compile --probe=<dir>`: call it in a throwaway `.c` file in that dir; a clean compile means it exists |
| Workbench, prefabs, resource names | the [Workbench MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md): look names up with its tools, never type a GUID by hand |

**Two kinds of Enfusion class, two oracles.** A scripted class (`SCR_BaseGameMode`,
`SCR_PlayerController`) ships as `.c` source: check it with `enf lookup` or `rg` over
`apps/mod/vanilla_reference/Source/`. A native engine class (`BaseWorld`, `Widget`, `IEntity`) is
compiled into the engine, has no source and is in no symbol index: a probe compile is the only
check. `NOT FOUND` from `enf lookup` for something engine-level proves nothing; probe before
concluding (`BaseWorld.GetBoundBox` looks absent from the index and is real).

**Probe hygiene.** Use a fresh, uniquely named directory per probe (`/tmp/probe-$$`): probe
directories are shared, and a leftover file from another agent pollutes a run. `mod compile
--probe` prints `(probing from <dir>)` and each `.c` file it stages, so contamination is visible.

**A green probe means nothing without a negative control.** Always compile a variant that must
fail, and confirm it does. Duplicate `switch` case labels compile clean in Enfusion, so a probe
testing enum distinctness passes, and so does its control; if the control passes, the probe
supports no conclusion.

**Probe by assigning to the narrower type, never by printing.** `Print` accepts an int, so
`Print(s.Replace(a, b))` compiles although `string.Replace` changes `s` in place and returns a
count. Enfusion coerces string to int implicitly but not int to string, so `int n = s.Foo();`
compiles whether `Foo` returns an int or a string and discriminates nothing. Assign to `string`:

```c
string out = s.Replace(a, b);   // fails: "Incompatible parameter", so Replace returns an int
string out = s.Trim();          // compiles, so Trim returns a string
```

A probe that would compile under both answers is not a probe.

## Oracle lanes

`slice-worktree new` links these into every worktree; all are gitignored, so a fresh tree has
none until that step runs. They are not equivalent, and the difference is legal, not stylistic.

| Lane | Licence | You may | You may not | Missing at `new` |
|---|---|---|---|---|
| `apps/mod/vanilla_reference` | Bohemia game source, extracted with `enf` | read for behaviour, cite | commit it, ship it | refuse (exit 1); no tree handed over |
| `apps/mod/crf_framework` | Arma Public License | read, cite (`@idx crf#OnPlayerAuditSuccess`), design-mirror | copy code, reuse asset GUIDs, vendor it | refuse (exit 1); no tree handed over |
| `apps/mod/playable_selector` | none at all | read to understand design | copy a single line, adapt, redistribute | warn; the tree is still handed over |

**No licence is worse than the Arma Public License, not better.** The APL grants terms;
PlayableSelector ships with no licence file, so default copyright applies and nothing permits
copying, adapting or redistributing any of it. It is a design mirror only: read it to learn how a
lobby or slot picker is shaped, close it, then write TBD's own. Having a PlayableSelector file open
beside a TBD file is already the mistake.

**Why PlayableSelector warns while the other two refuse.** The refusal exists for one failure: an
agent with no way to check an Enfusion API fact invents one. The vanilla and CRF lanes answer that
question and live inside the repository, provisioned by its tooling, so their absence is a broken
setup and the tree is withheld. PlayableSelector answers design questions, proves no Enfusion fact,
cannot be compiled against, and lives outside the repository on one operator's disk
(`$HOME/Projects/Archive/Reforger_Lobby/PlayableSelector-main` unless `TBD_PS_ORACLE` names another
folder), so it is legitimately absent on any other machine. Its absence is loud, and design work
that would have cited it stops and asks rather than guesses.

**The gate.** `cargo xtask verify no-crf-leak` (the name covers every lane) fails on a `CRF_` or
`PS_` identifier in the code of `apps/mod/tbd-framework/` and `apps/mod/tbd-export/`, and on an
oracle asset GUID reused there that no vanilla pak also contains. `apps/mod/tbd-emcp/` is
third-party and not scanned. Comment lines naming an oracle are allowed and encouraged — citing
what was design-mirrored is the practice — it is the prefix in code that fails.

**The deploys.** `cargo xtask deploy staging` and `cargo xtask deploy website` exclude every lane
from their rsync. The staging server only runs `apps/mod/tbd-framework`, so an oracle on it is
licence exposure for no benefit, and the main checkout holds the lanes as real folders, so a
missing exclude ships a whole lane.

**Adding a lane takes four edits:** the link step in
`tools_v2/xtask/src/commands/platform/slice_worktree/git_plain.rs`, the prefix and lane in
`tools_v2/xtask/src/verifications/licensing/upstream_code_leaks.rs`, and the `--exclude` in both
`tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs` and
`tools_v2/xtask/src/commands/deploy/website/rsync_argv.rs`. A lane missing any of them is a
liability.

## Verify agent prompt

The orchestrator dispatches this agent against merged `main` after every landed wave (rule 4).
Copy the block and fill in `{{WAVE}}` and `{{SLICES}}`.

```text
You are the adversarial verifier for wave {{WAVE}} of the TBD Reforger mod program, just merged
to main in this repository. Slices merged: {{SLICES}}.

Your job is to find what is WRONG. Each slice agent verified its own work in isolation; you
verify the integration, and you are sceptical of their claims. "All good" is acceptable only
after you have actively tried to falsify each claim and failed.

ENVIRONMENT
debian:12 container, glibc 2.36, no C toolchain; the host is Fedora. Prefix cargo and game
binaries with distrobox-host-exec. In-container `cargo build` -> "linker cc not found"; a host
binary in the container -> "GLIBC_2.39 not found". Neither means the repository is broken.

START HERE
  cargo xtask mod wave gate            # every step must pass; a failure IS a finding
  git log --oneline -15 main
  git diff --stat <pre-land commit>..main
Read documentation_v2/mod/tbd-framework/mod_design.md and
documentation_v2/runbooks/mod_slice_workflow.md first.

WHAT TO ATTACK, in priority order
1. Integration seams. The slices were written blind to each other. Do their assumptions agree?
   Did two slices hook the same lifecycle event? Does one call a method another removed,
   re-signed or never exposed?
2. Claims against reality. Check every claim in the slice reports independently: find the code
   path and confirm it, including the admin respawn path.
3. The non-negotiables (mod design section 2), above all ONE LIFE: is there ANY path by which a
   dead player returns to the world other than TBD_SpawnManager.AdminRespawn? Re-claiming a slot,
   releasing and re-claiming, reconnecting, a vanilla respawn path left standing: hunt for all of
   them. Also: data hard-coded that should come from the mission document; oracle code in the
   shipped addons. `cargo xtask verify no-crf-leak` checks the CRF_/PS_ prefixes and asset GUIDs;
   read the diff for structural copying too.
4. Enfusion correctness. Every API called must exist:
     cargo run -q -p developer-tools --bin enf -- lookup <Symbol>
     rg <pattern> apps/mod/vanilla_reference/Source/
   The compile gate catches undefined symbols, not wrong-but-existing usage: an
   [RplProp(onRplName:)] handler that assumes it fires on the authority (it fires only on the
   proxy), or set/array.Remove treated as by-key (it is by index).
5. Honest failure. Does bad input give a clear diagnostic or a silent half-broken state? Build
   the bad input and try it.
6. Stubs presented as working.

DO NOT
- Fix anything. Report only; the orchestrator decides what to fix and in which slice.
- Edit anything under .ai/tickets/ or change a ticket status.
- Leave probe files outside /tmp.

RETURN
One line per finding:  SEVERITY | file:line | what is wrong | how you proved it
SEVERITY is BLOCKER (a non-negotiable violated, or main broken), MAJOR (works but wrong) or
MINOR (quality). Then one line: is main safe to build the next wave on, yes or no? For a
category with no finding, name the falsification attempts you made; vague reassurance is a
failed verification.
```

## What agents cannot do

Nothing here returns a framebuffer. Agents prove compilation, not appearance or runtime
behaviour. Anything visual — the lobby, the briefing, the spectator view — needs the operator's
eyes. Batch those into one review session rather than interrupting per slice, and never report a
UI slice as done when it only compiles.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `mod wave land` prints `MERGE FAILED for <slice>` | `merge` refused: dirty tree, or `land: no gate verdict for <slice>`, a red verdict, or a verdict `STALE` against the branch tip | commit in the worktree, then run `cargo xtask platform wave gate --slice <slice id>` from the worktree and land again |
| `Gate FAILED after merge. Worktrees kept for inspection.` | a gate step failed on merged `main`; the four `make` steps and `ui layouts` fail on every run today | read each `FAIL` block; run the replacement checks under [The mod wave gate](#the-mod-wave-gate); after a real fix on `main`, run `cargo xtask mod wave gate`, then `platform slice-worktree -- reap` and `mod wave push` by hand |
| `new` prints `ERROR: <path> missing — cannot link the <lane> oracle lane` and `REFUSING` | a required lane is absent from the main checkout | restore `apps/mod/crf_framework/` or `apps/mod/vanilla_reference/`, then re-run `slice-worktree -- new <slice id>` |
| `WARNING: no playable_selector oracle at …` | no PlayableSelector checkout at the default path | set `TBD_PS_ORACLE=/path/to/PlayableSelector-main` and re-run `new`, or stop design work that needs it and ask |
| `REFUSING to bypass the LFS hook: <n> file(s) under assets_v2/terrains/` | the push range carries LFS-tracked files | with git-lfs installed, `git push origin main` |
| `enf lookup <Class> --index …/vanilla_api_classes.tsv` prints `NOT FOUND` for a listed class | `lookup` reads four-column symbol indexes; the API class index has two columns | `rg '^<Class>\t'` over the TSV instead |
| `mod wave status` exits 2 | the wave lock or `.ai/tickets/corpus-pins.toml` is missing or unreadable | `cargo xtask wave repack`, or restore the pin file |
| in-container `linker cc not found` or `GLIBC_2.39 not found` | a build or binary ran on the wrong side of the host bridge | run it through `cargo xtask` or `distrobox-host-exec` |

## Related

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the non-negotiables and the
  Enfusion facts every slice meets.
- [Mod program spec](/documentation_v2/tickets/specs/t181_event_mod_program.md) — the program the
  waves carry out.
- [Mod wave driver](/tools_v2/xtask/src/commands/mod_ops/wave_execution/README.md) — `status`,
  `prep`, `gate`, `land` and `push` in code.
- [Slice worktree lifecycle](/tools_v2/xtask/src/commands/platform/slice_worktree/README.md) — the
  worktree commands and their guards.
- [Platform wave driver](/tools_v2/xtask/src/commands/platform/wave_execution/README.md) — the slice
  gate and its verdict receipt.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the same worktree cycle for
  the platform program.
- [Workbench MCP bridge](/documentation_v2/mod/tbd-emcp/workbench_mcp_bridge.md) — bringing
  Workbench and the MCP tools up with `mod dev-bootstrap`.
- [Loadouts](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Loadouts/README.md) — the dev harness
  that equips `$profile:TBD_LoadoutTest.json` on a test body.
- [Enfusion script oracle](/tools_v2/developer-tools/src/enfusion_tooling/README.md) — `enf lookup`,
  `citations` and `capability`.
- [Upstream code leak gate](/tools_v2/xtask/src/verifications/licensing/README.md) —
  `verify no-crf-leak` in full.
- [Vanilla source coverage](/documentation_v2/mod/tbd-framework/vanilla_source_coverage.md) — which
  lane answers which vanilla question.
