**Status:** live

# Known traps

The traps that have cost the factory real hours, what each one looks like, and the habit that
catches them. Every slice brief rule and several tool refusals exist because of one of these.
Read it before a first wave, and again whenever a green result looks too easy.

## The signature defect

Almost every expensive failure in this codebase has one shape: **a tool reports success over an
input it never examined.** Each instance looks like a green check.

| Instance | What it silently did not examine | What guards it now |
|---|---|---|
| a gate runs the database test suite with no database | every test prints `skip:` and the step passes | the gate's API test step refuses with `REFUSING to call this a pass: <n> DB-backed test(s) SKIPPED.` |
| clippy runs without the crate's features | every feature-gated module | the gate's clippy and map engine tests use `--all-features` |
| a browser assertion checks "not null" | every probe returns an object, so every assertion passes | assert on the measured value; a claim about pixels needs a guard that measures pixels |
| a shared cargo target folder serves another worktree's binary | "126 passed" is not the slice's own code | private folders for tests (`platform wave test --slice`), gate steps and launched binaries |
| a landing command ignores an argument | it lands slices whose agents have not reported | every `land` and `wave --close` argument parser is an allowlist |
| a golden file under `#[serde(flatten)]` | a deleted field is re-emitted and the JSON stays byte-identical | read what the golden covers, not only that it matches |
| the collision analysis | tickets with no plan row are never candidates | `slice-collisions` warns about dispatchable tickets missing from the lock |
| a health check that is a TCP connect | a six-hour-old API binary reports as up | preflight reads `/healthz` and compares the API process's age with the newest API commit |
| a test that `include_str!`s its own file and greps for a literal | the needle matches the test's own assertion, forever | scrub the test module out of the haystack with `class_r_scrub::live_source` or `live_code` (`apps/website/frontend/src/v2/core/test_support/class_r_scrub/`) |
| `cargo check` under lock contention | it replays a cached verdict and prints `Finished`, exit 0, over code that does not compile | the gate checks into its own `target-gate-check` folder and invalidates fingerprints first |
| a piped gate (`cargo check … \| tail -5`) | the pipeline returns `tail`'s exit status | never pipe a gate or test; the wave driver captures each step's status itself |
| a grep loop that reads exit 2 as "no match" | an invalid pattern becomes a pass | read the exit status: 0 match, 1 no match, 2 error, 127 tool absent; the last two fail closed |

A class of its own is always a BLOCKER, however small it looks: a gate that reports success on code
it never examined makes every other claim in the program worthless until it is fixed. Treat any
green you did not watch fail first as unproven.

## Prerequisites

- A slice worktree or the main checkout with the change under test committed or saved.
- The check that is supposed to guard the change: a test, a gate step or a verification.

## Steps

The perturbation habit, which every slice brief demands and every verifier repeats: prove the
check can fail before trusting that it passes.

1. Break the code the check guards on purpose: delete the line, restore the old value, invert
   the condition.
2. Run the check and capture its red output verbatim. A check that stays green examined nothing.
3. Restore the code.
4. Touch the restored file. Cargo's freshness test compares modification times, and a file
   restored with `git checkout` can be older than the artifact built from the broken version, so
   the tool can report the broken verdict over correct source.

   ```bash
   touch <restored file>
   ```

   Expected: no output; the next build recompiles the owning crate (a `Compiling` or `Checking`
   line names it).

5. Run the check again and confirm it is green, then read the build output for the `Compiling`
   or `Checking` line of the crate: without it the verdict came from the cache.

## Verify

The slice report pastes the red output of step 2 and the green of step 5; the gate verdict tail
ends `SLICE GATE: PASS`. A report without the red output is asserted, not verified, and goes back
([Slice agent brief](/documentation_v2/runbooks/factory_waves/slice_agent_brief.md)).

## Build and cache traps

- **Cargo is a host binary.** The development container's glibc is older than the host's; host
  cargo, rustfmt, trunk and the xtask binary refuse to load inside it. Reach them through
  `distrobox-host-exec`, which forwards no environment: pass each variable with `env NAME=value`.
  `cargo xtask platform wave` detects the container and bridges its own steps; on the host it
  prints a `NOTE` and runs them directly.
- **One shared cache, several private ones.** Every worktree building into its own `target/`
  would cold-build the whole workspace and cost tens of gigabytes each; preflight blocks a
  worktree that did. The wave driver drops an inherited `CARGO_TARGET_DIR`, announces it, and
  uses `<main checkout>/target` for checks and clippy, `target-gate-*` and `dist-gate-frontend`
  for its own steps, `$HOME/.cache/tbd-target-<id>` for `platform wave test --slice`, and
  `<shared cache>/run-main` for binaries it launches from `main`.
- **Two checkouts, one artifact.** Cargo's artifact hash omits the manifest path, so a worktree
  and the main checkout building the same package into one folder share one binary, and the
  second build never recompiles. That is how a server on `main` comes to run unmerged slice
  code. `platform wave run` builds launched binaries into `run-main` with a `tbd-built-from
  <sha> <checkout>` stamp and refuses to run from a worktree; preflight blocks an unstamped or
  mismatched binary there.
- **A slice's own server.** A slice agent that runs its own API instance builds it into
  `target-<ticket id>-api` in the main checkout and greps the binary for a string unique to its
  change before trusting any HTTP result; `platform wave reclaim` sweeps the folder once the
  worktree is gone.
- **The shared cache replays verdicts.** Under lock contention the sign is `Blocking waiting for
  file lock`, then `Finished` with no `Compiling` or `Checking` line. `--quiet` hides that line,
  so a hand-run `cargo check --quiet` has no tell at all.
