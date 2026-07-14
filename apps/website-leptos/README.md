# website-leptos — T-159 Leptos rewrite

The Rust/wasm (Leptos CSR) rewrite of the website SPA. Parallel lane: the React app under
`apps/website/frontend` stays the default build until the cutover slice (T-159.24). This crate talks
to the existing API on `:8080` and, from T-159.15, links `map-engine-core`/`map-engine-render`
directly (the wasm-boundary collapse).

## Toolchain

- Rust stable (workspace pins 1.95.0) with the `wasm32-unknown-unknown` target
  (`rustup target add wasm32-unknown-unknown`).
- [Trunk](https://trunkrs.dev) 0.21+ (`cargo install trunk` or a prebuilt binary). Trunk
  auto-downloads the matching `wasm-bindgen` on first build.

## Dev run

```bash
cd apps/website-leptos
trunk serve            # http://127.0.0.1:3000  (COOP/COEP headers from Trunk.toml)
trunk build            # one-off build → dist/  (add --release for wasm-opt)
```

The API stays on `:8080` (`make api` at the repo root); dev-login works as today.

## Verify (G gate)

```bash
# native check keeps the workspace root build green (main() is wasm32-gated):
cargo check -p website-leptos
# wasm build pipeline:
cd apps/website-leptos && trunk build
# render proof — the wasm actually mounts + renders in a real headless browser:
node ../../.ai/artifacts/t159_gates/driver/render-check.mjs \
  --dir apps/website-leptos/dist --path / --expect "TBD Reforger — Leptos"
```

## Status

- **T-159.1** — scaffold: CSR hello, `Trunk.toml`, workspace member, wasm mount verified in-browser.
- Next: **T-159.2** Aegis CSS + shell chrome (`AppLayout`/`Sidebar`/`TopNav`), first V-shell gate.
