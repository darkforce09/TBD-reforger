# Editor gate runbook (`cargo xtask mk leptos-gates`)

How to run the editor CDP smokes + frozen V-suite, the environment they need, and how to debug the
one failure mode that has bitten hard (a boot wedge). Authority for the "gates must be reproducible +
fail-fast" contract: [`.cursor/rules/acceptance-gates-reproducible.mdc`](../../.cursor/rules/acceptance-gates-reproducible.mdc).
Pins: [`tools/tbd-tools/gate-env.json`](../../tools/tbd-tools/gate-env.json).

## Run it

```bash
cargo xtask db up          # Postgres :5434 (hydrate/mutations smokes need the API)
cargo xtask mk rust-api            # Axum API :8080 (migrates on boot)
cargo xtask mk leptos-gates   # trunk release build → gate doctor → editor-suite (18 smokes) → v-suite verify
```

`cargo xtask mk leptos-gates` runs **`gate doctor` first** (a prerequisite). The doctor validates the resolved
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
- **API on :8080** (`cargo xtask mk rust-api`) for the `hydrate` / `mutations` smokes. Most smokes don't need it.
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
   **This signature is NOT unique to this mode.** An undrained output pipe (KB-003, §Known harness
   gaps) produces the identical message from a browser that is still alive. **P-1 separates them by
   curl exit code** — do that before concluding "font abort".
   **Why only the editor route:** `/` and every other SPA route (`/dashboard`, `/missions`,
   `/missions/:id`, `/vehicles`, `/mortar` — all verified) render inside fonts they already matched
   and survive with the errors above; only the mission editor reaches a per-character fallback.
   **Fixed by:** `doctor::ensure_gate_font_cache` — every browser gate runs with a gate-owned
   `XDG_CACHE_HOME`. `gate doctor` also reports `✗ fonts` (a 1 s `--dump-dom` probe) and
   distinguishes `the headless browser process DIED` from a page that is merely not ready.
   **Operator remedy if you hit it outside the gate:** `rm -rf ~/.cache/fontconfig`.

## Known harness gaps (not wedges — read before blaming the app)

- **Chrome's output pipes MUST be drained (KB-003 / T-354, fixed in `cdp::launch`).** A pipe holds
  **64 KiB**. Pipe chrome's stdout/stderr and never read them, and the first `write(2)` past that
  blocks **the chrome thread that issued it** — permanently, because a pipe nobody drains never
  drains. `cdp::launch` had `.stdout(piped())` + `.stderr(piped())` and no reader, so this was a
  **live intermittent fault in the committed gate**, not only a trap for hand-rolled probes.
  **How chatty is chromium?** MEASURED with `--enable-logging=stderr --v=1`, written to a file so
  nothing throttled it: **87,583 bytes of stderr in the first second** on `about:blank` alone
  (109,581 by t+6 s) — 1.34× the buffer before a page exists. T-320's broken-font environment on its
  own produces 250–400 `render_text_harfbuzz` lines per launch. You do not need an exotic page to
  cross 64 KiB.
  **Why it is intermittent:** whichever chrome thread happens to own the write that crosses the
  threshold is the thread that parks. A `ThreadPoolForeground` thread parking is survivable; the
  **browser main thread** parking stops the DevTools endpoint dead. That is scheduling roulette, so
  it takes out a fraction of runs and reads as flake. MEASURED A/B over `gate doctor` against a
  deliberately chatty chrome, identical but for `cdp.rs`: **undrained 4 PASS / 2 FAIL of 6; drained
  18 PASS / 0 FAIL of 18** — and 6 s per run instead of 11–16 s, because the launch poll was
  stalling on the same block.
  **Symptom seen by the harness — and this is the trap:** the *exact* wedge-mode-4 signature.
  `cdp: ws call timed out (Runtime.evaluate)`, or `error sending request for .../json/new` when the
  block lands earlier, and `gate doctor` reporting **"the headless browser process DIED"**. It has
  not died. It is alive and blocked in `write`. T-232 lost its second hand-rolled harness to this;
  T-320 lost five sessions to that same signature from a genuinely different cause. **Do not read
  that message as a font abort until P-1 below has ruled this out.**
  **Fixed by:** `cdp::drain_pipe` — both pipes drained from spawn, last 200 lines kept and readable
  via `Browser::recent_output()` (chrome's stderr is the only copy of its own abort reason, and
  `launch` used to discard it). **If you write a probe that spawns chromium yourself, drain it or
  use `Stdio::null()`.** `doctor::check_fonts` shows the other correct answer: hand chrome a *file*,
  and never `Command::output()` — that waits for EOF on pipes chrome's zygote/crashpad children
  inherit, so it can block long after the browser itself exited.
  **Does this explain T-338's top-level-document wedge?** Probably not, and it should not be assumed
  to. T-338 wedged on **four consecutive** attempts doing IndexedDB + wasm-bridge work in a
  top-level editor document, and was stable once the editor moved into an iframe. This gap is
  scheduling-dependent rather than reproducible four times running, and document topology has no
  obvious path to stderr volume. But T-338's per-step timeout races would not have defeated a pipe
  block either, so it is **worth re-running that repro now that the pipes are drained** — and if it
  still wedges, `Browser::recent_output()` will for the first time show what chrome said about it.