- **Feature-gated crates.** `website-map-engine`'s default feature is `scenario` alone; a bare
  `cargo test -p website-map-engine` compiles only that tier. Pass `--all-features`;
  `platform wave test --slice` isolates the folder but does not add features.
- **Editions differ.** The frontend crate is edition 2021 and the other workspace crates 2024,
  and their import orders differ; the gate runs rustfmt on each changed file with its own crate's
  edition.
- **Never `cargo xtask ci ci-local` inside a wave.** It takes 15 to 40 minutes; a gate everyone
  routes around teaches agents that failures are noise. The language bans it carries
  (`verify no-python`, `no-node`, `no-shell`, `ci-shell`) are steps of the wave gate and of the
  `language-gates` CI job.
- **No `.py` file is ever committed.** `cargo xtask verify no-python` fails on one; scratch
  scripts stay outside the repository.

## Shell, git and host traps

- **`rg` is not installed.** An agent harness can inject a shell function named `rg`, so
  `command -v rg` succeeds in an agent's shell and fails in `bash -c`, a script or CI. A gate that
  calls `rg` passes only when an agent runs it.
- **`grep` may not be GNU grep in an agent's shell.** A harness can shadow it with another
  implementation that rejects bare ERE braces (`^GET /a/{id}$`), which every API route contains.
  Use `grep -F` for route-shaped patterns, read the exit status, and in Rust use
  `verification_core::gate::{require, ban}` (`tools_v2/verification-core/src/gate.rs`) instead of
  hand-rolled scans.
- **A grep answers a narrower question than the one asked.** `grep -w` is a narrowing step, not a
  verdict; read the matches before reporting a count.
- **Never `git stash`.** A stash in these trees can delete LFS pointer files. Preflight's BLOCK
  line still reads `dirty — commit or stash first`; commit instead.
- **Stage explicit paths.** `git add <folder>` while agents are writing into it has committed a
  file mid-write.
- **git-lfs.** When git-lfs is missing but `filter.lfs.process` is configured, plain `git status`
  and `git add` can abort; pass `-c filter.lfs.process= -c filter.lfs.required=false`, scoped to
  your paths. Push with `cargo xtask platform wave push`, which asks git which paths are LFS
  before it bypasses the hook.
- **The development API goes stale.** A running process keeps its old binary after cargo relinks.
  A stale API is worse than a dead one: it returns confident wrong answers that read as real
  defects. Preflight warns `healthy but STALE`; restart it.
- **Orphans leak.** A dead agent's API process can keep listening, and orphan build folders have
  filled the disk until gate steps failed with "No space left on device", which reads as a build
  error. `cargo xtask platform wave reclaim`; preflight blocks under 20 GB free.

## Agent and process traps

- **Agent reports are evidence, not testimony.** Agents are reliable about the code they touched
  and unreliable about bookkeeping: several reported follow-up tickets that were never filed,
  one of them a P0. Check every "I filed", "a sibling fixed it" and "this already works" against
  the registry and `git log`; it costs one command and has caught something every time.
- **An agent's chat summary is not an artifact.** Quotations, statistics and ticket scope have
  entered the record from summaries that said things their files did not. Cite the file.
- **A rate-limited agent can report `completed`.** A report carrying a rate-limit or reset message
  is a failed agent, not a finished one.
- **Measure, do not read.** "Does not reproduce" verdicts from reading the source have been wrong
  (a dialog really was off-screen because a `backdrop-filter` ancestor captured `position: fixed`);
  stale premises also exist. Check, and assume neither way.
- **Parallel arrays.** Build ids, positions, headings and tints in one pass over one sorted
  source; `vehicle_rows()` is id-sorted while `vehicle_xy_flat()` follows map order, and mixing
  them gives every vehicle another's heading.
- **The z policy.** `update_slot_position` (`apps/website/map-engine/src/data/store/rows/transforms.rs`)
  resets z to 0 when an edit moves x or y without a z, by design. A caller that must keep a
  manual z reuses `keep_z_rows` and `slot_z`
  (`apps/website/map-engine/src/data/store/operations/attrs.rs`) and reads z from the slot rows,
  never from the flattened arrays.
- **Scrubbed pins can go blind.** `class_r_scrub` cuts from the first `#[cfg(test)]` to the end of
  the file, so a file with a test-only item inside a production module is examined only up to
  that item, and the scrub reports every later needle as absent (it fails closed). A pin on such
  a file needs an anchor near its target and a canary.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| a test run reports passes but `--list` shows none of the slice's tests | another worktree's binary in the shared cache | `cargo xtask platform wave test --slice <ticket id> -p <package>` |
| a perturbation stays red after the restore | cargo kept the broken build | step 4: touch the restored file |
| a gate step fails with "No space left on device" | orphan build folders | `cargo xtask platform wave reclaim` |
| a gate that calls `rg` passes for an agent and fails in CI | `rg` is a harness function | use `grep -E` or the `verification_core` helpers |
| `GLIBC_2.<n> not found` | a host-built binary ran in the container | run it through `cargo xtask` or `distrobox-host-exec` |

## Related

- [Slice agent brief](/documentation_v2/runbooks/factory_waves/slice_agent_brief.md) — the rules
  these traps produced.
- [Platform wave driver](/tools_v2/xtask/src/commands/platform/wave_execution/README.md) — the
  target folders, the run stamp and the step capture.
- [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md) — every check and where it runs.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — headless browser traps of the
  editor smokes.
