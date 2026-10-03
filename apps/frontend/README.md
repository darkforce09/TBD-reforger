# Website frontend

The `frontend` crate: the web platform's single-page app, written in Rust with Leptos 0.8
and rendered in the browser as WebAssembly. It holds every page members use, the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator), and the client that talks to the
[API](/documentation/glossary/a_to_f.md#api); Trunk builds it into a static bundle that the dev server,
the API or the deployed host serves.

## Contents

```text
apps/frontend/
├── Cargo.toml            the `frontend` package: one binary, the map crates it links per target
├── index.html            the Trunk entry: the app and worker builds, stylesheet, icon font, web manifest
├── manifest.webmanifest  the web app manifest: name, start URL, scope, standalone display, colours
├── service_worker.js     the service worker loader: imports the Rust worker and forwards its events
├── src/                  the entry point and the layers: foundation, features, pages, workspaces, shell
├── style/                the Aegis stylesheet Tailwind compiles into the build
└── Trunk.toml            the build, the dev server on 127.0.0.1:3000 and its proxy to the API
```

## How it works

Trunk reads `index.html`: it builds the crate's binary for `wasm32-unknown-unknown`, runs
`wasm-bindgen`, and in a release build shrinks the module with `wasm-opt` at the level
`data-wasm-opt="z"` asks for; it compiles `style/aegis.css` with Tailwind CSS, and writes the page,
the module, its JavaScript glue and the stylesheet into `dist/`. In the browser, the start
function in `src/main.rs` mounts the app layout under a router, the layout restores a stored
session, and the route table in `src/app_routes.rs` and `src/foundation/route_table/` picks the page.

The same `index.html` builds the offline service worker from
`apps/offline_service_worker` as a Trunk worker (`data-type="worker"`,
`data-bindgen-target="no-modules"`): Trunk writes `offline_service_worker.js` and
`offline_service_worker_bg.wasm` to the root of `dist/` with no content hash and links neither from
the page. It copies `service_worker.js`, the loader the page registers as
`/service_worker.js?build=<id>`, and `manifest.webmanifest`, which the page links as its web app
manifest, unchanged to the root of `dist/`.

```text
index.html ─▶ trunk ─┬─▶ cargo build --target wasm32-unknown-unknown ─▶ wasm-bindgen
                     │       ─▶ wasm-opt at level z, release builds only ──────────▶ dist/
                     ├─▶ offline_service_worker worker build (no-modules) ─────────▶ dist/
                     ├─▶ copy service_worker.js, manifest.webmanifest ─────────────▶ dist/
                     └─▶ tailwindcss style/aegis.css ─────────────────────────────▶ dist/
browser ─▶ dist/index.html ─▶ start_app ─▶ AppLayout ─▶ route ─▶ page or workspace
                                               └─▶ /api/v1 and /map-assets on the same origin
```

The app calls the API on its own origin, under `/api/v1`, and the map engine loads terrain from
`/map-assets` on the same origin, so whatever serves the bundle also serves or proxies those
paths: Trunk's proxy in development, the API itself when its `SPA_DIST_DIR` points at `dist/`, and
Caddy on the deployed host. The dependency on the map crates runs one way,
`frontend ─▶ map_renderer, map_streaming_host ─▶ gpu_frame, gpu_device`: the app hands the map
renderer a canvas, the map host feeds it, and the GPU crates never learn a map noun.

The binary's `main` is empty and `start_app` exists only on `wasm32`, so the crate also compiles
for the host, where `cargo test -p frontend` runs every test of the native half: the route
table, the client's refresh policy, the wire types against the captured responses, and the source
pins.

## Getting started

Run these from the repository root. The first three bring the app up with live data, in this
order, and `mk rust-api` and `mk leptos` each stay in the foreground in a terminal of their own.
The last two check the crate without the dev server: `mk ci-local-leptos` needs no server at all,
and `mk leptos-gates` needs the API running with `APP_ENV=development`, which the `hydrate` smoke
of its editor suite signs in through:

```bash
cargo xtask db up               # Postgres for the API on host port 5434, detached
cargo xtask mk rust-api         # the API on port 8080; stays in the foreground
cargo xtask mk leptos           # trunk serve --release on 127.0.0.1:3000; stays in the foreground
cargo xtask mk ci-local-leptos  # fmt check, clippy for wasm32, the native tests, a release build
cargo xtask mk leptos-gates     # release build, then gate doctor, editor-suite and v-suite verify
```

The workspace `rust-toolchain.toml` pins Rust 1.95.0 with the `wasm32-unknown-unknown` target, so
rustup installs the target itself. Trunk has to be on `PATH`; the gates pin Trunk 0.21.14 and
`wasm-bindgen` 0.2.126 in `tools/browser_testing/browser_gate_suites/gate-env.json`, and Trunk fetches the
`wasm-bindgen` and Tailwind CSS versions it needs on its first build, with no npm involved.
`cargo xtask mk leptos-debug` serves an unoptimised build that rebuilds faster but whose frame
rates mean nothing, and `cargo xtask mk leptos-build` writes a release build into `dist/` without
serving it. `cargo run -q -p developer_tools --bin gate -- render-check --dir apps/frontend/dist`
checks that a built bundle mounts and renders in a headless browser.

To sign in without Discord through the [dev login](/documentation/glossary/a_to_f.md#dev-login), open
`/api/v1/auth/dev-login?role=admin` on the host `FRONTEND_URL` names (`http://localhost:3000` in
`apps/api/.env.example`); the proxy hands the redirect to `/auth/callback` back
unfollowed, so the session in its URL fragment reaches the app.

## Configuration

The app reads no environment variable: the settings are the build files'.

| Setting | Value | Read by |
|---|---|---|
| map crates, every build | `map_streaming_model` (preferences, boot progress) | Cargo, from `Cargo.toml` |
| map crates, browser build | `map_renderer`, `gpu_frame`, `map_asset_loading`, `map_streaming_host`, `map_render_diagnostics`, `paper_doll_renderer` (the `wasm32` target table) | Cargo, from `Cargo.toml` |
| `[build]` | `target = "index.html"`, output `dist` | Trunk, from `Trunk.toml` |
| `[tools]` | `tailwindcss = "4.3.2"` | Trunk |
| `[watch]` | ignores `dist` and `style/aegis.css`, the paths the build itself writes into, so a build never triggers the next | Trunk |
| `[serve]` | `127.0.0.1:3000`, with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: credentialless` for cross-origin isolation | Trunk |
| `[[proxy]]` | `/api` to `http://127.0.0.1:8080/api` with `no_redirect = true`; `/map-assets` to `http://127.0.0.1:8080/map-assets` | Trunk |
| `data-wasm-opt` | `z`, the size-first `wasm-opt` level of a release build | Trunk, from `index.html` |
| API root | `/api/v1` on the page's origin, `API_BASE` | the client's verbs, from `src/foundation/transport/client/mod.rs` |
| stored session | the `tbd-auth` key of local storage; `tbd-auth-refresh` names the Web Lock and the broadcast channel of a token refresh | `src/foundation/auth/session.rs` and `src/foundation/transport/client/refresh.rs` |

The [mission](/documentation/glossary/g_to_m.md#mission) store and compiler are in the native build
too, so `cargo test` checks the metadata the Mission Creator's export compiles from
(`compiled_meta_is_the_row_the_server_compiles_from` in
`src/foundation/transport/dto/tests/r_api_missions.rs`). The token refresh needs the Web Locks API, which
only a secure context offers: `localhost`, `127.0.0.1` or HTTPS.

## Public surface

- The `frontend` binary, with no library: `start_app` in `src/main.rs` is its WebAssembly
  start function, and no other crate links it.
- The bundle in `dist/`, served by `trunk serve` in development, by the API when `SPA_DIST_DIR` is
  set, and by Caddy on the deployed host through `deploy/caddy/Caddyfile`.
- The browser routes that `src/router.rs` declares, listed with their access tiers in the
  [source root README](/apps/frontend/src/README.md).

## Boundaries

- Depends on:
  - the map crates in the table above;
  - the API over HTTP, for `/api/v1` and `/map-assets`;
  - `leptos` and `leptos_router` 0.8 in client-side rendering, `gloo-net`, `web-sys`, `js-sys`,
    `wasm-bindgen`, `serde`, `serde_json`, `futures`, `url`, `base64` and
    `console_error_panic_hook`, and in the browser build `idb`, `gloo-timers` and
    `wasm-bindgen-futures`;
  - Trunk and Tailwind CSS at build time, and the Material Symbols Outlined font that
    `index.html` loads from Google Fonts at run time;
  - in tests, the recorded API responses in `contracts/fixtures/api_goldens/` that the golden
    and page tests embed, the API's route tables in `apps/api/src/` and the shared URL case table
    `crates/foundation/http_url_guard/src/cases.rs`.
- Used by:
  - the `mk leptos`, `mk leptos-debug`, `mk leptos-build`, `mk ci-local-leptos` and
    `mk leptos-gates` recipes of `tools/xtask/`, `cargo xtask ci ci-local` through
    `ci-local-leptos`, and `cargo xtask deploy website`, which runs `trunk build --release` on the
    deploy host that `TBD_SSH_HOST` names in `deploy/deploy.env`;
  - the `frontend` job of `.github/workflows/ci.yml` and the editor gates of
    `.github/workflows/editor-gates.yml`;
  - the headless browser gates in `tools/browser_testing/browser_gate_suites/`, which serve
    `dist/`, read `src/router.rs` and answer the app's requests from
    `contracts/fixtures/api_goldens/`;
  - the API, whose `SPA_DIST_DIR` serves `dist/`.
- Rules:
  - the crate compiles natively as well as for `wasm32`, and `cargo xtask mk ci-local-leptos` is
    the gate CI runs: `cargo fmt --check`, clippy for `wasm32-unknown-unknown` over all targets,
    `cargo test -p frontend`, and a release Trunk build;
  - the captures in `contracts/fixtures/api_goldens/` are taken from a fresh database seeded with
    `crates/api/api_database/seeds/registry_dev.sql` and then
    `crates/api/api_database/seeds/content_golden.sql`, by the recipe that closes
    `content_golden.sql`, and every golden reproduces from the seed through that recipe: the reads
    in `_index.tsv` order, then its writes in index order, each sending its `*.request.json` body
    when it has one;
  - every production file under `src/` passes the documentation audit of
    `src/tests/doc_audit/mod.rs`.

## Related documentation

- [Frontend documentation](/documentation/apps/frontend/README.md) — the route table: each
  route with its code folder and feature doc, and the page areas and workspaces.
- [Local development](/documentation/runbooks/local_development.md) — the full local setup,
  Discord sign-in included.
- [Editor gates](/documentation/runbooks/editor_gates.md) — running the headless editor gates
  and their environment.
- [Website deployment](/documentation/runbooks/website_deployment.md) — building and serving
  the bundle on the home server.
- [Design tokens](/documentation/design_system/design_tokens.md) — the design token reference.
