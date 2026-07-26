# Editor gate runbook (`make leptos-gates`)

How to run the editor CDP smokes + frozen V-suite, the environment they need, and how to debug the
one failure mode that has bitten hard (a boot wedge). Authority for the "gates must be reproducible +
fail-fast" contract: [`.cursor/rules/acceptance-gates-reproducible.mdc`](../../.cursor/rules/acceptance-gates-reproducible.mdc).
Pins: [`tools/tbd-tools/gate-env.json`](../../tools/tbd-tools/gate-env.json).

## Run it

```bash
make db-up          # Postgres :5434 (hydrate/mutations smokes need the API)
make api            # Axum API :8080 (migrates on boot)
make leptos-gates   # trunk release build → gate doctor → editor-suite (18 smokes) → v-suite verify
```

`make leptos-gates` runs **`gate doctor` first** (a prerequisite). The doctor validates the resolved
chromium + toolchain against `gate-env.json`, checks free RAM + orphaned chrome, checks that
**chromium can resolve a font at all** (T-320 — see wedge mode 4), verifies the `--dist` exists, and
runs a ~15 s editor liveness probe — so a wedge fails in seconds with a diagnosis, not a 130 s hang.

Every browser gate (`gate doctor`, every `gate smoke`, `gate render-check`, `gate r-auth`) points
chromium at a **gate-owned fontconfig cache** (`$TMPDIR/tbd-gate-cache`) instead of `~/.cache`, for
the reason in wedge mode 4. An `XDG_CACHE_HOME` you export yourself is respected and inherited.

Single smoke / doctor standalone:

```bash
cargo run -q -p tbd-tools --bin gate -- doctor            # preflight only
cargo run -q -p tbd-tools --bin gate -- smoke cur         # one smoke (see EDITOR_SUITE for names)
cargo run -q -p tbd-tools --bin gate -- v-suite verify    # frozen DOM oracle only
```

CI: [`.github/workflows/editor-gates.yml`](../../.github/workflows/editor-gates.yml) (nightly + on
demand + gate/editor-path PRs) runs the same, with a Postgres service + a curl-installed pinned chrome.

## Required environment

- **Chromium — the FULL `chrome` build, not `chrome-headless-shell`.** `find_chromium` (`cdp.rs`)
  prefers `~/.cache/ms-playwright/chromium-<n>/chrome-linux64/chrome` and adds `--headless=new`.
  Override with `CHROME_HEADLESS_SHELL=<path-to-a-chrome-binary>`. **The shell FATAL-crashes on font
  fallback** (see below) — the doctor warns if it resolves to the shell. Pinned build in `gate-env.json`.
- **Toolchain** pinned by the root [`rust-toolchain.toml`](../../rust-toolchain.toml) (rustc 1.95.0 +
  `wasm32-unknown-unknown`) + trunk. Validated by the doctor.
- **API on :8080** (`make api`) for the `hydrate` / `mutations` smokes. Most smokes don't need it.
- **map-assets** (LFS) for `fullmap` / `hillshade` (the full satellite + DEM + world objects).
- **`?force=webgl&sat=preview`** — the smokes pin the WebGL2/SwiftShader backend (`EDIT_PATH`); the
  default WebGPU/lavapipe path is unreliable headless (`smokes.rs` §force=webgl). `sat=preview` avoids
  the 205 MB satellite fetch except in `fullmap`.

## Known wedge modes

1. **Font-fallback crash (KB-002, resolved).** `chrome-headless-shell` aborts at
   `SkFontMgr_FontConfigInterface.cpp:163 "Not implemented"` when the page needs a fallback glyph → the
   renderer dies → a 130 s `Runtime.evaluate` hang. Fix: use the full `chrome` build (T-177). If you
   ever see this again, the resolved binary is wrong (a shell) — check `gate doctor` / `find_chromium`.
2. **Orphaned chrome starving the next smoke.** A crashed run can leave renderer/gpu children pegging
   every core under software GL (`cdp.rs` process-group note). The doctor scans for these; kill with
   `pkill -9 -f chrome-headless-shell; pkill -9 -f 'chrome-linux64/chrome'`.
3. **Memory pressure.** SwiftShader thrashes under a low RAM ceiling (`smokes.rs` §force=webgl). The
   doctor checks `MemAvailable` + cgroup limits.
