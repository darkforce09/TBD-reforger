# Request-shape primitives

Query and path parameters and parsing rules that are the same whichever resource a route serves:
the offset pagination that list endpoints share, and the path-parameter extractor every route
takes.

## Contents

```text
crates/api/api_foundation/src/http/
├── mod.rs                  the module tree
├── pagination.rs           `PageParams`: the `limit` and `offset` query parameters and their clamping rule
├── path_parameters.rs      `PathParams`: axum's `Path` with its rejection answered in the error envelope
├── required_text_field.rs  `required_trimmed_text`: a required text field, trimmed, refused with `400` when blank
└── tests/                  unit tests for the path-parameter extractor, its rejection mapping and the required text field
```

## How it works

A list handler extracts `Query<PageParams>` and calls `PageParams::bounds` for its `LIMIT` and
`OFFSET`. `limit` defaults to 20 and must lie between 1 and 100; `offset` defaults to 0 and must
not be negative. A value outside its range falls back to the default rather than failing, so a
malformed page link still renders a page.

Every handler that reads a path segment takes `PathParams<T>` in place of axum's `Path<T>`, in the
same argument position, and destructures it the same way (`PathParams(id): PathParams<Uuid>`). It
decodes exactly as `Path` does and hands a refusal to `ApiError::from_path_rejection`: a segment
that does not decode into `T` (a non-UUID id, a segment that is not UTF-8) answers `400` in the
`{error, details?}` envelope with the message `invalid path parameter: <reason>`, whose reason
names the parameter, and an extractor that does not match its route's parameters answers a logged
`500 internal error`. One extractor serves the whole API because axum's rejection already names
the refused parameter, so no handler needs a name of its own the way a query rejection does.

A write body's required text field (a modpack's name and version, a server's name) goes through
`required_trimmed_text`, which trims it and answers `400` with `<field> is required` when nothing
is left; the trimmed value is the one stored, so reads and writes agree.

## Boundaries

- Depends on: `serde`; `axum` and `api_foundation::error_handling` for the path extractor; the tests
  drive real routes through `tower`'s `ServiceExt::oneshot`.
- Used by: the list handlers of `api_administration` (audit logs, personnel roster),
  `api_community_content` (announcements, public and administrative), `api_missions` (approval queue,
  [mission deployments](/documentation/glossary/g_to_m.md#mission-deployment)) and `api_operations`
  ([event](/documentation/glossary/a_to_f.md#event) listing, leave requests,
  [ORBAT](/documentation/glossary/n_to_z.md#orbat) view).
- Used by (`PathParams`): every handler of the eight domains that reads a path segment.
- Used by (`required_trimmed_text`): the modpack writes of `api_community_content` and the server
  registry writes of `api_server_infrastructure`.
- Rules: a list endpoint that pages takes `PageParams` instead of parsing its own `limit` and
  `offset`, so every list clamps the same way; a handler reads its path through `PathParams`, never
  through axum's `Path`, so no route answers axum's plain-text rejection
  (`tests/path_parameters.rs`).
