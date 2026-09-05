# T-937 — Editor data layer: id arrays, undo, persist, payload

Owner: command center. Source: master audit S2 (2026-09-04), verified against main @ 072988d57
(README.md in this directory). Scope: TBD-Reforger only. Executor: claude-code.

## 1. Related existing tickets — referenced, never re-minted
T-257 undo scope · T-295 realtime collab · T-190 second tab · T-132 visual diff · T-159.22.1 (the Yjs-parity
undo decision T-937.2 supersedes) · T-227 (shipped) · T-393 (shipped).

## 2. Verified anchors (2026-09-04)
| Finding | Anchor | Verdict | Slice |
|---|---|---|---|
| slotIds/entityIds as Any::Array | store.rs:5162 retain_ids, :5171-5191 append_id clone-rewrite | TRUE | T-937.1 |
| ZeroClock, capture_timeout 0, no cap | store.rs:361-371; deliberate per :350-360 (T-159.22.1) | TRUE | T-937.2 |
| materialize per-slot resolution | store.rs:745-764 cached, :772-812 per slot | PARTIAL | T-937.3 |
| existence via materialize | operations/entity.rs:1885; warnings :1795/:1847 | TRUE | T-937.3 |
| QuotaExceeded silent | persist.rs:827-832 generic Err → console.warn (no quota code) | PARTIAL | T-937.4 |
| pagehide fire-and-forget | persist.rs:952-957 | TRUE | T-937.4 |
| note_unreadable lockout | persist.rs:252, :461, :815-826 | TRUE | T-937.4 |
| payload slots/editorLayers no items | mission-editor-payload.schema.json:42-43 | TRUE | T-937.5 |
| 8388608 not enforced in editor | mission.schema.json:6; api validate.rs:359,749; mission_library.rs:1452 = 64<<20 | TRUE | T-937.5 |

## 3. Design
- **T-937.1** `doc/id_arrays.rs`: helpers over a yrs array; `hydrate` migrates legacy Any::Array once; both
  forms read identically. Only `squad.slotIds` and `layer.entityIds` change representation.
- **T-937.2** `doc/undo_groups.rs`: gesture window 300 ms (injectable clock), `begin_group/end_group`, cap 200
  groups (evict whole oldest group). store.rs:350-371 comment becomes the dated decision naming T-937.2.
  `state/operations/batch.rs`: `with_batch(label, f)` for paste, delete-selection, align.
- **T-937.3** store.rs side-key cache keyed by slot id, invalidated by the existing change observer;
  `slot_exists(id)` on the raw map (hidden slots included); entity.rs:1885 switches to it.
- **T-937.4** `state/save_status.rs`: `SaveStatus {Saved, Saving, Failed(reason), Unreadable(retries)}`,
  chip + toast; persist.rs reports every Err, flushes on `visibilitychange` hidden, debounce ≤ 1 s,
  note_unreadable retries ×3 with backoff then lockout with Retry.
- **T-937.5** payload item schemas (slots, editorLayers), `operations/slot_ids.rs` duplicate guard by
  (callsign, id), ceiling 8388608 from one constant with a size readout in mission_library.rs.
Wave packing: store.rs chains .1 → .2 → .3; .4 is independent; .5 after .3. `operations.rs` is shared by
.2 and .5 (register batch.rs / slot_ids.rs) — already serialized by the chain.

## 4. Rules every slice encodes
Defect verified on main first (red pasted verbatim); perturbation with `touch` after restore;
`cargo test -p map-engine-core --all-features` only; no `git add -A`, `git stash`, `cargo xtask ci ci-local`;
`skip:` = FAIL; no .py/.sh/.mjs; allowlists never grow; Class-R parity tests scrub their own source;
agents never merge/push/change status. Report: pwd_branch · defect_verified_on_main · changes ·
perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits.

