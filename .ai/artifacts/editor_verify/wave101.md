# Wave 101 adversarial verification — T-639 / T-662 / T-663 (911844b9..HEAD)

**VERDICT: 0 BLOCKER / 1 MAJOR / 3 MINOR / 5 NOTE — all 423 frontend + 7/7 lod_gates tests green; no behavioral defect found; the MAJOR is a canonical-doc/claim divergence, not a code bug.**

Verifier: Fable 5, read-only. Tests run under `CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`
(distrobox-host-exec): `cargo test -p website-frontend` → **423 passed / 0 failed**;
`cargo test -p map-engine-core --features doc,mission,world lod_gates` → **7 passed**;
same without `world` → **0 matching** (module compiled out, as claimed);
`keys_nothing_reads_are_not_authored` → green.

---

## F-1 MAJOR — T-639: canonical §N3 LOD contract now wrong at every band; "EXTENDS" claim false

**Evidence.** `docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md:56-71`
("N3 — Master LOD band table (canonical — v2)") pins contour intervals per deckZoom:
−6…−4→100 m, −4…−2.5→50 m, −2.5…0→50→20 m, 0…+1→20 m, +1…+3→10 m, +3…+6→10 m.
Shipped code (`lod_gates.rs:130-140`, ladder {5,10,20,40,80}, ideal = 16.3095·0.19438·m_per_px
= 3.1702·m_per_px, nearest-log2 snap) produces, at those same zooms (m/px = 2^−z):
z=−6→**80 m** (N3: 100), z=−4→**40 m** (N3: 100/50 boundary), z=−2.5→20 m, z=0→**5 m** (N3: 20),
z=+1→**5 m** (N3: 10), z=+3→**5 m** (N3: 10). Every band's value differs except one; both ladder
ends changed (max 100→80, fine end 10/20→5). No file under `docs/` was touched in the range
(diffstat: 6 files, none docs). The ticket text (registry.json:15984, TICKET_LEAD.md:20) claims
"This EXTENDS rather than replaces the ladder in t090_render_lod_contract.md §N3" — as shipped it
replaces every number in it.

**Impact.** The doc labeled *canonical* is now the wrong authority for every downstream contour
ticket (T-640 tint, T-152.7 labels read it; T-152.7's spec already cites
`contour_interval_for_zoom`). LOD4 review row (`:113`) also still names `contourIntervalForZoom`
from the deleted `worldmap/lodGates.ts`.

**Disposition.** deferred-ticket (doc sync — Cursor owns doc writes; registry summary's "EXTENDS"
wording should be corrected in the same pass). Not a code fix.

## F-2 MINOR — T-639: `CONTOUR_REPRESENTATIVE_SLOPE` is not tan(11°) as its comment claims

**Evidence.** `lod_gates.rs:111`:
`pub const CONTOUR_REPRESENTATIVE_SLOPE: f64 = 0.194_380_309_147_231_4; // (11 deg).to_radians().tan()`.
True value (bc, 25 digits: `0.1943803091377184842431941`; awk agrees): **0.19438030913771848**.
The constant's digits diverge after the 10th significant digit (Δ=9.5e-12, rel 4.9e-11 ≈ 340k ULP)
— a fabricated digit tail, not print rounding. Contrast `TARGET_SPACING_PX:108`, which IS exactly
√266 to the last digit, so correctly-rounded constants were achievable.

**Impact.** Zero behavioral (switch points move ~1e-10 m/px). But the annotation is factually
false on a load-bearing constant in a slice whose whole pitch is "derived, not fudged".

**Disposition.** fix-in-wave (one-line constant correction) or record-only.

## F-3 MINOR — T-639: tan(11°) has no real-terrain provenance in the tree; the acceptance test that "proves" it is circular

**Evidence.** Searched `.ai/`, `docs/`, registry for any Everon DEM slope statistic, "11°",
"median gradient", "rolling interior" — nothing outside T-639's own code/commit exists. The only
in-tree "derivation" is `everon_like_median_slope()` (`lod_gates.rs:240-289`): a synthetic
sinusoid whose amplitude `AMP: f64 = 1.184` is documented in-comment (:245-248) as "the only free
knob", tuned so the median "lands at" tan(11°) — yet the pinning test's doc (:292-293) says it
"**Proves** `CONTOUR_REPRESENTATIVE_SLOPE` is a **real terrain statistic, not a fudge factor**",
and the production comment (:102-104, :109-110) asserts "Everon's rolling-interior median
gradient (DEM slope statistics)". A knob tuned to hit the target then asserted to hit the target
proves nothing about Everon; the real 6400² DEM (packages/map-assets, LFS) was never measured.
What tan(11°) actually is: the value implied by fitting the 3.41→10 m and 6.20→20 m corpus
anchors to band centre (10.2°–11.2°) while sacrificing the 5 m anchors (which need 14.3°–19.1°)
— i.e. a fitted parameter, consistent with the on-record inconsistent-anchors caveat.
`contour_spacing_in_band_at_four_zoom_levels` compounds this: its five zooms are chosen AT rung
centres (m/px = interval/(T·s)), where spacing ≡ TARGET by construction — the "CORE ACCEPTANCE"
cannot fail while the tuning assertion holds.

