# T-165.5 verify log — CDP harness core → Rust

Scope: the reusable harness (`cdp` + `serve` + `inject` + `vsuite` + `sroutes` in
`tools/tbd-tools`) + the `gate` bin. The Node driver stays untouched this slice (deleted @
T-165.6 after the smokes port); this slice proves the Rust harness on the V lane.

## Ported

| Node | Rust | Notes |
|---|---|---|
| `driver/cdp.mjs` (226 LOC) | `tbd_tools::cdp` | tokio-tungstenite WS; chromium via `CHROME_HEADLESS_SHELL` → `~/.cache/ms-playwright` scan (same order); identical flag set (SwiftShader WebGL2 + lavapipe WebGPU); request-id → oneshot map, one-shot event waiters + persistent handlers (mpsc); evaluate (`returnByValue`, 120 s), navigate (load-waiter registered BEFORE `Page.navigate`), waitFor, screenshot, Input dispatch, Fetch fulfill/continue; SIGTERM kill via libc (tokio `Child::kill` would SIGKILL) |
| `driver/serve.mjs` (122 LOC) | `tbd_tools::serve` | axum fallback handler: COOP `same-origin` + COEP `credentialless` + `no-store` on every response; extension-less → index.html SPA fallback; traversal guard; `/map-assets/` passthrough; `/api/` same-origin proxy (reqwest, host rewritten) |
| `driver/freeze.js` / `driver/dom.js` | `tbd_tools::inject` | **verbatim consts** (post template-literal unescape: `\\s` → `\s`); `payloads_match_node_driver_sources` test pins byte-equality vs the .mjs sources while they exist |
| `driver/gate_v_suite.mjs` (290 LOC) | `tbd_tools::vsuite` + `gate v-suite` | 25-route table; localStorage seed (preserve_order key order); fixture interception incl. refresh/logout/fallback-`{}`; SETTLE + stability loop (two consecutive identical serializations); diffNode (entry-cap 40, JS Set-insertion key union, `JSON.stringify`-undefined-key omission); dist identity; manifest freeze/accept |
| `manifests/extract-leptos-routes.mjs` (54 LOC) | `tbd_tools::sroutes` + `gate s-routes` | RouteDef block regex; sorted CSV vs `routes.csv`; row-level diff report |

## Parity (side-by-side, same Leptos dist, sequential runs)

- **`gate s-routes`** vs `node extract-leptos-routes.mjs`: stdout byte-identical
  (`{"gate":"S-routes","pass":true,"routes":26}`), rc 0/0.
- **`gate v-suite verify --leptos-dir apps/website-leptos/dist`** vs
  `node gate_v_suite.mjs verify`: **25/25 PASS, stdout BYTE-IDENTICAL** — every per-route
  line incl. byte counts (`diff` = empty). Two fixes were required to get to byte-identity,
  both JS-semantics shims: `js_len` (JS `String.length` = UTF-16 code units — 13 routes
  carry non-ASCII, e.g. 97827 UTF-16 vs 97831 UTF-8 on dashboard) and reqwest TLS drop
  (`rustls-no-provider` panics on `Client::new`; the harness is loopback-plain-HTTP only).
- **Negative probe** (golden corruption: `notfound.dom.json` `"404"` → `"405"`): node rc=1,
  rust rc=1, failure stdout **byte-identical** (FAIL line + `1` diff + the pretty-printed
  failure row incl. `first[0]` path `approot/main[1]/section[0]/h1[1]/text[0]`); golden
  restored, re-run rc=0.
- `cargo test -p tbd-tools`: 3/3 (incl. the inject byte-parity pin).
- `cargo clippy -p tbd-tools --all-targets -- -D warnings` rc=0 · `cargo fmt --check` rc=0
  · `./scripts/ticket check` OK.

## Notes

- The frozen goldens under `.ai/artifacts/t159_gates/v/oracle-freeze/` are untouched — the
  Rust `verify` reproduces the Node verdicts against the exact committed bytes.
- `freeze` mode is ported for provenance but its default `--oracle-dir`
  (`apps/website/frontend/dist`) no longer exists post T-159.29.3.
- `gate serve` replaces the `node serve.mjs --dir … --port …` CLI (Ctrl-C to stop).
- Makefile `leptos-gates` still drives the Node runner — the flip + driver deletion is
  T-165.6 acceptance (both harnesses green on the same dist, then `git rm driver/`).
