# Editor factory — wave 115 adversarial verification

Range: base f2c2f88d → T-634 cec45271 → T-670 43678e6b → T-688 fbc2fa59 (HEAD).
Gate: 30/30 run 1. Verifier: Fable, on merged main. Tree confirmed clean before and after
(`git status --porcelain` empty; the one mutation experiment below was restored and re-checked).

---

## FINDINGS

### MAJOR | crates/map-engine-core/src/mission/flatten.rs:1491 ↔ apps/website/frontend/src/eden_env.rs:186 | The compiled-mission flow defaults exist twice with no cross-crate pin — drift ships green

**Evidence.** `FLOW_DEFAULT_BRIEFING_S/SAFESTART_S/TIMELIMIT_S/JIP` are `pub const` in both files.
The only guard, `eden_env.rs:421-424`, restates the literals (`assert_eq!(FLOW_DEFAULT_BRIEFING_S, 600)`)
against the *frontend's own copy*. **Repro performed:** edited flatten.rs:1491 `600 → 900`, ran
`cargo test -p website-frontend` → **`test result: ok. 800 passed; 0 failed`** while the compiler
emits 900s briefings and every editor surface (Mission Settings flow section, eden_settings.rs:681;
T-688's aggregated view context) still shows 600. Edit reverted; `git diff` empty. The frontend
already depends on `map-engine-core` with the `mission` feature (frontend Cargo.toml:23), so a real
cross-crate `assert_eq!` was buildable natively and simply was not written. flatten.rs:1460-1464's own
doc admits the invariant ("if they ever disagree, the dialog is lying about an unauthored mission")
and defers the fix to "a later slice". A third reference in `apps/website/api/.../missions.rs:1630`
is doc-comment only.

**Impact.** This is precisely the defect class T-688 was filed to prevent (a value's authority
duplicated instead of pointed at), one layer beneath the surface T-688 audited. The duplication
predates the wave, but T-688 built a "diff-from-default" feature on top of it and shipped a
disclosure instead of a pin; the moment either copy changes, the editor lies about every unauthored
mission and the whole 800-test suite stays green.

**Disposition.** Not fixed, not ticketed (standing instruction). One-line native pin is possible
today. Slice agent's hand-off: CONFIRMED in full.

### MAJOR | apps/website/frontend/src/eden_settings.rs:1634-1640 | T-688's click-through-to-owner is effectively undelivered for every row that has an owner

**Evidence.** The ticket requires rows to click through to the owning entity. The row handler does
attempt `validation_panel::route_select_by_subject_id` first (:1634) — but that router
(validation_panel.rs:423-424 → mission_editor.rs:2140-2179) resolves **slots and vehicles only**, and
the aggregation's sole `SettingOwner::Entity` construction is `kind: "Zone"` (eden_settings.rs:1367).
So 100% of entity-owned rows fall through to `toasts.message(...)` — and the toast (toast.rs:55-57)
is a text list entry: it does not navigate, focus, or open the Zones panel it names. The rows still
render `cursor-pointer hover:` affordance styling (:1617-1621).

**Impact.** "Rows click through to the owning entity" is true of zero rows today. Honestly disclosed
in code/commit and pinned as-is, but judged against the ticket constraint it is a dead click dressed
as an affordance, not a partial delivery.

**Disposition.** Documented, not fixed. Follow-up belongs with whatever ticket extends the T-655
router to zones.

### MINOR | apps/website/frontend/src/eden_settings.rs:2188-2210, :2127-2178 | T-688's textual pins are narrower than advertised

The single-constructor pin counts the needle `"Self::Schema {"` — a second construction spelled
`SettingDefault::Schema {` outside the impl evades it. The `FLOW_DEFAULT_` name-ban scans five named
function bodies but not `from_schema_node` itself. Both holes are backstopped behaviorally
(`the_view_and_the_schema_agree_key_for_key` :2042, `Declared`-assertions for all 7 mission keys,
`NotInSchema` for unknown keys), so a hardcoded default that the view can show still goes red — the
guarantee is real, the advertised mechanism is weaker than claimed.

### MINOR | apps/website/frontend/src/eden_toolbelt.rs:~1690 | T-670's "scrubbed-source check fails if T-639 rebases" is overstated

The pin fixes the exact strings `"let m_per_px = 2.0_f64.powf(-zoom);"` and
`"contour_interval_for_zoom(m_per_px)"` in dem_vectors.rs. An in-place rebase breaks it (good), but a
rebase expressed as an *adjustment line between* dem_vectors.rs:121 and :122, or an upstream re-based
`zoom` argument, passes untouched; the numeric half of the test recomputes `2^(-z)` itself
(self-referential — pins eden_toolbelt's conversion, not the ladder's). Residual hole is small
because `contour_interval_for_zoom` takes m/px directly. Also: the slice agent cited the ladder
feed as `crates/.../dem_vectors.rs:121` — right line, wrong path; the file is
`apps/website/frontend/src/world_assets/dem_vectors.rs`.

### MINOR | apps/website/frontend/src/eden_toolbelt.rs:47-53, :122-142 | Scale formatter degenerate corners

A non-finite **zoom** maps to `m_per_px = 1.0` (pre-existing T-667 convention), so a NaN camera
prints a confident `"1.00 m/px"` instead of the `"— m/px"` sentinel (the em-dash path triggers only
on a bad raw m/px). Band-top rounding carry prints 4 significant figures (`9.996 → "10.00 m/px"`,
compiled-probe verified) — cosmetic width wobble, monotonic, within-clamp values all correct
(`64.0`, `0.0625`, `0.0156` all verified against the test table).

### MINOR | wasm bundle | mission.schema.json (91,438 B) is embedded twice

`eden_zones.rs:628` and `eden_settings.rs:1063` both `include_str!` the same path — drift between
them is impossible, but the dev-serve wasm artifact (`dist/...bg.wasm`, built one minute after the
merge) contains the full schema bytes **exactly twice** (empirical `bytes.count`); rustc/LLD did not
dedupe. ~91 KB uncompressed cost (near-zero gzipped); release/wasm-opt behavior unverified. Stale
comment: eden_zones.rs:626 still says "~40 KB of JSON" — the schema is now 91 KB.

### MINOR | apps/website/frontend/src/eden_settings.rs:1623-1640 | Mission-owned rows are inert focusable `<button>`s

`selectable=false` short-circuits the click, but the element remains a focusable button that does
nothing — a11y nit.

### NIT | apps/website/frontend/src/eden_top_strip.rs:2486 | The T-692 hint-mount pin checks presence, not position

`code.contains("ControlsHint open=hint_open")` would still pass if the mount migrated outside the
gated subtree — exactly the premise-relaxation trap this wave was told to look for. This wave did
NOT trip it (position verified by structure, below), but the pin does not defend the invariant it
narrates.

### NIT | apps/website/frontend/src/eden_layout.rs:232 | `STRIP` is production-dead, silenced by a pre-existing crate-level allow

Its only remaining consumer is T-634's own test (eden_top_strip.rs:2618, :2669 — which pins the
two-row shell to STRIP's surface recipe verbatim, so the corpse is load-bearing as a reference).
`#![allow(dead_code)]` (eden_layout.rs:33) predates the wave (present at f2c2f88d), so nothing
warns. No other consumer exists (repo-wide grep). Claim confirmed exactly as disclosed.

### NIT | T-670 mechanism corrections (behavior fine)

(a) The claim's native-fallback story is misdescribed: on native, `scale_mpp` is not absent — it is
seeded `m_per_px(-2.0)` = 4.0 (mission_editor.rs:1553) and always passed (:4118), so ScaleBar takes
the early return with the seed; the `camera_snapshot()` fallback is currently dead code (numerically
identical outcome to the old path). (b) First rAF frame always publishes one redundant write
(`last_scale_text` starts empty). (c) The SCL cell's no-prop fallback is static while ScaleBar's is
live — a future caller omitting the prop on wasm would show a frozen number beside a live bar; no
such caller exists (single mount, mission_editor.rs:4109).

---

## VERDICT

**Is `main` safe to build the next wave on — YES.**
No BLOCKER. Both MAJORs are latent (values currently agree; the dead click is disclosed and pinned
as-shipped); nothing on main is broken, no operator data is at risk, and the gate examined what it
claimed to examine.

---

## VERIFIED-CLEAN REGISTER
Claims re-proved from primary evidence; falsification attempts that found nothing.

**T-634 (eden_top_strip.rs):**
- **No pre-existing pin weakened/deleted/edited** — proved from the hunk map, not the green run: the
  diff's last content hunk ends at old line 1295; every pre-existing test module spans old lines
  1600–2413 and the final hunk (`@@ -2411,3 +2598,278`) contains zero `-` lines — pure append.
  34 `#[test]`s before, 42 after, `cargo test eden_top_strip` → 42/42 green.
- **Scrubber commits on settle** — the strip mounts `ui::Slider` (:1293) whose only DOM handler is
  `on:change` (ui.rs:187, :239; no `on:input` anywhere in ui.rs live code); the strip's callback runs
  `author_env` + `row_mirror.set_time` on that settle event only. T-192's ~30Hz debounce is intact.
- **ControlsHint still inside the gated subtree** — mount at :1463 sits between the rows' close
  (:1452) and the `STRIP_ROWS` root div's close (last `</div>` of the view), and that root is
  rendered inside `(!chrome_hidden.get()).then(...)` at mission_editor.rs:4033-4035. Backspace
  hide/show holds by construction.
- **Height contract** — re-derived independently of the generated CSS: the 48px comes from the
  *unchanged* wrapper `<div class="absolute inset-x-0 top-0 z-30 h-12">` (mission_editor.rs:4034,
  outside T-634's diff); `STRIP_ROWS` is `h-full flex-col`; row 1 `h-6 shrink-0` (24), row 2
  `flex-1 min-h-0` (remainder = 24, border-t inside border-box). The strip did not grow; no
  `STRIP_TOP_PX` consumer moves; eden_layout.rs untouched this wave.
- **Census consolidation is lossless** — element-for-element diff of old :1098-1147 vs new
  :1149-1198: WEST/EAST/IND/conditional-UNA/TOTAL spans, all five tooltips, `data-slot-census`,
  `data-mission-summary`, `max-w-[22rem] truncate` + full-text tooltip all survive; only layout
  classes changed (flex-col→row + divider).
- **Ellipsis rule** — `an_ellipsis_is_a_promise_of_a_dialog` (:2856) is a *bidirectional* check over
  the whole MENUS table (`ends_with('…') ⇔ Save|Settings`); the only `…` labels are `Save Version…`
  and the two `Mission Settings…` rows, all dialog-opening; `Export Compiled` (row 2, outside MENUS)
  carries no ellipsis either.

**T-670 (eden_toolbelt.rs, mission_editor.rs):**
- **Performance guard is real** — write is string-compared before `.set` (mission_editor.rs:4452-4458),
  `last_scale_text` persists across frames; `scale_mpp` has exactly one writer in the crate;
  `RenderEngine::zoom()` → `OrthoCamera::zoom()` is a plain f64 field read (engine.rs:1720,
  ortho.rs:174), no JS boundary; the block executes per-frame *before* the separate `>= 1000.0` HUD
  gate — not stale, not 60fps re-rendering.
- **Pan cannot change scale** — ortho camera writes `scale` only from `zoom` via `.exp2()`
  (ortho.rs:127, :200, :342); pan mutates only `target`; no latitude term exists. Dropping the
  `cursor` subscription on the signal path loses nothing.
- **The printed number is the ladder's input** — both sides are literally `2^(-zoom)`
  (eden_toolbelt.rs:47-53; dem_vectors.rs:121-122), same engine zoom convention; and the `world`
  feature is genuinely absent from the frontend's native dep graph (Cargo.toml:23 vs the
  wasm-only :110; `cargo tree -e features` confirms), so the direct native test truly was impossible.
- **Precision across the clamp** — MIN/MAX_ZOOM are ±6.0 (ortho.rs:27-29) ⇒ 64.0 / 0.015625 m/px;
  both endpoints, the 0.0625 regression case, and the NaN/inf/0/negative → `"— m/px"` paths verified
  against the test table and an independently compiled probe.
- No pre-existing T-667/t636/t642/t668 test edited (both files: pure appends); `data-status-scale`
  unique; status-bar change is one span inside the existing group.

**T-688 (eden_settings.rs):**
- **Defaults are schema-sourced** — the only construction of a value-carrying default is
  `Self::Schema {` at :1105 inside `from_schema_node`, fed from `node.get("default")`;
  `MISSION_SETTING_POINTERS` (:1225-1239) holds `(key, json-pointer)` pairs only; `SettingRow.default`
  is assigned solely via `schema_default`/`NotInSchema`; all 15 t688 pins pass; the pre-existing
  `FLOW_DEFAULT_*` uses at :675-688 are T-224's *editing* section, disjoint from the new block, and
  the pin's scope statement is honest about that.
- **Schema counts exact** — recounted with python3/json against
  packages/tbd-schema/schema/mission.schema.json: all 7 mission-level pointers resolve and **0**
  declare `default`; `$defs/zoneRules/properties` has exactly **23** entries, **12** without
  `default`. Both claimed counts correct.
- **Read-only surface** — zero `<input>/<select>/<textarea>/on:change/on:input/author_*` in the new
  block's live code; every interactive element enumerated: pointer row, overlay, close, filter
  (a `<button type="button">` toggling a UI signal), row buttons (router/toast). Nothing writes the
  document; pinned by a live-source scan that keeps markup literals.
- **No third schema copy, no drift** — exactly two `include_str!`s of one path; full-content byte
  count in the artifact is exactly 2; no runtime schema fetch.
- **Reachability (cross-slice #16)** — the two slices agree: T-634's gear (strip :1345-1350) and the
  `Mission Settings…` menu rows set `settings_open` → `MissionSettingsDialog` mounted *ungated* at
  mission_editor.rs:4136 → `data-open-all-settings` pointer row (:508) → sibling `AllSettingsDialog`
  (:494/:1456), pinned by `the_view_is_reachable_from_mission_settings`. The aggregated view is
  reachable in the running app.

**Cross-slice #17:** strip stayed 48px (wrapper untouched), so the status bar and map pane geometry
are unmoved; all four wave-touched `data-` hooks (`data-status-scale`, `data-slot-census`,
`data-mission-summary`, `data-open-all-settings`) are single-claimed repo-wide; no new z-index
entrants (strip z-30/scrim z-40/dropdowns z-50 hierarchy unchanged; T-670 added no positioned
element).

**Hygiene:** the flatten.rs mutation was reverted and `git status --porcelain` is empty; the only
path created is this report.
