# T-912 — Wave Lockfile Architecture

Program hub: tickets are the only source of `owns` and `depends_on`; `cargo xtask wave repack` compiles a committed [`.ai/tickets/wave.lock`](../../.ai/tickets/wave.lock); both `wave_plan.tsv` files are deleted in the same commit as the first lock.

**Do not write the `STRICT_LEGACY` phrase matching `Track [ABC]\b` in this spec, the ticket title/summary/notes, README, or any commit subject/body that `ticket sync` copies into `docs/TICKET_*.md`.** Scanner: `xtask/src/constants.rs`.

Design contract: approved plan sections “Wave Lockfile Architecture” (originally numbered T-911) plus operator amendments 2026-08-14. Registry redesign is **T-911** (shipped). Metrics producers are **T-913**, not this program.

## Slices

`parallel_ok` is **false**. Both slices own `.ai/tickets/` and xtask wave/check paths. Serial waves only.

| Id | Status at mint | What |
|----|----------------|------|
| T-912.1 | ready | Owns on Work tickets + dependency graph on tickets. Populate `owns` from both TSVs. Merge the 11 `DEPS` edges into `depends_on`. `pack_last` on T-290. Delete `const DEPS` / `RUN_LAST`. Nonempty owns required on Work `{queued,ready,running,review}`. TSV still exists. |
| T-912.2 | queued | TOML `wave.lock` compiler + `wave check`. Delete both TSVs in the **same commit** as the first lock. Retarget every reader. `TBD_WAVE_PLAN` and `TBD_WAVE_GENERATION_FLOOR` die. Land runs repack (lifecycle a). Candidate order after TSV death = ticket `order`, tie-break id. |

T-912.2 depends on T-912.1 landed.

## Verified pins (do not invent)

- `owns` / `pack_last` already exist on `WorkTicket` / `ProgramTicket` (`crates/tbd-tickets`). Zero live files set them. T-912 does **not** add a new corpus key. The deliberate T-912.1 commit is the nonempty-owns **check**, not an `ALLOWED_NEW` widen (`ALLOWED_NEW` is T-913.1).
- `const DEPS` at `xtask/src/slice_collisions.rs:86` (11 edges) + `RUN_LAST = ["T-290"]`. Packer does not read ticket `depends_on` today.
- T-212 `depends_on` is `["T-685"]` — merge, do not replace: `T-685`, `T-241`, `T-257`.
- `load_phase2_tree` loads **parents only**. Compiler and owns-check must glob every `T-*.toml`.
- Missing TSV today: `ledger::plan_rows` `unwrap_or_default` → false-green “no waves”. Missing lock after T-912.2 = DidNotRun refuse.
- Cap: `TBD_MAX_CONCURRENT` default 8.
- Mine `git show 6ae8069c:xtask/src/wave_lock.rs`. **Never restore** that commit. Re-derive against encoding C. Draft lock was JSON; **on-disk format is TOML**.

## T-912.1 must-ship

1. Copy `owns` from `docs/platform/wave_plan.tsv` and `docs/mod/wave_plan.tsv` onto every matching `T-*.toml` (any status). Queued/ready/running/review Work **not** in either TSV: assign owns from that ticket’s named paths in the **same commit**.
2. Work `{queued,ready,running,review}` with empty `owns` → `ticket check` red. Glob **all** ticket files, not `tickets[]` parents only.
3. Merge the 11 `DEPS` edges onto ticket `depends_on`. `pack_last = true` on T-290.
4. Delete `const DEPS` and `RUN_LAST`. Packer reads ticket `depends_on` / `pack_last` / `owns`. A test asserts `slice_collisions.rs` no longer contains `const DEPS`.
5. Do not delete the TSVs. Do not add `created_at`. Do not invent `ALLOWED_NEW`.

### T-912.1 proofs (paste stdout)

1. Empty owns on a queued/ready Work → `ticket check` red; restore.
2. `rg 'const DEPS' xtask/src/slice_collisions.rs` empty; unit test fails if the const returns.
3. Each of the 11 edges present on the dependent ticket. T-290 has `pack_last = true`.
4. `cargo xtask ticket check --strict` prints `check OK`.
5. `cargo xtask platform wave gate --slice T-912.1` prints `GATE: PASS`.

## T-912.2 must-ship