## Claude Code prompt — T-937.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-937.1 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-937_1_plan.md; store.rs:5140-5200 and the hydrate path; doc/mod.rs; the Class-R oracle parity tests.
═══ PROBLEM ═══
append_id/retain_ids clone-rewrite a whole Any::Array per call: O(N^2) and concurrent appends or undo drop ids.
═══ SHIPPED ═══
MissionDocCore hydrate, Class-R oracle parity suite. Do not change their fixtures.
═══ LANGUAGE GATE ═══
Rust (map-engine-core).
═══ LOCKED ═══
- Only squad.slotIds and layer.entityIds change representation; legacy documents migrate once in hydrate.
- Oracle parity fixtures untouched and green.
- All callers route through id_arrays.rs; store.rs gets no new list logic.
═══ DO ═══
1. Verify on main: two synced peers append concurrently → one id lost; paste the red.
2. doc/id_arrays.rs (read/append/retain/move + tests) registered in doc/mod.rs.
3. hydrate migration; switch every store.rs caller; run the parity suite.
4. Perturbation: skip the migration branch → legacy-document test red; restore, touch, green.
═══ DO NOT ═══
No undo changes (T-937.2); no frontend edits; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask platform wave gate --slice T-937.1
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-937.2

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-937.2 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-937_2_plan.md; store.rs:340-380; doc/id_arrays.rs (T-937.1); state/operations.rs:1-60; T-257 for undo scope rules.
═══ PROBLEM ═══
UndoOptions{capture_timeout 0, ZeroClock} makes every transaction an undo step and nothing caps the stack;
a drag needs one undo per tick.
═══ SHIPPED ═══
T-937.1 id_arrays; T-257 undo scope semantics (unchanged here).
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos).
═══ LOCKED ═══
- Window 300 ms via an injectable clock; explicit groups win over the window; cap 200 whole groups.
- The :350-371 comment is replaced by a dated decision naming T-937.2 and the rationale.
- Undo scope rules from T-257 unchanged.
═══ DO ═══
1. Verify on main: three ops within 50 ms need three undos; paste the red.
2. doc/undo_groups.rs (tests: window, explicit group, cap) in doc/mod.rs; wire UndoOptions in store.rs.
3. operations/batch.rs with_batch in operations.rs; use it from one existing multi-op path.
4. Perturbation: window 0 → grouping test red; restore, touch, green.
═══ DO NOT ═══
No materialize changes (T-937.3); no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-937.2
═══ MANUAL ═══
Drag a slot for two seconds; one Ctrl+Z restores it.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-937.3

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-937.3 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-937_3_plan.md; store.rs:730-830 and its change observer; operations/entity.rs:1780-1900.
═══ PROBLEM ═══
materialize resolves position, three strings, stance and side key per slot every call; entity.rs:1885
answers existence by materializing, which drops hidden slots (:1795/:1847 warn).
═══ SHIPPED ═══
T-937.1/.2 store changes; layer/hidden cache at store.rs:745-764.
═══ LANGUAGE GATE ═══
Rust (map-engine-core, Leptos).
═══ LOCKED ═══
- Cache invalidated by the existing observer only; no manual invalidation calls at call sites.
- slot_exists reads the raw map: hidden slots count as existing.
- materialize output identical to a fixture snapshot.
═══ DO ═══
1. Verify on main: hidden slot → slot_attrs_exists false; paste the red.
2. Side-key cache + slot_exists in store.rs; test-only resolution counter (≤ 1 per distinct side over 500 slots).
3. entity.rs:1885 uses slot_exists; update the :1795/:1847 comments.
4. Perturbation: skip invalidation → stale-side test red; restore, touch, green.
═══ DO NOT ═══
No undo or id-array changes; no files outside owns.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-937.3
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-937.4

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-937.4 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-937_4_plan.md; persist.rs:240-270,450-470,800-840,940-960; state/mod.rs; the shell toast surface.
═══ PROBLEM ═══
Save errors vanish into console.warn (:827-832); the pagehide save is fire-and-forget (:952-957);
note_unreadable locks saving out until reload (:815-826). Authors lose work without knowing.
═══ SHIPPED ═══
Shell toast surface; existing debounce + pagehide hook in persist.rs.
═══ LANGUAGE GATE ═══
Rust (Leptos, wasm).
═══ LOCKED ═══
- Every Err reaches SaveStatus; quota failures are named.
- One in-flight guard: hidden flush and pagehide never double-save.
- Three retries with backoff before lockout; lockout offers Retry.
═══ DO ═══
1. Verify on main: force save_state_as Err → no visible state change; paste the red.
2. state/save_status.rs (signal, chip, toast) in state/mod.rs.
3. persist.rs: report Errs; visibilitychange hidden flush; debounce ≤ 1 s; retry loop.
4. Perturbation: swallow the Err again → status test red; restore, touch, green.
═══ DO NOT ═══
No storage backend changes; no files outside owns.
═══ VERIFY ═══
cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-937.4
═══ MANUAL ═══
Fill localStorage to quota, edit, observe the toast + chip; switch tabs, reload, edits present.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```

## Claude Code prompt — T-937.5

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-937.5 && export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target && git branch --show-current
═══ READ ═══
Spec §3; docs/plans/t-937_5_plan.md; mission-editor-payload.schema.json (61 lines); mission_library.rs:1430-1470; operations.rs:1-60; api contract/validate.rs:350-365,740-755.
═══ PROBLEM ═══
slots/editorLayers items are unconstrained; duplicate slot ids under one callsign pass until the API;
the editor ceiling is 64 MiB while the API enforces 8388608.
═══ SHIPPED ═══
T-937.3 slot_exists; API validate.rs ceiling; payload schema top-level open per its :50 rationale.
═══ LANGUAGE GATE ═══
JSON Schema, Rust (Leptos).
═══ LOCKED ═══
- Measure committed fixtures and goldens before tightening items; the golden is updated in the same commit.
- Ceiling = 8388608 from one constant; the readout shows bytes / ceiling before upload.
- Duplicate guard keyed by (callsign, id); message names both.
═══ DO ═══
1. Verify on main: slot item {bogus: 1} validates; paste the red.
2. Payload schema item refs for slots and editorLayers; `cargo xtask ci schema-validate`.
3. operations/slot_ids.rs (tests) in operations.rs; save path refuses duplicates.
4. mission_library.rs:1452 ceiling + readout; perturbation: drop the callsign key → red; restore, touch, green.
═══ DO NOT ═══
No API changes; no top-level payload closure; no files outside owns.
═══ VERIFY ═══
cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask platform wave gate --slice T-937.5
═══ MANUAL ═══
Upload a 9 MB mission: refused with the size shown.
═══ RETURN ═══
Report schema per spec §4. Ready for Cursor doc sync.
```
