**Status:** live

# Template: known bug

**When to use:** a defect that is recorded rather than fixed now, because it lies outside the
realistic envelope, is minor, waits behind other work or comes from the environment; and a fixed
defect whose analysis is worth keeping. Each entry is one file in `documentation_v2/known_bugs/`,
named `kb_<NNN>_<subject>.md` with the next free number, and gets a row in that folder's README. A
resolved entry stays in the folder with its status set to resolved. The
[README standard](/documentation_v2/standards/readme_standard.md) holds the writing rules an entry
shares with other documents.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. The sections
come in this order, spelled this way.

````markdown
**Status:** live

# KB-<NNN> — <the defect, as the user or operator meets it>

## Status

<Open, deferred or resolved, and why; the severity and the area. A resolved entry names what
fixed it.>

## Symptom

<What is seen, with error text and log lines quoted exactly, and the conditions that bring it on.>

## Cause

<Why it happens, as the code or the environment shows it, with the evidence that proves it and
what was ruled out.>

## Workaround

<What to do until it is fixed, or "None." when nothing is needed.>

## Fix

<The fix that shipped, naming the code it changed; or the fix a future change would make.>

## Related tickets

- [<ticket id> — <ticket title>](<link to its spec, or to its .ai/tickets file when it has none>)
  (<status>): <what it did or will do>
````

## Worked sample

Written from `documentation_v2/known_bugs/kb_002_editor_gate_boot_wedge.md`, the editor gate
runbook's wedge modes and the gate harness in `tools_v2/developer-tools/src/browser_testing/`,
checked against the code as it stands. The sample sits in a fenced block, so no gate reads its
links.

````markdown
**Status:** live

# KB-002 — The editor gate wedges at boot on a font-fallback crash

## Status

Resolved. Severity high while it lasted: no editor gate could run, `cargo xtask mk leptos-gates`
included. Area: the browser gate harness in `tools_v2/developer-tools/src/browser_testing/` and the
Chromium it launches.

## Symptom

`cargo xtask mk leptos-gates`, or one smoke on its own, hangs and fails after 130 s with:

```text
gate: driver error: cdp: ws call timed out (Runtime.evaluate)
```

The suite stops at its first smoke, so the run fails once, after the full timeout, with no
diagnosis. When the browser process aborts instead, the crash comes a few hundred milliseconds
after the Mission Creator page is navigated, and the harness reports the same timeout or
`timeout waiting for Page.loadEventFired`.

## Cause

Chromium's Skia font manager aborts on any per-character font fallback
(`SkFontMgr_FontConfigInterface.cpp:163`, "Not implemented"), and the Mission Creator's text needs
fallback glyphs. In `chrome-headless-shell` the abort kills the renderer at boot; the harness sees
only a dead DevTools socket, so the `Runtime.evaluate` call waits out its 130 s timeout. The full
`chrome` build reaches the same abort from its browser process when it resolves no font at all,
which happens when a container sharing the home directory has written its own
`~/.cache/fontconfig`. Chromium's own stderr shows the abort; the app code, the build profile and
memory pressure were ruled out.

## Workaround

None needed. If the wedge returns, `cargo xtask mk gate-doctor` reports which Chromium build
resolved and whether the gate's font cache is in place.

## Fix

- `find_chromium` (`tools_v2/developer-tools/src/browser_testing/cdp/sleep_ms.rs`) prefers the
  full `chrome` build (`chrome-linux64/chrome`) over `chrome-headless-shell`, and
  `launch_with_gpu`, behind `launch`, passes it `--headless=new`.
- The same launch points the Chromium child's `XDG_CACHE_HOME` at a font cache the gate owns, so a
  cache written by another distribution is never read.
- `gate doctor`, which `cargo xtask mk leptos-gates` runs first, checks the resolved build and runs
  a short liveness probe, so a recurrence fails in seconds; its pins live in
  `tools_v2/developer-tools/gate-env.json`.

## Related tickets

- [T-177 — MC chrome UX + ORBAT dock cutover](/documentation_v2/tickets/specs/t177_mc_chrome_orbat_cutover.md)
  (shipped): the full Chromium build and `--headless=new`.
- [T-320 — Gate harness wedges on editor — CDP unverifiable](/.ai/tickets/T-320.toml) (shipped):
  the gate-owned font cache.
````
