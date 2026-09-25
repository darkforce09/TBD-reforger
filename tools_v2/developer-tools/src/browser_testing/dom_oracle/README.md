# DOM oracle gate

The body of `gate v-suite`, declared in
`tools_v2/developer-tools/src/browser_testing/dom_oracle.rs`: the route list, the per-route capture
against the committed [API](/documentation_v2/glossary.md#api) fixture corpus, the request router
that feeds it, and the verify and accept modes that compare the capture with the frozen goldens.

## Contents

```text
tools_v2/developer-tools/src/browser_testing/dom_oracle/
├── fixture_router.rs  what each intercepted request receives: a fixture, a canned reply or a miss
├── routes.rs          the 25 routes, the auth seed, `capture_route`, `diff_node` and `run`
└── run_modes.rs       the verify and accept loops over the selected routes
```

## How it works

`run` (`gate v-suite <verify|accept> [--leptos-dir <dist>] [--only <slug>] [--note <why>]`) checks
its arguments, launches one Chromium on debug port 9341 and hands the selected routes to
`run_modes`. The routes are the platform's leaf pages, 23 signed in with the admin seed and two
signed out (`login`, `callback`); the [mission](/documentation_v2/glossary.md#mission),
[event](/documentation_v2/glossary.md#event) and [ORBAT](/documentation_v2/glossary.md#orbat) pages
use the committed seed ids.

```text
route ──capture_route──▶ static server on the dist, fresh page with FREEZE_SRC,
                         DOM_SERIALIZER_SRC and the tbd-auth seed
        Fetch.requestPaused ──fixture_router::route──▶ fixture | canned | miss | passthrough
        settle loop: serialize '#root>:first-child' until two reads match (60 tries)
        ──▶ Capture { dom, png }
verify: diff_node(golden, capture), at most 40 differences ──▶ PASS / FAIL per route
accept: validate_accept_dom ──▶ <slug>.dom.json, <slug>.png, manifest.json row
```

`fixture_router.rs` maps a request to the corpus in `apps/website/frontend/tests/fixtures/api/`:
the method, two underscores, and the path after `/api/v1/` with every `/` as `__`, then `.json`
(served minified) or `.sse.txt` (served as `text/event-stream`); the query string never selects a
fixture. `/api/v1/auth/refresh` gets an access token from
`tools_v2/developer-tools/src/browser_testing/session_tokens.rs` and `/api/v1/auth/logout` an empty
object. A signed-in capture answers 401 to an API request that carries no bearer token, as the
API would, so the app's refresh installs the session. An API request with no fixture is let through
and recorded, and once the page settles the route fails with every unanswered URL and the file that
would answer it: a page that rendered without its data would settle into a stable error screen, and
that is not a baseline.

`verify` reports every route before it returns, and a route that cannot be captured counts as one
failure. `accept` needs `--only` and `--note`; it copies the existing golden to
`<slug>.react.dom.json` the first time, refuses a capture that is `null`, not JSON, or shorter than
`MIN_ACCEPT_DOM_JS_LEN` (256 UTF-16 units), and records the note, size and SHA-256 in
`manifest.json`. There is no whole-tree mode: `freeze` exits 2 because the goldens cannot be
regenerated from any dist the repository builds. `run` exits 0 when every selected route matches,
1 on any difference or missing golden, and 2 on a usage error.

## Boundaries

- Depends on: `tools_v2/developer-tools/src/browser_testing/cdp.rs`, `server.rs`,
  `session_tokens.rs`, and the `FREEZE_SRC` and `DOM_SERIALIZER_SRC` payloads of
  `fixture_injection.rs`; the fixture corpus in `apps/website/frontend/tests/fixtures/api/`; the
  goldens in `tools_v2/developer-tools/fixtures/dom_oracle/oracle-freeze/`.
- Used by: `dom_oracle.rs`, which re-exports `routes`, `capture_route`, `diff_node`, `js_len`,
  `validate_accept_dom`, `MissingFixture`, `run` and, to the crate, `seed_script`; `gate v-suite`
  in `tools_v2/developer-tools/src/browser_testing/cli.rs`, run as `gate v-suite verify` by
  `cargo xtask mk leptos-gates`; `seed_script` also seeds `gate render-check --seed-auth` and the
  doctor's liveness probe.
- Rules: the router answers nothing with a placeholder
  (`an_unanswered_api_call_is_reported_rather_than_filled_in` and the rest of
  `tools_v2/developer-tools/src/browser_testing/tests/dom_oracle/fixture_router.rs`); accept refuses
  an empty capture (`accept_refuses_literal_null`, `accept_refuses_undersized_object` in
  `tools_v2/developer-tools/src/browser_testing/tests/dom_oracle/tests.rs`); goldens change one
  route at a time, with a note.

## Related documentation

- [DOM oracle fixtures](/tools_v2/developer-tools/fixtures/dom_oracle/README.md) — the goldens,
  their manifest and the route table.
- [Editor gates](/documentation_v2/runbooks/editor_gates.md) — running `gate v-suite` and updating
  a reference.
