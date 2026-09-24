**Status:** archived

# Pre-Phase-3 baseline — engine split

Phase 3 of the engine split moves editing logic into `website-map-engine`, reshapes the editor
into `v2/apps/editor/`, and enforces the 500-line ceiling. Its contract is **zero behavior
change**, so it is measured against the state captured here rather than against "green".

**Three gates are already red before Phase 3 touches anything.** None is caused by the engine
split, and none is repaired by it except where its own scope happens to cover the file. They are
recorded so that "still red the same way" can be told apart from "Phase 3 broke it".

| check | state here | cleared by |
|---|---|---|
| `gate v-suite verify` | 21 of 25 routes fail | T-986, deferred out of Phase 3 |
| `cargo xtask verify file-length` | exit 1, 9 unallowlisted SIZE-3 | 3C |
| `cargo test -p xtask` | 818 passed, 10 failed | not Phase 3 work |
| `cargo xtask verify engine-layers` | PASS | — |
| `cargo test -p website-frontend` | 1433 passed, 0 failed | — |

`verify file-length` runs inside `verify-coding-standards`, which runs inside
`cargo xtask ci ci-local`. **So `ci-local` is red here too, and cannot go green until 3C lands.**

## Capture point

```
commit   4273c962852324953518925e0b4a28e7d2459ac6
subject  chore(gate): engine-layers' tests move to their own file — SIZE-3 (2D)
date     2026-09-16T13:57:08Z
```

---

## 1. Route oracles — `gate v-suite verify`

Compares each route's rendered DOM against a frozen oracle under
`tools/tbd-tools/fixtures/t159/oracle-freeze`. Those oracles are stale, so the gate is red.

The oracles are the only instrument checking route DOM output. Re-freezing them inside Phase 3
would bake any regression Phase 3 introduces into the new oracle, where it becomes permanently
invisible — so they are left alone and acceptance is a diff against this capture.
Refreshing them is [`T-986`](/.ai/tickets/T-986.toml), deferred out of Phase 3 by operator
decision.

### Acceptance rule

1. **Set equality on failing route names, not cardinality.** Twenty-one failures before and
   twenty-one after can be a different twenty-one. The set must be unchanged or smaller.
   Fail → fail is fine. **Pass → fail is a hard stop.**
2. **The four clean routes are the real gate.** They carry live signal and must still pass.
   Hard fail, not diff.

Command: `cargo run -q -p tbd-tools --bin gate -- v-suite verify` → exit 1.

### The four clean routes — must still pass

| route | diffs |
|---|--:|
| `notfound` | 0 |
| `eventmgr` | 0 |
| `callback` | 0 |
| `login` | 0 |

### Failing routes — the set Phase 3 may shrink but never grow

Diff counts are for diagnosis only. A route that stays failing with a different count is still
the same member of the set; a route that joins the set is a Phase 3 regression.

| route | diffs |
|---|--:|
| `dashboard` | 14 |
| `approvals` | 18 |
| `audit` | 3 |
| `content` | 11 |
| `personnel` | 16 |
| `servercontrol` | 41 |
| `announcements` | 2 |
| `deployments` | 38 |
| `events` | 4 |
| `eventhub` | 36 |
| `orbat` | 1 |
| `leaderboards` | 24 |
| `missions` | 3 |
| `missionview` | 8 |
| `modpacks` | 12 |
| `serverintel` | 1 |
| `settings` | 2 |
| `mortar` | 29 |
| `vehicles` | 27 |
| `wiki` | 40 |
| `wikislug` | 40 |

**21 failing, 4 clean, 25 total.** T-986's title and the program document both say "22 of 25",
which does not sum against four clean routes. The measured set above is authoritative.

---

## 2. File length — `cargo xtask verify file-length`

**Exit 1.** 1373 `.rs` files scanned, 106 SIZE-1 warnings, **9 SIZE-3 violations that carry no
allowlist row**. `verify file-length` runs inside `verify-coding-standards`, which runs inside
`cargo xtask ci ci-local` — so `ci-local` is red here too, before Phase 3 starts.

