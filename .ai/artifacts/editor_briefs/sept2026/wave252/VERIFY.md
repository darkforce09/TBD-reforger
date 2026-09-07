# Wave 252 adversarial verify

HEAD at verify start: after ship+stamp+repack. Base `884fb3627` (wave 251 CLOSED). Dispatch HEAD `b59b99116`. Host cargo, `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`. `:3000` / `:8080` left listening.

This was a **custom 10-wide pack**, not lock row `n = 252` (`T-936.6 T-941.3 T-941.5`). T-936.6 stayed `ready` (depends on T-936.5; not dispatched). `--reserve` of the ten refused: carry already derived emptied `{T-301, T-302, T-304}` (lock wave 255, all three shipped) and relabeled it 252. Plain `wave repack` froze that 3-set as `[[emptied]] n = 252`. Close uses `--tickets` of all ten — the gated span since `884fb3627`.

| Ticket | Merge | Agent |
|---|---|---|
| T-301 | `11a785b055c1f1b71a25d5fd2a27a344fed77a3f` | briefing kit gear rows |
| T-302 | `9387e6237e3533babac6e8834767804da0a0d1ce` | four-weapon equip boot proof |
| T-304 | `d8415afe7245084747645392237a4600df61ce55` | RegistryScan weapon weight |
| T-937.1 | `fbad77817eb993f18edce39b11141eb58bf6cdcf` | native YArray id lists |
| T-938.1 | `e8eb9bbe299cf28b11824cf20ee91596c38c3f06` | GPU lane buffer pool |
| T-940.4 | `689321476bbad257636a1bd45c6da3f8184e3379` | nested telemetry counters |
| T-941.3 | `8b934eefebdf8a5fd2375433c8d09803d7ccf9f3` | END/DEBRIEF overlays |
| T-941.5 | `2b7e65ff92a9be6d8d789ecc19001d988b323a8e` | spectator default 2000 m |
| T-941.6 | `f777603eac79949400870a12166e5d2b2e531370` | `#tbd link` off public chat |
| T-941.7 | `49d5ea8acbad512200a41a459910c32f54d285bf` | script-side radio fallback |

## Findings

### 1. MAJOR — SpectatorHost still treats 0 as unlimited (T-946.50)

T-941.5 clamps `SpectatorComponent` to 2000 m. Unowned `TBD_SpectatorHost.c` still documents `0 = unlimited` and `ClampToRange` skips when `s_fMaxRangeM <= 0`. Filed `T-946.50`. Not a reject: only caller today is the component this slice owns.

### 2. T-940.4 `files_outside_owns` — `apps/website/api/tests/telemetry.rs`

T-393 goldens asserted flat-body / NULL deaths. Those asserts *were* the defect. Accepted. Nested kill/team_kill/longest_kill_m stay honest zeros (ONE LIFE has deaths only).

### 3. MAJOR leftover — flatten still drops authored blocks (T-946.36, T-946.41, T-946.43, T-946.44, T-946.47)

Unchanged. `authored_blocks_root` still copies only `winConditions` / `tasks` / `radioPlan`. Not re-filed.

### 4. MAJOR leftover — panels unmounted (T-946.33, T-946.39, T-946.45, T-946.49)

Unchanged. Not re-filed.

### 5. NIT — T-941.3 kills counted on FrameworkManager

ResultsReporter has no kill feed. DEBRIEF deaths from `SpawnManager.IsPlayerDead`; kills from `TBD_FrameworkManager.OnPlayerKilled`. Honest; do not file.

### 6. NIT — T-941.7 `TuneLine` still special-cases `NO_BACKBONE`

Sibling client. World `RadioManagerEntity` missing is the quoted 2026-09-04 deferral. Do not file.

### 7. NIT — T-941.6 PLAYTEST_RUNBOOK / T-327 still deferred

Docs and `executor: workbench` ticket are not owned. Do not file.

### 8. T-304 registry JSON still 0/107 `weight_kg`

Code twins are byte-identical. Operator Workbench re-scan remains. No hand-patch.

### 9. T-302 overall world-boot FAIL is T-299 warning ratchet

`2 > baseline 1`. T-302 four-weapon assert passed (`4 ok`, slots 0–3). Do not add a second faction.

### 10. T-938.1 other `create_buffer_init` paths left alone

T-938.3 shares `engine.rs`. In scope for later lock waves.

---

## Attacked and FAILED to break

- **All ten slice gates:** PASS. COL=0 vs merge-base. merge-tree vs then-HEAD: OK.
- **T-301 kit rows:** launcher/handgun/throwable on briefing kit. Twins. Slice gate PASS.
- **T-302 four-weapon:** `--selftest` + live `--compiled` assert `ok=4` slots 0–3. Ratchet FAIL is T-299, not this ticket.
- **T-304 weight:** `StorageComponent`; most-derived buckets; twins 1793 lines identical. Scan JSON still empty until Workbench.
- **T-937.1 YArray:** `squad.slotIds` / `layer.entityIds` native arrays. Other id lists unchanged (locked).
- **T-938.1 GPU pool:** slot/cluster lanes pooled. Other upload paths untouched.
- **T-940.4 nested ingest:** flat counters folded; IT goldens updated (owns exception above).
- **T-941.3 END/DEBRIEF:** overlays + FrameworkManager. chimeraMenus presets still missing (Workbench GUID; fallback path).
- **T-941.5 spectator 2000 m:** component clamp. Host `<= 0` skip is T-946.50.
- **T-941.6 account-link:** `#tbd link` consumed before public `OnNewMessage`. Other admin commands still public (ticket-locked).
- **T-941.7 radio fallback:** script-side when backbone missing. `NO_BACKBONE` client copy is sibling.
- **UNREAD:** no new schema keys this pack. Fire-once stays `framing` / T-212. Remaining rows: objectives, framing, autoLose, seats, size, shape, area, gadgets.
- **Close label:** `ticket check` OK after plain repack. `[[emptied]] n = 252` tickets `T-301 T-302 T-304`. Open waves start at 253 (`T-936.6 T-941.4 T-941.8`). `--tickets` names the ten; emptied subset is covered.

## main_left_clean

Command-center bookkeeping only on `main` (ticket TOMLs, `wave.lock`, T-946.50, this file). Application code landed via `--no-ff` merges of `slice/T-*`.
