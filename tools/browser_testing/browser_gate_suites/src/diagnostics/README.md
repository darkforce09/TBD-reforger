# Gate doctor and font cache

The two halves of `gate doctor`, the preflight declared in
`tools/browser_testing/browser_gate_suites/src/diagnostics.rs`: the gate-owned font cache every
browser launch runs against, and the checks that fail fast with a diagnosis instead of letting an
editor smoke hang for 130 s.

## Contents

```text
tools/browser_testing/browser_gate_suites/src/diagnostics/
├── check_fonts.rs             the font probe, the dist check, the stray-Chromium count and liveness
└── ensure_gate_font_cache.rs  the gate font cache, the doctor's `run`, and the pin and memory checks
```

## How it works

`ensure_gate_font_cache` runs once, in the `gate` binary's single-threaded prologue, and sets
`XDG_CACHE_HOME` to the folder `cdp::gate_font_cache_dir` decides: `$TMPDIR/tbd-gate-cache-<distro>`,
keyed on `ID` and `VERSION_ID` of `/etc/os-release`, or `TBD_GATE_FONT_CACHE` when that is set. It
ignores an inherited `XDG_CACHE_HOME`: a fontconfig cache written by a container that shares the
home directory leaves Chromium with zero fonts, and the first per-character font fallback then
aborts the browser. `chrome_devtools_protocol::launch_with_gpu` hands the same folder to every browser it spawns.

`run` (`gate doctor [--dist <dir>] [--strict]`) prints one line per check:

| Check | Reads | Outcome |
|---|---|---|
| chromium | `chrome_devtools_protocol::find_chromium`, `--version`, `gate-env.json` `chromium.version` | not found, the headless shell or version drift warns |
| rustc, trunk | `--version` against `gate-env.json` `toolchain` | drift warns |
| memory | `MemAvailable` in `/proc/meminfo` against `limits.min_mem_available_mib` (1024) | below the floor warns |
| processes | `/proc/*/comm` named `chrome` or `chrome-headless` | any stray warns |
| fonts | Chromium's log for `Could not find any font`, watched for 5 s | the marker fails; a probe that could not run warns |
| dist | `<dist>/index.html`, `crates/frontend/shell/frontend_application/dist` by default | missing warns |
| liveness | the Mission Creator at `/missions/smoke/edit?force=webgl&sat=preview` | not ready, or the browser died, fails |

The pins live in `tools/browser_testing/browser_gate_suites/gate-env.json`. The liveness probe serves the dist on
port 5299 with the map assets and no API proxy, seeds an admin session,
launches Chromium on debug port 9399, evaluates `1+1` with an 8 s timeout, then polls once a second
for a canvas and `window.__editorCam`, within `limits.liveness_timeout_secs` (15) plus a 12 s hard
cap. Before reporting it asks the browser's own `/json/version` whether it is still alive, which
separates a crashed browser from a slow page. A zero-font result stops the doctor before the
liveness probe runs. Only the fonts marker and liveness fail on their own: the doctor exits 0 when
liveness passes, 1 on either failure, and 1 with `--strict` when any warning was counted.

## Boundaries

- Depends on: the parent's `FontProbe` and `Liveness` types and constants;
  `tools/browser_testing/chrome_devtools_protocol/src/browser_session.rs` (with the font cache decision,
  `CacheOrigin` among it) and `server.rs`; `admin_session_seed_script` from
  `tools/browser_testing/browser_gate_suites/src/session_tokens.rs`; `crate::gate_layout` and the
  `repository_layout` crate; the pins in `tools/browser_testing/browser_gate_suites/gate-env.json`.
- Used by: `diagnostics.rs`, which re-exports `ensure_gate_font_cache` and `run`; the `gate`
  command line in `tools/browser_testing/browser_gate_suites/src/command_lines/gate.rs`; `cargo xtask mk gate-doctor` and `cargo xtask mk leptos-gates`, and
  `.github/workflows/editor-gates.yml` through them.
- Rules: the font probe inherits `XDG_CACHE_HOME` rather than setting its own, so it measures the
  cache the smokes get; it never waits for Chromium to exit or reads its pipes to EOF; a liveness
  failure always exits 1, so `cargo xtask mk leptos-gates` stops before the suite.

## Related documentation

- [Editor gates](/documentation/runbooks/editor_gates.md) — running the doctor and reading its
  wedge diagnoses.
