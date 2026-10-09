**Status:** live

# Editor gates

Runs the headless browser gates of the single-page app: the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) smokes (`gate editor-suite`) and the
`gate doctor` preflight that runs before them, and shows how to diagnose a gate that hangs or
fails. `cargo xtask mk leptos-gates` runs both;
after the release build, the doctor takes about 15 seconds and the whole suite a few minutes. What
each gate asserts is in the
[browser gate suites README](/tools/browser_testing/browser_gate_suites/README.md); this runbook
does not repeat it.

## Prerequisites

- **The full Chromium build, never `chrome-headless-shell`.** `find_chromium` takes
  `CHROME_HEADLESS_SHELL` when it names an existing file, else the highest-numbered
  `chromium-*/chrome-linux64/chrome` or `chromium-*/chrome-linux/chrome` under
  `PLAYWRIGHT_BROWSERS_PATH`, then under `~/.cache/ms-playwright`, and adds `--headless=new`. The
  headless shell aborts on per-character font fallback, and the doctor warns when it resolves to
  one. The pinned version is `chromium.version` in `tools/browser_testing/browser_gate_suites/gate-env.json`. Check:
  the doctor's `chromium` line.
- **The toolchain** pinned by the root `rust-toolchain.toml` (1.95.0 with the
  `wasm32-unknown-unknown` target) and Trunk at the version in `gate-env.json`. Check: the doctor's
  `rustc` and `trunk` lines.
- **At least 1024 MiB of available memory** (`limits.min_mem_available_mib`): SwiftShader thrashes
  below it. Check: the doctor's `memory` line.
