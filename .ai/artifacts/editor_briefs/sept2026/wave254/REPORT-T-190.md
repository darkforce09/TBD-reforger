# REPORT — T-190 · Two tabs on one mission clobber each other

> Transcribed by the command centre: the agent harness refused its `Write` to this path
> ("Subagents should return findings as text, not write report files"), and the agent did not route
> around the refusal via Bash. Content is the agent's. Independently re-verified by the command
> centre: branch clean, four commits, six owned paths, no `store.rs` / `Cargo.toml` / allowlist /
> `save_status.rs` edit, and `overlays.rs` still carries zero real `#[cfg(test)]` (its two hits are
> prose, same count as main). Merged to main as `e27fd905cdabe43838734218758cd6101f9e9cf9`.

## pwd_branch
```
/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-190
slice/T-190
```

## defect_verified
Proved before any fix, in its own commit (`8beed28ee`). Six pins in the new `state/tab_lock.rs`, all
RED on the branch point:
```
running 8 tests
test ...tab_lock::tests::t190_conflict_info_carries_counts_and_timestamps ... FAILED
test ...tab_lock::tests::t190_run_save_merges_the_stored_record_before_it_writes ... FAILED
test ...tab_lock::tests::t946_54_note_unreadable_latches_before_it_retries ... FAILED
test ...tab_lock::tests::t946_51_save_in_flight_is_read_by_something ... FAILED
test ...tab_lock::tests::t190_load_server_version_is_marked_destructive ... FAILED
test ...tab_lock::tests::t190_a_second_tab_cannot_silently_overwrite_the_first ... FAILED
test result: FAILED. 2 passed; 6 failed; 0 ignored; 0 measured; 1342 filtered out; finished in 0.54s
```
Assertion text, verbatim, included the modal as shipped — `ConflictInfo` was exactly `payload_json` +
`semver`, no counts and no timestamps, and "Load server version" was the `bg-primary` **affirmative**
button:
```
ConflictInfo must describe BOTH options; missing `local_objects`. info=pub struct ConflictInfo {
    pub payload_json: String,
    pub semver: Option<String>,
}

the destructive choice must use the codebase's destructive tokens (text-error / bg-error), not bg-primary.

SAVE_IN_FLIGHT is written and never read — either make it load-bearing or delete it (T-946.51).
Occurrences that are not a `.set(` or the declaration: 0

the key must be latched into UNREADABLE BEFORE the first backoff, or run_save passes the T-374 guard
for the whole retry window (T-946.54).
```

**On the test vehicle.** The brief and plan both say "wasm test". This repo has **no
wasm-bindgen-test harness** (asserted in `world_assets/labels.rs:16`, `attributes_modal.rs:978`,
`dock_right.rs:3537`, four more), and `website-frontend` links `map-engine-core` with
`default-features = false, features = ["mission","blueprint"]` off wasm32 (`Cargo.toml:26`) versus
`["doc","mission","png","world"]` on it (`:115`) — so **`MissionDocCore` and `yrs` do not exist
natively**, and `state/persist` is `#[cfg(target_arch = "wasm32")]`. A yrs-level convergence test is
unreachable from every file in the owns list, and enabling `doc` is a `Cargo.toml` change requirement
5 forbids. The slice used the oracle this codebase already uses at exactly this boundary — pure
native behavioural tests over the policy, plus `class_r_scrub` source pins over the wasm-only writer,
the same vehicle `save_status.rs` uses over `persist.rs` — plus a live `window.__missionTabs` bridge
and two refusal counters so the operator check asserts the guard **fired**.

## changes
**`state/tab_lock.rs` (new, 993 lines, ungated in `state/mod.rs`)** — `navigator.locks` writer role
per mission (`tbd-mission-writer:<id>`) via `js_sys::Reflect`, per `core/client.rs:422-430`. Held by
a promise this module keeps the `resolve` for, so the browser releases it when the page goes away
(crash included) and `release()` can hand it over on a route change. BroadcastChannel
`tbd-mission-tabs:<id>` (`client.rs:509-532` precedent) carrying `hello` / `here` / `bye` / `saved`;
`saved` is what makes a read-only tab converge. `TabLockBanner` renders no DOM while this tab is the
writer; `role="status"`, `aria-live`, `border-error/40 bg-error/10 text-error`. Pure and natively
tested: `decide_save` (the policy `run_save` obeys), `elect` (the no-Web-Locks fallback; total, ties
broken by id so exactly one tab writes), `Stamp`, `ago`, `short_utc`, `banner_copy`.

**`state/persist.rs`** (1442 → 1708; already SIZE-3 allowlisted, no row added) — `run_save` checks the
role **before** `encode_state` (O(document) — 460 ms at 100k slots per the T-374 measurement; a
read-only tab must not pay it every second) and **re-arms** rather than drops, via `install_pending`
extracted from `arm_debounced`. The re-arm keeps the original `owner`, restating T-221.
`merge_before_write` → `read_raw` → `merge_stored` → re-encode. `merge_stored` is
`MissionDocCore::apply_update` (CRDT union, never a JSON diff) into the live doc, then `refresh_hud` +
`rebind_engine_from_doc`, guarded by a `(mission_id, DocHandle)` pair parked by `register_tab_sync`,
so the merge is exact rather than best-effort. Success stamps the key and `announce_saved`s, both
after the bytes land (T-779/T-804 ack discipline). `pull_peer_record` is the read-only tab's
convergence path; `draft_written_at` is the modal's local timestamp; `blocked_writes()` gains
`read_only` and `merged`.

- **T-946.51** — `SAVE_IN_FLIGHT` is now READ, by `pull_peer_record`, which stands down while a save
  is between `get_bytes` and `put_raw`: a merge in that window would put blocks into the document
  that the landing write does not carry, i.e. the record would go backwards silently. Both comments
  crediting the flag with `lock_for(id)`'s serialisation are corrected.
- **T-946.54** — `note_unreadable` latches into `UNREADABLE` **before** its first backoff. The old
  order left the set empty for 80+160+320 ms while the idle debounce is 1 s. Every arm that *learns*
  the key is safe (`Hit`, `Miss`) still clears it in the same breath.

**`state/hydrate.rs`** — the `Local::Diverged` arm (`:353-358`, as the brief said, not `:220-239`)
builds the four new facts. Counts are the same numbers `classify_local` just compared, so the modal
cannot disagree with the decision that raised it; the local instant is the T-190 write stamp (absent
⇒ `"not recorded on this browser"`, never a guessed "now"), the server instant is
`current_version.created_at`.

**`canvas/overlays.rs`** (972 → 994) — `ConflictInfo` gains `local_objects` / `server_objects` /
`local_saved` / `server_saved`; the dialog renders a two-column comparison plus a warning line; "Load
server version" moves to `bg-error/15` + `text-error` (`shell/layout.rs:271`,
`faction_manager.rs:311`). **No `#[cfg(test)]` added** — its `:12-14` note is respected.

