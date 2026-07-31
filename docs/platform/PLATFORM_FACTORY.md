# Platform factory — command center, worktrees, waves

**Operator-defined, binding. Read this before dispatching any platform slice agent.**
This is the process for the **T-182…T-295 backlog** (website, Mission Creator, contract, data,
infra). The mod program has its own: [`../mod/SLICE_WORKFLOW.md`](../mod/SLICE_WORKFLOW.md).

Do not start this program until **T-181 is finished**. Operator instruction.

---

## ⏹ THE FACTORY STOPPED AFTER WAVE 77 — 2026-08-01. Read this before restarting it.

**358 platform tickets shipped. 42 open. Zero P0. Zero P1.**

**Wave 77 was the tooling wave** — nine tickets, all of them the project's own checks rather than
anything a user meets. It exists because waves 75 and 76 kept tripping over instruments that lied:
a gate that narrowed its own scope and reported 26/26, a health grep with no reachable exit 0, a
validator ratchet red since before anyone looked, a sentinel test that could not fail, and a backup
verifier that accepted the wrong database. All nine are fixed and adversarially verified.

**The playtest is unblocked and the Workshop skew is closed — measured, on 2026-08-01.** The operator
re-published the mod; the stale 1.0.1 was still cached locally and would have been preferred. Cleared
it, booted `-config` only (the exact path a joining client takes), and the engine fetched **1.0.2**
(`data.pak` 570,489 B vs 41,288) emitting **151** current-format `[TBD][` lines against 1.0.1's zero.

The operator's call, and it was the right one: the active `ready`/`queued` lane is empty, nothing
open is P0, and **the only thing left that can tell us whether the product actually works is a live
two-client playtest.** Waves were producing roughly −5 shipped / +2 filed each, because every wave
ends with an adversarial verifier whose entire job is finding bugs and whose standing triage rule is
*document, don't fix.* That converges, but slowly, and it never reaches zero while the verifier runs.
More waves was not the constraint. Contact with reality was.

### What to do next, in order

1. **Run [`PLAYTEST_RUNBOOK.md`](PLAYTEST_RUNBOOK.md).** Nothing blocks it any more. One session closes
   **T-181.16 and T-068.14** — the last slices of the last two open programs. Start with
   `scripts/mod/run-playtest-server.sh`. The remaining unknown is the friend's first Direct Join,
   which nobody has ever exercised; §6.2's `#tbd` chat probe tells you in one line which mod the
   client actually loaded.
2. **Only then consider more waves.** And if you do, read the two entries below first — the tooling
   is honest now, but it is not infallible.

### Still open on staging (T-607, no longer playtest-blocking)

`deploy-staging.sh:1153` omits `-addonsDir` in config mode and `:1155` registers no room, so staging
is still broken both ways. Copy the shape T-604 proved in `scripts/mod/run-playtest-server.sh` —
**both flags together**. STAGING-SERVER.md's pass criteria were already corrected by T-606. The old
1.0.1 pak is kept at `~/.cache/tbd-workshop-1.0.1-backup`; it is the only copy, and the stale-build
detectors were validated against it, so do not delete it casually.

### State of the two remaining programs

Both are one slice from done and it is the **same** slice: a live two-client E2E on a dedicated
server. T-181 (54 slices shipped; the mod boots, all five screens open, objectives/radio/play-area/
briefings/markers run) and T-068 (cargo ladder shipped through .12). Neither is agent-actionable.

### What the 47 open tickets are

15 with no priority, 15 P3, 16 P2, **1 P1** (T-607), **0 P0**. Roughly 29 are `deferred` — bugs that
are already found, diagnosed, and written down with repro steps. That was a deliberate trade, made
repeatedly and on the record: *recording them is the value; fixing them now is optional.* About half
predate this run entirely, some from the T-085–088 era. **Do not treat the pile as a debt to burn
down.** Promote a ticket when it blocks something real.

Three worth knowing about because they describe the *tools*, not the product:

- **T-601 — Class-R pin hollowness is systemic**, ~23 known instances of one defect class. Wave 76's
  verifier defeated three pins shipped in that same wave. Two proven cures exist; the ticket names
  both. **Do not respond by blocklisting dead-code shapes** — that is the fourth round of that game
  (T-517 → T-567 → T-570) and a fifth wrapper always exists.
