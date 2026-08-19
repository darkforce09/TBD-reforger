# Wave 211 adversarial verification — T-842 / T-826 / T-843

Verifier: Cursor Grok 4.5, 2026-08-19. Verified MERGED MAIN at **33f715b11** (`git rev-parse HEAD` = `33f715b113d9151d2e19ef003e52541144aba12c`).

| Pin | Sha / note |
|---|---|
| Wave base (last editor-relevant close) | `d05562066` — wave 132 CLOSED — editor wave 210 |
| Merge T-842 | `eb3fc965b` (slice `bb5baa1ce`) |
| Merge T-826 | `43bfc1d0c` (slice `049788e2a`) |
| Merge T-843 | `33f715b11` (slice `6a2417826`) |
| HEAD at dispatch + exit | `33f715b11` — **nothing landed after dispatch** |
| Wave gate | **GATE: PASS** (`/tmp/wave211-gate.log`) — wave gate still runs **zero** editor smokes |
| Rect-smoke path (option b) | `cargo xtask mk leptos-gates` → editor-suite **20/20** then `v-suite verify` **mass-FAIL** (`/tmp/wave211-leptos-gates.log`) |

**Environment left as found:** no repo files mutated except this report; no commits; no tickets filed. Perturbations restored byte-exact (`cmp` vs `/tmp/w211-*.bak` and vs `git show HEAD:…`) + `touch` on touched paths. Ephemeral probe debris under `/tmp/w211-*` / `/tmp/wave211-*` (not in repo). Pre-existing `:8080` API and `:3000` trunk left running. `git status` at exit: clean working tree aside from this untracked report path under `.ai/artifacts/`.

**Surface:** Class-R via `cargo xtask ai run` (`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`; `NO_COLOR` unset). Headless Chromium via `gate smoke entrance-motion-rect` for T-843 hollow attack. Live leptos-gates log consumed for editor-suite + v-suite evidence (process completed; xtask dead).

---

## FINDINGS

### F1 — `pendingBriefingMarkers` drop on server JSON save/reload (T-826 found_not_fixed)
`MAJOR | crates/map-engine-core/src/mission/compile.rs:compile_payload + store.rs hydrate | parked pre-mint markers are session/meta state; Save→server→hydrate can drop them | source audit + Class-R shape`

- Evidence:
  - Parking lives on yrs `meta.pendingBriefingMarkers` (`store.rs` `PENDING_BRIEFING_MARKERS`, upsert/promote helpers). `small_maps_json` emits full `meta.to_json`, so **local** snapshots see the park (Class-R `t826_lazy_mint_…` asserts then clears it on `add_faction`).
  - `compile_payload` body contains **no** `pendingBriefingMarkers` reference — it only pulls known meta fields (`terrain` / `map` / `title` / `environment` / `schemaVersion` / briefing path). Mission editor Save feeds `compile_payload(small_maps_json, slots_json, …)` (`mission_editor.rs` ~4045).
  - Hydrate **explicitly clears** pending on load (`store.rs` ~3437–3438: “pending pre-mint markers are session/doc state; a hydrate replaces the faction graph and must not leave parked rows”).
- Impact: Marker-only authoring before first slot/squad mint: **server JSON save/reload can lose parked markers**. Local yrs `encode_state` persist keeps them. Ticket acceptance (no phantom faction / no V1 on marker-only; V1 on first mint) still holds.
- Disposition: **MAJOR** — confirms slice `found_not_fixed`. Data-loss for the parking layer on the server round-trip. Not a silent skip of T-826 acceptance. Verifier does not file tickets.

### F2 — `mk leptos-gates` exit-0 broken by `v-suite verify` after green editor-suite (T-843 found_not_fixed)
`MAJOR | xtask/src/mk_build.rs:358 (v-suite step of leptos_gates) | editor-suite 20/20 then SPA golden mass-FAIL | live /tmp/wave211-leptos-gates.log`