**Impact.** Behavior fine (corpus switch points genuinely reproduce: derived 2.230/4.461/8.922
m/px vs claimed 2.2/4.5/8.9). The finding is false provenance: per brief, a free parameter is
fine; one documented as a measured DEM statistic, with a test titled to certify that, is not.

**Disposition.** record-only (comment/test-doc honesty pass); optionally a deferred ticket to
measure the real DEM median slope and either confirm 11° or re-tune.

## F-4 MINOR — T-663: remaining-census claim wrong about location; two stale comments attached

**Evidence.** Claim: "8 remaining census hits are prose/test-literals in
eden_env.rs/eden_top_strip.rs". Workspace grep (rs/ts/tsx/c/json/sql/toml over apps, packages,
crates, scripts, xtask): camelCase `viewDistance`/`thermals` also live at
`apps/website/api/src/services/mission_compile.rs:548,560-561` (test payload in
`authored_environment_beats_a_stale_mission_row`) and prose at `eden_settings.rs:32`;
plus snake_case prose in `eden_env.rs` (:27 etc.). The API-side hits are a test literal proving
stray env keys keep the compiled document schema-clean — **no reader; the deletion is safe** —
but they are exactly the kind of hit the census claimed not to exist outside the two named files.
Attached rot: `mission_compile.rs:548` still says the keys "are in the payload because **the
dialog really writes them**" — false since T-193 removed the controls (see `eden_env.rs:15-29`).

**Impact.** None behavioral. Claim-accuracy defect (census under-scoped to frontend/src), which
this factory's verification model treats as reportable.

**Disposition.** record-only; the :548 comment is a trivial fix-in-wave if a wave touches the file.

## N-1 NOTE — T-639: the default boot view sits at the ladder's worst point (record-only)