- **T-609 — the world-boot ratchet has been red for every golden mission** since before wave 76, and
  no gate noticed because `cmd_gate` has no world-boot step. Filed as *decide, don't widen*: widening
  the baseline makes the red go away while preserving whatever it was warning about.
- **T-613 — the gate's base derivation verifies itself with its own oracle.** T-602 (wave 77) fixed
  `wave.sh gate` silently narrowing its scope — it now derives the base from the `wave N CLOSED`
  marker and refuses a base that starts after the wave opened. But **derive and verify call the same
  function**, so a commit subject that merely continues past `CLOSED` becomes the base *and*
  self-approves. Proven in a clone. Latent — no historical subject matches and `wave --close` writes
  the format — but it is the signature defect living inside the fix for it. **A checker that consults
  the thing it is checking is not a check.**

### The one lesson worth carrying forward

The recurring defect in this codebase has a single shape: **a tool reports success over an input it
never actually examined.** It showed up this run as a gate step that never ran, a `cargo check`
replaying a cached verdict, a test greping a string in its own assertion, a launcher that printed
SERVER UP and launched nothing, a staging check that passed *because* the mod was stale, and — most
instructively — inside the failure branch of the very script written to eliminate it. Treat any
green you did not watch fail first as unproven.

---

## COLD START — a fresh command-center session, first five minutes

The command center is **deliberately a short-lived chat**. Once a session has been compacted a
couple of times, every turn pays to re-read a large context before doing any work, and compaction
drops precision nobody chose to drop. Start a new chat and read state from the repo instead — the
repo has always been the source of truth, which is what makes this cheap.

```bash
bash scripts/platform/preflight.sh          # must print PASS; fix BLOCKs before anything else
bash scripts/platform/wave.sh status        # what is shipped, in flight, ready
python3 scripts/platform/slice-collisions.py --repack
python3 scripts/platform/slice-collisions.py   # the dispatch set, + any UNPLANNED warning
```

