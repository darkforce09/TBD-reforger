# Text handling

Text rules the whole [API](/documentation/glossary/a_to_f.md#api) shares: the guard every stored URL
passes before it is written, the policy for the links and images of authored content, and the HTML
sanitizer with the preview helpers that shorten text for lists and Discord messages.

## Contents

```text
apps/website/api_v2/src/core/text/
├── content_url_policy.rs  which link targets and image sources authored content may carry
├── html_sanitizer.rs      `sanitize_html` and the `snippet`, `truncate`, `cap_runes` previews
├── http_url_guard.rs      `is_http_url`: whether a string is an absolute `http` or `https` URL
├── mod.rs                 the module tree
└── tests/                 unit tests, among them the URL cases shared with the single-page app
```

## How it works

`is_http_url` accepts an absolute URL whose scheme is `http` or `https` and whose host is not
empty, and refuses anything holding an ASCII control character or surrounding whitespace, so the
bytes checked are the bytes a browser will follow. Code that stores a URL (an announcement
thumbnail, an [event](/documentation/glossary/a_to_f.md#event) banner, a
[mission](/documentation/glossary/g_to_m.md#mission) thumbnail, a Discord avatar, the
`aar_replay_url` of a match results revision, checked by its decoder before the transaction opens)
refuses a failing value instead of storing it. The guard is not a server-side request forgery
check: a loopback or metadata address passes.

The content URL policy judges the targets inside authored content: the links and images of wiki
markup and the vehicle database image. An image source is `https://` followed by a host, or a site
path `/…` (never the protocol-relative `//…`); a link target may also be `http://…`, `mailto:…`, a
`#fragment` or the site root `/`. Everything else is refused, `javascript:`, `data:`, `vbscript:`
and `file:` included, and so is any value holding a backslash, a control character or whitespace
anywhere. The prefixes match in lowercase only, so `HTTPS://` is refused rather than normalised.
`is_external_link` is true for the safe absolute `http`, `https` and `mailto` targets. The rule is
the same one the `href`, `src` and `profile_image_url` patterns of
`contracts/definitions/wiki-page.schema.json` and
`contracts/definitions/vehicle-database.schema.json` state for the wire.

`snippet` collapses whitespace and cuts to a number of characters, `truncate` cuts and appends
`…`, and `cap_runes` cuts so that the result, ellipsis included, never exceeds the cap.
`sanitize_html` cleans HTML with `ammonia` for a field that renders HTML; announcement bodies are
stored as authored plain text and never pass through it, because the single-page app renders them
as text.

## Boundaries

- Depends on: `url` and `ammonia`; its tests read the case table
  `apps/website/shared/is_http_url_cases.rs`, which the single-page app's own guard in
  `apps/website/frontend/src/v2/core/auth/url_guard.rs` is tested against too, and compile the
  link and image patterns of `contracts/definitions/wiki-page.schema.json` and
  `contracts/definitions/vehicle-database.schema.json` with `regress`.
- Used by: `community_content` (the announcement thumbnail check, the previews and the Discord
  webhook's caps, and the content URL policy for the vehicle database image and the wiki markup),
  `identity_and_access` (the Discord avatar), `match_telemetry` (the results
  revision's `aar_replay_url`), `missions` (the thumbnail validation) and `operations` (event banners), and integration
  suites under `apps/website/api_v2/tests/`.
- Rules: `http` and `https` stay an allowlist, never a denylist of bad schemes; both guards agree on
  every shared case (`matches_the_frontend_guard_on_every_shared_case` in `tests/http_url_guard.rs`);
  the content URL policy accepts exactly what the contract patterns accept
  (`agrees_with_the_contract_patterns_on_every_character_position` in
  `tests/content_url_policy.rs`), so a pattern change and a policy change land together;
  announcement bodies are never sanitized (`apps/website/api_v2/tests/cms_announcement_body.rs`).
