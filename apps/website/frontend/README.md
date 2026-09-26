# Website frontend

The `website-frontend` crate: the web platform's single-page app, written in Rust with Leptos 0.8
and rendered in the browser as WebAssembly. It holds every page members use, the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator), and the client that talks to the
[API](/documentation_v2/glossary/a_to_f.md#api); Trunk builds it into a static bundle that the dev server,
the API or the deployed host serves.

## Contents

```text
apps/website/frontend/
├── .gitignore   keeps the build output folders `dist/` and `dist-debug/` out of git
├── Cargo.toml   the `website-frontend` package: one binary, and the map engine's features per target
├── index.html   the Trunk entry: the wasm build, the stylesheet link, the icon font, the dark theme
├── src/         the entry point, the route table and the `v2/` tree of pages, workspaces and core
├── style/       the Aegis stylesheet Tailwind compiles into the build
├── tests/       the captured API responses in `fixtures/api/` that the golden and page tests embed
└── Trunk.toml   the build, the dev server on 127.0.0.1:3000 and its proxy to the API
```

## How it works

Trunk reads `index.html`: it builds the crate's binary for `wasm32-unknown-unknown`, runs
`wasm-bindgen`, and in a release build shrinks the module with `wasm-opt` at the level
`data-wasm-opt="z"` asks for; it compiles `style/aegis.css` with Tailwind CSS, and writes the page,
the module, its JavaScript glue and the stylesheet into `dist/`. In the browser, the start
function in `src/main.rs` mounts the app layout under a router, the layout restores a stored
session, and the route table in `src/app_routes.rs` and `src/router.rs` picks the page.

```text
index.html ─▶ trunk ─┬─▶ cargo build --target wasm32-unknown-unknown ─▶ wasm-bindgen
                     │       ─▶ wasm-opt at level z, release builds only ──────────▶ dist/
                     └─▶ tailwindcss style/aegis.css ─────────────────────────────▶ dist/
browser ─▶ dist/index.html ─▶ start_app ─▶ AppLayout ─▶ route ─▶ page or workspace
                                               └─▶ /api/v1 and /map-assets on the same origin
```

The app calls the API on its own origin, under `/api/v1`, and the map engine loads terrain from
`/map-assets` on the same origin, so whatever serves the bundle also serves or proxies those
paths: Trunk's proxy in development, the API itself when its `SPA_DIST_DIR` points at `dist/`, and
Caddy on the deployed host. The dependency on the engines runs one way,
`website-frontend ─▶ website-map-engine ─▶ website-graphics-engine`: the app hands the map engine
a canvas and never names the graphics engine.

The binary's `main` is empty and `start_app` exists only on `wasm32`, so the crate also compiles
for the host, where `cargo test -p website-frontend` runs every test of the native half: the route
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
`wasm-bindgen` 0.2.126 in `tools_v2/developer-tools/gate-env.json`, and Trunk fetches the
`wasm-bindgen` and Tailwind CSS versions it needs on its first build, with no npm involved.
`cargo xtask mk leptos-debug` serves an unoptimised build that rebuilds faster but whose frame
rates mean nothing, and `cargo xtask mk leptos-build` writes a release build into `dist/` without
serving it. `cargo run -q -p developer-tools --bin gate -- render-check --dir apps/website/frontend/dist`
checks that a built bundle mounts and renders in a headless browser.

To sign in without Discord through the [dev login](/documentation_v2/glossary/a_to_f.md#dev-login), open
`/api/v1/auth/dev-login?role=admin` on the host `FRONTEND_URL` names (`http://localhost:3000` in
`apps/website/api_v2/.env.example`); the proxy hands the redirect to `/auth/callback` back
unfollowed, so the session in its URL fragment reaches the app.

## Configuration

The app reads no environment variable: the settings are the build files'.

| Setting | Value | Read by |
|---|---|---|
| map engine features, every build | `world`, `io`, `store`, `editing`, without the defaults | Cargo, from `Cargo.toml` |
| map engine features, browser build | adds `render` and `streaming` (the `wasm32` target table) | Cargo, from `Cargo.toml` |
| map engine features, native tests | adds `streaming` (dev-dependencies) | Cargo, from `Cargo.toml` |
| `[build]` | `target = "index.html"`, output `dist` | Trunk, from `Trunk.toml` |
| `[tools]` | `tailwindcss = "4.3.2"` | Trunk |
| `[watch]` | ignores `dist` and `style/aegis.css`, the paths the build itself writes into, so a build never triggers the next | Trunk |
| `[serve]` | `127.0.0.1:3000`, with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: credentialless` for cross-origin isolation | Trunk |
| `[[proxy]]` | `/api` to `http://127.0.0.1:8080/api` with `no_redirect = true`; `/map-assets` to `http://127.0.0.1:8080/map-assets` | Trunk |
| `data-wasm-opt` | `z`, the size-first `wasm-opt` level of a release build | Trunk, from `index.html` |
| API root | `/api/v1` on the page's origin, `API_BASE` | the client's verbs, from `src/v2/core/api/client/mod.rs` |
| stored session | the `tbd-auth` key of local storage; `tbd-auth-refresh` names the Web Lock and the broadcast channel of a token refresh | `src/v2/core/auth/session.rs` and `src/v2/core/api/client/refresh.rs` |

The [mission](/documentation_v2/glossary/g_to_m.md#mission) store and compiler are in the native build
too, so `cargo test` checks the metadata the Mission Creator's export compiles from
(`compiled_meta_is_the_row_the_server_compiles_from` in
`src/v2/core/api/dto/tests/r_api_missions.rs`). The token refresh needs the Web Locks API, which
only a secure context offers: `localhost`, `127.0.0.1` or HTTPS.

## Public surface

- The `website-frontend` binary, with no library: `start_app` in `src/main.rs` is its WebAssembly
  start function, and no other crate links it.
- The bundle in `dist/`, served by `trunk serve` in development, by the API when `SPA_DIST_DIR` is
  set, and by Caddy on the deployed host through `tools_v2/xtask/deploy/Caddyfile.website`.
- The browser routes that `src/router.rs` declares, listed with their access tiers in the
  [source root README](/apps/website/frontend/src/README.md).
- The captured responses in `tests/fixtures/api/`, which the API's contract tests and the browser
  gates read too.

## Boundaries

- Depends on:
  - `website-map-engine`, with the features above, and never `website-graphics-engine`;
  - the API over HTTP, for `/api/v1` and `/map-assets`;
  - `leptos` and `leptos_router` 0.8 in client-side rendering, `gloo-net`, `web-sys`, `js-sys`,
    `wasm-bindgen`, `serde`, `serde_json`, `futures`, `url`, `base64` and
    `console_error_panic_hook`, and in the browser build `idb`, `gloo-timers` and
    `wasm-bindgen-futures`;
  - Trunk and Tailwind CSS at build time, and the Material Symbols Outlined font that
    `index.html` loads from Google Fonts at run time;
  - in tests, the API's route tables in `apps/website/api_v2/src/` and the shared URL case table
    `apps/website/shared/is_http_url_cases.rs`.
- Used by:
  - the `mk leptos`, `mk leptos-debug`, `mk leptos-build`, `mk ci-local-leptos` and
    `mk leptos-gates` recipes of `tools_v2/xtask/`, `cargo xtask ci ci-local` through
    `ci-local-leptos`, and `cargo xtask deploy website`, which runs `trunk build --release` on the
    deploy host that `TBD_SSH_HOST` names in `tools_v2/xtask/deploy/deploy.env`;
  - the `website-frontend` job of `.github/workflows/ci.yml` and the editor gates of
    `.github/workflows/editor-gates.yml`;
  - the headless browser gates in `tools_v2/developer-tools/src/browser_testing/`, which serve
    `dist/`, read `src/router.rs` and answer the app's requests from `tests/fixtures/api/`;
  - the API, whose `SPA_DIST_DIR` serves `dist/`, and whose contract tests in
    `apps/website/api_v2/tests/` read `tests/fixtures/api/`.
- Rules:
  - the crate compiles natively as well as for `wasm32`, and `cargo xtask mk ci-local-leptos` is
    the gate CI runs: `cargo fmt --check`, clippy for `wasm32-unknown-unknown` over all targets,
    `cargo test -p website-frontend`, and a release Trunk build;
  - no file imports `website_graphics_engine` (`cargo xtask verify engine-layers`);
  - the captures in `tests/fixtures/api/` are taken from a database seeded with
    `apps/website/api_v2/seeds/content_golden.sql`, by the recipe that closes that file, and every
    one but `GET__registry.json` reproduces that way, since
    `apps/website/api_v2/seeds/registry_dev.sql` leaves the ids of the
    [registry](/documentation_v2/glossary/n_to_z.md#registry) items to Postgres;
  - every production file under `src/v2/` passes the documentation audit of
    `src/v2/tests/doc_audit/mod.rs`.

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md) — the route table: each
  route with its code folder and feature doc, and the page areas and workspaces.
- [Local development](/documentation_v2/runbooks/local_development.md) — the full local setup,
  Discord sign-in included.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running the headless editor gates
  and their environment.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — building and serving
  the bundle on the home server.
- [Design tokens](/documentation_v2/design_system/design_tokens.md) — the design token reference.
