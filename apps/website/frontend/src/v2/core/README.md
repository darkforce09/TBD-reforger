# Shared foundations

The building blocks every page and workspace of the single-page app shares, with no business
logic of their own: the transport to the [API](/documentation_v2/glossary/a_to_f.md#api) and its wire
types, the session and its [role](/documentation_v2/glossary/n_to_z.md#role) checks, the interface
primitives, small utilities, and the helpers the crate's own tests share.

## Contents

```text
apps/website/frontend/src/v2/core/
├── api/           the HTTP client, typed endpoint calls, wire types and the live status stream
├── auth/          the session store, the role ladder, the route guard and the link guard
├── mod.rs         the module tree
├── test_support/  the source scrubber, the captured API responses and the source pins, for tests
├── ui/            the interface primitives: gates, form controls, overlays, notices, layouts
└── utils/         timestamps, the countdown, the avatar sanitiser and the clipboard write
```

## How it works

The app layout provides the two contexts everything else reads, the `AuthStore` of `auth/` and
the toast queue of `ui/`, and spawns `api::client::bootstrap`, which restores a stored session.
From then on a page fetches through `api/`, gates on `auth/`, and draws with `ui/` and `utils/`.

| Module | Uses within `core` |
|---|---|
| `api/` | `auth/`: the store's tokens, the stored session, the single flight and the refresh transaction |
| `auth/` | `api/`: `dto::MeResponse` and the client's refresh lock; `ui/`: the notice for a refused route |
| `ui/` | `auth/`: the store, for the gates |
| `utils/` | `auth/`: the link guard; `ui/`: the toast context and the placeholder avatar |

`mod.rs` declares `api/`, `auth/`, `ui/` and `utils/` without a `cfg` gate, so the native build
compiles all four; browser-only code below them carries its own `#[cfg(target_arch = "wasm32")]`,
on its item or on its `pub mod` line, so `cargo test -p website-frontend` runs the logic of all of
it without a browser. `test_support/` compiles only in test builds.

Nothing here imports from `pages`. Four places reach into the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator): the auth store purges a
departing account's local drafts through
`crate::v2::apps::editor::shell::hydrate::purge_local_documents`, and the search box, select and
slider take their hover and disabled classes from `crate::v2::apps::editor::shell::layout`. The
route guard in `auth/` reads the route table of `crate::router`.

## Public surface

- `api::client`: the request verbs, `bootstrap` and the failure types; `api::dto`: every wire
  type; `api::endpoints`: the typed calls; `api::sse`: the live status stream.
- `auth`: `AuthStore`, `Role`, `has_min_role`, `has_min_role_authed`, the session types and
  their storage (`User`, `RefreshResponse`, `AUTH_PERSIST_KEY`, `persist`, `load_persisted`), and
  `url_guard::is_http_url`.
- `ui`: `Dialog`, `Sheet`, `AuthGate`, `AdminGate`, `MaterialIcon`, `DEFAULT_AVATAR`,
  `PageHeader`, `cn`, `SearchBox`, `Select`, `Slider`, `badge_class`, the `split_pane` layout, the
  `toast` context and viewport, and `modal_stack`, which the Mission Creator's overlays register
  with.
- `utils`: `countdown_label`, `safe_avatar_url`, the `datefmt` and `utc_timestamp` readers, and
  `clipboard::write_clipboard`.
- `test_support`: the scrubber, fixtures and pins, for the crate's tests.

## Boundaries

- Depends on:
  - `crate::router` (`apps/website/frontend/src/router.rs`), for the route table's tiers;
  - the Mission Creator's `crate::v2::apps::editor::shell`, in four places: `auth/store.rs` calls
    `hydrate::purge_local_documents`, and `ui/search_box.rs`, `ui/select.rs` and `ui/slider.rs`
    import `layout::HOVER_FILL` and `layout::DISABLED_GLYPH`;
  - `website_map_engine::data`, in the wire types;
  - `leptos`, `leptos_router`, `gloo-net`, `gloo-timers`, `web-sys`, `js-sys`, `wasm-bindgen`,
    `wasm-bindgen-futures`, `futures`, `serde`, `serde_json`, `url` and `base64`.
- Used by: `apps/website/frontend/src/router.rs`, for the roles; the pages under
  `apps/website/frontend/src/v2/pages/`; the Mission Creator under
  `apps/website/frontend/src/v2/apps/editor/`. The debug benches under
  `apps/website/frontend/src/v2/apps/debug/` use none of it.
- Rules: nothing here imports from `pages`, and nothing from `apps` beyond the four `shell`
  imports above (no gate checks either direction); `mod.rs` declares every child ungated except
  `test_support`, which stays under `#[cfg(test)]` so no shipped code can reach it; every
  production file opens with a `//!` header, stays within 500 lines, documents every visible item
  and holds no inline test module (`v2_production_files_meet_the_documentation_standard` in
  `apps/website/frontend/src/v2/tests/doc_audit/mod.rs`).

## Related documentation

- [Frontend documentation](/documentation_v2/website/frontend/README.md#shared-foundations) — where these foundations
  sit among the routes, pages and workspaces they serve.