- **The [API](/documentation/glossary/a_to_f.md#api) on `127.0.0.1:8080`** for the `hydrate` smoke in the suite and for `gate smoke
  mutations`; the other smokes need none. Start it as in
  [Local development](/documentation/runbooks/local_development.md).
- **The Everon map assets from Git LFS** (`assets/terrains/everon/`) for the `fullmap` and
  `hillshade` smokes and the perf probes.
- **The gate font cache.** Every `gate` command sets `XDG_CACHE_HOME` to
  `$TMPDIR/tbd-gate-cache-<distro>`, keyed on `ID` and `VERSION_ID` of `/etc/os-release` (for
  example `tbd-gate-cache-debian-12`), and every Chromium launch gets it. The only override is
  `TBD_GATE_FONT_CACHE`; an `XDG_CACHE_HOME` you export yourself is ignored, because a shared
  cross-distribution cache is what leaves Chromium with zero fonts (wedge mode 4 below). Check:
  the doctor's `fonts` line names the cache.

## Steps

Run every command from the repository root. The gate commands build and run the `gate` binary of
`tools/developer_tools` with the host's toolchain; a binary built against a newer glibc than a
container's does not run inside that container.

### Run the whole gate

1. Start the local database.

   ```bash
   cargo xtask db up
   ```

   Expected: compose starts the `tbd_reforger_db` container on host port 5434.

2. In a second terminal, run the API; it applies the migrations on boot and stays in the
   foreground.

   ```bash
   cargo xtask mk rust-api
   ```

   Expected: the API logs `listening on 0.0.0.0:8080`.

3. Build the app and run the doctor and the smoke suite.

   ```bash
   cargo xtask mk leptos-gates
   ```

   Expected: the three commands in order, as `--dry-run` prints them:
   `cd crates/frontend/shell/frontend_application && trunk build --release`, then
   `cargo run -q -p developer_tools --bin gate -- doctor` and `… -- editor-suite`; the doctor ends `== gate doctor: OK — 0 warning(s)`, each smoke prints
   its JSON verdict, and the run exits 0. It stops at the first step that fails, with that step's
   exit code.

An editor factory [wave](/documentation/glossary/n_to_z.md#wave) runs step 3 after its wave gate
passes and before it closes: `cargo xtask platform wave gate` runs no Chromium, so this is the only
automated run of the rect smokes (`save-dialog-rect`, `entrance-motion-rect`). The wave procedure
is in [Factory waves](/documentation/runbooks/factory_waves/README.md).

### Run one gate

4. Run the preflight alone. `cargo xtask mk gate-doctor` does the release build first.

   ```bash
   cargo run -q -p developer_tools --bin gate -- doctor
   ```

   Expected: one line per check, `✓` or `!` or `✗`: `chromium`, `rustc`, `trunk`, `memory`,
   `processes`, `fonts`, `dist`, `liveness`, then `== gate doctor: OK — 0 warning(s)` and exit 0.
   `--strict` turns any warning into exit 1; `--dist <dir>` points it at another build.

5. Run one smoke by name; the names are the table in the
   [smoke tests README](/tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/README.md).

   ```bash
   cargo run -q -p developer_tools --bin gate -- smoke cur
   ```

   Expected: the smoke's JSON verdict of named checks and exit 0; 1 on a failed check, 2 on a
   missing prerequisite, 3 on a driver error.

## Verify

Run the suite again after a fix, and again once the fix is on `main`.

```bash
cargo xtask mk leptos-gates
```

Expected: the doctor ends `== gate doctor: OK — 0 warning(s)`, every smoke verdict passes and the
run exits 0.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `✗ fonts       chromium resolves NO font ('Could not find any font') …` | the Chromium font cache holds another distribution's entries (wedge mode 4) | clear the cache the line names, or set `TBD_GATE_FONT_CACHE` to an empty directory |
| `! chromium    resolved to chrome-headless-shell …` | the Playwright cache holds only the headless shell, or `CHROME_HEADLESS_SHELL` names it | install the full build, or point `CHROME_HEADLESS_SHELL` at a full `chrome` |
| `! processes   N stray chrome process(es) …` | an earlier run crashed and left Chromium children behind | `pkill -9 -f chrome-headless-shell; pkill -9 -f 'chrome-linux64/chrome'` |
| `✗ liveness    the headless browser process DIED during the probe …` | a crash, or a browser blocked on a full output pipe: the same signature | P-1 of the debug recipe tells them apart |
| `✗ liveness    editor page did not become ready within the budget` | a slow or stalled page, a missing build or low memory | check the `dist` and `memory` lines, then the debug recipe |
| `smoke_hydrate: backend not reachable on :8080`, exit 2 | the API is not running; a gate that could not run never reports green | step 2 |
| `gate: driver error: cdp: ws call timed out (Runtime.evaluate)` after about 130 s | a call to a browser that died or blocked; the per-call timeout is 130 s | run the doctor, then P-1 |

### Known wedge modes

1. **Font-fallback crash in the headless shell.** `chrome-headless-shell` aborts at
   `SkFontMgr_FontConfigInterface.cpp:163 "Not implemented"` when a page needs a fallback glyph;
   the renderer dies and the harness waits out a 130 s `Runtime.evaluate`. The gates use the full
   `chrome` build for this reason; if it recurs, the resolved binary is a shell. Recorded as
   [KB-002](/documentation/known_bugs/kb_002_editor_gate_boot_wedge.md).
2. **Stray Chromium starving the next smoke.** A crashed run can leave renderer and GPU children
   pegging every core under software GL. The doctor counts them; kill them with the command in
   the table above.
3. **Memory pressure.** SwiftShader thrashes under a low memory ceiling. The doctor checks
   `MemAvailable` only, not a cgroup limit; check `memory.max` of your cgroup yourself inside a
   container.
4. **Font-fallback crash in the browser process.** The same `SK_ABORT` also fires in the full
   `chrome` build, on a `ThreadPoolForeground` thread of the browser process:
   `onMatchFamilyStyleCharacter` is unimplemented in both builds, so any per-character fallback
   is fatal once Chromium has no fonts at all.
   - Precondition: Chromium logs `Could not find any font: , sans` at startup and every UI text
     run shapes to `glyph_count: 0`.
   - Cause: a fontconfig cache written by a container that shares the home directory describes
     that container's fonts; Chromium's bundled fontconfig accepts it and never rescans, so it has
     zero fonts while `fc-list` on the host lists hundreds. An empty cache directory fixes it.
   - Symptom: `cdp: ws call timed out (Runtime.evaluate)` or `timeout waiting for
     Page.loadEventFired` a few hundred milliseconds after the editor navigates. Other routes
     render with fonts they already matched and survive; only the Mission Creator reaches a
     per-character fallback.
   - The same signature comes from a browser blocked on a full output pipe (the first harness gap
     below), so run P-1 before concluding a font abort.
   - Handled by the gate font cache in the prerequisites. The doctor's font probe launches
     Chromium on `about:blank`, watches its log for `Could not find any font` for 5 s, then kills
     the process group; it inherits `XDG_CACHE_HOME`, so it measures the cache the smokes get.
     Outside the gates, `rm -rf ~/.cache/fontconfig` clears a poisoned cache.

### Known harness gaps

- **Chromium's output pipes must be drained.** A pipe holds 64 KiB, and Chromium with
  `--enable-logging=stderr --v=1` writes more than that in its first second. An undrained pipe
  blocks whichever Chromium thread writes next; when that is the browser's main thread, the
  DevTools endpoint stops answering and the doctor reports that the browser died. `chrome_devtools_protocol::launch`
  drains both pipes from spawn and keeps the last 200 lines for `Browser::recent_output()`, the
  only copy of Chromium's own abort reason. A probe that spawns Chromium itself drains its pipes,
  sends them to a file or uses `Stdio::null()`, and never calls `Command::output()`, which waits
  for EOF on pipes that Chromium's zygote and crashpad children inherit.
- **`innerText` returns the text CSS renders.** Under `text-transform: uppercase`, `innerText`
  reads `ATTACHED MISSIONS` where `textContent` reads `Attached Missions` (the heading in
  `crates/frontend/pages/administration_pages/src/event_manager/mission_picker.rs:216` carries
  the `uppercase` class). `render-check --expect` matches against `document.body.innerText`; use
  `textContent` in `--assert-js` for source-exact text, or compare case-insensitively.
- **`aside` is ambiguous.** The desktop sidebar
  (`crates/frontend/shell/frontend_application/src/shell/sidebar.rs:51`), the mobile drawer
  (`crates/frontend/shell/frontend_application/src/shell/layout.rs:148`) and the membership notice
  (`crates/frontend/shell/frontend_application/src/shell/membership_status.rs:101`) are all `<aside>`, and
  `document.querySelector('aside')` returns the first in DOM order whether or not it is shown.
  Select on a discriminating class or scope to a landmark.
- **`render-check` proxies `/api` to a live API.** `--api-proxy` defaults to
  `http://127.0.0.1:8080`, which `--seed-auth` needs to hydrate a signed-in page; without an API
  there, API requests fail.

### Debug recipe (P-1 to P6)

For a smoke that hangs or fails when the doctor does not already name the cause, cheapest decisive
check first.

- **P-1: is the browser alive?** While the call hangs, ask the browser rather than the page:
  `curl -sv http://127.0.0.1:<debug-port>/json/version`, and read curl's exit code.
  - 7, connection refused: nothing listens; the browser is gone (a crash or `SK_ABORT`). Go to P2;
    the abort reason is on Chromium's stderr or in `Browser::recent_output()`.
  - 52, empty reply: the browser is alive and its main thread is blocked. `cat
    /proc/<browser-pid>/syscall` starting with `1` (`write`) and `/proc/<browser-pid>/wchan`
    reading `anon_pipe_write` mean a full output pipe, not a crash. A `--headless=new` browser
    blocked this way still shows every thread in state `S`, so `ps` alone does not tell.
- **P0: processes and resources.** `pgrep -af 'chrome-headless-shell|chrome_crashpad'`, `uptime`,
  `MemAvailable` in `/proc/meminfo` and the cgroup's `memory.max`; kill strays, free memory, retry.
- **P1: environment drift.** The resolved Chromium's `--version`, `rustc --version` and
  `trunk --version` against `gate-env.json`, and any recent graphics-driver or kernel update.
- **P2: Chromium's own stderr, decisive for a crash.** Serve the built app, then launch Chromium
  on the editor with `--enable-logging=stderr --v=1` and look for `FATAL`, `SkFontMgr` or
  `Received signal`:

  ```bash
  cargo run -q -p developer_tools --bin gate -- serve --dir crates/frontend/shell/frontend_application/dist --port 5199 --api-proxy http://127.0.0.1:8080 --map-assets assets/terrains
  ```

- **P3: renderer thread state.** While it hangs, field 3 (state) of
  `/proc/<renderer-pid>/task/*/stat` and `wchan`: every thread running in a `swiftshader` thread
  means a CPU-bound shader compile; `D` or `S` on a futex means a wait on GPU IPC. Then
  `gdb -p <pid> -batch -ex 'thread apply all bt'`, which names the shared libraries even in a
  stripped build.
- **P4: one-flag levers.** Drop `--enable-unsafe-webgpu`; add `--in-process-gpu` or
  `--disable-gpu-compositing`.
- **P5: application breadcrumbs.** Rebuild with `leptos::logging::log!("[BOOT] …")` lines through
  the Mission Creator's boot and the render engine's creation; the last line printed locates the
  stall.
- **P6: land the fix.** Make it durable as a harness flag or a pin, and revert every probe.

## Related

- [Browser gate suites](/tools/browser_testing/browser_gate_suites/README.md) — every `gate`
  command, what it asserts and its exit codes.
- [Gate doctor and font cache](/tools/browser_testing/browser_gate_suites/src/diagnostics/README.md)
  — each doctor check and the font cache.
- [Mission Creator smoke tests](/tools/browser_testing/browser_gate_suites/src/editor_smoke_tests/README.md)
  — the smokes, `r-auth` and `render-check`.
- [KB-002](/documentation/known_bugs/kb_002_editor_gate_boot_wedge.md) — the headless-shell
  font-fallback crash.
- [Testing and CI](/documentation/runbooks/testing_and_ci.md) — the other gates.
  `.github/workflows/editor-gates.yml` runs this gate nightly and on demand: a Postgres service, the pinned
  Chrome for Testing from `cargo xtask ci ci-chrome`, the API from `cargo xtask ci editor-api-boot`,
  then `cargo xtask mk gate-doctor` and `cargo xtask mk leptos-gates`.
- [Editor capture](/documentation/runbooks/editor_capture.md) — screenshots of a running
  Mission Creator on the real GPU.
