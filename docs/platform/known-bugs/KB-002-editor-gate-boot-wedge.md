# KB-002 — Editor CDP gate wedges at boot (chrome-headless-shell font-fallback crash)

| | |
|---|---|
| **Status** | **RESOLVED** — T-177 (harness now uses the full `chrome` build + `--headless=new`) |
| **Severity** | High (while active) — the entire editor acceptance gate (`make leptos-gates`) could not run |
| **Area** | Gate harness (`tools/tbd-tools`) + headless chromium / Skia fontconfig |
| **Discovered** | 2026-07-19, during T-177 verification |

## Symptom

`make leptos-gates` (or `gate smoke <name>`) hangs, then fails after ~130 s with:

```
gate: driver error: cdp: ws call timed out (Runtime.evaluate)
```

The suite fails-closed on the **first** smoke (`selfcheck`, whose first `Runtime.evaluate` is
`!!document.querySelector('canvas')`), so the cost is ~130 s once, not ×18 — but with no useful signal.

## Root cause

The harness resolved playwright's **`chrome-headless-shell`**, whose stripped Skia font manager stubs
per-character font fallback as a hard abort:

```
[FATAL:third_party/skia/src/ports/SkFontMgr_FontConfigInterface.cpp:163] Not implemented.
```

The editor chrome renders text that needs fallback (icons / em-dash / emoji ranges), so the **renderer
core-dumps at boot**. Over CDP the harness only sees a dead WebSocket → `Runtime.evaluate` never
answers → the 130 s per-call timeout. It is **environment-dependent** (it fires only when this box's
fontconfig forces a fallback the shell can't satisfy), which is why it "worked before": the same
`chrome-headless-shell` build (149.0.7827.55) was the last-known-good, and only a system/font change
tipped it over.

**Ruled out** (all verified): the T-177/app code (the clean T-176 dist wedged identically), the wasm
build profile (debug wedged too), the chromium version (1223 & 1228 both), multi-GPU Vulkan
(lavapipe-only still crashed), memory/cgroup pressure (20 GB free, `memory.max = max`), orphaned
processes. Basic CDP + WebGL2 both worked. The decisive evidence was chromium's own stderr
(`--enable-logging=stderr --v=1`) showing the Skia FATAL.

## Fix (T-177)

- **`cdp.rs` `find_chromium`** now prefers the **full `chrome` build** (`chrome-linux64/chrome`) over
  `chrome-headless-shell` — the full build has the complete font backend and does not crash. (Also
  fixed a latent path bug: playwright's full-chrome dir is `chrome-linux64`, not `chrome-linux`.)
- **`cdp.rs` `launch`** adds **`--headless=new`** for the full build (the shell is always headless and
  ignores it).
- **Fail-fast:** `gate doctor` (a prerequisite of `make leptos-gates`) validates the resolved chromium
  and runs a ~15 s liveness probe, so a future recurrence fails in seconds with a diagnosis instead of
  the 130 s hang. Pins live in [`tools/tbd-tools/gate-env.json`](../../../tools/tbd-tools/gate-env.json).

Two stale/behavioral smoke assertions were exposed once the suite could finally run past `selfcheck`,
both fixed in the same pass (neither was the wedge):
- **fullmap** asserted `landcover_polygons === 36`; **T-176** intentionally removed the 32 m landcover
  wash → now `=== 0`.
- **keyboard-settings** Ctrl+C/V are natively intercepted by the full chrome's clipboard handling →
  driven via JS `KeyboardEvent` instead of `Input.dispatchKeyEvent`.

## If it recurs

See [`docs/website/EDITOR_GATE_RUNBOOK.md`](../../website/EDITOR_GATE_RUNBOOK.md) §Known wedge modes +
the P0–P6 debug recipe (chrome stderr, `/proc` thread state, `gdb -p <renderer>`, flag levers).
