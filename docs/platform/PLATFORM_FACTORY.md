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
9. **EVERY agent runs on Opus 5** — slice agents, verifiers, throwaway research agents alike.
   Operator instruction. Pass `model: "opus"` explicitly on every dispatch. Do not downgrade to
   work around 529s, rate limits, latency or cost; retry instead.
10. **`owns` is load-bearing and partly derived.** 75 of the 114 rows were built from file
    citations in the ticket summary; **39 fell back to their area's default directory**. Those are
    deliberately over-broad — they will serialise more than necessary rather than risk a collision.
    Narrow a row before dispatching its wave, and re-run `--repack`.

## The backlog

114 tickets, **T-182 → T-295**, all filed as `idea` so this program cannot start itself while
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

15 waves, packed 8-wide by disjointness in priority order. Wave 1 is entirely priority-0.

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
