# Offline service worker binaries

The crate's one binary, the service worker Trunk builds next to the single-page app.

## Contents

```text
apps/website/offline-service-worker/src/bin/
└── offline_service_worker/  the service worker: install, activate and fetch handlers
```

## Commands

- `offline_service_worker`: no command line. Trunk builds it for `wasm32-unknown-unknown` from
  `apps/website/frontend/index.html` (`data-type="worker"`, `data-bindgen-target="no-modules"`)
  into `offline_service_worker.js` and `offline_service_worker_bg.wasm`; the browser runs it
  through `apps/website/frontend/service_worker.js`. The native build is an empty `main` that
  exits 0.

## Boundaries

- Depends on: the crate's library, `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`, `web-sys`.
- Used by: `apps/website/frontend/index.html` and `apps/website/frontend/service_worker.js`.
- Rules: every handler module is compiled only on `wasm32`, so workspace builds and host lints
  compile the binary with no browser.