- **`innerText` returns the text CSS *renders*, so `text-transform: uppercase` is applied.** An
  assertion against `'Attached Missions'` fails against a rendered `'ATTACHED MISSIONS'`. Not
  hypothetical for this gate: `smokes::render_check` matches `--expect` against
  `document.body.innerText`, and both "Attached Missions" headings (`events.rs:398`,
  `event_manager.rs:1206`) carry the Tailwind `uppercase` class. MEASURED on the pinned chromium: a
  `text-transform: uppercase` element yields `innerText` `"ATTACHED MISSIONS"` and `textContent`
  `"Attached Missions"`; on `/` the live `<h3>`s report `textContent` `"Command Center"` against
  `innerText` `"COMMAND CENTER"`. **Use `textContent` for source-fidelity text assertions**, or
  compare case-insensitively. This cost T-232 part of a misleading 26/44 and led T-226 to "fix" a
  non-bug before it caught itself.
- **An `<aside>` selector is ambiguous — there are two.** The platform sidebar (`layout.rs:296`,
  `hidden … lg:flex`) and the mobile drawer (`layout.rs:97`, `fixed inset-y-0 … lg:hidden`) are both
  `<aside>`. At the gate's 1440×900 viewport the desktop sidebar is the visible one, but
  `document.querySelector('aside')` returns whichever comes first in the DOM regardless of which is
  displayed — not a stable thing to assert on. Select on a discriminating class or scope to a
  landmark. Also part of T-232's 26/44.
- **`render-check` never proxies `/api`.** `smokes::render_check` builds its `Harness` with
  `api_proxy: None` (`smokes.rs`), so an `/api/v1/...` fetch falls through to the SPA index.html
  instead of a backend. Harmless for the editor route — verified: it boots, installs
  `__missionDoc` / `__missionPersist` / `__missionBackup` and answers `--assert-js` — but a probe that
  needs real API data cannot use `render-check` today. Wiring an `--api-proxy` through
  `bin/gate.rs` → `RenderCheckArgs` is the fix (T-320 found it; the CLI is outside that slice).
- **`gate smoke hydrate` / `mutations` need the API on :8080** and return exit **2** with
  `backend not reachable` when it is down. That is deliberate — they are data-safety gates and a gate
  it could not run must not report green. Start `cargo xtask mk rust-api` rather than reinterpreting the code.
- **`gate v-suite` launches its own chromium** (`vsuite.rs`) and therefore does **not** get the
  T-320 gate-owned font cache. It renders ordinary routes, which survive a broken font environment,
  so it is unaffected today; moving `ensure_gate_font_cache()` into `cdp::launch` would close it for
  every caller at once.

## Debug recipe (P0–P6, cheapest-decisive first)

When a smoke hangs/fails and the doctor doesn't already name it:

- **P-1 — is the browser still alive?** Ask it, not the page:
  `curl -sv http://127.0.0.1:<debug-port>/json/version` while the call is "hanging". No answer means
  every timeout after that is a symptom, not the fault (T-320). **Read curl's exit code — "no
  answer" has two distinct causes** (T-354, both measured):
  - **exit 7, connection refused** — nothing is listening. The browser is genuinely gone (a crash /
    `SK_ABORT`). Go to P2; the abort reason is on chromium's stderr, or already in
    `Browser::recent_output()`.
  - **exit 52, empty reply** — the socket is accepted and then nothing is written. The browser is
    **alive** but its main thread is blocked. Confirm with
    `cat /proc/<browser-pid>/syscall` (leading `1` = `write`) and `/proc/<browser-pid>/wchan`
    (`anon_pipe_write` = a full, undrained output pipe → the KB-003 gap above, not a crash).
    A `--headless=new` browser blocked this way still shows all threads in state `S`, so `ps` alone
    will not tell you.
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
