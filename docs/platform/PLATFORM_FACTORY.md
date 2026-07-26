# Platform factory — command center, worktrees, waves

**Operator-defined, binding. Read this before dispatching any platform slice agent.**
This is the process for the **T-182…T-295 backlog** (website, Mission Creator, contract, data,
infra). The mod program has its own: [`../mod/SLICE_WORKFLOW.md`](../mod/SLICE_WORKFLOW.md).

Do not start this program until **T-181 is finished**. Operator instruction.

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