**`mission_editor.rs`** — two call-site lines only: `register_tab_sync(doc, id)` beside
`register_flush_on_hide`, and the `TabLockBanner` mount beside `ConflictDialog`. Nothing above
`:3033`.

**`store.rs:462-478`, quoted as asked.** The guard is *"the update claims my own client id has
progressed past the last clock I issued"*. It deliberately does not fire on *"a **restore into a
fresh doc**, where `my_clock == 0`"* or on *"a **re-exchange**, where a peer echoes our own
already-integrated blocks back at us"*. Two tabs restored from one record each mint a fresh 53-bit id
in `MissionDocCore::new()`, so a sibling's blob never trips it. `store.rs` was **not edited**;
`materialize(&self) -> SlotSoa` is unchanged.

## perturbation (RED verbatim)
The one the brief specifies — skip the merge (`merge_before_write(...)` → bare `bytes`):
```
test ...t190_run_save_merges_the_stored_record_before_it_writes ... FAILED

panicked at apps/website/frontend/src/editor/state/tab_lock.rs:802:13:
run_save must READ the record it is about to overwrite. run=async fn run_save(id: &str, pending: PendingSave) {
    let lock = lock_for(id);
    let _guard = lock.lock().await;
    SAVE_IN_FLIGHT.set(true);
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1356 filtered out; finished in 0.09s
```
Restored with `git checkout --` **and `touch`ed** (T-421 trap); the green run carries
`Compiling website-frontend v0.1.0 (.../worktrees/T-190/apps/website/frontend)`.

