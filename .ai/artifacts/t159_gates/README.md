# T-159 gate harness — run-output area

**T-171: this directory is pipeline OUTPUT only.** The committed goldens that used to live here
moved to their consumers (the T-171 fixture convention — fixtures live crate-local, never in
`.ai/artifacts/`):

| Corpus | New home | Consumer |
|--------|----------|----------|
| R-api response goldens (21 + `_index.tsv`) | `apps/website/frontend/tests/fixtures/api/` | `dto.rs` R-api cargo tests (`include_str!`) + `gate` smokes |
| S-gate manifests (`routes.csv`, `hooks.csv`, `components.csv`, `css_tokens.txt`, `deps.csv`) | `tools/tbd-tools/fixtures/t159/manifests/` | `gate s-routes` |
| V-suite frozen React DOM oracle (25 routes + `manifest.json`) | `tools/tbd-tools/fixtures/t159/oracle-freeze/` | `gate v-suite verify` / `accept` |

The oracle is **non-regenerable** (captured from the final React dist at T-159.29.1; the React
app was deleted at T-159.29.3). `gate v-suite freeze` was retired at T-171 for exactly that
reason — `apps/website/frontend/dist` is now the live Leptos dist, and a re-freeze would
overwrite the oracle. Route-level intentional divergence goes through
`gate v-suite accept --only <slug> --note "<why>"`.

What still lands here at run time (all gitignored): `v/<slice>/` capture artifacts
(`*.dom.json` / `*.png` diffs) and `logs/<slice>/gate_table.json` verify tables from
`make leptos-gates` (`gate editor-suite` + `gate v-suite verify`, `gate s-routes`,
`gate r-auth`, `gate render-check`, `gate smoke <name>` — all in `tools/tbd-tools`).

History: the Node driver (`driver/` — `cdp.mjs`, `freeze.js`, `dom.js`, `serve.mjs`,
`gate_v_suite.mjs`, the 15 `smoke_*_editor.mjs`) was ported to Rust at T-165.5/.6; the
freeze/dom payloads live as verbatim consts in `tools/tbd-tools/src/inject.rs`.
