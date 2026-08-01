# `__editorCamSet` panics the render engine — and 8 gate smokes drive the editor with it

**Found 2026-08-01** while trying to photograph the map at several zooms to re-scope T-641.
**Not confirmed on a real browser.** Read the "what this needs" section before acting.

## What happens

Boot the editor headless on the real GPU (`GPU_MODE=vulkan`, the path that renders correctly and
produced a 3.7 MB canvas earlier in the session). The engine boots, the overlay clears in 7 s, the
satellite basemap comes up. Then call the documented debug API:

```js
window.__editorCamSet(6400, 6400, -2)
```

and the renderer dies:

```
panicked at wgpu-29.0.4/src/backend/webgpu.rs:2697:14
```

Every subsequent call then panics at `apps/website/frontend/src/mission_editor.rs:2360:37` — the
`engine.borrow_mut()` inside `cam_set` — because the first panic left the `RefCell` poisoned.
`window.__editorCam()` returns `undefined` from that point on, and every canvas read afterwards
returns a **44,075-byte black rectangle** instead of the ~3.7 MB of real map.

Reproduced across two runs, six zooms and four zooms, all inside and outside the
`[HEIGHT_LABEL_MIN_ZOOM, HEIGHT_LABEL_MAX_ZOOM]` band, so it is not a zoom-range guard.

Repro: `tools/editor-capture/zoomsweep.mjs`.

## Why it matters more than the thing I was looking for

`__editorCamSet` is not only a debug convenience. It is **how the gate harness drives the editor
camera** — 8 call sites in `tools/tbd-tools/src/smokes.rs`:

| Line | Use |
|---|---|
| `:697`, `:722`, `:732` | `typeof window.__editorCamSet === 'function' && (window.__editorCamSet(6400, 6400, …))` |
| `:3186` | `window.__editorCamSet(6400, 6400, 0.5); await sleep(1500);` |
| `:3217` | pan probe, `6400 + off` |
| `:3231` | camera sequence `c[0], c[1], c[2]` |
| `:3262` | `window.__editorCamSet(6400, 6400, 0.5)` |

Its own doc comment (`mission_editor.rs:2328`) says it exists **"so `smoke_fullmap` can Class-R
probe tree glyphs at zoom ≥ 0 without relying on CDP `mouseWheel` → DOM `wheel` delivery"** — i.e.
it was added *because* wheel delivery was unreliable, and is now the only camera driver those
smokes have.

**If this panic reproduces in the gate's own environment, those smokes assert against a dead
engine.** The guard `typeof … === 'function'` checks that the function *exists*, then calls it and
keeps going — it cannot distinguish "camera moved" from "camera moved and the renderer died". That
is the shape `EDITOR_UI_HANDOFF.md` names as this codebase's recurring defect: *a tool reports
success over an input it never actually examined.* It would be the third independent instance found
in this program, after FNF's `AnalyzeSQM.ps1` (14 of 27 checks running) and the T-216 ledger
(`make verify-t180` green while six authored values were dropped).

## What this needs — a 30-second check, and it is not mine to make

**This may be a headless artifact.** Headless chrome already failed once in this session in a way
that looked exactly like a broken engine and was not: `Failed to initialize vulkan surface` made
`Page.captureScreenshot` return a black map over correct DOM chrome. The correct posture is that
**this is unverified on a real browser**, and no ticket should be filed against the gate until
someone checks.

The check, in the operator's own Chrome with the editor open:

```js
window.__editorCamSet(6400, 6400, 0)
```

- **Map keeps rendering and moves** → headless-only artifact. Note it in the capture harness
  README, use wheel events for headless zoom, and stop there.
- **Map goes black / console panics** → real bug, and the 8 smoke call sites are driving a corpse.
  That is a ticket, and it invalidates whatever those smokes have been asserting.

## Consequence for T-641

The re-scope of T-641 (height labels) is **not done.** The plan was to photograph the map at six
zooms and read the label density; the zoom driver crashes the thing being photographed.

What is established without the photographs, from source:

- `HEIGHT_LABEL_MIN_ZOOM = -2.0`, `HEIGHT_LABEL_MAX_ZOOM = 3.0` — outside that band
  `declutter_height_labels` returns empty (`crates/map-engine-core/src/dem/peaks.rs:184-187`)
- `PEAK_LABEL_MAX = 48` — a hard global cap (`peaks.rs:11`)
- `height_label_min_sep_m(z) = 80.0 * 2^(-z)` (`peaks.rs:45-46`) — separation in **metres**, halving
  per zoom step, which is **screen-space-constant** if metres-per-pixel also halves per step. That
  is the same mechanism Eden uses, so the design is right in principle.
- Declutter is importance-first: sort by `value_m` desc, keep if `dist ≥ sep` to all kept

`INFERRED:` the suspicious part is the interaction of the **48 cap** with the band edge. The
editor's default camera sits at **z = −2.00**, exactly `HEIGHT_LABEL_MIN_ZOOM` — one step further
out and labels vanish entirely. Whether that is what the operator is seeing is exactly what the
photographs were meant to settle, and it remains unsettled.

**T-641 should not be filed as either "greenfield" or "zoom-band defect" until the map is looked
at.** Both are now guesses.