Four more, so no pin here is vacuous — each applied, rebuilt (`Compiling` present every time), RED,
restored + `touch`ed, green:

| perturbation | pin that went RED |
|---|---|
| `note_unreadable` latch moved back after the retry loop | `t946_54_note_unreadable_latches_before_it_retries` |
| `save_in_flight()` read removed from `pull_peer_record` | `t946_51_save_in_flight_is_read_by_something` |
| "Load server version" reverted to `bg-primary` | `t190_load_server_version_is_marked_destructive` |
| read-only refusal removed from **both** sites | `t190_a_second_tab_cannot_silently_overwrite_the_first` |

**Recorded honestly:** removing *only* the early role check did **not** turn that last pin red,
because the late `decide_save` `Defer` arm still upholds the guarantee — the early check is a
performance guard, not the guarantee. Widened to both sites it went RED: *"a deferred save must be
RE-ARMED, not dropped: the read-only tab inherits the writer role when the other tab closes and its
work has to survive to that point."*

Final state after all restores: `git status --short` empty, fresh `Compiling website-frontend`,
`test result: ok. 1357 passed; 0 failed`.

## gate_verdict_tail
```
  cargo check              PASS      T-278 catalogue drift    PASS
  wasm32 (frontend)        PASS      db_migrate claim/persist PASS
  fmt (changed)            PASS      T-439/444/440/438        PASS
  clippy (changed crates)  PASS      T-456/468/437/586        PASS
  test (frontend, changed) PASS      T-296/452, no-python     PASS
  schema                   PASS

  gate verdict PASS @ b6b32658c92f recorded: .ai/artifacts/verdicts/T-190.json
SLICE GATE: PASS
```

## files_outside_owns
`[]` — exactly the six owned paths. No `Cargo.toml`, no `store.rs`, no `save_status.rs`, no
`.coding-standards-allowlist.yaml`, no `docs/`, no `.ai/tickets/`, no sibling-owned path.
Independently confirmed by the command centre.

## found_not_fixed
1. **`hydrate.rs::resolve_conflict_local` — stale doc comment + dead parameter.** Its T-370 note says
   the one call site is "`mission_editor.rs`, the ConflictDialog's 'Keep local' arm"; T-934.11 moved
   that dialog to `canvas/overlays.rs`, so the comment names the wrong file, and `_mission_id` is
   still unused. Left out as unrelated churn through the gate.
2. **A residual cross-tab window, by design and documented in `run_save`.** The stamp is read just
   before the write; a peer landing bytes between that read and the `put` can be passed over. Not a
   permanent loss (the peer still holds those blocks, its `saved` makes this tab pull, its next save
   writes the union) but it is a window, and IndexedDB offers no cross-tab transaction to close it
   from one tab.
3. **A boot ordering nothing pins.** `mission_editor.rs` arms the initial `save_state_debounced`
   (step 2) before `register_tab_sync` (step 4.5). Steps 3–5 are await-free so the role is decided
   inside the 1 s debounce — but if a future slice inserts an `.await`, a second tab's boot save
   would run once as Writer. Still non-destructive (foreign stamp ⇒ `Merge`), but the invariant is
   load-bearing and unasserted.
