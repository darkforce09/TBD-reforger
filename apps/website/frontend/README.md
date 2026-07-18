# website-frontend — the Leptos SPA (T-159 rewrite)

The Rust/wasm (Leptos CSR) website SPA — the ONLY frontend since T-159.29.3 (the React app
is deleted; the crate lives at `apps/website/frontend/` since T-171). This crate talks
to the existing API on `:8080` and, from T-159.15, links `map-engine-core`/`map-engine-render`
directly (the wasm-boundary collapse).

## Toolchain

- Rust stable (workspace pins 1.95.0) with the `wasm32-unknown-unknown` target
  (`rustup target add wasm32-unknown-unknown`).
- [Trunk](https://trunkrs.dev) 0.21+ (`cargo install trunk` or a prebuilt binary). Trunk
  auto-downloads the matching `wasm-bindgen` on first build.
- [tailwindcss](https://tailwindcss.com) v4 standalone binary on PATH — Trunk runs it for the
  `data-trunk rel="tailwind-css"` link; no npm / node_modules. Aegis tokens live in `style/aegis.css`
  (`@theme`), ported byte-for-byte from the React `index.css`.

## Dev run

```bash
cd apps/website/frontend
trunk serve            # http://127.0.0.1:3000  (COOP/COEP headers from Trunk.toml)
trunk build            # one-off build → dist/  (add --release for wasm-opt)
```

The API stays on `:8080` (`make api` at the repo root); dev-login works as today.

## Verify (G gate)

```bash
# native check keeps the workspace root build green (main() is wasm32-gated):
cargo check -p website-frontend
# wasm build pipeline:
cd apps/website/frontend && trunk build
# render proof — the wasm actually mounts + renders in a real headless browser
# (T-165.6: the Rust CDP harness; run from the repo root):
cargo run -q -p tbd-tools --bin gate -- render-check \
  --dir apps/website/frontend/dist --path / --expect "COMMAND CENTER"
```

## Status

- **T-159.1** — scaffold: CSR hello, `Trunk.toml`, workspace member, wasm mount verified in-browser.
- Next: **T-159.2** Aegis CSS + shell chrome (`AppLayout`/`Sidebar`/`TopNav`), first V-shell gate.
