**Status:** live

# Cold start and preflight

Starts an orchestrating session for the factory: bring up the local services the gates and agents
use, sweep leftover build caches, prove the machine and checkout can run an unattended
[wave](/documentation_v2/glossary/n_to_z.md#wave), and read where the program stands. Run it at the start
of every orchestrating session; it takes a few minutes once the build cache is warm.

The orchestrator is deliberately a short-lived session. Once a session has been compacted a few
times, every turn pays to re-read a large context and compaction drops detail nobody chose to
drop, so start a fresh session and read the state from the repository instead. Do not try to
reconstruct an earlier session's reasoning: a decision that mattered is in a ticket or a code
comment, and one that is in neither was not recorded and is re-derived from the code.

## Prerequisites

- The repository checked out on `main`, with no uncommitted changes of the orchestrator's own.
- The host bridge. In the development container `cargo`, `rustfmt` and `trunk` are host binaries
  (the container's glibc is older); the wave driver detects the container and runs each step
  through `distrobox-host-exec`, and on the host it runs them directly. Either shell works for
  `cargo xtask platform wave`.
- A container runtime for the local Postgres, as
  [Local development](/documentation_v2/runbooks/local_development.md) describes.
- The oracle lanes `apps/mod/crf_framework/` and `apps/mod/vanilla_reference/` in the main
  checkout (both gitignored); `slice-worktree new` refuses to create a worktree without them.

## Where the state lives

| Source | What it holds |
|---|---|
| `.ai/tickets/wave.lock` | which tickets run together, compiled by `cargo xtask wave repack` |
| `.ai/tickets/T-<id>.toml` | every ticket's full record; the summary and acceptance fields are the slice brief, written to be pasted |
| `.ai/artifacts/worktrees/` | one worktree per slice in flight (`platform slice-worktree -- list`) |
| `.ai/artifacts/verdicts/` | the slice gate verdict receipts `land` reads |
| `.ai/artifacts/last-verified` | the commit the last adversarial verifier examined (gitignored) |
| the `metrics` folder of `.ai/tickets/` | the run receipts `platform slice-run` writes and `land` stamps; absent until the first receipt |
| this runbook folder | the process |
| [frontend data provenance](/documentation_v2/archive/audits/frontend_data_provenance.md) | an archived audit of which render sites show real data and which show mock data |

## Steps

1. Start the local Postgres on port 5434.

   ```bash
   cargo xtask db up
   ```

   Expected: compose starts the `tbd_reforger_db` container. The wave gate's API test step and
   its migration step both reach it through `podman exec tbd_reforger_db`.

2. In a second terminal, run the development API; leave it running.

   ```bash
   cargo xtask mk rust-api
   ```

   Expected: cargo builds the `api` binary into `target-dev-api/`, then the API logs
   `migrations applied` and `listening on 0.0.0.0:8080`.

3. In a third terminal, serve the single-page app in debug mode for the length of the wave; the
   release build (`cargo xtask mk leptos`) is for the operator's eye-pass only.

   ```bash
   cargo xtask mk leptos-debug
   ```

   Expected: `trunk serve` in `apps/website/frontend`, listening on `127.0.0.1:3000`. Leave it
   running during gates: the wave gate builds into its own `dist-gate-frontend` and
   `target-gate-trunk` folders and never touches the served `dist/`.

4. Sweep build caches that finished slices left behind.

   ```bash
   cargo xtask platform wave reclaim
   ```

   Expected: the live slices listed as spared, one line per removed folder with its size, and the
   free space at the end. The warm gate folders stay unless `--gate-dirs` is passed; the
   [reclaim README](/tools_v2/xtask/src/commands/platform/wave_execution/reclaim/README.md) lists
   every sweep.

5. Check the machine and the checkout.

   ```bash
   cargo xtask platform preflight
   ```

   Expected: one line per check and `PREFLIGHT: PASS (<n> warn)`, exit 0. Two warnings are
   normal: `CARGO_TARGET_DIR` unset in this shell, and worktrees of parked slices. Fix every
   `✗ BLOCK` line before anything else; the
   [preflight README](/tools_v2/xtask/src/commands/platform/preflight/README.md) gives each check's
   BLOCK and WARN conditions.

6. Read where the program stands.

   ```bash
   cargo xtask platform wave status
   ```

   Expected: `═══ platform program ═══`, `plan:  .ai/tickets/wave.lock`, the verify debt,
   `open:  <open> / <total> tickets`, `wave:  <n>`, then one line per ticket of the current wave
   (`SHIPPED`, `READY TO LAND`, `IN PROGRESS (uncommitted)`, `tree clean, no commits yet` or
   `not started`) and the collision command to run next. `verify: … <- OVERDUE` means eight or
   more landings have no verifier after them.

7. Check whether the current wave may be closed or the next one opened.

   ```bash
   cargo xtask platform wave wave
   ```

   Expected: `═══ wave <n> — <shipped>/<total> shipped ═══`, the open tickets, the verify debt,
   and either `STATUS: wave <n> is OPEN — finish it before dispatching wave <n+1>.` or
   `STATUS: wave <n> tickets are all shipped. Run 'cargo xtask platform wave wave --close' to gate
   and advance.`

## Verify

```bash
cargo xtask platform slice-worktree -- list
```

Expected: `git worktree list` output: the main checkout on `[main]` and one
`.ai/artifacts/worktrees/<slice id>` row per slice in flight, on `[slice/<slice id>]`. A row for a
slice that has shipped is a leftover; `cargo xtask platform slice-worktree -- reap` removes merged,
clean ones.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `✗ BLOCK working tree  dirty — commit or stash first` | uncommitted changes in the main checkout | commit them, or ask whose they are; never `git stash` (see [Known traps](/documentation_v2/runbooks/factory_waves/known_traps.md)) |
| `✗ BLOCK disk  <n>G free — below the 20G floor` | build caches filled the disk | step 4; `cargo xtask platform wave reclaim --gate-dirs` also removes the gate folders, which the next gate rebuilds cold |
| `✗ BLOCK run target …` | a binary under the shared cache's `run-main` has no `tbd-built-from` stamp, or one naming another commit or checkout | rebuild it from the main checkout with `cargo xtask platform wave run <cargo arguments>` |
| `! WARN api :8080  healthy but STALE — running since …, API code changed …` | the running API kept its old binary after cargo relinked | stop it and repeat step 2; a stale API answers confidently with old behaviour, which reads like a real defect |
| `✗ BLOCK ticket check  registry INVALID — every wave gate will fail` right after a `ticket ship` | the shipped ticket has no `shipped_at` yet | `cargo xtask ticket stamp-sha <ticket id> <landing sha>` |
| `✗ BLOCK wave lock  cargo xtask wave check failed — stale or missing …` | the lock no longer matches the tickets | `cargo xtask wave repack`, then commit the lock |
| `wave: NOTE — this is the HOST shell, not the dev container.` | the driver runs on the host | nothing: it runs cargo, rustfmt and trunk directly |

## Related

- [Local development](/documentation_v2/runbooks/local_development.md) — the local stack in full.
- [Running a wave](/documentation_v2/runbooks/factory_waves/running_a_wave.md) — the next step.
- [Platform factory commands](/tools_v2/xtask/src/commands/platform/README.md) — every
  `platform` subcommand and its exit codes.