4. **`SAVE_IN_FLIGHT` is process-wide, not per-mission.** Equivalent today (one editor mounted at a
   time), but two editors in one page would need the `(mission_id, …)` treatment `MERGE_DOC` got.

## deviations
1. **Test vehicle** — see `defect_verified`. The "wasm test" is not constructible from this slice's
   owns. Substituted the codebase's own oracle. No acceptance item dropped.
2. **No Web Lock around the save's read-merge-write**, only around the writer role. With Web Locks
   present exactly one tab is the writer so concurrent same-mission saves essentially cannot happen;
   with Web Locks absent there is no lock to take either. Residual window is `found_not_fixed` 2.
3. **"Read-only" means "does not write the shared draft", not "the editor refuses input."** The
   second tab's edits are held in memory and merged in when it inherits the role. Blocking input
   would touch dock/toolbar/command surfaces outside owns.
4. **T-946.51 resolved by making the flag load-bearing, not by deleting it.** Deleting it would have
   required editing `save_status.rs` (outside owns) — that route would have been a STOP.
5. **`tab_lock.rs` landed at 993 lines** — under SIZE-3, no allowlist row added (rule 7). The SIZE-3
   violations `xtask verify file-length` prints (`prefab.rs`, `water.rs`, `ops.rs`, `check.rs`,
   `forest_smooth.rs`, `backfill_stamps.rs`, `estimate_tokens.rs`, `gate_mod_compile.rs`) are
   pre-existing and unowned — verified byte-identical at branch point `875ca3ffd`.
6. **The report file itself** — the harness refused the Write; content delivered as text.

## commits
```
b6b32658c  T-190 step 3: join() lets go of the previous mission
ccbbfb338  T-190 step 2: the conflict modal names both options and marks the destructive one
6761cc545  T-190 step 1: writer role, read-merge-write, and the two folded-in findings
8beed28ee  T-190 step 0: prove the two-tab clobber before fixing it
```
Step 3 was a defect found in the slice's own step-1 code: the editor is a **client-side route**, so
opening mission A then B never reloads the wasm module — `join()` saw `JOINED` as `Some` and
returned, leaving the tab holding A's writer lock and channel for life. B got no presence, and a
third tab genuinely on A was told wrongly and permanently that A was taken. Fixed by keeping the lock
promise's `resolve` so `release()` can settle it.

Merged to main as `e27fd905cdabe43838734218758cd6101f9e9cf9`.

## manual_checklist
Two tabs, one mission. **Web Locks need a secure context**, so use `http://localhost:3000` (a bare
LAN IP over http is not secure and exercises the fallback election instead).
1. Open `/missions/<uuid>/edit` in **A**. `__missionTabs.role()` → `"writer"`, `.peers()` → `0`.
2. Open the same URL in **B**. B shows the read-only banner naming the other tab; `role()` →
   `"read-only"`. A reports `peers()` → `1`.
3. Place an object in **A**, wait ~1 s. B's `__missionPersist.blocked_writes()` shows `read_only`
   climbing (saves re-armed, not dropped) and B's OBJ count picks up A's object.
4. Delete everything in **B** (the F-32 repro). Nothing is written from B; A's record untouched.
5. **Close A.** B's banner disappears (`role()` → `"writer"`); its next debounce is a
   read-merge-write — `blocked_writes().merged` increments.
6. Reload B — both tabs' edits present. The honest expectation for step 4's delete-all is that the
   deletions survive alongside A's additions, not that deleted objects come back.
7. **The modal.** Force `Local::Diverged`. Expect two labelled columns —
   `"N objects · written 4m ago"` vs `"N objects · saved 2026-09-07 11:22 UTC"` — the line "Loading
   the server version will discard your local copy. One Ctrl/Cmd+Z puts it back.", and "Load server
   version" in the red destructive style.
8. **Same-page mission switch** (the step-3 fix): from B open a different mission. `peers()` resets,
   and a third tab on the first mission must report `role()` → `"writer"`.