4. **Font-fallback crash, part two — the BROWSER process (KB-002b / T-320).** The same
   `SkFontMgr_FontConfigInterface.cpp:163 "Not implemented"` `SK_ABORT`, reached from the **full
   `chrome` build** rather than the shell, on a **`ThreadPoolForeground` thread of the browser
   process**. T-177 moved the crash, it did not remove it: `onMatchFamilyStyleCharacter` is
   unimplemented in *both* builds, so any per-character font fallback is fatal.
   **Precondition:** chromium resolves **no font at all** — it logs
   `ERROR:ui/gfx/platform_font_skia.cc: Could not find any font: , sans` at startup and every UI text
   run shapes to `glyph_count: 0` (`render_text_harfbuzz.cc:1016`, hundreds of lines).
   **Cause:** a **cross-distro `~/.cache/fontconfig`.** A container that shares the home directory
   (distrobox/toolbox) writes cache entries describing *its* font set; chromium's bundled fontconfig
   accepts them and never rescans, so it comes up with zero fonts even though `fc-list` on the host
   reports hundreds. Proven by A/B: the same cache **copied to a different path** still kills it, an
   **empty** cache dir fixes it.
   **Symptom seen by the harness:** `cdp: ws call timed out (Runtime.evaluate)` or
   `timeout waiting for Page.loadEventFired` ~200–400 ms after navigating the editor. It is not a
   slow page — the browser is a corpse and nothing will ever answer that websocket.
   **Why only the editor route:** `/` and every other SPA route (`/dashboard`, `/missions`,
   `/missions/:id`, `/vehicles`, `/mortar` — all verified) render inside fonts they already matched
   and survive with the errors above; only the mission editor reaches a per-character fallback.
   **Fixed by:** `doctor::ensure_gate_font_cache` — every browser gate runs with a gate-owned
   `XDG_CACHE_HOME`. `gate doctor` also reports `✗ fonts` (a 1 s `--dump-dom` probe) and
   distinguishes `the headless browser process DIED` from a page that is merely not ready.
   **Operator remedy if you hit it outside the gate:** `rm -rf ~/.cache/fontconfig`.

## Known harness gaps (not wedges — read before blaming the app)

- **`render-check` never proxies `/api`.** `smokes::render_check` builds its `Harness` with
  `api_proxy: None` (`smokes.rs`), so an `/api/v1/...` fetch falls through to the SPA index.html
  instead of a backend. Harmless for the editor route — verified: it boots, installs
  `__missionDoc` / `__missionPersist` / `__missionBackup` and answers `--assert-js` — but a probe that
  needs real API data cannot use `render-check` today. Wiring an `--api-proxy` through
  `bin/gate.rs` → `RenderCheckArgs` is the fix (T-320 found it; the CLI is outside that slice).
- **`gate smoke hydrate` / `mutations` need the API on :8080** and return exit **2** with
  `backend not reachable` when it is down. That is deliberate — they are data-safety gates and a gate
  it could not run must not report green. Start `make api` rather than reinterpreting the code.
- **`gate v-suite` launches its own chromium** (`vsuite.rs`) and therefore does **not** get the
  T-320 gate-owned font cache. It renders ordinary routes, which survive a broken font environment,
  so it is unaffected today; moving `ensure_gate_font_cache()` into `cdp::launch` would close it for
  every caller at once.

## Debug recipe (P0–P6, cheapest-decisive first)

When a smoke hangs/fails and the doctor doesn't already name it:

- **P-1 — is the browser still alive?** Ask it, not the page: `curl -s http://127.0.0.1:<debug-port>/json/version`
  while the call is "hanging". No answer = it crashed, and every timeout after that is a symptom, not
  the fault (T-320). Then go straight to P2 — the abort reason is on chromium's stderr.
- **P0 — process + resources:** `pgrep -af 'chrome-headless-shell|chrome_crashpad'` + `uptime`;
  `/proc/meminfo` `MemAvailable`; cgroup `memory.max`. Kill strays / free RAM → retry.
- **P1 — env drift:** resolved chromium `--version` vs `gate-env.json`; `rustc`/`trunk` `--version`;
  `rpm-ostree status` (a Mesa/kernel bump correlating with "last worked").
- **P2 — chrome's own stderr (decisive for a crash):** launch chromium on the served editor with
  `--enable-logging=stderr --v=1` and grep for `FATAL` / `SkFontMgr` / `Received signal`. Serve it with
  `gate serve --dir apps/website/frontend/dist --port 5199 --api-proxy http://127.0.0.1:8080 --map-assets packages/map-assets`.
- **P3 — renderer thread state:** while hung, `/proc/<renderer-pid>/task/*/stat` field 3 (State) +
  `wchan` — all-R in a `swiftshader` thread = CPU-bound sync compile; D/S on a futex = GPU-IPC wait.
  Escalate to `gdb -p <pid> -batch -ex 'thread apply all bt'` (shows `.so` names even stripped).
- **P4 — one-flag levers:** drop `--enable-unsafe-webgpu`; `--in-process-gpu`; `--disable-gpu-compositing`.
- **P5 — app breadcrumbs (last; needs a rebuild):** `leptos::logging::log!("[BOOT] …")` through the
  `mission_editor` boot + `engine.rs` `RenderEngine::create`; the last line printed localizes the stall.
- **P6 — land the fix durably** (a harness flag / a pin) + **revert every probe**.
