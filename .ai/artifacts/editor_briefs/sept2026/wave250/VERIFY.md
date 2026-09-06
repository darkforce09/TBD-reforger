# Wave 250 adversarial verify

HEAD at verify start: after ship+UNREAD. Base `fc2b957e7`. Merges T-705 `d67220cb7`, T-936.4 `6fade4b70`, T-291 `eded386a6`. UNREAD `532226edb`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening.

## Findings

### 1. MAJOR — flatten omits `slot.gadgets` (T-946.43)

`flatten.rs` has no `gadgets` hits. Live `/compiled` never reaches `TBD_GadgetFlags`. Hand-staged 1.3 JSON does. Same class as T-946.36/41, new key.

### 2. MAJOR — flatten drops `weatherTimeline` (T-946.44)

`EditorPayload::authored_blocks_root` still copies only `winConditions` / `tasks` / `radioPlan`. AUTHORED_BLOCKS + `weather.rs` carry the payload; `/compiled` drops it. Same class as T-946.35. T-291 owns flatten this wave but was briefed not to add the field.

### 3. MAJOR — weather timeline panel never mounts (T-946.45)

`weather_timeline.rs` is registered in `panels/mod.rs`. `settings_modal.rs` has no mount, same as T-946.33/39. Not those tickets; weather-specific child.

### 4. MAJOR — unauthored `nightVision` strips NVG (T-946.46)

JsonLoadContext binds omitted `settings.nightVision` as `false`. `TBD_FrameworkManager` then strips NVG on spawn. Goldens already author `false`. Omit ≠ authored-false is unreachable. Same class as T-946.40, different field.

### 5. MAJOR leftover — flatten still drops params / group AI / vehicle / entity / scatter (T-946.36, T-946.41)

Unchanged. Not re-filed.

### 6. NIT — authored `gps: true` cannot add a GPS

No GPS item in the TBD registry. Withhold still works. Schema says true forces the gadget on. Not filed.

---

## Attacked and FAILED to break

- **T-705 bind/apply:** new `TBD_GadgetFlags.c` twins byte-identical ASCII. Loader `Bind()` after EnvironmentReader. `map<string,bool>` presence (Count()==0 omit). Slice gate FAIL on compass/watch/gps was expected; CC retired those rows and re-pinned `gadgets` 6→33 (`unrelated` in why). `mod compile` perturbation (ApplyToBody rename) went red.
- **T-936.4 timeline:** schema `$defs/weatherTimeline`, `weather.rs` strictly-increasing `atMinutes`, AUTHORED_BLOCKS row, panel add/edit/delete/reorder, `TBD_WeatherRuntime.c` twins identical. Slice gate PASS. Equal-atMinutes perturbation went red then green. Unread count did not include `weatherTimeline`.
- **T-291 readers:** flatten already emitted spectatorPolicy/nightVision/windDirDeg (T-259/T-682). FrameworkManager latches policy + NVG strip; SpectatorController client-reads replicated policy (`none` / `own_side_delayed_60s` / `free`). color/radio/layers documented editor-only and stay off the wire. SpectatorPolicy trim perturbation went red.
- **UNREAD:** `compass`/`watch`/`gps` retired (baseline 0). `gadgets` re-pinned 6→33. Fire-once stays `framing` / T-212. `cargo test -p xtask unread_wire_field_tests` → 5 passed. `cargo xtask schema validate` → All contracts valid (**9** unread at baseline).
- **Wave-level `mod compile`:** OK, 5755 files / **11434** classes, 0 TBD warnings.
- **Slices edited schema_gates.rs:** none. T-936.4 owns schema and edited `mission.schema.json` only (allowed). CC retired UNREAD after land.

## main_left_clean

- `:3000` and `:8080` still LISTEN