| file | lines | cleared by |
|---|--:|---|
| `apps/website/frontend/src/editor/canvas/overlays.rs` | 1507 | 3C splits it |
| `crates/tbd-tickets/src/ops.rs` | 2476 | 3C grandfather row |
| `tools/tbd-tools/src/world/forest_smooth.rs` | 1243 | 3C grandfather row |
| `xtask/src/backfill_stamps.rs` | 1282 | 3C grandfather row |
| `xtask/src/check.rs` | 2329 | 3C grandfather row |
| `xtask/src/estimate_tokens.rs` | 1647 | 3C grandfather row |
| `xtask/src/wave/base.rs` | 1124 | 3C grandfather row |
| `xtask/src/wave/land.rs` | 1659 | 3C grandfather row |
| `xtask/src/wave_lock.rs` | 2169 | 3C grandfather row |

Phase 3 closes all nine: one is in its own scope, eight are covered by the grandfather rows 3C
generates when the SIZE-3 threshold becomes file-kind aware. **`ci-local` cannot be green until
3C lands**, and that is expected, not a regression.

---

## 3. xtask unit tests — `cargo test -p xtask`

**FAILED, and unstable.** Two consecutive runs of this same commit gave `818 passed; 10 failed`
and `820 passed; 8 failed` — these tests are flaky, so acceptance here is set membership on
names, never a count.

The failing set traces to `apps/mod/**`, which spec §7 puts out of scope:

```
gate_t437::tests::collapsed_returns_fail_registry_pins
gate_t437::tests::live_tree_holds
gate_t437::tests::paraphrase_injection_is_caught
schema_gates::t212_objective_spine_tests::objective_spine_is_read_in_the_objectives_lane
schema_gates::t212_objective_spine_tests::the_lane_scan_can_still_report_zero
schema_gates::t212_side_fallback_tests::invalid_side_is_neutral_but_absent_and_valid_sides_keep_their_roles
schema_gates::t212_staged_golden_tests::the_staged_1_3_golden_objectives_row_binds_to_the_reader
schema_gates::unread_wire_field_tests::all_1_3_fields_are_unread_on_the_live_tree
```

All of them fail on one missing Enfusion script:

```
FAIL: missing apps/mod/tbd-framework/Scripts/Game/TBD/Objectives/TBD_ObjectiveRegistry.c
```

That path is absent from the working tree **and** untracked at this commit, so the pin names a
file that does not exist here. Nothing in Phase 3 touches `apps/mod/**`.

Phase 3 adds tests to `xtask/src/gate_engine_layers_tests.rs`. Those must pass. A failure
outside the eight names above is a Phase 3 regression; these eight are not.

---

## 4. Frame cost — `window.__editorBench(500)`

The bench is only meaningful under three conditions, and getting any of them wrong produces a
number that looks real and is not:

- **`?force=webgl`** pins SwiftShader WebGL2. On the default WebGPU/Dawn backend
  `render_bench` traps (`RuntimeError: unreachable`), which poisons the wasm module and hangs
  the harness.
- **`--no-freeze`** is required. `render-check` otherwise stubs `performance.now()` for
  determinism, and every interval in the bench comes back `0.000` with `fps_equiv 10000000.0`.
- **`?sat=preview`** keeps the run off the 152 MB full satellite GET.

Command:

```
gate render-check --dir apps/website/frontend/dist \
  --path "/missions/smoke/edit?force=webgl&sat=preview" \
  --seed-auth --map-assets packages/map-assets --no-freeze \
  --assert-js "<await window.__editorBench(500)>"
```

```json
{"n":500,"submit_wall_ms":11760.100,"total_wall_ms":25097.925,"cpu_avg_ms":0.0562,
 "cpu_p95_ms":0.1050,"cpu_max_ms":0.1650,"submit_avg_ms":23.4625,"fps_equiv":17777.8,
 "drained":false}
```

**`cpu_avg_ms` and `cpu_p95_ms` are the rows that matter.** They are the CPU cost of building
and encoding one frame's packet — precisely what a §2C rule-1 or rule-3 regression would move.
`submit_avg_ms` is software-rasteriser cost and carries SwiftShader variance; do not gate on it.

---

## 5. Gates and tests that ARE green here

```
cargo xtask verify engine-layers   ENGINE-LAYERS: PASS
                                   rules 1, 2, 3a, 3b, 4, 7 — rules 5 and 6 are not yet
                                   implemented; Phase 3A writes them
cargo test -p website-frontend     ok. 1433 passed; 0 failed; 0 ignored
cargo run -p tbd-tools --bin gate -- doctor    OK — 0 warning(s)
```