- Evidence (this run):
  - Editor-suite section: **20 gates**, `"pass": true` on all — including `editor-save-dialog-rect-smoke`, `editor-entrance-motion-rect-smoke`, `editor-virtual-outliner-smoke` with `"v5_orbatWindowed": true` (T-829 absorb).
  - Then `gate v-suite verify`: **4 PASS / 21 FAIL** (dashboard, approvals, wiki, missions, …) — SPA frozen-oracle golden diffs, not editor canvas asserts.
  - `leptos_gates()` chains `gate-doctor` → `editor-suite` → `v-suite verify`; `run_steps` stops and propagates the child’s non-zero rc. Log ends after wikislug FAIL JSON; `xtask mk leptos-gates` process dead → composite **did not exit 0**.
  - Standalone `gate smoke entrance-motion-rect` re-green after restore: `"pass": true`.
- Impact on pre-close contract: **Yes — breaks the literal recipe** “required pre-close = `cargo xtask mk leptos-gates`” when local SPA goldens drift, even though option (b)’s **editor** half (rect smokes + suite) is green and wave gate correctly stays chromium-free. Docs (`EDITOR_FACTORY_FOR_CURSOR.md` §5, `EDITOR_GATE_RUNBOOK.md`) name the full composite including v-suite without an editor-only escape hatch.
- Disposition: **MAJOR** — confirms slice `found_not_fixed`. Does **not** refute T-843’s editor-suite / docs / chromium-out-of-wave-gate pins. Command center must not stamp “leptos-gates PASS” from this log; editor close evidence is **editor-suite 20/20** (plus doctor), not the composite exit code.

No BLOCKER. No NIT beyond the two MAJORs above.

---

## Safe-line

**Yes — `main` at `33f715b11` is safe to build the next editor wave on**, with the two MAJOR residuals above (parked-marker server round-trip; literal `mk leptos-gates` exit-0 vs v-suite goldens).

T-842 wrap is real (Class-R + hollow clamp RED). T-826 no-mint + lazy promote is real (Class-R + hollow mint RED; frontend `Pending::Marker` → `side_faction_id`). T-843 editor-suite + rect smokes + recipe docs + wave-gate chromium ban are real; do not confuse that with a green full `leptos-gates` composite on this host.

---

## VERIFIED-CLEAN REGISTER

### T-842 — world glyph wrap

| Claim | Result | Evidence |
|---|---|---|
| `glyph_math` wraps into `(-180,180]`; 181..359 no longer south plateau | **PASS** | `wrap_deg_180` + `yaw_to_snorm16` uses wrap not clamp (`glyph_math.rs` ~109–131); `yaw_snorm16_wraps_not_clamps` green |
| Class-R: 270≠180 tip/extent | **PASS** | `world_rotation_270_differs_from_180` — 1 passed |
| Class-R: 0..360 monotonic / no plateau | **PASS** | `every_world_rotation_of_the_compass_gets_its_own_facing` — 1 passed |
| `yaw_encoders_agree` full-domain byte parity (outside owns: `slots_gpu.rs`) | **PASS** | `yaw_encoders_agree` — 1 passed (`--features world`) |
| Hollow attack (clamp restore) | **PASS (anti-hollow)** | Perturbed `yaw_to_snorm16` to `(angle/180).clamp(-1,1)` → RED `270 and 180 share encoding — western half collapsed onto due south` left=right `-32767`; restored byte-exact vs HEAD + re-green |

`cargo xtask ai run -- 'cargo test -p map-engine-core --features world <name>'` for the four pins above: all green after restore.

### T-826 — markers don't mint factions (option a)

| Claim | Result | Evidence |
|---|---|---|
| Marker place uses no-mint path; chip stays 0 / no phantom V1 | **PASS** | `editor_ops.rs` `Pending::Marker` → `side_faction_id` only (~5834–5842); `ensure_side_faction` docs forbid markers. Class-R `t826_marker_without_faction_parks_and_does_not_declare_players`: empty `factionsById`, no `V1-PLAYER-SPAWN` |
| First slot/faction mint → V1 can fire | **PASS** | `t826_lazy_mint_promotes_pending_markers_and_v1_fires_without_slots`: `add_faction` promotes park, clears pending, V1 fires with no slots |
| Outside owns: `store.rs` parking + promote on `add_faction` | **PASS** | `set_faction_briefing_marker` parks when faction missing (~3888–3891); `add_faction` calls `promote_pending_briefing_markers` (~904–905) |
| found_not_fixed: pending drop on server JSON save/reload | **CONFIRMED — MAJOR (F1)** | compile_payload ignores key; hydrate clears pending; see F1 |
| Hollow attack (mint-on-place) | **PASS (anti-hollow)** | Else-branch forced `add_faction` + recurse → RED `marker place must not mint a faction: Object {"faction-BLUFOR":…}`; restored byte-exact vs HEAD; `t826_` **2 passed** |

