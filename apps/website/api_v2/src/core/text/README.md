# Text handling

Text rules the whole [API](/documentation_v2/glossary.md#api) shares: the guard every stored URL
passes before it is written, and the HTML sanitizer with the preview helpers that shorten text for
lists and Discord messages.

## Contents

```text
apps/website/api_v2/src/core/text/
├── html_sanitizer.rs  `sanitize_html`, and the `snippet`, `truncate` and `cap_runes` preview helpers
├── http_url_guard.rs  `is_http_url`: whether a string is an absolute `http` or `https` URL
├── mod.rs             the module tree
└── tests/             unit tests, among them the URL cases shared with the single-page app
```

## How it works

`is_http_url` accepts an absolute URL whose scheme is `http` or `https` and whose host is not
empty, and refuses anything holding an ASCII control character or surrounding whitespace, so the
bytes checked are the bytes a browser will follow. A handler that stores a URL (an announcement
thumbnail, an [event](/documentation_v2/glossary.md#event) banner, a
[mission](/documentation_v2/glossary.md#mission) thumbnail, a Discord avatar, a match replay)
refuses a failing value instead of storing it. The guard is not a server-side request forgery
check: a loopback or metadata address passes.

`snippet` collapses whitespace and cuts to a number of characters, `truncate` cuts and appends
`…`, and `cap_runes` cuts so that the result, ellipsis included, never exceeds the cap.
`sanitize_html` cleans HTML with `ammonia` for a field that renders HTML; announcement bodies are
stored as authored plain text and never pass through it, because the single-page app renders them
as text.

## Boundaries

- Depends on: `url` and `ammonia`; its tests read the case table
  `apps/website/shared/is_http_url_cases.rs`, which the single-page app's own guard in
  `apps/website/frontend/src/v2/core/auth/url_guard.rs` is tested against too.
- Used by: `community_content` (the announcement thumbnail check, the previews and the Discord
  webhook's caps), `identity_and_access` (the Discord avatar), `match_telemetry` (match replay
  links), `missions` (the thumbnail validation) and `operations` (event banners), and integration
  suites under `apps/website/api_v2/tests/`.
- Rules: `http` and `https` stay an allowlist, never a denylist of bad schemes; both guards agree on
  every shared case (`matches_the_frontend_guard_on_every_shared_case` in `tests/http_url_guard.rs`);
  announcement bodies are never sanitized (`apps/website/api_v2/tests/cms_announcement_body.rs`).
