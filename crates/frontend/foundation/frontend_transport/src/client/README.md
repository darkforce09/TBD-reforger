# API client

The one HTTP client the app calls the [API](/documentation/glossary/a_to_f.md#api) through: the request
verbs with their bearer token and their single retry, the error body readers behind the crate's
`Error` they return, and the refresh policy that keeps a session alive across tabs without spending a refresh token twice.

## Contents

```text
crates/frontend/foundation/frontend_transport/src/client/
├── errors.rs        the error-body reader: a backend `{"error", "details"}` body folded into one sentence
├── fetched.rs       `Fetched`: a settled request's data or its named failure, with no empty default
├── mod.rs           the module tree; re-exports the verbs, the error body readers and the refresh policy
├── public_reads.rs  `public_get`: anonymous, cancellable GETs that never touch the session
├── rate_limit_retry.rs  the rate-limit retry: a `429` waited out (`Retry-After`, 2 s default, 60 s cap) and sent again, three sends at most
├── refresh.rs       the refresh policy: one retry per 401, the shared single flight, the cross-tab spend rule
├── refusals.rs      `decode_answer`: a kept answer decoded into its data or an `Error` with the reason its `details` names
├── requests.rs      the request verbs and the file upload, generic over the token provider; browser-only
├── single_flight.rs  `SingleFlight`: one in-flight refresh shared by the callers of a session generation
└── tests/           unit tests for the retry contract, the single flight, the rate-limit retry, the refresh generations, refusals and `Fetched`
```

## How it works

```text
verb ─▶ wait for the cold-start restore ─▶ fetch /api/v1<path> with the bearer token
     ─▶ on 401: per-tab single flight ─▶ Web Lock `tbd-auth-refresh`
                ─▶ adopt a peer tab's rotation, or spend the stored refresh token
                ─▶ retry once with the rotated access token
     ─▶ body, or the crate's Error (with the refusal's reason for the `_keeping_refusal` verbs)
```

Every verb takes the session as a `TokenProvider` (`../token_provider.rs`), which the session
store implements: the verbs read its access token and generation, hand its per-tab `SingleFlight`
cell and its locked refresh to `send_with_refresh_for_generation`, and give the rotated pair back
through `set_tokens`. The per-tab cell, the Web Lock, the peer-rotation channel and the refresh
request itself live in the session refresh (`crates/frontend/foundation/frontend_session/src/session_refresh.rs`),
above the transport. Every JSON verb and the multipart form verb go through one private `request`
function, and the file upload through the same refresh contract. A retry that is still 401, and any other status, goes back to the caller; status
`0` means the request never reached the backend or its body could not be read. A request whose
session generation changed while it ran answers `(401, None)`, so no answer lands in the session
that replaced the one that asked. Refresh tokens are single-use and the API revokes the whole
family when a spent one comes back, so the token is chosen inside the lock: the tab adopts a pair
a peer announced on the `tbd-auth-refresh` broadcast channel, or removes the freshest stored token
and spends it on `POST /api/v1/auth/refresh` with a 30 s deadline, persisting and announcing the
successor, or ending the session when that fails. Web Locks need a secure context; without them
the refresh and the cold-start read return nothing and leave the stored credentials untouched.

`public_get` in `public_reads.rs` is the one read that bypasses all of this: it sends no bearer
token and no credentials, reads and changes no session state, and runs at most four requests at
once, each cancellable through its `AbortSignal` and retried up to twice after a 429, waiting as
`Retry-After` says (at most 60 s). The equipment data viewer reads through it.

The session refresh's `bootstrap`, spawned once by the app layout, restores the stored session
under the same lock and fetches `GET /api/v1/me` through `api_get`. `Fetched` and the crate's `Error` keep a dead session, a refusal and a network
failure apart, so a render site that holds one cannot show a refusal as an empty list; the pages
match on its variants or its `status` and word it with `Error::message_or`.

## Boundaries

- Depends on: `crate::token_provider::TokenProvider` and
  `frontend_api_dtos::RefreshResponse`; nothing above the transport; `gloo-net`
  and `web-sys` for fetch; `futures`, `serde` and `serde_json`.
- Used by: the endpoint calls in `crates/frontend/foundation/frontend_transport/src/endpoints/`; the session
  refresh in `crates/frontend/foundation/frontend_session/src/session_refresh.rs`, for the request path, the
  refresh policy and the cell type; the page crates under `crates/frontend/pages/`; the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) under
  `crates/frontend/workspaces/mission_creator_workspace/src/`; `public_get`, by the equipment data viewer in
  `crates/frontend/workspaces/debug_benches/src/data_viewer/data/`.
- Rules:
  - one refresh and one retry per 401 (`retries_once_after_refresh`,
    `no_loop_if_retry_still_401` and `two_concurrent_401s_share_one_refresh` in `tests/client.rs`);
  - two tabs never spend one refresh token twice, and a waiting tab adopts its peer's rotation
    (`two_tabs_racing_one_single_use_token_both_keep_their_session` and
    `the_waiter_adopts_the_peer_rotation_instead_of_spending_a_second_one` in `tests/client.rs`);
  - every request path refreshes through the provider's one cell and its locked refresh, and an
    older generation never clears a newer flight
    (`older_generation_completion_cannot_clear_newer_pending_flight` in `tests/single_flight.rs`);
  - a stale generation neither joins nor adopts a refresh
    (`stale_generation_at_first_401_cannot_join_or_replace_current_flight` in
    `tests/refresh_generation.rs`);
  - a refusal keeps its reason and a 401 still takes the refresh path
    (`a_coded_refusal_keeps_its_reason_and_its_fields` in `tests/refusals.rs`), and a failure
    never reads as empty data (`an_empty_result_and_a_401_are_different_values` in
    `tests/fetched.rs`);
  - the access token is never written to storage (`persist_blob_shape_matches_tbd_auth` in
    `crates/frontend/foundation/frontend_session/src/tests/auth.rs`).

## Related documentation

- [Identity transactions](/documentation/crates/api/api_server/design_notes/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API, and the browser's generations.
