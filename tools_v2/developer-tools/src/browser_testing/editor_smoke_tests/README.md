# Mission Creator smoke tests

The headless browser smokes of the [Mission Creator](/documentation_v2/glossary.md#mission-creator),
declared in `tools_v2/developer-tools/src/browser_testing/editor_smoke_tests.rs`, and the auxiliary
gates that share their harness: the session-refresh gate, the generic render check and the
performance probe.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/editor_smoke_tests/
├── arsenal.rs                    `arsenal`: the loadout tab against the committed registry fixture
├── browser_assertion_helpers.rs  panic capture, typed `evaluate` readers, verdicts and exit codes
├── browser_fixture_helpers.rs    the editor auth seed and the registry, arsenal and faction taps
├── browser_input_helpers.rs      trusted mouse, drag, click, double-click and key-chord input
├── cur.rs                        `cur`, `attributes` and `keyboard-settings`
├── editor_boot_scenarios.rs      `editor` and `selfcheck`: the canvas boots, the GPU self-checks pass
├── fullmap.rs                    `fullmap`, `hillshade` and `doc`: map assets and the mission document
├── marquee_drag.rs               `marquee-drag` and `undo`
├── mutations.rs                  `mutations`, `perf`, `r-auth`, `render-check`, the `--assert-js` verdict
├── outliner_drag/                the parts of `outliner-drag`: missions, helpers and case groups
├── outliner_drag.rs              the `outliner-drag` module tree and its fixture mission ids
├── outliner_palette.rs           `outliner-palette`: the left and right docks
├── pan.rs                        `pan`, `persist`, `select` and `save-export`
├── run_smoke.rs                  `run_smoke`, the name dispatcher, and `editor_suite`
├── save_dialog_rect.rs           `save-dialog-rect` and `entrance-motion-rect`
└── virtual_outliner.rs           `virtual-outliner` and `hydrate`
```

## How it works

Every smoke builds a `Harness` (in `editor_smoke_tests.rs`): a static server on the dist, a
SwiftShader Chromium from `cdp::launch` on its own port pair, the admin `tbd-auth` seed on every
new document, a 1440×900 viewport, and a panic tap that records any console, log or exception line
matching `panic`, `unreachable` or `already mapped`. It opens the editor at `EDIT_PATH`,
`/missions/smoke/edit?force=webgl&sat=preview`: WebGL2 because the WebGPU path stalls the page
under software rendering, and the satellite preview because the full 152 MB bundle freezes headless
Chromium. Input goes through trusted DevTools events (a held button always carries its `buttons`
mask; a key chord is `rawKeyDown` plus `keyUp` only), and assertions read the editor's window hooks
(`__editorCam`, `__editorSelection`, `__editorHistory`, `__missionDoc`, `__missionPersist`,
`__selfChecks`). Each smoke prints a JSON verdict of named checks, most of them including a
no-panic check, and shuts Chromium's process group down before the next one starts.

`gate editor-suite` runs `EDITOR_SUITE` in order and stops at the first non-zero exit:

| Smoke | What it asserts |
|---|---|
| `selfcheck` | every `window.__selfChecks` GPU readback matches byte for byte; runs first to prove the harness |
| `arsenal` | the [Arsenal](/documentation_v2/glossary.md#arsenal) loadout tab against the committed registry fixture |
| `attributes` | the Attributes modal opens, edits and undoes |
| `cur` | the toolbelt coordinate read-out matches the camera arithmetic |
| `doc` | the hosted [mission](/documentation_v2/glossary.md#mission) document is live, seeded and round-trips |
| `editor` | the canvas mounts, the engine renders and a wheel zoom changes the view |
| `entrance-motion-rect` | dialog and menu entrance animations move the surface under 8 px, finish within 0.15 s, and not at all under reduced motion |
| `fullmap` | the map-asset host wiring with the full terrain assets |
| `hillshade` | the elevation raster is fetched, decoded and uploaded |
| `hydrate` | a mission saved through the live [API](/documentation_v2/glossary.md#api) hydrates back intact |
| `keyboard-settings` | delete and undo, copy and paste, and Mission Settings |
| `marquee-drag` | marquee selection and drag-move |
| `outliner-drag` | outliner, [ORBAT](/documentation_v2/glossary.md#orbat) and height drags over controlled missions |
| `outliner-palette` | the dock layout and palette placement |
| `pan` | right-button pan and a wheel zoom mid-pan |
| `persist` | the mission survives a reload through IndexedDB |
| `save-dialog-rect` | the Save Version input sits inside the viewport at 1920×1080 and 1366×768, with focus and Tab trapped |
| `save-export` | the save and export compile bridges produce schema-valid payloads |
| `select` | left-click picking and toggling |
| `undo` | undo boundaries between drags, the redo button and the keydown guard |
| `virtual-outliner` | the outliner and ORBAT trees render only a window of rows above the threshold |

Outside the suite, `gate smoke` also runs `mutations` (the live API with `TOKEN` and `REFRESH` set),
`perf` and `perf-strict` (frame, upload and chunk-fetch metrics on the full map assets; strict
fails on duplicate or idle fetches, counter churn during a pan, or a bench below 60 fps). `r-auth`
answers a seeded session's first `/api/v1/me` with 401 and passes when the app refreshes exactly
once and loads the signed-in user. `render-check` loads any path of a dist and passes when the body
renders, `--expect` is in its text and `--assert-js` returns `true` or an object with
`"pass": true`; any other value fails.

Exit codes: 0 pass, 1 a failed check, 2 a missing prerequisite (`hydrate` and `mutations` without
an API on `127.0.0.1:8080`, `mutations` without its tokens, `r-auth` without a dist). An unknown
smoke name and a driver fault surface as errors, which the `gate` binary exits 3 on.

## Public surface

- `EDITOR_SUITE`, `run_smoke` and `editor_suite`, run by `gate smoke` and `gate editor-suite`.
- `r_auth`, `render_check` and `RenderCheckArgs`, run by `gate r-auth` and `gate render-check`.
- One `smoke_*` function per smoke, re-exported for the dispatcher.

## Boundaries

- Depends on: `tools_v2/developer-tools/src/browser_testing/cdp.rs`, `server.rs`,
  `session_tokens.rs`, `fixture_injection.rs` (`FREEZE_SRC` for `render-check`) and
  `dom_oracle/` (`seed_script`, `js_len`); `crate::repository_layout` for the map assets; the
  fixture corpus in `apps/website/frontend/tests/fixtures/api/`; the Mission Creator's window hooks
  in `apps/website/frontend/src/v2/apps/editor/`; a live API for `hydrate` and `mutations`.
- Used by: the `gate` command line in `tools_v2/developer-tools/src/browser_testing/cli.rs`;
  `cargo xtask mk leptos-gates`, which runs `gate editor-suite`, and
  `.github/workflows/editor-gates.yml` through it.
- Rules:
  - `selfcheck` stays first in `EDITOR_SUITE`, and a new smoke joins both `EDITOR_SUITE` and
    `run_smoke`;
  - `--assert-js` never passes on truthiness (`pass_false_object_fails`,
    `diagnostic_string_is_echoable_not_pass` and the rest of
    `tools_v2/developer-tools/src/browser_testing/tests/editor_smoke_tests/assert_js_ok_tests.rs`);
  - smokes run one at a time on fixed server and debug ports, and each reaps Chromium's process
    group before returning.

## Related documentation

- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the suite, its environment
  and the wedge modes.
- [Mission Creator](/apps/website/frontend/src/v2/apps/editor/README.md) — the app whose window
  hooks the smokes read.
