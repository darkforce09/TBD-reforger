# T-151 operator sign-off — consolidated visual/perceptual gates (one session)

**Purpose:** close audit findings **C-ALL-01** (zero operator S-gates ever recorded closed across
W0–W9) and **C-8-01** (T-151.8 S4 band table never filled) in ONE session against the current
build. Every row below is a gate some slice promised and no log ever recorded. Automated + GPU-R
evidence already covers everything machine-checkable (see `t151_11_verify_log.md`) — this list is
only what needs eyes.

**Setup:** `make db-up && make api` (separate terminal) · `make web` · open a mission with slots
on Everon · DEV build (debug HUD visible). Fill PASS / FAIL + notes; one signature line at the end
closes the program's perceptual debt. FAIL rows → file follow-up tickets, do not soft-pass.

## A — Basemap & terrain (T-151.1 S1–S3, T-151.9 S1)

| # | Check | PASS/FAIL | Notes |
|---|-------|-----------|-------|
| A1 | Open editor: coarse satellite **preview appears within ~1 s**, full-res satellite replaces it when the 153 MB stream finishes (T-151.11.4 P-03 — new behavior) | | |
| A2 | Satellite style: unified texture pans/zooms with no tile churn or seams | | |
| A3 | Map style: pyramid tiles render right-side-up; paper tint underlay | | |
| A4 | Hillshade toggle + opacity slider 0→40→100 %: relief blends, 0 % = gone | | |
| A5 | Grid toggle on/off; grid brighter over hillshade; **grid lines sit UNDER slot rings/clusters** (T-151.11.1 P-01 fix) | | |

## B — World lanes (T-151.3 S1, T-151.4 S1, T-151.4.1, T-151.5 S1–S3, T-151.5.1 S1–S3)

| # | Check | PASS/FAIL | Notes |
|---|-------|-----------|-------|
| B1 | Buildings: dark fills + near-black outlines at zoom ≥ −2.5; towns stay populated while panning (no wipe) | | |
| B2 | Roads: casing + per-class colors; dirt/track dashed; runway white | | |
| B3 | Sea band + contours legible; contour density steps with zoom | | |
| B4 | Forest: mass at low zoom → tree glyphs at zoom ≥ 0, no bloated fill over fields | | |
| B5 | **Layer toggles**: trees / props / **buildings — toggling buildings now hides footprints+outlines+badges together** (T-151.11.3 P-04) and takes effect immediately | | |
| B6 | WebGPU browser (chrome://gpu shows WebGPU): tree glyphs render UNDER slot rings/grid/marquee (T-151.11.1 X-01 fix — not verifiable headless) | | |

## C — Mission lanes & interaction (T-151.6 S1–S5, T-151.7 S1–S5, 7.1–7.3 S-rows, T-151.9 S2)

| # | Check | PASS/FAIL | Notes |
|---|-------|-----------|-------|
| C1 | Slot rings visible; click select / Ctrl-toggle / empty-click clear | | |
| C2 | **Marquee: translucent fill + visible 1 px border** (T-151.11.1 P-02 restore) | | |
| C3 | Drag ~1000 slots: smooth overlay, commit lands, undo restores | | |
| C4 | Zoom-at-cursor stable under RMB-hold + wheel | | |
| C5 | ≥ 500 slots + zoom ≤ −4: cluster discs; click/dbl-click drills in | | |
| C6 | Dbl-click slot → Attributes; Space centers selection; Ctrl+C/V at cursor; Delete | | |
| C7 | Save Version → 201; Export downloads (flip regression, T-151.9 S2) | | |
| C8 | Arland mission: camera cannot pan beyond the 4,096² world (T-151.11.2 X-02 fix) | | |

## D — T-151.8 S4 band table (fps + gpu_frame_ms per LOD band; DEV HUD numbers, not eyeball)

| Band (deck zoom) | fps | gpu_frame_ms | notes |
|------------------|-----|--------------|-------|
| −6 (whole island, heatmap rung) | | | |
| −4 (cluster band) | | | |
| −2 (default) | | | |
| 0 (tree glyphs on) | | | |
| +3 (props on) | | | |
| +6 (max) | | | |

## E — Prod build spot-check (`npm run build` + preview server)

| # | Check | PASS/FAIL | Notes |
|---|-------|-----------|-------|
| E1 | No debug HUD panel in the editor (error banner still appears if init fails) (P-05) | | |
| E2 | `/_spike/wgpu` unreachable (A-10) | | |
| E3 | Idle editor: GPU/utilization drops when untouched (damage-driven render, P-06) | | |

## F — Real-hardware GPU proof (WebGPU browser)

Run `BASE_URL=http://localhost:5173 node scripts/website/verify-wgpu-gpu.mjs` against a dev
server opened in a WebGPU-capable environment, or open `/_spike/wgpu` and run
`await window.__selfChecks.computeCull()` in the console — paste the `{cpu, gpu, pass}` JSON:

```
(paste here)
```

---

**Sign-off:** date ________ · operator ________ · overall PASS/FAIL ________

Filled = C-ALL-01 + C-8-01 close in the tracker (`t151_10_fable_audit_report.md`); FAIL rows
become tickets. Future slices: record S-gate closures in the slice verify log at ship time —
this backlog session exists because none ever were.
