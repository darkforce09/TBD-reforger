# Wave 245 adversarial verify

HEAD confirmed: `ee7f4a234b1bd51300ca6ac9681cab098a754a0d` (matches dispatch). Base `ddc49daaa`. Merges T-682 `8ec91d056`, T-677 `be07d1d06`, T-936.2 `fb50b757a`, then CC `ee7f4a234`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. No fixes, no commits, no tickets. Probes restored. Porcelain empty. `:3000` / `:8080` left listening.

## Findings

### 1. MAJOR — T-936.2 tasks never reach the compiled mod document

`crates/map-engine-core/src/mission/flatten.rs:1404-1410` `EditorPayload::authored_blocks_root` copies **only** `winConditions`. `flatten_to_mod_document` feeds that reconstructed object to `ExtensionBlocks::from_payload` (`:3606-3609`). After `0d9c07394` the `EditorPayload.tasks` field and the `root.insert("tasks", …)` one-liner are gone. Serde `#[serde(default)]` silently ignores a payload-root `tasks` key.

T-936.1's own comment (`flatten.rs:1389-1392`) says a sibling slice adds that one-liner here. The owns revert removed it. `compile_payload` still promotes `environment.tasks` onto the **save** payload (AUTHORED_BLOCKS row at `extensions.rs:81` is real). `/compiled` then strips them. `TBD_TaskStateMachine.c:403-423` reads `GetRawJson()` (the compiled body) and logs idle when `tasks` is empty.

**Probe (restored):** inserted `probe_t9362_flatten_emits_root_tasks` on FIXTURE + root `tasks[]`, ran, then `git checkout -- flatten.rs`.

```
cargo test -p map-engine-core --all-features --lib probe_t9362 -- --nocapture
thread 'mission::flatten::tests::probe_t9362_flatten_emits_root_tasks' panicked at flatten.rs:7651:9:
compiled document must carry tasks[] after AUTHORED_BLOCKS registration
test result: FAILED. 0 passed; 1 failed
```

`a_three_tier_mission_copies_to_the_payload_root` is a false-green: it asserts `compile_payload` + `ExtensionBlocks::from_payload(&p)` and never calls `flatten_to_mod_document`. Wave gate therefore PASSed T-936.2 without examining the emit path the dedicated server reads.

`tasks` is not in `KNOWN_EDITOR_PAYLOAD_TOP_LEVEL_KEYS` (`compile.rs:46-63`). Same parking rule as `winConditions`: extras skip AUTHORED_BLOCKS, so a root `tasks` key that is not also in the env bag dies on the next save. The panel writes the env bag — and is unmounted (finding 2).