`cargo xtask ai run -- "cargo test -p map-engine-core --features 'doc,mission' t826_"`: **2 passed**.

### T-843 — editor suite + leptos-gates pre-close (option b)

| Claim | Result | Evidence |
|---|---|---|
| Stale `cur`/`undo` expectations fixed; editor-suite green incl. rect smokes | **PASS** | `/tmp/wave211-leptos-gates.log` editor-suite: 20/20 `"pass": true` — `editor-cur-smoke`, `editor-undo-smoke`, both rect smokes green |
| Recipe docs name `cargo xtask mk leptos-gates` as required pre-close; chromium out of wave gate | **PASS** | `EDITOR_FACTORY_FOR_CURSOR.md` §5; `EDITOR_GATE_RUNBOOK.md` pre-close clause; `mk_build.rs` ~340–358; `wave/gate.rs` ~7–9 |
| Absorbs T-829: `v5_orbatWindowed` deterministic | **PASS** | Live suite log `"v5_orbatWindowed": true` on `editor-virtual-outliner-smoke` |
| Outside owns: `doctor.rs` auth seed | **PASS (source)** | `doctor.rs` liveness uses `api_proxy: None` + `vsuite::seed_script` / auth seed (~609–617) — matches slice disclosure |
| found_not_fixed: `gate v-suite verify` mass-fails after editor-suite 20/20 | **CONFIRMED — MAJOR (F2)** | Live log 4 PASS / 21 FAIL after suite; breaks literal composite exit-0; see F2 |
| Hollow attack (`checks_pass` 24→25 on entrance-motion-rect) | **PASS (anti-hollow)** | Perturbed expected count → verdict `"pass": false` with all surface checks still true; restored byte-exact vs HEAD; re-run `"pass": true` |

Deviations disclosed by slice (Unfiled(10), pan→MMB, save-export slot-id sort, virtual-outliner yrs unwrap filters, checks_pass perturbation) — re-checked only as attack surface; no additional functional miss found in suite evidence.

---

## Attacked and FAILED to break

1. **T-842 wrap is cosmetic / still clamps** — intentional clamp RED collapses 270≡180; green path differs + monotonic sweep + encoder parity.
2. **T-842 `yaw_encoders_agree` still documents divergence** — full-domain byte parity green on merged main.
3. **T-826 marker path still mints via `ensure_side_faction`** — live `Pending::Marker` uses `side_faction_id` only; Class-R empty factions + no V1.
4. **T-826 parking / promote hollow** — mint-on-place RED; promote + V1 Class-R green after restore.
5. **T-843 rect / cur / undo / orbat window still red** — live editor-suite 20/20; both rect smokes + `v5_orbatWindowed` true.
6. **T-843 recipe still omits leptos-gates / sneaks chromium into wave gate** — docs + `wave/gate.rs` + `mk_build.rs` pin option b.
7. **T-843 entrance-motion assert vacuous** — bumping `checks_pass` expected count forces `"pass": false`.
8. **HEAD drift after dispatch** — remained `33f715b11` for the whole verification.
9. **Wave GATE PASS as substitute for editor smokes** — standing note: wave gate ran zero editor smokes; editor evidence taken from leptos-gates editor-suite / targeted Class-R / one rect smoke, not from wave gate.

---

## Environment left as found

- Repo: HEAD `33f715b11`, no verifier commits, no ticket registry edits, no app source edits left behind.
- Restored after hollow attacks: `glyph_math.rs`, `store.rs`, `smokes.rs` — byte-equal to `HEAD`; touched only for rebuild mtime where needed.
- Ephemeral probe debris: `/tmp/w211-*`, `/tmp/wave211-*.log` — not in the repo.
- Pre-existing `:8080` API and `:3000` trunk left as found.