1. `cargo xtask wave repack` compiles `.ai/tickets/wave.lock` from tickets (greedy file-disjoint packing, today’s algorithm). **Only legal writer**, also invoked as the final step of `platform wave land`.
2. Lock is **TOML** (same stack as `T-*.toml`). Native `#` header: auto-generated, do not hand-edit. Not JSON, not JSON-with-a-preamble. Deterministic emit (pinned key/table order, trailing newline). Fields: `version`, `max_concurrent`, `waves` (`n` + `tickets`), `owns`, `depends_on`, `pack_last`.
3. First lock candidate order = **TSV row order** (platform TSV then mod TSV). First lock’s open-ticket packing **set-equals** today’s TSV packing per wave (labels may remap to 1..N); owns snapshot included; wave 0 = undispatchable / landed, frozen. Paste the table.
4. **After the TSVs die**, every later repack sorts **open** candidates by ticket `order`, tie-broken by `id`. Filesystem glob order is forbidden. Wave 0 ignores `order`.
5. `cargo xtask wave check` recomputes and structurally equals the lock. Missing lock = DidNotRun, never “no waves”. Wired into `ticket check`, `platform wave gate`, and `platform_preflight`.
6. **Lifecycle (a):** `platform wave land` runs `wave repack` as its final step (lock commit rides the land). `wave check` stays a full structural equals. Command-center `ticket ship` / `set-status cancelled` after this slice exists must invoke the same writer. Rejected: open-only check with “pending repack”.
7. Delete `docs/platform/wave_plan.tsv` and `docs/mod/wave_plan.tsv` in the **same commit** as the first lock. `TBD_WAVE_PLAN` and `TBD_WAVE_GENERATION_FLOOR` die; reading either is check-red. The lock subsumes the floor: wave 0 = landed, waves 1+ = open only, `current_wave` = first lock wave n>0 with an unshipped id.
8. Retarget live readers (xtask wave/mod_wave/preflight/fixtures + FACTORY / EDITOR_FACTORY / CLAUDE.md / SLICE_WORKFLOW). Historical allowlist for old docs — plant a TSV path in `wave/mod.rs` → check red. `git grep wave_plan.tsv` minus allowlist is EMPTY.
9. `slice-collisions --repack` becomes an alias of `wave repack` or is deleted. Unplanned warning must not key on dropped `program=="platform"`.

### T-912.2 proofs (paste stdout)

1. `wave repack` twice on frozen tickets → `cmp` byte-identical lock.
2. Two ready tickets with overlapping owns → never same wave; perturb owns without repack → `wave check` red.
3. Unshipped `depends_on` cannot pack earlier; remove an edge without repack → check red.
4. After TSV delete: `git grep wave_plan.tsv` minus historical allowlist is EMPTY; plant a TSV path in `wave/mod.rs` → check red.
5. Migration table pasted (open-id sets + owns snapshot).
6. Missing lock → DidNotRun, not “no waves”.
7. Amendment 1: ship one ticket on a copy, repack, repack again → byte-identical; reorder an OPEN ticket, repack → waves 1+ may change, wave 0 untouched.
8. Amendment 2 (a): ship via land (writer already ran) without a second manual repack → check/gate **green**; perturb open `owns` without repack → **red**.
9. Lock file parses as TOML (a JSON parser on the bytes is not required).
10. `cargo xtask ticket check --strict` prints `check OK`.
11. `cargo xtask platform wave gate --slice T-912.2` prints `GATE: PASS`.

## Illegal in this program

T-913 metrics (`created_at`, `completed_at`, `tokens_consumed`, `slice-run`). Restoring `scratch/track-b-draft`. Pushing to origin.

## Claude Code prompt — T-912.1 (copy-paste)

Authority: this spec. Factory slice worktree. Do not edit the primary checkout.

```
Read CLAUDE.md first.

Implement **T-912.1** — Owns on Work tickets and dependency graph on tickets.

═══ PREFLIGHT ═══
  CWD = the T-912.1 worktree. Branch slice/T-912.1.
  export CARGO_TARGET_DIR=<worktree>/target-ctr
  Never shared target/. Never target-container.
  Do not push. Do not restore 6ae8069c.

═══ DO ═══
  Populate owns from both wave_plan.tsv files onto every matching T-*.toml.
  Require nonempty owns on Work queued/ready/running/review (glob all files).
  Merge the 11 DEPS edges onto depends_on; pack_last=true on T-290.
  Delete const DEPS and RUN_LAST. Packer reads tickets.
  Paste proof stdout. GATE: PASS before asking to land.

═══ DO NOT ═══
  Delete wave_plan.tsv. Add created_at. Write Track [ABC] in any synced prose.
  Invent ALLOWED_NEW. Implement wave.lock (T-912.2).
```

## Claude Code prompt — T-912.2 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-912.2** — Compile wave.lock and delete wave plan TSV files.

═══ PREFLIGHT ═══
  CWD = the T-912.2 worktree. T-912.1 must already be on this main.
  export CARGO_TARGET_DIR=<worktree>/target-ctr
  UNSET CARGO_TARGET_DIR before wave gate. cargo xtask db up first.
  Mine git show 6ae8069c:xtask/src/wave_lock.rs — never restore that commit.

═══ DO ═══
  TOML wave.lock; cargo xtask wave repack is the only writer; land runs it last.
  First lock: TSV row order; set-equal TSV open packing; paste the table.
  After TSV delete: candidate order = ticket order, tie-break id.
  wave check = full structural equals. Missing lock = DidNotRun.
  Delete both TSVs in the SAME commit as the first lock. Retarget readers.
  TBD_WAVE_PLAN and TBD_WAVE_GENERATION_FLOOR die (grep ban).
  Proofs including amendment 1 and 2(a). GATE: PASS.

═══ DO NOT ═══
  JSON lock. JSON-with-hash-preamble. Open-only check with pending-repack.
  Glob-order sorting. Leave a TSV path in live xtask/docs.
  Write Track [ABC] in any synced prose. Push.
```