Boot/default zoom is −2.0 (`world_assets/mod.rs:285,327,386` fallback; T-065 note "default zoom
-2"). m/px = 4.0, next switch at 4.461: the default view lands near the 10→20 m boundary, where
representative-slope spacing is 10/(0.19438·4) = **12.9 px — below the 14 px floor**, and the
interval at the default view flips old-20 m → new-10 m (denser contours). Numerically inside the
on-record boundary caveat (≈[11.5, 23] px, verified: T/√2=11.53, T·√2=23.06), so not re-filed as
wrong — recorded because it is the first view every operator sees, and the acceptance test only
samples rung centres.

## N-2 NOTE — T-639: name/parameter trap is live in a queued spec

`contour_interval_for_zoom` keeps its name but takes m/px (docstring :126-128 warns).
`t152_7_height_markers.md:28` still reads "label every `contour_interval_for_zoom` level" under a
"deckZoom ≤ −1" gate — a T-152.7 implementor passing deck_zoom gets intervals wrong by the full
2^(2·zoom) factor, silently (e.g. z=−2: passes −2.0 → clamps to finest 5 m instead of 10 m; the
≤0 guard masks rather than flags). No stale caller exists today (workspace-wide: definition,
`world/mod.rs:71` re-export — byte-identical, empty diff 911844b9..HEAD — and the one corrected
caller `dem_vectors.rs:113-114`; no TS twin survives, no wasm export remains). Also: the claim
"7/7 new tests" is 7 passing / **4 new** (tree_band, fence_pier, exhaustive_zoom predate the
slice). Disposition: deferred to T-152.7's wave brief.

## N-3 NOTE — T-662: place-drop still insets by chrome constants while chrome is hidden

`mission_editor.rs:1685-1688` cancels a palette place released inside the static
DOCK_LEFT_PX/DOCK_RIGHT_PX/STRIP_TOP_PX/TOOLBELT_BAND_PX bands regardless of `chrome_hidden`,
contradicting the ":2059-2060 while hidden … every px is a map gesture" comment. Reachable only
by arming a place then pressing Backspace mid-gesture (the palette is unmounted while hidden);
outcome is a silent cancel. `select_tool.rs:456-489` (`farthest_empty_px`) also insets but is a
smoke-gate helper — conservative, correct either way. Record-only.

## N-4 NOTE — T-662: stale de-alias comment

`editor_ops.rs:318` still documents `delete_selection()` as "Delete/Backspace — remove the
selected slots" after T-662 de-aliased Backspace. Trivial fix-in-wave / record-only.
(Related, pre-existing and unchanged: the container-level `prevent_default` on contextmenu also
suppresses the native menu — including paste — over the ungated dialogs' text inputs, since the
dialogs are container descendants; T-662 did not alter this.)

## N-5 NOTE — cross-slice forward interactions (no defect in this wave)

- **T-664**: must mount its menu beside the UNGATED modals (`mission_editor.rs:2129-2151`
  pattern), not inside a `chrome_hidden` gate, or hide-interface eats the open menu; its
  contextmenu listener will see `defaultPrevented == true` (set at :1870) and must not treat that
  as "handled". Its ticket text ("T-662 … removed the blanket prevent_default") describes the
  shipped state loosely — prevent_default remains, only the pan-eating and stop_propagation-style
  suppression are gone; the handler body is theirs to replace.
- **T-636**: splitting BottomToolbelt into toolbar + full-width status bar ("two mount points,
  not one") must keep BOTH behind the `chrome_hidden` gate; the debug HUD now lives inside the
  gated toolbelt block (:2115-2126), so hide-interface hides the rf readout — Eden parity, by
  design, but telemetry moved there inherits the gate.
- **T-706** (wave 120 schema widening): no interaction defect — `mission.schema.json` environment
  is closed (`additionalProperties:false`, dateTime/weatherPreset/windDirDeg only; :143-151) and
  never carried viewDistance/thermals; but `eden_env.rs:309`'s census test pins those key NAMES as
  never-authored — T-706 must consciously edit that list if it ever re-adds either key.
- **Backspace vs dialog typing (the F-question): refuted.** The keydown guard
  (`mission_editor.rs:1015` → `mission_history::in_editable_field()`:459-471, INPUT/SELECT/
  TEXTAREA/contentEditable) means Backspace in any dialog field edits text and never toggles
  chrome. Listener is on `window` (:1049-1053), leaked/forgotten — survives all unmounts, so
  un-hide is always reachable; Esc routing (ui.rs one-Escape-one-dialog) is orthogonal. No stuck
  state found.

---

## Verified claims (spot-check ledger)

**T-639.** Sign convention proven end-to-end: `ortho.rs:105-111` zoom = log2(px/m), scale = 2^zoom;
`engine.rs:1720` `zoom()` → camera; `dem_vectors.rs:113` `2^(−zoom)` → correct m/px. Switch points
re-derived: 2.230/4.461/8.922 (claims 2.2/4.5/8.9 ✓). Corpus-breakpoint asserts (1.03→5, 1.30→5,
3.41→10, 6.20→20) re-derived by hand and pass ✓. TARGET_SPACING_PX = √266 exact ✓. Snap/clamp
edge behavior (≤0, NaN, 64, 1000 → clamped) ✓. Re-export byte-identical (empty diff) ✓. Single
caller workspace-wide ✓. Sobel in the test is form-identical to `dem/hillshade.rs:52-62` ✓.
Feature gating: `world` cfg-gates the module; doc,mission alone → 0 tests ✓. Both on-record
caveats verified numerically and NOT re-filed (rung-boundary band ≈[11.53, 23.06] px ✓; anchors
mutually inconsistent: 5 m@1.03 needs slope ≥0.256 vs 10 m@3.41 needs ≤0.209 ✓).

**T-662.** Pan guard `ev.button() == 1` only (:1419) ✓. oncontextmenu prevent_default-only,
bubble phase, on the container (:1868-1871, attach :1947-1950); comment forbids stop_propagation ✓.
Reachability PROVEN: the only capture-phase listener in the entire frontend is the wheel
(:1395-1400); no other pointerdown/mousedown/contextmenu listener outside mission_editor.rs;
chrome overlay is pointer-events-none and stops pointerdown only; canvas has no pointer-events
CSS; pointer capture exists only during MMB pan/promoted LMB gestures. **select_tool.rs contains
zero `button()`/`buttons()` reads — the "now-dead RMB branch" premise is refuted; nothing to
remove.** Four mounts gated (:2066-2128, incl. HUD inside toolbelt block), dialogs deliberately
ungated (:2129-2151) ✓. Canvas is full-bleed `inset-0` and resize (:2009) uses the full container
rect — hiding docks leaves no letterbox and needs no resize; the wheel guard's
`closest("[data-eden-chrome]")` degrades correctly when mounts unmount (canvas becomes the
target) — the (a)/(b) pair is CONSISTENT except the N-3 place-drop corner. 4 new
source-inspection tests confirmed (backspace_hides_chrome_and_does_not_delete,
chrome_hidden_signal_gates_the_four_mounts, rmb_no_longer_pans,
contextmenu_is_unsuppressed_but_stops_the_browser_menu), pinned on live_code scrub ✓.
Claimed line numbers drift ≤3 lines (chrome_hidden :736 not :733) — cosmetic.

**T-663.** Diff is exactly 12 deletions / 0 insertions in dto.rs + editor_ops.rs ✓. Zero readers
workspace-wide (rs/ts/tsx/c/json/sql/toml across apps, packages, crates, scripts, xtask) ✓.
`MissionEnv` derives Clone/Debug/PartialEq only — hand-parsed, no serde, no R-api golden ✓. Two
`MissionEnv::default()` sites (`eden_settings.rs:65`, `eden_top_strip.rs:554`) unaffected ✓.
mission.schema.json never carried the keys; `serverMaxViewDistance`/`networkViewDistance` in
scripts/mod server configs are an unrelated namespace ✓. apps/mod clean ✓.
`keys_nothing_reads_are_not_authored` green ✓. 419 (T-663) + 4 (T-662) = 423 — counts coherent ✓.

**Counts (G).** website-frontend **423/423**; lod_gates **7/7** under `--features
doc,mission,world` (4 new + 3 pre-existing); **0 compiled** without `world` ✓.
