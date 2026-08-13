# T-911 — Ticket registry redesign: per-file split + typed schema

Program hub for splitting [`.ai/tickets/registry.json`](../../.ai/tickets/registry.json) into one TOML file per parent and per existing `slice_plan` / `slices[]` key, then typing Scope/Status.

**Do not write the `STRICT_LEGACY` `Track [ABC]\b` phrase in this spec, the ticket title/summary/notes, README, or any commit subject/body that `ticket sync` copies into `docs/TICKET_*.md`.** The scanner in `xtask/src/constants.rs` walks `docs/`.

Wave lockfile compile and metrics producers (`created_at` / `completed_at` / `tokens_consumed`) are **T-912+**, not this program.

## Slices

| Id | Status at mint | What |
|----|----------------|------|
| T-911.1 | ready | Phase 1: isomorphic TOML. Same 27 corpus keys. Loader + `.ai/tickets/ROOT` + the 12 fixtures that plant `.ai/tickets/registry.json`. Dual-read `wave_ledger_unshipped_at`. Byte-identical JSON emit vs the **cutover** monolith, then delete the monolith in the same commit. |
| T-911.2 | queued | Phase 2: typed Scope/Status, encoding C, frozen-49, Ready prose, T-159.23 shipped, T-674/T-675 children via mapper. Depends on T-911.1 landed. |

`parallel_ok` is **false**. Both slices own `.ai/tickets/` and the xtask loader. Serial waves only.

## Corpus count (do not hardcode)

Minting this parent (+ two `slice_plan` children) happens **before** T-911.1. Parent `N` at cutover is `tickets.length` of the committed monolith **in that session**, not 884.

Proofs 1–3 **read** `N` from the cutover blob. Invariant is **set-equality** of ids (parents ∪ `slice_plan` keys ∪ `slices[]` == on-disk `T-*.toml` stems), not `== 884`.

## FROZEN_27 (phase 1 union of parent keys)

Kept: `id`, `title`, `summary`, `status`, `executor`, `notes`, `depends_on`, `unblocks`, `priority`, `spec`, `user_story`, `acceptance`, `shipped_at`, `order`

Renamed in phase 2 only: `active_slice`, `slices`, `implements`, `route`

Dropped in phase 2 only: `impact`, `surfaces`, `program`, `stream`, `targets`, `branch`, `milestone`, `parallel_ok`, `slice_plan`

Phase 1 keeps JSON field names as TOML keys. A 28th corpus key fails.

Schema-only ghosts (`surface_detail`, `assignee`, `conflict_surfaces`) are **not** in the 27 (0 corpus hits at the proposal freeze).

## T-911.1 must-ship

1. One `T-*.toml` per parent id plus one file per existing `slice_plan` key / `slices[]` entry.
2. Loader + `.ai/tickets/ROOT` replaces `registry.json` as `find_repo_root` marker. Retarget all 12 fixtures that write `.ai/tickets/registry.json` in the **same commit** as the split. Do not retarget mod `Data/registry.json`.
3. `next_id` derived from max parent id (plant `T-950.toml` → next add is T-951).
4. `wave_ledger_unshipped_at` dual-read: JSON blob if `git cat-file -e {rev}:.ai/tickets/registry.json`, else TOML dir; neither → refuse, never an empty map.
5. README cutover SHA is the 40-hex parent of the delete; `git show $SHA:.ai/tickets/registry.json` exists. No banned phrase.
6. Delete the monolith in the same commit as the TOML tree (or the consecutive pair the factory allows; prefer one SHA).
7. Mine conversion logic from `git show 6ae8069c:path`. **Never restore** `scratch/track-b-draft` onto `main`.

### T-911.1 proofs (paste stdout; “done” is not acceptance)

1. Set-equality of ids; `N` measured from the cutover monolith.
2. TOML tree → `format_json_unicode_preserve` → `cmp` against that blob. Perturb one summary character → `cmp` red, then restore.
3. Union of keys across those `N` parents == FROZEN_27.
4. Reader identity diffs empty: `ticket list` / `next` / `brief T-090` / `brief T-090 --slice T-090.6` / `prompt`, ledger `is_shipped`, `slice-collisions` dispatch set, then `ticket sync` and `diff` of `docs/TICKET_LEAD.md`, `docs/TICKET_REGISTRY.md`, `docs/TICKET_BRAINSTORM.md`, CLAUDE marker, `.ai/tickets/queue.json`.
5. Dual-read throwaway A=JSON / B=TOML; plus real-repo `PHASE1^` vs `PHASE1` for T-904 shipped.
6. `git show PHASE1 --stat` includes `ROOT` and the 12 stub sites; grep those paths for `.ai/tickets/registry.json` is empty (mod `Data/registry.json` exact-line allowlist).
7. Glob budget: `ticket list` wall-clock < 500 ms.
8. `cargo xtask ticket check --strict` prints `check OK`.
9. `cargo xtask platform wave gate --slice T-911.1` prints `GATE: PASS`.

**Illegal in T-911.1:** typed Scope, encoding C, dropping `impact`/`surfaces`, Ready prose, shipping T-159.23, `wave.lock`, deleting `wave_plan.tsv`, `created_at` / `tokens_consumed`, minting T-674.1 or the lockfile parent.

## T-911.2 must-ship

`crates/tbd-tickets` (`Scope` / `Status` / `Ticket::{Program,Work}`), encoding C (flat `status` + sibling `order`; `[scope.repo]` / `[scope.engine]` etc.), Live-order uniqueness among `{queued,ready,running,review}`, frozen-**49** needs-operator set-equality, Ready prose on T-090 / T-090.4 / T-090.6 / T-090.7 / T-090.9, T-159.23 `shipped` @ `69dc5da5`, T-090.6 `[scope.engine]`, T-673-family maps, T-674/T-675 Program + mapper-minted T-674.1/2 and T-675.1/2, `trybuild` `ModLayer::Frontend` must not compile, proptest `parse(render(t))==t`, `#![deny(clippy::wildcard_enum_match_arm)]`.

Re-run the mapper on the **phase-1 tree**. Do not copy rewritten TOML from `6ae8069c`.

**Illegal in T-911.2:** `wave.lock`, TSV delete, metrics producer.

## Frozen 49 (phase 2; do not coerce)

T-067, T-071, T-110, T-111, T-113, T-130, T-134, T-144, T-145, T-146, T-147, T-148, T-149, T-151, T-160, T-161, T-162, T-163, T-164, T-165, T-183, T-241, T-242, T-251, T-252, T-253, T-259, T-275, T-280, T-290, T-291, T-311, T-415, T-419, T-439, T-460, T-462, T-541, T-543, T-545, T-604, T-605, T-606, T-607, T-608, T-609, T-612, T-617, T-619.

Park as Work `[scope.repo] layers=["docs"]` (or Program if they have children).
