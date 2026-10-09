# Frontend shell crates

The shell layer of the frontend crates, the top of their layer order: the single-page app that
links every frontend crate below it and mounts the platform frame and the route table, and the
offline service worker that Trunk builds from the app's `index.html` and the page registers beside
it.

## Contents

```text
crates/frontend/shell/
├── frontend_application/    `frontend_application`: the single-page app binary: start function, route table, app frame, Trunk build files
└── offline_service_worker/  `offline_service_worker`: the offline service worker binary: cache policy and byte ranges in the browser
```

## How it works

The two crates are peers in one layer, and neither depends on the other. They meet only in the
browser build: Trunk reads `frontend_application/index.html`, builds the app's binary into
`frontend_application-<hash>.js` and `frontend_application-<hash>_bg.wasm`, and builds the worker
from `../offline_service_worker/Cargo.toml` as a `no-modules` worker into
`offline_service_worker.js` and `offline_service_worker_bg.wasm`, with no hash. The page registers
the app's loader `/service_worker.js?build=<id>`, which imports the worker; `<id>` is the hash of
the app's bundle names, and both sides derive their cache names from it through
`offline_cache_policy`.

```text
frontend_application/index.html ─▶ trunk ─┬─▶ frontend_application binary ─▶ frontend_application-<hash>.js, _bg.wasm
                                          └─▶ offline_service_worker binary ─▶ offline_service_worker.js, _bg.wasm
browser ─▶ start_app ─▶ register /service_worker.js?build=<hash> ─▶ importScripts(offline_service_worker.js)
```

Both are binaries with no library. Their browser code exists on `wasm32` alone: the app's mount
chain and the worker's event handlers sit in `cfg(target_arch = "wasm32")` code and dependency
tables, so on the host the app builds its frame's pure half with its tests and the worker an
empty `main`.

## Public surface

- `frontend_application`: the WebAssembly start function `start_app` and the built bundle in its
  `dist/`; see its README.
- `offline_service_worker`: the exports `on_install`, `on_activate` and `on_fetch`, which the
  loader `frontend_application/service_worker.js` calls; see its README.

## Boundaries

- Depends on: the app on every frontend crate of the lower layers (foundation, features, pages and
  workspaces); the worker on `offline_cache_policy` and the browser bindings alone.
- Used by: no crate; Trunk builds both from `frontend_application/index.html`, and the deploy, the
  CI lanes and the browser gates build or serve that bundle.
- Rules: the two crates never depend on each other, in any dependency table (the shell crate
  order of `cargo xtask verify frontend-layering`); the worker links no graphics, map rendering,
  paper doll or streaming crate and not `wgpu` (the crate firewalls of
  `cargo xtask verify crate-tiers`); only these two crates and `browser_platform` carry a
  `#[wasm_bindgen]` export (the crate firewalls).

## Related documentation

- [Frontend crates](/crates/frontend/README.md) — the layer order every frontend crate follows.
- [Frontend documentation](/documentation/crates/frontend/shell/frontend_application/README.md) —
  the route table: each route with its code folder and feature doc.
