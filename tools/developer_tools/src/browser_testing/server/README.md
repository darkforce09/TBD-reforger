# Gate server API corpus

The recorded API corpus route of the gate server in
`tools/developer_tools/src/browser_testing/server.rs`: with `ServeConfig::api_fixture_corpus`
set, every `/api/` request is answered from a directory of recorded response bodies instead of an
upstream, so a gate can serve the app's public reads from a real HTTP origin.

## Contents

```text
tools/developer_tools/src/browser_testing/server/
└── api_fixture_corpus.rs  `corpus_file_name`, the API-down switch and the route that answers `/api/` from the corpus
```

## How it works

```text
GET /api/v1/ballistics-catalogs/vanilla_mortars/versions/1?x
      ├▶ corpus marked down (set_api_down) ─▶ 502, empty body (a proxy whose API is down)
      └▶ corpus_file_name ─▶ GET__ballistics-catalogs__vanilla_mortars__versions__1.json
            present ─▶ 200, the file's bytes, application/json, the server's isolation headers
            absent, any other method, or an unsafe name ─▶ 404 {"error": {"code": "not_found", …}}
```

The file name is the method, `__`, and the path after `/api/v1/` with its trailing slash dropped
and every `/` replaced by `__`, plus `.json`: the naming of the frontend's recorded API corpus in
`apps/website/frontend/tests/fixtures/api/`, which the DOM oracle reads through request
interception. A name holding anything but ASCII letters, digits, `-`, `_` and `.`, or holding `..`,
is refused, so no request reaches outside the directory. With a corpus set, `/api/` never falls
through to the API proxy or to the single-page fallback.

`set_api_down(corpus, true)` makes every `/api/` request of that corpus answer `502 Bad Gateway`
with an empty body, as Caddy does in production while the API behind it is stopped; the app and
`/map-assets/` are still served. The switch is process-wide but keyed by corpus directory, so a
server over another corpus is never affected; `is_api_down` reads it.

## Boundaries

- Depends on: the parent's `respond`, which adds the cross-origin isolation headers; `axum`,
  `tokio` and `serde_json`.
- Used by: `server.rs`, which consults it before the proxy; `gate mortar-offline`
  (`tools/developer_tools/src/browser_testing/mortar_offline/`), which serves the recorded
  catalog reads through it, names them with `corpus_file_name` and marks the API down for its
  proxy step.
- Rules: the route and the naming are pinned by
  `tools/developer_tools/src/browser_testing/tests/server/api_fixture_corpus.rs`.

## Related documentation

- [Headless browser gates and captures](/tools/developer_tools/src/browser_testing/README.md) —
  the gate server and the gates that use it.