State lives in: [`wave_plan.tsv`](wave_plan.tsv) (what runs together) · `.ai/tickets/registry.json`
(every ticket's full history — **the summaries are the handoff**, read the ones you are dispatching)
· this file (process) · [`frontend_data_provenance.md`](frontend_data_provenance.md) (which render
sites are real vs mock — saves a 150k-token re-derivation).

**Do not** try to reconstruct a previous session's reasoning. If a decision mattered, it is in a
ticket summary or a code comment. If it is in neither, it was not recorded and you should re-derive
it rather than trust a recollection.

## OPERATOR CHANGE — 2026-07-26: Cursor + Grok 4.5 now runs this factory

Claude Code's token budget ran out after wave 5. **The command center is now Grok 4.5 in Cursor.**

> **If you are Grok/Cursor, read [`FACTORY_FOR_CURSOR.md`](FACTORY_FOR_CURSOR.md) instead of this
> file.** It is the same process written as a literal procedure — exact commands, checklists, and
> decision tables rather than the judgement calls this document assumes. This file remains the
> *why*, and the handoff below is still authoritative on what happened; that file is the *how*.

The Cursor rule that switches modes and suspends the normal "Cursor may not write app code" gate is
[`.cursor/rules/platform-factory-mode.mdc`](../../.cursor/rules/platform-factory-mode.mdc).

## WHERE THE RUN LEFT OFF — 2026-07-26, waves 3-5, PAUSED

Read this before dispatching. Everything below is either not derivable from the tickets, or is
sequencing that a cold start would otherwise have to guess.

**Shipped, gated, verified, pushed:** waves 3 (T-405, T-406, T-399), 4 (T-240, T-241, T-243),
5 (T-215, T-216, T-244, T-420, T-421). Tip `77214f04` + the T-421 merge. **Eleven tickets shipped,
twelve filed** (T-409…T-426, minus the shipped ones).

**Wave shape in force is the OPERATOR'S, and it overrides rule 3 / correction 2 below:**
3 agents → **all three report** → merge all → wave gate → **one** adversarial verifier after the
merge → triage → close. Tokens and command-center attention are the constraint, not wall clock.
The no-barrier rule below is suspended; do not restore it without the operator.

**`wave.sh land` refuses tickets promoted out of plan-wave order.** All three waves were merged by
hand — `git merge --no-ff` per slice, then `wave.sh gate <base>` on merged main, then drop the
worktrees. That is `cmd_land`'s exact sequence and it keeps every safety property. The refusal is
correct behaviour, not a bug; do not "fix" it into landing the wrong set.

**Point `TBD_GATE_DB` at a fresh cold database for every wave gate.** All three waves passed only
because of this. The shared `tbd_gate_it` is at ~26 `pending_approval` rows and reds
`missions.rs:1002` on any run. That is T-410/T-411, not your slice.

### The three wave.sh tickets are SEQUENCED. Do not merge them into one pass.

| order | ticket | why this order |
|---|---|---|
| ~~1~~ | ~~T-421~~ | **SHIPPED.** The mtime/foreign-artifact BLOCKER. Everything else stands on it. |
| 2 | **T-409** | T-406's hard-fails that false-red on legitimate slices. Blocks real work today. |
| 3 | **T-422** | `gate_schema`'s wrongly-excluded green gate + silently-narrowing tripwire. |
| 4 | T-426 | Residue: `include_str!` inputs escape invalidation; gate dirs unreclaimable. |

Each was found by attacking the thing that says green, and each is one small change from being
reachable. T-421 partially mitigated T-422's third defect — read that note before scoping it.

### What waves 3-5 actually cost, and what they were worth

Roughly 4-5 M subagent tokens for eleven tickets. **The verifiers repeatedly earned more than the
slices**: wave 3's found T-406's new hard-fails refusing legitimate work (the inverse of the usual
defect); wave 5's proved `cargo check --workspace` returning rc=0 over a file containing
`THIS IS NOT RUST AND CANNOT COMPILE`. Two of three verifiers also **disproved** command-center
worries, which is worth as much — do not treat a "nothing found" verifier as wasted.

### Judgement calls made deliberately — do not re-litigate without new evidence

- **T-216 shipped ZERO of its six fields on purpose.** `mission.schema.json` closes `$defs/slot`,
  `$defs/group` and the root, and `/compiled` 500s on violation, so emitting them would take the
  route down for every mission. It shipped a compile-boundary **ledger** that goes red when T-242
  widens the schema — a dead feature converted into visible work. That is the intended outcome.
- **T-241 declared minima only**, leaving every maximum to T-275 so the two cannot disagree. It
  also declined conditional `required` for `holdSeconds`/`targetAlias` — a different defect, and the
  shape most likely to collide with T-201/T-211/T-212.
- **T-215 map-placed vehicles carry no `squadId`.** Attaching would hit
  `place_orbat.rs:157-161`, close the side's current squad, and silently split the fireteam being
  built around it (the T-321 defect). Side goes on `factionId` instead.
- **T-240 blocks only on capacity faults**, not the whole fault list. Making compat faults blocking
  is an unmeasured behaviour change that would strand an author with an incompatible optic.

### Two tickets were STALE and were corrected in place

T-242 (claimed `$defs/entity` has no inventory model; T-198 landed `inventory` in `2070eecdd`) and
T-410 (missed the `missions.rs:1002` ratchet). **The command center briefed an agent off T-242's
stale summary and was corrected by the agent.** Assume more of the `idea` backlog is stale; verify
the defect on `main` before writing code, every time.

**Next wave when work resumes:** T-245, T-247, T-248 — plan rows correct, `owns` verified, dispatches cold.

## THE SIGNATURE DEFECT — what to be suspicious of

Across the 2026-07-26 run, **seven independent instances** of one pattern were found, and it caused
more wasted work than every code bug combined:

> **A tool reports success over an input it never actually examined.**

Every instance looked like a green check:

| instance | what it silently did not examine |
|---|---|
| gate ran the DB suite with no database | 30 tests printed `skip:` and it printed PASS |
| clippy step was feature-blind | 7,377 LOC behind `--features world` never linted |
| browser assertions computed "not null/false/0/empty" | every probe returns an object, so **all** passed |
| shared `CARGO_TARGET_DIR` served stale/foreign binaries | a test's "126 passed" was not its own code |
| `wave.sh land T-204` discarded its argument | landed 4 slices, 2 whose agents had not reported |
| `assert_golden` under `#[serde(flatten)]` | a deleted field is re-emitted, JSON byte-identical (T-394) |
| `slice-collisions.py --repack` | 36% of open tickets had no plan row and were never candidates |
| preflight's `:8080` check was a TCP connect | reported a **six-hour-stale binary** as "up" |

**The lesson that generalises:** a passing check is worth nothing until you know *what it looked at*.
Prove non-vacuity by perturbation — break the thing deliberately and confirm the check goes red.
T-204 did this unprompted (restored the hardcode, watched its own new tests fail) and it is the
single most valuable habit in this program. **Demand it in every slice brief.**

## Agent reports are evidence, not testimony

Agents are reliable about code they touched and **unreliable about bookkeeping**. On 2026-07-26,
**three separate agents claimed to have filed a follow-up ticket and none had** — including a P0
that broke all production telemetry. Others contradicted the command center's own framing ten times,
and were right most of them.

So: **verify every claim of the form "I filed / a sibling fixed / this already works" against the
repo before acting on it.** Grep the registry for the finding. Check `git log` for the sibling fix.
It costs one tool call and it has caught something every single time it was done.

Corollary: when an agent contradicts *you*, assume it looked and you remembered. Verify, then
record the correction in the ticket and tell the operator plainly.

## Known traps that cost real hours

- **`slice-worktree.sh` subcommand is `new`, not `create`.** `create` prints usage and exits 2.
- **`git push` fails** — the pre-push hook needs git-lfs, absent from the container PATH. Always
  `bash scripts/platform/wave.sh push`.
- **The operator's dev API on `:8080` goes stale.** A running process keeps its old inode when cargo
  re-links, so a fresh binary on disk does not mean fresh code serving. Preflight now compares
  process start time to the last API commit. A stale API is worse than a dead one — dead fails
  loudly, stale returns confident wrong answers that read as genuine defects.
- **`trunk serve` on `:3000` races the gate's `trunk build`** over the same `dist/`. Already caused
  one gate-red that read exactly like a code fault. Stop it before an unattended run.
- **Agents must never `git stash`** — it deletes LFS pointer files.
- **Orphan agent processes and target dirs leak.** A dead agent's API was found still listening 53
  minutes later; ~116 GB of orphan target dirs once filled the disk and two gate steps failed with
  "No space left on device", which reads as a build error. `wave.sh reclaim`.
- **`distrobox-host-exec` does not forward env** — pass it explicitly via an `env` whitelist.
- **A rate-limited subagent reports `<status>completed</status>`.** Treat the reset string as a
  FAILURE or you will mark dead agents as done.
- **A `git checkout` restore does not reliably re-trigger a cargo rebuild.** Cargo's freshness test
  is mtime-based, and a restored file can be *older* than the artifact built from the perturbed
  version — so the tool keeps reporting the perturbed verdict over correct source. **This attacks
  the perturbation habit itself**: every loop is break → see red → restore → see green, and the
  GREEN half can be stale. Measured twice on 2026-07-26 (T-244 mid-verification; T-420 running a
  foreign `xtask` its own tree could not produce). T-421 fixed the gate via `touch_workspace`;
  **a slice agent's own manual loop is still exposed** — `touch` the file after restoring, and
  brief agents to check the file rather than believe the tool.
- **`grep` IS NOT GNU grep in an agent shell — and this bites the FIX for the `rg` trap above.**
  Measured 2026-07-31 (T-586). In an agent shell `grep` is a harness-injected **function** resolving
  to **ugrep 7.5.0**; in a plain `bash script.sh` it is `/usr/bin/grep`, **GNU 3.8**. They disagree on
  ERE braces:
  ```
  pattern '^GET /a/{id}$'   ugrep -> exit 2 "invalid repeat"   GNU -> exit 0
  ```
  **Every API route path contains `{id}`.** So a bare-brace pattern is green in one shell and a hard
  error in the other, and a loop that reads exit 2 as "no match" is fail-open — T-586's own prototype
  printed **50 false findings** that way before it was caught. Use `-F` literal for anything
  route-shaped or brace-bearing, and read the exit status (0/1/2/127) rather than collapsing it to a
  boolean. `scripts/mod/lib/gate-grep.sh` already does this; use it.
- **`make` is not on the container PATH either**, alongside cargo/rustfmt/xtask. Route it through
  `distrobox-host-exec` like the rest. Only `wave.sh` runs directly, never wrapped.
- **A slice agent running its own server instance needs a PRIVATE `CARGO_TARGET_DIR`.** Measured
  twice in one wave (T-581, T-582): two slices building the same crate into the shared `target/`
  served each other stale binaries. Symptom is `Blocking waiting for file lock on artifact
  directory`, then `Finished` with **no `Compiling` line**, and the binary does not contain the
  slice's code — one run reported `328 passed` while `--list` showed none of its own tests. It reads
  exactly like a correct fix not working. Use `target-<slice>-api`, **grep the binary for a string
  unique to your version before trusting any HTTP or test result**, and delete the dir on cleanup —
  `wave.sh reclaim` reaps `target-<SLICE>` orphans since T-589, but only if the worktree is gone.
- **`cargo check` ITSELF can report PASS over source that does not compile.** Measured 2026-07-31
  (T-596): under sibling contention on the shared target dir, `cargo check -p website-frontend
  --target wasm32-unknown-unknown` printed `Finished ... in 8.73s`, **exit 0**, while the tree still
  had two unresolved identifiers. Clippy caught it seconds later. **The tell is the absence of the
  `Checking <crate>` line** — the green run showed only `Blocking waiting for file lock on build
  directory` and a replayed warning, i.e. a *cached verdict* rather than a build.
  **So: assert on the `Checking <crate>` line, or use a private target dir.** Note the gate's own
  step is `cargo check --workspace --quiet`, and `--quiet` SUPPRESSES that line — the gate is safe
  only because T-421 gave it a private `target-gate-check`; a slice agent running `cargo check` by
  hand has neither protection. This is the signature defect inside the tool used to verify against it.
- **`rg` DOES NOT EXIST — corrected 2026-07-27 (T-556). The earlier claim here that it was
  "container-only" was WRONG and this doc propagated it into a code comment.** ripgrep is installed
  nowhere: not in the container, not on the host, no rpm. `command -v rg` succeeds only because an
  agent harness (Claude Code, and presumably Cursor) injects a shell **function** named `rg` that
  routes to its own bundled copy. `type rg` → "rg is a function"; `bash -c 'command -v rg'` → absent,
  because functions are not exported to subshells. **So any gate using `rg` passes only when an AI
  agent is the thing invoking it** — measured: two gate steps were red on `main` for exactly this.
  Use `grep -E` (container, host, and every CI runner) and **read the exit status** rather than
  collapsing it to a boolean: 0 match / 1 no-match / 2 file missing / 127 tool absent. Only the last
  two are new information and both must fail closed, naming which happened.
  The shared helpers are `scripts/mod/lib/gate-grep.sh` (`gate_ban`, `gate_require`, …) — use them
  rather than hand-rolling, which is how this defect kept being reborn by copy-paste.

## The shape

Identical to T-181. The main chat is the **command center**: it never implements, it dispatches,
integrates, verifies and sequences. Each ticket is one **full-capability Opus subagent** in its own
git worktree, so the command context stays clear and each ticket gets a whole context window.

```
main ──┬── worktree slice/T-190 → agent A ──┐
       ├── worktree slice/T-191 → agent B ──┼── each lands the MOMENT it is green
       ├── worktree slice/T-192 → agent C ──┘   (no barrier — see rule 3)
       ├──────────── wave gate on merged main
       ├──────────── adversarial VERIFY agent
       └──────────── verify green → dispatch the next disjoint set automatically
```

## Three corrections to how T-181 ran

These are measured, not stylistic. Each cost real time on the mod program.

### 1. Rust worktrees need a shared `CARGO_TARGET_DIR`

T-181's slices were Enfusion `.c` — worktrees cost nothing. **These slices are Rust.** With no
`CARGO_TARGET_DIR`, every worktree starts a cold build of a 609-crate workspace, and the repo's
own `target/` is already 52 GB. Eight cold worktrees is a dead afternoon.

`scripts/platform/wave.sh` exports `CARGO_TARGET_DIR="$ROOT/target"` for every tree. Cargo's lock
then serialises builds instead of duplicating them — and a warm `cargo check --workspace` is
**6.8 s measured**, so the wait is cheap and the cache stays hot for everyone.

**Slice agents must not override it.** A worktree that builds into its own `target/` will appear to
work and will quietly cost 40 GB and several minutes.

### 2. No wave barrier

T-181 rule 3 was *"merge only when all three complete."* Measured cost: **89% of the program's wall
clock was spent waiting between merges that themselves take zero seconds** — mean 64 minutes
between lands. Finished, gate-green slices sat blocked behind unfinished ones. At one point three
completed slices idled behind a single dirty tree, including the identity-linking work that the
end-of-round results POST was inert without.

`wave.sh land` merges **any** slice that is committed, clean and has commits — immediately.
`land --wave` restores the old barrier if it is ever genuinely wanted; it prints why you probably
do not want it.

### 3. Tiered gates, and never `make ci-local`

A slice pays only the **cheap gate** (`cargo check` + `cargo fmt`, ~10 s). The expensive suite runs
once per wave on merged main.

`make ci-local` is **deliberately excluded**. It has been red for weeks — `verify-no-python.sh`
fails on `scripts/mod/slice-collisions.py` and on inline `python3` in `wave.sh`/`world-boot.sh` —
and it costs 15-40 minutes, not the 22.7 s the docs still claim (that figure was measured before
the Go→Rust and React→Leptos migrations). A gate everyone routes around teaches agents that gate
failures are noise.

### 4. Cargo does not run inside the container

Measured, and it would have made every gate red on day one:

```
cargo check --workspace   in container → exit 101 (E0463, 15 errors)
cargo check --workspace   on host      → exit 0
```

The container is glibc 2.36; the host is 2.39. Proc-macro build scripts and `target/debug/xtask`
are compiled against the host and refuse to load in here. **Every cargo, rustfmt and xtask call in
`wave.sh` goes through `hostrun` (`distrobox-host-exec`).**

Beware the failure mode that hid this: `cargo check ... | tail -5` reports `$?` from `tail`, not
cargo. A piped gate looks green while the build is failing. `wave.sh` captures exit status
directly for exactly this reason — do not "simplify" it into a pipeline.

Related: `cargo fmt --all --check` is **not** used. 32 files are already unformatted on `main`
(mostly `tools/tbd-tools/src/bin/enf.rs`, written during T-181 and never formatted), so a
workspace-wide check would fail for every agent regardless of their work. `fmt_changed` scopes it
to the slice's own diff against `main`.

## Rules

1. **One worktree per ticket.** `bash scripts/mod/slice-worktree.sh new T-190` — that script is
   program-agnostic, it keys off the branch name only. Sub-slices (`T-190.1`) live in the parent's
   tree.
2. **Concurrency = file-disjointness, computed, never guessed.**
   ```bash
   python3 scripts/platform/slice-collisions.py              # max dispatch set
   python3 scripts/platform/slice-collisions.py T-190 T-191  # what may join those in flight
   python3 scripts/platform/slice-collisions.py --check T-195
   ```
   Worktrees make concurrent edits *safe* but do not prevent **merge conflicts**. The `owns`
   column in [`wave_plan.tsv`](wave_plan.tsv) is the only thing standing between eight agents and
   a merge pile-up. The cap is `TBD_MAX_CONCURRENT` (default 8) and the binding constraint is
   **integration attention** — how many dense agent reports the command center can actually read
   and act on — not disk and not CPU.
3. **Land each slice the moment it is green.** No barrier. See correction 2.
4. **Then run an adversarial verify agent** against merged `main`. Its job is to find what the
   slice agents got wrong, not to confirm they were right. On T-181 this caught two live MAJORs.
5. **Push after every landing group.** Work must not be trapped on one machine. `wave.sh push`
   refuses `--no-verify` if the range touches `packages/map-assets/**` (the only LFS-tracked path).
6. **Verify green → dispatch the next disjoint set automatically.** Do not wait to be asked.
7. **Agents never self-ship.** They implement, gate-verify, and report. The command center owns
   `.ai/tickets/registry.json` and every status transition.
8. **Agents leave their tree green** and put throwaway probes in `/tmp`, never in the source tree.
9. **EVERY agent runs on Opus 5 — no exceptions, restated 2026-07-26.**
   Slice agents, adversarial verifiers, fix agents, throwaway research agents alike. Pass
   `model: "opus"` **explicitly** on every single dispatch — never rely on inheritance, because a
   default can change underneath you and the downgrade is silent.
   **Do not downgrade to work around 529s, rate limits, latency or cost. Retry instead.** A
   rate-limited Opus agent is parked and resumed; a Sonnet fallback is a quality regression that
   lands in `main` and nobody notices until the morning.
   Operator instruction, given twice: *"I don't want you to use Sonnet for any of the agents… I
   don't care what it is. Always use Opus five for the agents."*
10. **`owns` is load-bearing.** It is the only thing preventing two concurrent agents colliding on
    one file. All 113 rows were narrowed to real file paths by hand on 2026-07-26 from the original
    audit citations — **zero directory-level claims remain**. Do not widen a row back to a
    directory to "make it fit"; re-run `--repack` instead.

    **But `owns` only constrains what an agent is TOLD, not what it does.** In wave 1, T-182 was
    widened to five files and edited seven — reaching into `TBD_SpawnManager.c` (a real bug: its
    `HasAuthoredLoadout` gate would otherwise have routed launcher-only slots away from the loadout
    path entirely) and `mission_compile.rs`, **which T-192 was editing in the same wave**. Both
    landed and composed only because git happened to merge disjoint regions of the file. That was
    luck. When an agent reports having touched a file outside its list, verify the composition by
    hand before trusting the green gate.

12. **The adversarial verifier runs every wave, and its findings are TRIAGED — not all promoted.**
    Operator instruction, 2026-07-26, after the reviews generated work faster than the run closed it
    and the feature backlog sat still for hours.

    Rule 4 says run the verifier. This says what to do with what it finds:

    | Verdict | Action |
    |---|---|
    | **BLOCKER** — main is broken, or data is at risk | Fix **in this wave**. The wave does not close. |
    | **MAJOR** — a shipped ticket does not do what it claims | Fix in this wave **if** it can lose authored work or blocks a feature; otherwise file `deferred`. |
    | **MINOR / NIT** | File as `deferred` **immediately**. Do not create wave work. |

    **File deferred, never drop.** A diagnosed, reproducible ticket costs nothing to hold and is most
    of the finding's value. `dispatchable()` in `slice-collisions.py` already filters `deferred`, so a
    deferred ticket cannot enter a wave until someone promotes it.

    Two criteria promote a non-BLOCKER, and only these two:
    - it can **destroy work the operator authored**, or
    - it **unblocks a feature** on the original backlog.

    Everything else waits. The operator's framing, which is the correct one: *there will always be
    bugs — that is what developing is. Knowing them is good. Spending the budget on things that do not
    need fixing right now means nothing ships.*

    **Precedent, so the calibration is legible.** On 2026-07-26 the reviews produced 46 findings.
    Five were kept: two that could overwrite an authored mission with an empty document, one that
    destroyed an authored emblem on every save, and two that were the briefings and markers blockers
    (features, not defects). **Forty-one were deferred**, including an admin-lockout route and four
    gate defects — all real, all recorded, none urgent.

    The one class that is **always** a BLOCKER regardless of severity: **a gate that reports success
    on code it never examined.** Four independent instances turned up in one night — the DB suite run
    with no database, clippy run with no features, `render-check` unable to fail, and a fonts failure
    that only warned. Each made every other claim in the program worthless until fixed, and none
    would have surfaced without someone attacking the thing that says green.

11. **Never land a slice until its agent has REPORTED.** `tree_state` answers "is the tree
    committed and clean", which is not the same question as "is the agent finished". In wave 1
    T-182 committed mid-run, `land` merged it and dropped the worktree, and the agent then found
    its own tree deleted underneath it. The commit survived in the shared object store and nothing
    was lost, but only because it had already committed — an amend in flight would have raced.
    The command center holds the merge until the agent's report is in. This is a discipline rule,
    not something the script enforces.

## The backlog

**113 runnable tickets, T-182 → T-297**, all `idea` so this program cannot start itself while
T-181 is live. Promote to `queued`/`ready` as you dispatch.

| Area | Tickets | Where |
|---|---:|---|
| Contract / `flatten` | 14 | `crates/map-engine-core/src/mission/` |
| Website — dead pages | 14 | `apps/website/frontend/src/` |
| Mission Creator — authoring | 12 | `apps/website/frontend/src/` |
| Website — event lifecycle | 11 | `apps/website/api/src/` |
| Mission Creator — data loss | 8 | `apps/website/frontend/src/` |
| Website — server manager | 8 | `apps/website/api/src/` |
| Arsenal / loadouts | 7 | frontend + `flatten.rs` |
| Data pipeline | 7 | `packages/tbd-schema/`, `tools/` |
| End-to-end test lane | 6 | `scripts/mod/`, CI |
| Discord | 6 | `apps/website/api/src/` |
| Infra / deploy | 6 | root, `docs/` |
| Registry hygiene | 6 | `xtask/`, `.ai/tickets/` |
| Mission Creator — collab | 5 | `apps/website/frontend/src/` |
| Mission Creator — settings | 4 | frontend + schema |

**Sizes: 50 S · 43 M · 16 L · 5 XL.** 15 are priority-0 — live bugs, not features.

**15 waves, all 8 wide except the last.** Wave 1 is entirely priority-0; wave 15 is T-290 alone.
Verified mechanically: zero intra-wave file collisions, zero directory-level `owns` claims, every
`DEPS` ordering constraint satisfied.

Three cancelled before the run because T-181 fixed or absorbed them: **T-184** (single-faction
hard-reject, fixed by T-181.46 43 minutes after the audit filed it), **T-210** (dead mission-list
route, fixed by T-181.51), **T-252** (merged into T-187 — same file).

The five most contended files are sequenced across waves rather than shared within one:
`flatten.rs` and `doc/store.rs` (9 tickets each), `eden_chrome.rs` (6),
`api/src/handlers/events.rs` and `mission/compile.rs` (5 each).

### Token budget

Measured from the 2026-07-25 transcripts: **93.1M non-cache-read tokens per 5-hour window**, 73% of
it cache *creation* — which is roughly fixed per agent, because every agent re-creates the system
prompt plus the 88 KB `CLAUDE.md`. True cost per agent ≈ **824k**, about 4-5× what the reported
`subagent_tokens` figure suggests.

| | |
|---|--:|
| 113 ticket agents (400k S / 700k M / 1.2M L / 2.0M XL) | 78.1M |
| 15 wave verifiers @ 600k | 9.0M |
| +20% retry / self-heal | 17.4M |
| **Total** | **104.5M** |
| **Windows** (93.1M each) | **1.12** |

Concurrency does not change the budget, only how fast it is spent — which is why 8 beats 4: fewer,
wider waves means fewer verifier runs. Expect exactly one park at the rate limit, then a short
finish after the reset.

## Sequencing note

Two tickets change the economics of everything after them, and should land early regardless of
wave order:

- **`make mod-world-boot-compiled`** (T-186) — feeds an API-compiled mission into the real Enfusion
  parser. No such test exists today, which is why the single-faction hard-reject shipped. This
  converts every future contract drift from *discovered in production* into a CI failure.
- **Seed a populated content golden** (T-193 area) — every frontend fixture is currently an empty
  `{"data":[]}`, which is precisely why six pages were shipped with their populated render branch
  never written, and why the `server_fps` type mismatch went unnoticed. Nothing in the
  "dead pages" area is safely fixable until the golden has rows in it.
