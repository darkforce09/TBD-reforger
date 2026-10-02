# Foundation crates

The lowest tier of the library crates: small building blocks that depend on no workspace crate,
so every other crate may link them.

## Contents

```text
crates/foundation/
└── http_url_guard/  `http_url_guard`: whether a string is an `http` or `https` URL a browser follows
```

## Boundaries

- Depends on: external crates only.
- Used by: the API and the single-page app.
- Rules: a foundation crate declares `category = "crates/foundation"` and depends on no
  workspace crate (`cargo xtask verify crate-tiers`).
