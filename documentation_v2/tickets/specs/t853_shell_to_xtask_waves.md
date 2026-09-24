# T-853 — shell → xtask, the wave-dispatched slices (T-854…T-880)

Slice spec for waves **218–226**. One ticket = one shell script = one slice. Three run per wave.

The hard, high-risk half of T-853 (`wave.sh`, `deploy-staging.sh`, the DB lane, the engine
wrappers, the two self-referential gates) is **not** in these waves and is not yours — see
[§Not in scope](#not-in-scope).

---

## The one rule that matters

**A gate that reports nothing is indistinguishable from a gate that checks nothing.**

Every acceptance in this program is byte-for-byte against the bash it replaces, on a clean tree
**and** on deliberately broken ones. A green run alone is not evidence and will be rejected. This
is T-556's anti-vacuity discipline, and it is not ceremony — it has already caught, in this
program:

- a gate whose first cut **passed the known-broken files** (`verify-ui-layouts`, GUID strings
  desynced its brace counter);
- an anti-vacuity harness that was **itself vacuous** (`verify-t440` printed three *fabricated*
  `RED proof: … FAIL (expected)` lines when a `2>/dev/null` swallowed a decode error);
- a verifier agent whose own test arm was vacuous — it perturbed the wrong file and both sides
  "agreed" at rc=0. It only noticed because its harness **asserts the bash side went red before
  believing a diff**. Do that.

---

## Procedure per slice

### 1. Capture the baseline FIRST, before writing any Rust

```bash
export CARGO_TARGET_DIR="$PWD/target-container"     # never bare cargo — see §Environment
bash <the-script> > /tmp/t853/<name>.old 2>&1; echo "rc=$?" >> /tmp/t853/<name>.old
```

Then run it **twice more** and diff the captures against each other. If they differ, the output is
not reproducible and a plain diff cannot be your acceptance — see §Non-reproducible output.

### 2. Find every caller before you touch anything

```bash
git grep -n '<script-basename>' -- Makefile scripts .github docs
```

**"No Makefile target" does NOT mean dead.** That heuristic was wrong twice in this program:
T-462 wired three `verify-t*.sh` into `scripts/platform/wave.sh` only, and `verify-ui-layouts.sh`
lives in `scripts/mod/wave.sh`. Grep the whole repo, always.

### 3. Write the port against `crates/tbd-gate`

Do **not** hand-roll verdicts, process handling, or file walking. Read the library first:
`crates/tbd-gate/src/{lib,verdict,gate,pattern,scan,proc,report,lock}.rs`.

| need | use |
|---|---|
| `grep -E` / `-F` / `-i` | `Pattern::regex` / `Pattern::literal` / `.case_insensitive()` — multi-line, so `^`/`$` are LINE anchors exactly like grep |
| whole-file ban/require | `gate::{ban, require, ban_str, require_str} -> Verdict` |
| compound conditions | `gate::{probe_files, probe_str} -> Result<bool, NotRun>` — `?` propagates "did not run" |
| `grep -rn` over a tree | `scan::{walk_files, grep_lines, with_extension}` → `Hit{path,line_no,line}`; `walk_files` **sorts**, so output never depends on readdir order |
| run a program | `proc::Run` — raw exit codes, signals are `NotRun::Signalled`, timeouts kill the process **group** |
| capture `2>&1` | `proc::Run::merged_output()` — one shared pipe. `output()` drains the two streams separately and re-joining them **invents an interleaving** |
| `which` | `proc::which` → `NotRun::ToolAbsent` |
| many checks, one exit | `report::Report` |

`Verdict` is `Held | Failed(Finding) | DidNotRun(NotRun, Finding)` with **no bool conversion** — you
must `match`. That is the point: "the check did not run" cannot fold into "the check passed".

**Every fail-open shape you find must be closed and named in a comment.** The common ones:
`2>/dev/null`, `|| true`, `if grep …; then fail; fi` (exit 127 reads as "no match"), and
`$(…)` swallowing a subprocess death.

### 4. Prove it bites

Build throwaway roots under `/tmp` — most of these scripts derive `ROOT` from `$0`, and
`find_repo_root()` looks for a `.ai/tickets/registry.json` marker, so create one. Run **both** the
bash and your port against each perturbation and diff stdout+stderr+rc.

**Never perturb the real tree.** If unavoidable: `sha256sum` first, restore, and prove
`git status` is clean afterwards.

Minimum two broken arms; more for anything with several independent checks. Your harness must
assert the bash side actually went red before it believes a diff.

### 5. Land it — script, inventory, and callers in ONE commit

- delete the `.sh`
- delete its line from `scripts/shell-inventory.txt` — **both ratchets fail on a stale entry**, so a
  deletion that leaves the line behind turns `main` red
- delete its line from `scripts/python-inventory.txt` if the port removed the last `python3` call
- repoint every caller found in step 2

In `scripts/platform/wave.sh`, cargo **must** go through `checkrun`, never bare:

```bash
run "T-4xx label"  checkrun cargo run -q -p xtask -- verify <name>
```

`checkrun` pins `CARGO_TARGET_DIR=$GATE_CHECK_TARGET`. A bare `cargo` there writes into the shared
52 GB cache — the cross-worktree false-binary class T-742 exists to prevent.

---

## Two traps that break a CORRECT tree

**`verify-t468-ci-schema-parity.sh` pins other gates' Makefile recipes** as
`^\t@?bash <exact-script-path>`, over a `bash_pins` tuple. Repointing a pinned recipe to cargo
fails that gate unless its tuple is dropped in the same commit. It currently pins `verify-t456`
and `verify-t468` — both are mine, not yours, but check `bash_pins` before repointing anything.

**Some gates read their own call sites.** `verify-t440` pins that both `wave.sh` paths invoke it,
matching on a literal; repointing without moving the const fails the gate on a correct tree. If
your script inspects the Makefile or `wave.sh`, its consts and its call sites are one atomic change
— and derive test fixtures **from** the const so they cannot drift.

---

## Preserve bash oddities; do not "fix" them

If the bash does something wrong, **reproduce it, pin it with a test, and document it**. Changing
what a gate reports is a behaviour change and belongs in its own ticket. Ports in this program have
already carried over, deliberately: a defeated blank-line filter, two unreachable branches, an
over-broad ban pattern, a duplicate-key overwrite, and a comment-stripper that wedges on a
backslash inside single quotes.

The exception is a fail-open that makes the gate lie about having run. Close those, and say so.

---

## Non-reproducible output

Some scripts shell out to cargo, and their output embeds wall clocks, `Compiling` lines from a cold
cache, and `$CARGO_TARGET_DIR` paths. For those, a stored baseline can **never** diff clean — not
even bash against itself (`verify-t180` differs on 9 of 803 lines between two consecutive runs).

Acceptance there: run bash and the port **back to back in one warm target dir**, normalise
(`sed -E 's/ in [0-9]+\.[0-9]+s/ in Xs/g'`), diff, and separately show the bash-vs-bash noise floor
is identical. Say in your report that you did this and why.

---

## Environment

Agent containers are `debian:12` / glibc 2.36. If `cargo build` dies with ``linker `cc` not found``:

```bash
sudo apt-get update -qq && sudo apt-get install -y -qq build-essential
```

**Always** `export CARGO_TARGET_DIR="$PWD/target-container"`. Two glibcs building into one target
dir thrash fingerprints, and the repo's shared `target/` is ~52 GB and warm for every worktree
(T-253/T-322). `target-*/` is already gitignored.

## Size

Keep a module under 600 lines (SIZE-1 warn). If an honest port cannot fit, **report it with a
proposed split** rather than exceeding silently or deleting substance — that is an accepted
outcome. Landed so far: 596 / 599 / 715 / 832 / 879 after review. Never exceed **1000** (SIZE-3
hard fail); split at a seam with no shared state, as `gate_ui_layouts` / `gate_ui_layouts_awk` did.

## Definition of done

- [ ] clean-tree diff empty (or the normalised recipe above, with the noise floor shown)
- [ ] ≥2 broken arms, bash-vs-port diffs empty, rc matching, harness asserts bash went red
- [ ] script deleted; `shell-inventory.txt` line deleted; `python-inventory.txt` line deleted if applicable
- [ ] every caller repointed; `git grep '<basename>'` returns no executable reference
- [ ] `cargo fmt --all --check`, `cargo clippy -p xtask --all-targets -- -D warnings`, `cargo test -p xtask` all clean
- [ ] `cargo run -q -p xtask -- verify no-shell` count dropped by exactly one
- [ ] `cargo xtask verify no-python` still passes
- [ ] both `wave.sh` files pass `bash -n`

---

## Not in scope

Do not touch these. They stay on the human/Claude lane because each one either runs the factory you
are dispatching from, drives a remote host, or is blocked behind something that does:

| script | lines | why it is not waved |
|---|---|---|
| `scripts/platform/wave.sh` | 3614 | **Is the factory.** Porting it while running it is an engine swap mid-flight; it also encodes 63 commits of measured fixes and needs a two-wave verdict-diff before its bash can be deleted |
| `scripts/mod/deploy-staging.sh` | 1889 | Drives a remote host over SSH with no local reproduction |
| `scripts/mod/run-playtest-server.sh` | 973 | Live dedicated-server lifecycle |
| `scripts/mod/slice-worktree.sh` | 314 | Creates the worktrees the wave runner is using **right now** |
| `scripts/lib/hostrun.sh` | 86 | The container↔host bridge; it is what makes the others runnable at all, so it dies last |

**Parked, not skipped —** T-879 (`lib/paths.sh`, `lib/mcpd-bin.sh`, `lib/xtask-run.sh`) and T-880
(`lib/gate-grep.sh`) are correct deletes that are simply not legal yet: those libs are still sourced
by scripts above and by slices in waves 227–229. Land them only when
`git grep -l '<lib-basename>' -- scripts/` returns nothing but the inventory. Do not force them.

### Waves 227–230 moved the line

The original leaf/setup/utility set is finished (waves 218–226, 47 → 22 scripts). Waves 227–230 hand
over the deterministic-but-nontrivial remainder: the two mutually-pinned gates, the LANG-2 enforcer,
the ticket shim, the four-script DB lane, the MCP daemon, factory preflight, the **mod** wave driver,
and the two engine wrappers.

Three of those carry a hazard the earlier waves did not — read before starting the slice:

- **`restore-db.sh` is destructive.** Its T-381 allow-list *refuses* `tbd_reforger` and verifies the
  dump before writing. Both are load-bearing; prove the refusal still fires. Test against a scratch
  DB from `cargo xtask db up`, never the dev database.
- **`compile.sh` / `world-boot.sh` have a 0/1/2 exit contract** that `mod-gates.yml` and `ci.yml`
  branch on with `|| rc=$?; case $rc`. `--selftest` passes **only** on exit 1 — a selftest that exits
  0 means the gate is hollow.
- **`verify-t456` and `verify-t468` are one slice, not two.** t468's `bash_pins` pins t456's Makefile
  recipe *and its own*, so porting either alone fails the other on a correct tree.

You **will** edit `scripts/platform/wave.sh` and `scripts/mod/wave.sh` to repoint your own call
sites. That is expected. Do not restructure them.
