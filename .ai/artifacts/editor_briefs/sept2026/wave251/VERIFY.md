# Wave 251 adversarial verify

HEAD at verify start: after ship+UNREAD. Base `aec833eff`. Merges T-654 `953ce41e5`, T-936.5 `919682b1f`, T-941.2 `81a44009b`. UNREAD `02c9c006e`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening.

## Findings

### 1. MAJOR — flatten drops `audio` (T-946.47)

`EditorPayload::authored_blocks_root` still copies only `winConditions` / `tasks` / `radioPlan`. AUTHORED_BLOCKS + `audio.rs` carry the payload; `/compiled` drops it. Same class as T-946.44 weatherTimeline. T-936.5 was briefed not to edit flatten.rs.

### 2. MAJOR — variant filter misses `objectives[]` / `editorTriggers[]` (T-946.48)

T-654 filters typed MissionLoader arrays before spawn. `TBD_ObjectiveRules` and `TBD_TriggerRuntime` still `GetRawJson()` a second pass, so variant-excluded rows still arm. `GetActiveVariantIds()` is the hook. Kept vehicles may still list a variant-excluded crew `slotId` (naming `seats` would bump T-675's unread pin of 12).

### 3. MAJOR — audio emitters panel never mounts (T-946.49)

`audio_emitters.rs` is registered in `panels/mod.rs`. `settings_modal.rs` has no mount, same as T-946.33/45. Not those tickets; audio-specific child.

### 4. MAJOR leftover — flatten still drops params / group AI / vehicle / entity / scatter / gadgets / weatherTimeline (T-946.36, T-946.41, T-946.43, T-946.44)

Unchanged. Not re-filed.

### 5. NIT — T-941.2 stale LOBBY-wave comments in unowned files

`TBD_LobbyController.c` / `TBD_LobbyData.c` still talk about the old LOBBY auto-deploy wave. Not this ticket. Do not file.

---

## Attacked and FAILED to break

- **T-654 bind/filter:** both `TBD_MissionLoader.c` twins. Export is the ASCII fold of framework (framework has UTF-8 punctuation; export is ASCII). `ApplyVariantFilter` after typed parse, before validator. `$profile:TBD_VariantConfig.json` else `default: true`. Crew of excluded vehicles dropped without naming typed `seats` (pin stays 12). Slice gate FAIL on `variants` 0→9 was expected; CC retired the row. Perturbation (undefined call) went red then green.
- **T-936.5 audio:** schema `$defs/audio`, `audio.rs` `radius_m > 0`, AUTHORED_BLOCKS row, panel registered, `TBD_AudioEmitter.c` twins byte-identical. Slice gate PASS. `radius >= 0` perturbation went red then green. Unread count did not include `audio`. Enforce `event` keyword rewritten to `cueEvent` at read; JSON contract stays `event`.
- **T-941.2 deploy:** `m_mDeployedHolders` one deploy per player; LOBBY→BRIEFING `ScheduleDeployClaimedHolders`; DEPLOY locked while pending; pause **Change slot** → `OpenFromPause`. Slice gate PASS. `map<string,bool>` perturbation went red then green. World-boot PASS per slice report. Twins ASCII-fold.
- **UNREAD:** `variants` retired (baseline 0). Fire-once stays `framing` / T-212. Remaining rows: objectives, framing, autoLose, seats, size, shape, area, gadgets. `cargo test -p xtask unread_wire_field_tests` → 5 passed. `cargo xtask schema validate` → All contracts valid (**8** unread at baseline).
- **Wave-level `mod compile`:** OK, 5756 files / **11457** classes, 0 TBD warnings.
- **Wave gate:** PASS, base `aec833eff`, cold `TBD_GATE_DB=tbd_wave251_close_cold`. Census `tbd_wave251_close_cold_missions_it`: **35 tables / 38 missions / 22 migrations**.
- **Slices edited schema_gates.rs:** none. T-936.5 owns schema and edited `mission.schema.json` only (allowed). CC retired UNREAD after land.

## main_left_clean

- `:3000` and `:8080` still LISTEN
