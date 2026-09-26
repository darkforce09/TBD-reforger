# Request-shape primitives

Query parameters and parsing rules that are the same whichever resource a route serves: the
offset pagination that list endpoints share.

## Contents

```text
apps/website/api_v2/src/core/http/
├── mod.rs         the module tree
└── pagination.rs  `PageParams`: the `limit` and `offset` query parameters and their clamping rule
```

## How it works

A list handler extracts `Query<PageParams>` and calls `PageParams::bounds` for its `LIMIT` and
`OFFSET`. `limit` defaults to 20 and must lie between 1 and 100; `offset` defaults to 0 and must
not be negative. A value outside its range falls back to the default rather than failing, so a
malformed page link still renders a page.

## Boundaries

- Depends on: `serde`.
- Used by: the list handlers of `administration` (audit logs, personnel roster),
  `community_content` (announcements, public and administrative), `missions` (approval queue,
  [mission deployments](/documentation_v2/glossary/g_to_m.md#mission-deployment)) and `operations`
  ([event](/documentation_v2/glossary/a_to_f.md#event) listing, leave requests,
  [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) view).
- Rules: a list endpoint that pages takes `PageParams` instead of parsing its own `limit` and
  `offset`, so every list clamps the same way.
