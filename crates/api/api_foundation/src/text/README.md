# Text handling

Text rules the whole [API](/documentation/glossary/a_to_f.md#api) shares: the policy for the links
and images of authored content, and the HTML sanitizer with the preview helpers that shorten text
for lists and Discord messages. The guard every stored URL passes before it is written,
`is_http_url`, lives in the `http_url_guard` crate (`crates/foundation/http_url_guard/`).

## Contents

```text
crates/api/api_foundation/src/text/
├── content_url_policy.rs  which link targets and image sources authored content may carry
├── html_sanitizer.rs      `sanitize_html` and the `snippet`, `truncate`, `cap_runes` previews
├── mod.rs                 the module tree
└── tests/                 unit tests of the content URL policy and the sanitizer
```

## How it works

`http_url_guard::is_http_url` accepts an absolute URL whose scheme is `http` or `https` and whose
host is not empty, and refuses anything holding an ASCII control character or surrounding
whitespace, so the bytes checked are the bytes a browser will follow. Every writer of a URL column
calls it and never stores a failing value (the avatar writer stores `""` in its place, the others
refuse the request):

| Column | Writer |
|---|---|
| `announcements.thumbnail_url` | `community_content/handlers/announcements_admin.rs` |
| `events.banner_image_url` | `operations/services/event_authoring/event_creation.rs` |
| `missions.thumbnail_url` | `missions/validation/mission_fields.rs` |
| `users.avatar_url` | `identity_and_access/services/account_registration.rs`, over the avatar URL of the Discord profile |
| `matches.aar_replay_url` | `match_telemetry/models/match_results_revision.rs`, in the decoder before the transaction opens; an empty value means no replay and is tested before the guard |

The guard is not a server-side request forgery check: a loopback or metadata address passes.

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

- Depends on: `ammonia`; its tests compile the link and image patterns of `contracts/definitions/wiki-page.schema.json` and
  `contracts/definitions/vehicle-database.schema.json` with `regress`.
- Used by: `api_community_content` (the previews and the Discord webhook's caps, and the content URL
  policy for the vehicle database image and the wiki markup) and integration suites under
  `crates/api/api_server/tests/`.
- Rules: every URL column in the table above is written only after `is_http_url` passes; the
  content URL policy accepts exactly what the contract patterns accept
  (`agrees_with_the_contract_patterns_on_every_character_position` in
  `tests/content_url_policy.rs`), so a pattern change and a policy change land together;
  announcement bodies are never sanitized (`crates/api/api_server/tests/cms_announcement_body.rs`).
