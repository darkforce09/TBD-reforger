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
chromium + toolchain against `gate-env.json`, checks free RAM + orphaned chrome, and runs a ~15 s
editor liveness probe — so a wedge fails in seconds with a diagnosis, not a 130 s hang.

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

## Debug recipe (P0–P6, cheapest-decisive first)

When a smoke hangs/fails and the doctor doesn't already name it:

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
