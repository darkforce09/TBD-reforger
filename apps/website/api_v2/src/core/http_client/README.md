# Outbound HTTP retry

The retry policy for outbound calls that a remote service answers with `429 Too Many Requests`,
honouring its `Retry-After` within a bound.

## Contents

```text
apps/website/api_v2/src/core/http_client/
├── mod.rs           the module tree
├── retry_on_429.rs  `send_with_retry_on_429`: resends a request while the answer is 429
└── tests/           unit tests for the `Retry-After` parse and its clamp
```

## How it works

`send_with_retry_on_429` takes a closure that builds the `reqwest` request, so each attempt sends
a fresh body. It sends up to three times while the answer is `429`, waiting the `Retry-After`
seconds between attempts (fractions allowed, at most 5 s; 1 s when the header is missing or
unreadable), and returns the last response, a final `429` included, for the caller's own non-2xx
handling.

## Boundaries

- Depends on: `reqwest` and `tokio`'s timer.
- Used by: the announcement webhook client in
  `apps/website/api_v2/src/community_content/services/discord_webhook.rs`. The Discord OAuth2 and
  guild-member client in `apps/website/api_v2/src/identity_and_access/services/discord_client.rs`
  keeps its own retry with the same limits.
- Rules: every wait is bounded, so a hostile or broken `Retry-After` cannot park a request longer
  than 5 s per attempt.
