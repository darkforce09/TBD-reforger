**Status:** live

# KB-002 — The editor gate wedges at boot on a font-fallback crash

## Status

Resolved: fixed in the gate harness, which launches the full Chromium build against a font cache
it owns and checks both before any smoke runs. Severity high while it lasted: no editor gate could
run, `cargo xtask mk leptos-gates` included. Area: the browser gate harness in
`tools_v2/developer-tools/src/browser_testing/` and the Chromium it launches.

## Symptom

`cargo xtask mk leptos-gates`, or one smoke on its own (`gate smoke <name>`), hangs and fails after
130 s with:

```text
gate: driver error: cdp: ws call timed out (Runtime.evaluate)
```

The suite stops at its first smoke, `selfcheck`, whose first evaluation looks for the canvas, so
the run fails once, after the full timeout, with no diagnosis. When the browser process aborts
instead, the crash comes a few hundred milliseconds after the
[Mission Creator](/documentation_v2/glossary.md#mission-creator) page is navigated, and the harness
reports the same timeout or `timeout waiting for Page.loadEventFired`.

## Cause

Chromium's Skia font manager aborts on any per-character font fallback:

```text
[FATAL:third_party/skia/src/ports/SkFontMgr_FontConfigInterface.cpp:163] Not implemented.
```

The Mission Creator's text needs fallback glyphs (icons, dashes, symbol ranges). In
`chrome-headless-shell` the abort kills the renderer at boot; the harness sees only a dead
DevTools socket, so the `Runtime.evaluate` call waits out its 130 s timeout (`send` in
`tools_v2/developer-tools/src/browser_testing/cdp.rs`). It depends on the machine's fonts: the same
shell build ran clean until a font change made the fallback necessary. The full `chrome` build
reaches the same abort from its browser process when it resolves no font at all, which happens
when a container sharing the home directory has written its own `~/.cache/fontconfig`.

Chromium's own stderr (`--enable-logging=stderr --v=1`) showed the abort. Ruled out: the
application code (an older build wedged the same way), the build profile (a debug build wedged
too), the Chromium version (two builds), multi-GPU Vulkan (a software-only device still crashed),
memory pressure (20 GB free, no cgroup limit) and orphaned processes. A bare page with WebGL2 ran
fine over the same connection.

## Workaround

None needed. If the wedge returns, `cargo xtask mk gate-doctor` reports which Chromium build
resolved and whether the gate's font cache is in place.

## Fix

- `find_chromium` (`tools_v2/developer-tools/src/browser_testing/cdp/sleep_ms.rs`) prefers the full
  `chrome` build (`chrome-linux64/chrome`) over `chrome-headless-shell`, and `launch_with_gpu`,
  behind `launch`, passes it `--headless=new`; the shell is used only when no full build exists.
- The same launch sets the Chromium child's `XDG_CACHE_HOME` to the font cache the gate owns
  (`gate_font_cache_dir` in `tools_v2/developer-tools/src/browser_testing/diagnostics.rs`), so a
  cache written by another distribution is never read.
- `gate doctor`, which `cargo xtask mk leptos-gates` runs first, checks the resolved build, probes
  Chromium's log for `Could not find any font` and runs a liveness probe of about 15 s, so a
  recurrence fails in seconds with a diagnosis. Its pins live in
  [`tools_v2/developer-tools/gate-env.json`](/tools_v2/developer-tools/gate-env.json).

The [editor gates runbook](/documentation_v2/runbooks/editor_gates.md#known-wedge-modes) lists
both wedge modes and the debug recipe for a recurrence, and the
[editor capture runbook](/documentation_v2/runbooks/editor_capture.md) the environment rules for
headless screenshots.

## Related tickets

- [T-177 — MC chrome UX + ORBAT dock cutover](/documentation_v2/tickets/specs/t177_mc_chrome_orbat_cutover.md)
  (shipped): the full Chromium build, `--headless=new` and the gate doctor's liveness probe.
- [T-320 — Gate harness wedges on editor — CDP unverifiable](/.ai/tickets/T-320.toml) (shipped):
  the gate-owned font cache for the browser-process abort.
- [T-653 — Preserve the three headless editor-screenshot findings](/documentation_v2/tickets/specs/t653_headless_screenshot.md)
  (ready, [plan](/documentation_v2/tickets/plans/t-653_plan.md)): the writable font cache, the
  Vulkan-only ANGLE flag and the canvas capture, recorded in the runbooks with a cross-link here.
