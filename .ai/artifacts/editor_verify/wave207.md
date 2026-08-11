# Wave 207 adversarial verification — T-808 / T-802 / T-794 (THE VISUAL WAVE)

Verifier: Claude (Fable), 2026-08-11. Verified MERGED MAIN at **c6b875de** (base 7badb44d; merges
2812a3e8 T-808 · 489d1bba T-802 · 785b2410 T-794; completions 2f012e14 ff7c2c81 bb03516a 3e19524f
0bc55dc5 f7f4a02e; c6b875de itself is import-reorder only — verified, 2 lines of `use` order in
draw_order.rs). Tree left exactly as found: every perturbation restored byte-exact
(`git diff --exit-code` per file) and `touch`ed; `git status` clean at exit save this report;
nothing committed, no tickets filed. All `VERIFY207 *` missions deleted (final API list: zero).
Zero chromium processes at exit; my probe profiles removed.

**Surface used:** live `:3000` (trunk serve with a STALLED watcher, serving agent C's 01:46 dist).
Verified before trusting: served wasm is **byte-identical** to `apps/website/frontend/dist`
(sha256 `9a6c8395…` both sides), contains `wrap_deg_180` ×4; served CSS carries `overlay-fade` ×4 +
`prefers-reduced-motion`. The dist is a **dev-profile build** (agent C's trunk_build.log) — pixel
semantics identical to release; perf numbers below are debug-lane, within-build comparisons only.
Probe lane: `?force=webgl&sat=preview`, SwiftShader, fresh chromium profile per probe (the band's
stdlib CDP harness; real `Input.dispatch*` events). WebGPU/Vulkan NOT exercised (established lane;
no cheap check made). `window.__vpanics` empty in every session. Cargo suites via
`wave.sh test --slice` into private target dirs; gate binary rebuilt from HEAD before use
(host bridge; 6.2 s incremental).

## FINDINGS

### F1 — the editor smoke SUITE cannot run green on main; the wave's new smoke is unreachable through it, and the last four "GATE PASS 30/30" stamps postdate the breaks
`MAJOR | tools/tbd-tools/src/smokes.rs:1666,2445 | two stale smokes (cur, undo) fail on any
current dist, so gate editor-suite dies at smoke #4 and never reaches entrance-motion-rect (#7) |
proven by running both smokes on merged main + git-dating the breaking commits`

- `gate smoke cur` FAILS: `c1_centreIsTarget` / `c2_offsetMath` compare the cursor readout to
  `"6400.000"` / `"5920.000"`, but the readout now renders `"6400.000 m"` — the **values are
  exact to the metre**, only the ` m` unit suffix is new. It came from `fmt_coord_eden`
  (T-793, 7ed91e00, **08-09 20:39** — pre-dates the wave base). The smoke's expected strings are
  unchanged since 07-17 (ac7bdaaee).
- `gate smoke undo` FAILS: `a6_docksMounted` wants an `<aside>` containing `"Editor Layers"` and
  one containing `"Factions"`. Measured live: left dock header is now `"Layers"`+`"Locations"`
  (T-696/T-637, **08-07**), and no aside carries `"Factions"` text; the ORBAT button IS present.
  Stale text expectations, not a broken dock.
- `editor_suite` stops at the **first** failure (smokes.rs:3782-3788); `cur` is suite #4, the
  wave's new `entrance-motion-rect` is #7, `undo` #19 — so **no end-to-end suite run can have
  been green since 08-09 at the latest**, and the suite can never reach the smoke this wave added.
  The last four wave closes (203: 08-10 05:22 · 204: 09:30 · 205: 19:29 · 206: 23:08) each stamp
  `GATE PASS 30/30` and **all postdate both breaking commits**. Whatever the 30 items measure, a
  green editor-suite execution has not been among them. If the wave-207 orchestrator's "30/30"
  claims to include the editor suite, this is the BLOCKER class ("success on code never
  examined") — escalate; on the evidence available (the runbook separates `make leptos-gates`
  from the wave.sh gate) I classify MAJOR: gate coverage is lying by omission, main itself is not
  broken.
- The features behind both stale smokes are HEALTHY (the smoke's own readback proves the cursor
  unproject to the millimetre; my probes prove the docks). The new smoke itself is sound
  standalone: see the T-794 register below (green 24/24 on the real dist, red 16/24 on a true
  pre-fix sheet).
- Disposition: re-ticket — update `cur`'s two expected strings (strip/expect ` m`) and `undo`'s
  a6 dock-text needles, then a full `gate editor-suite` run on main; and record what the close
  ritual's "30/30" actually executes.

### F2 — the draw_order extractor follows a SUFFIX-renamed function instead of going red
`NIT | crates/map-engine-render/src/draw_order.rs:944-951 | the "renamed path is a red pin" claim
only holds for renames that break the signature PREFIX | proven by perturbation`

- Rename `vehicles_bind_symbology` → `vehicles_bind_symbolo_gy` (substring broken): pin panics
  `engine.rs has no 'pub fn vehicles_bind_symbology'` — RED as claimed. Rename →
  `vehicles_bind_symbology_v2` (prefix preserved): `find`/`contains` still match, pin stays
  **GREEN** and silently pins the renamed body. Content needles still bind that body, so the
  guard's substance survives — only the rename-detection claim is overbroad. Ambiguity refusal
  is real: planting the sig in a comment → `not unique … would pin the wrong body`, RED.

### F3 — (pre-existing, out-of-wave, unfiled) Delete on a selected vehicle is a silent no-op
`OBSERVATION | apps/website/frontend/src/editor_ops.rs:485-506 | delete_selection partitions the
selection into comments and slot-ids only; a vehicle id falls through remove_slots and the
keypress is swallowed | measured live: click v-m → selection ["v-m"], Delete → document AND pixels
unchanged (crop delta 0.0)`. Vehicles predate this wave as pick-only (T-425); no wave-207 claim
touches vehicle deletion. Not in the registry (checked); flagged for the band, not filed.

### F4 — agent B's pre-fix perturbation sheet was narrower than its "EXACT pre-T-794 sheet" label
`NIT | scratchpad smoke_perturbed3.log | their scratch sheet restored the transform keyframes +
durations but KEPT the reduced-motion block, so only the 8 first-pass checks went red and the rm_
half stayed green | proven by my own scratch dist carrying the FULL pre-fix sheet (PRM block
deleted): 16/24 red including all rm_ checks`. Their conclusion (smoke fails pre-fix) stands and
their travel numbers reproduce exactly; only the evidence label overstates.

### F5 — completion C's "constant ~6° glyph-tip offset" is not constant — and that's fine
`NIT | wave-207 completion report wording | measured slot tip-offsets vary −4.7°…+6.2° smoothly
with heading (range 10.9°) — a raster-aliasing signature, NOT an encoding nonlinearity | vehicles
on the SAME encoder measure ±2° (registration) / ±1.1° (principal axis); snorm16 quantization is
0.0055°; the clamp-catching pin (every_heading_of_the_compass_gets_its_own_facing) goes verbatim
red when wrap is reverted to clamp`.

## VERIFIED-CLEAN REGISTER (measured numbers)

**Heading sweep — slots** (9451/9452; 10 slots at 0/45/90/135/180/225/270/315/359 + h1, 1 m/px):
tip bearings 355.3 / 40.8 / 86.5 / 134.4 / 186.1 / 231.2 / 274.8 / 315.1 / 354.6 (h1: 355.4).
Adjacent gaps 39.5–51.7° — strictly monotonic, **no plateau anywhere** (the clamp defect drew
225/270/315/359 all at ~186 — dead). Wrap: h359↔h1 = 0.8°, h359↔h0 = 0.7° — continuity, no cliff.
Direct 359-vs-1 mask registration: Δ=2.0°, IoU 0.901.

**Heading sweep — vehicles** (M1025 ×10, same headings): rotation-registration vs h0 = 0 / 47 /
88 / 133 / 178.75 / 227 / 268 / 313 / 361 — 9 distinct, gaps 41–48.2, offsets vs authored ≤2°.
Principal axes track heading mod 180 within **1.1°** at every point (h1→178.9, h359→0.7 — the h1
"−7°" registration outlier was light-gray road pixels inside the BLUFOR tolerance; axes and
crop-identity refute it). Completion C's cardinal numbers (354.6/87.1/186.0/272.6) reproduced
within noise by the same method family.

**Role symbology** (z=0 → 1 m/px AND z=−2 → 4 m/px): Medic/Rifleman/Squad-Leader/AT/MG pairwise
IoU at 1 m/px: 0.365–0.862 (all <1: distinct; weakest pairs Rifleman|AT 0.862 and Leader|MG 0.812
at 4 m/px — subtle by design, noted, not a defect). Garbage role `"Zzz Quartermaster9"` and empty
role: **IoU 1.000 with Rifleman at both zooms** (pixel-identical default), zero panics.
Cross-kind unit/vehicle/comment/marker: all pairs ≤0.652 — four distinct shapes.

**Zoom threshold** (SYMBOLOGY_MAX_M_PER_PX=8.0; z→m/px = 2^−z, calibrated by measured px
spacing): transition fires between z=−2.98 (7.89 m/px) and −3.02 (8.11 m/px) — exactly at 8.0.
Above: medic → plain disc ≡ leader disc, vehicle silhouette → disc, comment bubble → dot (ascii
masks on file). Fixed-z at the boundary: 5 consecutive frames, crop diff 0.00 — **no flicker**;
sweep −2.90→−3.10 in 0.04 steps: exactly one transition, no ABAB.

**Column alignment under mutations** (3 vehicle kinds M1025/M113/M923 at 45/135/270 + OPFOR
M1025 at 0; seed's id-sort order ≠ insertion order by construction — v-o before v-z): initial
render correct (axes 45.9/137.7/90.0; OPFOR red 187px vs 0 blue; BLUFOR blue 168 vs 0 red).
Middle-slot delete → every survivor crop delta **0.0** (all four vehicles + both slots); undo →
all seven crops 0.0. Truck drag +200 px → doc x 6600→6800 exactly, neighbours 0.0, old spot
vacated (Δ107.9); undo → doc back, crops 0.0 except the dragged truck's own selection ring
(Δ10.9, localized ring pixels). Roles stayed with positions throughout.

**Drag preview** (attack 4): vehicle mid-gesture at the cursor: registration vs pre-drag mask
rot **−1.0°, IoU 0.963** (M113 silhouette + 135° heading persist); the OTHER vehicle's crop delta
mid-drag **0.0** (the every-vehicle-drops-to-discs defect is dead). Committed vs preview at the
commit position: entity ink identical — the 7.8/31.4 crop deltas are the transient gesture chrome
(guide line + readout box), mapped pixel-by-pixel. Slot drag: medic cross + 45° facing visible
mid-gesture (ascii), selected-amber form as designed. Comment drag: doc +120 m exact; stationary
unselected bubble amber **0 px**; dragged bubble = the selected form (amber 260 = stationary
selected 260). Ring-rotate mixed selection (slot+vehicle, ctrl-click, key 3, ring from
`[data-transform-widget]`, east→south arc): doc rotations land **exactly** on bearing-to-aim
(s-1: 180 vs expected 180.0; v-1: 0 vs 0.0); pixels: vehicle axis 0.0, slot south-tick visible.

**T-802 hover**: cursor `pointer` over slot/vehicle/comment, `default` over marker and empty —
the marker exclusion is honest: marker click selects **0** (consistent with known T-838, not
re-filed). Parked hover 40 samples / 2 s: one state, no churn. Hysteresis at the TRUE boundary
(pick-miss at +6 px, HOVER_RELEASE_PX = 6): wiggle spanning the boundary inside the band → **0
transitions** (an earlier 29-transition read was my own out-of-band amplitude — retracted).
Suppressed during drag: cursor `default` mid-gesture over a pickable. Perf (within-build,
dev-profile dist): rAF 60.15 fps idle vs 58.65 sweeping (delta includes the synthetic-dispatch
harness); direct handler cost **0.025 ms/move over entities vs 0.028 ms empty** (400-move
batches) — the slice's +0.06 ms is reproduced as an upper bound, far inside the ~1.2 ms
cross-build variance. HUD `rf 13.80ms` visible after Ctrl+Alt+D.

**T-794 + the new smoke**: green run on the real dist: pass=true, exactly **24 boolean checks
all true**, every surface dx=dy=**0.0** (ctxmenu, settings, save, orbat; both passes), all
durations 0.12 s; reduced-motion pass: 0.0 travel + `overlay-fade` ×4. Registration verified in
BOTH places (EDITOR_SUITE line 45; run_smoke arm line 3762; an array-only orphan would hard-error
in editor_suite, not pass). `checks_pass` (line ~311) requires exact count AND all-true. RED
proof, my own scratch dist with the TRUE pre-fix sheet (transform keyframes restored, 200/150 ms,
PRM block deleted, integrity attrs stripped): pass=false, **16/24 red**, travels reproduce the
ticket verbatim — ctxmenu 115.2/99.5, settings 245.8/432.6, save 215.0/156.6, orbat 528.0/376.0,
durs 0.2 s, rm-pass travels identical with `dialog-in` (no PRM protection).

**Marker glyphs**: picker grid has exactly **11 rows** (canon count). Map-vs-picker mask IoU
(bbox-normalized, live pixels): attack 0.854 / defend 0.854 / flag 0.609 / destroy 0.756 /
waypoint 0.628 / medical 0.815 — six spot-checks (≥5 required) all matching, and orientation
proven for every asymmetric glyph (unrotated beats 180°-rotated: flag 0.609 vs 0.068, chevron
0.628 vs 0.204, triangles 0.854 vs 0.435). Map pairwise ≤0.546 (distinct). Atlas invariants:
suite tests re-run green (border ink 0.00 exact, non-triangle centroids ≤2 px, triangles bbox ≤1
px + intrinsic ±7.33, all 55 pairs distinct, cells 0/1 == slot atlas); agent B's precision claim
verified two ways — their measurement binary's coverage fns are **byte-identical** to shipped
scene.rs (brace-extraction diff) and output 9 glyphs at |off| ≤ 0.01 px; independently, my
FLAG_DX+3.0 perturbation red-lined at "3.01 px off" ⇒ shipped offset 0.01 px. The flag's
centroid-honest (not bbox-honest) anchoring: reasoning holds — the anchor is the point the marker
marks; live pixels show the flag seated on it and matching the picker.

**Hollow-pin sweep** (6 perturbations, 4 files + the CSS class, each restored byte-exact +
touched, verbatim reds on file):
1. engine.rs — deleted the `symbology_base = None` clear → `ensure_slot_atlas_widens…` RED.
2. engine.rs — prefix-breaking rename of `vehicles_bind_symbology` → extractor panics
   `engine.rs has no …` RED (missing-sig refusal REAL).
3. engine.rs — sig planted in a comment → `not unique … would pin the wrong body` RED
   (ambiguity refusal REAL). (2b: suffix rename stays green — F2.)
4. slots_gpu.rs — wrap reverted to clamp → `every_heading_of_the_compass_gets_its_own_facing`
   RED. (`yaw_encoders_agree` alone cannot catch this BY CONSTRUCTION — it composes the same wrap
   into its expectation; the pairing is sound, noted.)
5. mission_history.rs — unplaced-vehicle skip moved after the first column write (a true
   misalignment) → `the_vehicle_lane_columns_come_from_one_sorted_reader` RED with the
   skip-must-precede-every-write message.
6. scene.rs — FLAG_DX +3.0 → `marker_glyphs_are_centred_and_unclipped` RED at 3.01 px.
Plus the stylesheet class: the full pre-fix aegis sheet → smoke RED 16/24 (above).

**Suites** (private target dirs): website-frontend **1186/0** (wave's +19 over 1167) ×2 runs
(before probes and after all restores) · map-engine-core **673/0 --all-features** ×2 ·
map-engine-render **77/0** ×2. Register smokes re-run: save-dialog-rect PASS · select PASS ·
entrance-motion-rect PASS · cur FAIL (F1) · undo FAIL (F1).

**Register spots run**: comment drag + precedence (probe D), rotate-ring boundary + landing
(D4), marker lane feed (suite: t780/marker atlas tests green), Save-dialog rect (gate smoke),
keep-multi/armed-pointerup/zone-Esc/export-latch/validation-chip/draft-chip/Type-picker/Esc-ladder
(suite-pinned in the 1186 green; keep-multi and picker also exercised incidentally by probes D/G).
Not re-run individually beyond that — stated, not claimed.

## Safe-line

**Yes — main is safe to build the next wave on.** Every shipped behavior this wave claims was
re-measured from pixels and holds: the compass wrap is fixed on both symbology lanes (no plateau,
no wrap cliff), roles/kinds/sides/headings stay aligned through mutations and drags, the hover
cursor tells the truth cheaply, entrances no longer move the furniture, and map markers finally
match their picker. The MAJOR (F1) is gate-coverage debt — two stale pre-wave smokes that mask
the suite's tail and put the "30/30" ritual's meaning in question — it needs a smoke-maintenance
ticket and an honest full-suite run, but nothing in it makes this tree unsafe to build on.

## Attacked and FAILED to break

Slot heading sweep (9 facings, monotonic, both wrap seams) · vehicle heading sweep (registration
+ principal axes) · offset-constancy (resolved as raster aliasing, encoder linear) · role glyph
distinctness at two zooms · unknown/empty-role default (pixel-identical, no panic) · cross-kind
shape distinctness · the 8.0 m/px boundary from both sides, at rest, and through a slow sweep ·
vehicle-lane column alignment through slot-delete / undo / drag / drag-undo with id-order ≠
map-order in force · mid-drag vehicle symbology (dragged AND bystanders) · committed-vs-preview
pixel identity · slot drag facing/role persistence · comment drag + amber semantics · ring-rotate
mixed-selection landing (doc + pixels) · hover cursor per kind · marker-exclusion honesty ·
parked-hover churn · boundary hysteresis · drag suppression · hover cost (three methods) ·
entrance smoke double registration · 24-check all-must-be-true arithmetic · green-on-real-dist ·
red-on-true-pre-fix-dist (stronger than the completion's own evidence) · 11-glyph canon count ·
6 map-vs-picker shape+orientation matches · atlas border/centroid/55-pair invariants + agent B's
0.01 px numbers (verified byte-identical code + independent perturbation) · extractor
missing-sig and ambiguous-sig refusals · 6 pin perturbations (verbatim red, byte-exact restore) ·
scope purity of the smokes.rs diff · the full three suites twice each.

## Environment left as found

HEAD c6b875de untouched; `git status` clean (this report the only addition). All perturbed files
byte-identical to HEAD and touched. VERIFY207 missions: zero remaining (each probe deleted its
own; final list re-checked after re-login). The pre-existing `VERIFY205 shape` mission was there
before me and was left alone. Chromium: 0 processes; my `profile-206-v207*` dirs removed; the
completion agents' scratchpad artifacts (t808*, smoke logs, before.css, glyphs_* binaries)
preserved as evidence. Gate binary rebuilt in the shared target (a build product, as the factory
uses it). Scratch dists (`prefix_dist`) live in the scratchpad only. No DB writes beyond the
deleted probe missions; no packages installed.