**Fix:** restore the two-line `EditorPayload.tasks` + `authored_blocks_root` insert (T-936.1's prescribed one-liner). Add a flatten integration test that would have failed this probe. Widen owns or land it as CC.

### 2. MAJOR — tasks panel never mounts

`settings_modal.rs:949` renders `{win_conditions_card(ctrl)}` then `render_prefs_section`. `rg tasks_panel settings_modal.rs` → no hits. `panels/mod.rs:33` only `pub mod tasks_panel`. Slice comment (`tasks_panel.rs:10-12`) and REPORT-T-936.2 admit the T-936.1 pattern: wait for CC to mount. Wave 243 CC mounted the win-conditions card. Wave 245 CC (`ee7f4a234`) did not.

Operator cannot author tasks in Mission Settings. Unit tests (`cargo test -p website-frontend tasks_panel` → 10 passed) exercise `add_task` / env_patch off-screen.

**Fix:** `{tasks_panel(ctrl)}` immediately after `{win_conditions_card(ctrl)}`.

### 3. MAJOR — T-677 spawn gate is not “waypointed unclaimed LIVE”

`ShouldEnableAIAtSpawn` (`TBD_WaypointRuntime.c:243-256`) is `GroupHasWaypoints` (non-empty list only, `:320-321`) **and** `GetStage()==LIVE`. It does **not** test claimed seats. `SpawnSlotBody` (`TBD_SpawnManager.c:1241`) is the only disable site. Callers:

| site | when | gate result |
|---|---|---|
| `:994` `MaterializeSlotBodies` | load / not LIVE | always `DisableBodyAI` |
| `:2525` player rematerialize | can be LIVE | waypointed slot **skips** disable |

So the only body that skips `DisableBodyAI` is a **player** LIVE rematerialize into a waypointed group — the opposite of “unclaimed AI seats”. Comment at `:1240` (“Players (claimed seats) are never enabled by T-677”) is false. Unclaimed filter exists only later in `CollectUnclaimedBodies` (`:460-461`) for `AddAIEntityToGroup`.

`rg -n 'ActivateAI\(' apps/mod --glob '*.c'` → **zero** calls. Header comment (`TBD_WaypointRuntime.c:61`) claims the runtime “ActivateAI's” via `AddAIEntityToGroup`. Load-spawned AI stay `DeactivateAI`'d (`:1505/:1517`) unless that engine call unparks them (unproven; human checklist).

**Fix:** always `DisableBodyAI` on the player rematerialize path; if AI respawn needs the skip, pass an explicit unclaimed/AI-seat flag. Call `ActivateAI()` on unclaimed members when arming if `AddAIEntityToGroup` does not unpark.

---

## Attacked and FAILED to break

- **Gate vacuity / unexamined code (BLOCKER class):** `tbd_wave245_cold` exists; `missions_it` is 35 tables / 38 missions / 22 migrations. `target-gate-mapengine` fingerprints `2026-09-06 21:19` (after CC `21:16`). Gate test binary lists `t682_*` (4) and `mission::tasks::tests::*` (16) plus `an_unlisted_environment_key_is_not_promoted_to_the_payload_root`. `test api` cannot see this wave's files (no website-api diff) — expected, not the only step. Wave gate still does not run `mod compile` (T-946.17; they ran it by hand). Not re-filed.
- **T-936.2 still editing flatten.rs / compile.rs after revert:** `git log ddc49daaa..HEAD -- flatten.rs` = `2f998c3f4` (T-682) only. `compile.rs` = `ee7f4a234` dummy-key retarget only. Merge `fb50b757a` has neither file. Owns breach stayed reverted. The hole is the missing one-liner (finding 1), not a leftover edit.
- **AUTHORED_BLOCKS / ExtensionBlocks drop tasks:** row present (`extensions.rs:81`). `copy_authored_blocks` and `from_payload` tests pass. `cargo test -p map-engine-core --all-features --lib -- mission::tasks an_unlisted_environment_key` → **17 passed**. Dummy retarget probe: restoring `tasks` as the unlisted key **fails** with payload root `"tasks":[{"id":"t1"}]` — compile-side promotion works; flatten is the break.
- **compile.rs dummy still `tasks` (map-engine-core red on main):** live dummy is `audio` (`compile.rs:1315-1322`). After restore, `an_unlisted_environment_key_is_not_promoted_to_the_payload_root` **ok**.
- **T-682 applies dateTime/weatherPreset / changes boot without fog/wind/viewDistance:** `Apply()` (`TBD_EnvironmentReader.c:59-82`) only fog/wind/windDirDeg/viewDistance vs `ABSENT`. `dateTime`/`weatherPreset` bind, never applied. `t682_absent_axes_omit_the_keys_and_do_not_bump_version` **ok** (keys omitted, `schema_version` stays `1.2`, `weatherPreset` still `"clear"`).
- **T-677 enables AI for ALL spawns:** failed in that form. Unwaypointed still `DisableBodyAI`. Empty `waypoints` skipped at collect. Runtime `AddAIEntityToGroup` is unclaimed-only. Remaining hole is finding 3, not a global enable.
- **Export twins / T-946.26:** new files byte-identical ASCII (`cmp -s` on WaypointRuntime, EnvironmentReader, TaskStateMachine, TaskHud). SpawnManager/Loader twins differ only in pre-existing comment punctuation (emdash vs hyphen), not the T-677/T-682 hunks (`ShouldEnableAIAtSpawn` both at `:1241`).
- **UNREAD tripwires:** `fog`/`wind`/`viewDistance`/`waypoints`/`vehicleUid`/`speedMode`/`behaviour` have **no** `UNREAD_WIRE_FIELDS` rows. `combatMode`/`formation` remain expected 0 (T-678). `size` expected 3, `shape` expected 34. `formation` hits in WaypointRuntime are `//!` comments only. `cargo test -p xtask all_1_3_fields_are_unread_on_the_live_tree` → **ok**.
- **Schema still saying NOTHING reads this wave's fields:** `fog`/`wind`/`viewDistance`/`waypoints`/`vehicleUid`/`environment`/`tasks` descriptions say READ SINCE T-682/T-677/T-936.2. Group `behaviour`/`speedMode` note global identifier count. `$defs/waypoint.behaviour`/`speedMode` lack a READ SINCE line (NIT only; they do not claim unread).
- **env.rs author_env / T-682 added controls:** `git diff ddc49daaa..HEAD -- env.rs` empty. `CARRIED_ENV_KEYS` still five (`time`/`weather`/hillshade/grid). `keys_nothing_reads_are_not_authored` still names `viewDistance`/`thermals`/`windDirDeg`/`fog`/`wind`. `cargo test -p website-frontend keys_nothing_reads_are_not_authored` → **ok**.

NIT (not a finding row): invalid AUTHORED_BLOCKS refusals still go through `diagnostics.win_conditions` / subject `/winConditions` (`flatten.rs:1252-1258,3611-3618`), so a bad `tasks` block is labelled as a win-conditions warning.

## main_left_clean

- tracked tree clean (only untracked deliverable: this `VERIFY.md`)
- `git rev-parse HEAD`: `ee7f4a234b1bd51300ca6ac9681cab098a754a0d`
- Probes restored (`flatten.rs`, `compile.rs`)
- `:3000` and `:8080` still LISTEN
