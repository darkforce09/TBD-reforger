# API client

The one HTTP client the app calls the [API](/documentation_v2/glossary/a_to_f.md#api) through: the request
verbs with their bearer token and their single retry, the failure types they return, and the
refresh policy that keeps a session alive across tabs without spending a refresh token twice.

## Contents

```text
apps/website/frontend/src/v2/core/api/client/
├── errors.rs    the `(status, message)` failure pair, the error-body reader and `ApiFailure`
├── fetched.rs   `Fetched`: a settled request's data or its named failure, with no empty default
├── mod.rs       the module tree; re-exports the verbs, the failure types and the refresh policy
├── refresh.rs   the refresh policy: one retry per 401, the per-tab single flight, the cross-tab lock
├── refusals.rs  `ApiRefusal`: a refused answer kept whole, with the reason its `details` names
├── requests.rs  the request verbs, the file upload and the cold-start `bootstrap`; browser-only
└── tests/       unit tests for the retry contract, the refresh generations, refusals and `Fetched`
```

## How it works

```text
verb ─▶ wait for the cold-start restore ─▶ fetch /api/v1<path> with the bearer token
     ─▶ on 401: per-tab single flight ─▶ Web Lock `tbd-auth-refresh`
                ─▶ adopt a peer tab's rotation, or spend the stored refresh token
                ─▶ retry once with the rotated access token
     ─▶ body, or (status, message), or ApiRefusal for the `_keeping_refusal` verbs
```

Every JSON verb goes through one private `request` function, and the file upload through the same
refresh contract. A retry that is still 401, and any other status, goes back to the caller; status
`0` means the request never reached the backend or its body could not be read. A request whose
session generation changed while it ran answers `(401, None)`, so no answer lands in the session
that replaced the one that asked. Refresh tokens are single-use and the API revokes the whole
family when a spent one comes back, so the token is chosen inside the lock: the tab adopts a pair
a peer announced on the `tbd-auth-refresh` broadcast channel, or removes the freshest stored token
and spends it on `POST /api/v1/auth/refresh` with a 30 s deadline, persisting and announcing the
successor, or ending the session when that fails. Web Locks need a secure context; without them
the refresh and the cold-start read return nothing and leave the stored credentials untouched.

`bootstrap`, spawned once by the app layout, restores the stored session under the same lock and
fetches `GET /api/v1/me`. `Fetched` and `ApiFailure` keep a dead session, a refusal and a network
failure apart, so a render site that holds one cannot show a refusal as an empty list; no page
uses them, and the pages read a failure as the `(status, message)` pair through
`api_error_message`.

## Boundaries

- Depends on: `crate::v2::core::auth` (the store's signals, the stored session, the single flight,
  the refresh transaction) and `crate::v2::core::api::dto::MeResponse`; `gloo-net`, `web-sys` and
  `js-sys` for fetch, the Web Locks API and the broadcast channel; `gloo-timers` for the refresh
  deadline; `futures`, `serde` and `serde_json`.
- Used by: the endpoint calls in `apps/website/frontend/src/v2/core/api/endpoints/`; the session
  in `apps/website/frontend/src/v2/core/auth/session.rs`, for the refresh lock; the pages under
  `apps/website/frontend/src/v2/pages/`, the app layout's `bootstrap` among them; the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) under
  `apps/website/frontend/src/v2/apps/editor/`.
- Rules:
  - one refresh and one retry per 401 (`retries_once_after_refresh`,
    `no_loop_if_retry_still_401` and `two_concurrent_401s_share_one_refresh` in `tests/client.rs`);
  - two tabs never spend one refresh token twice, and a waiting tab adopts its peer's rotation
    (`two_tabs_racing_one_single_use_token_both_keep_their_session` and
    `the_waiter_adopts_the_peer_rotation_instead_of_spending_a_second_one` in `tests/client.rs`);
  - a stale generation neither joins nor adopts a refresh
    (`stale_generation_at_first_401_cannot_join_or_replace_current_flight` in
    `tests/refresh_generation.rs`);
  - a refusal keeps its reason and a 401 still takes the refresh path
    (`a_coded_refusal_keeps_its_reason_and_its_fields` and
    `the_refusal_arm_leaves_a_401_to_the_refresh_contract` in `tests/refusals.rs`), and a failure
    never reads as empty data (`an_empty_result_and_a_401_are_different_values` in
    `tests/fetched.rs`);
  - the access token is never written to storage (`persist_blob_shape_matches_tbd_auth` in
    `apps/website/frontend/src/v2/core/auth/tests/auth.rs`).

## Related documentation

- [Identity transactions](/documentation_v2/website/api_v2/verification_evidence/identity_transactions.md)
  — sessions, refresh rotation and replay revocation in the API, and the browser's generations.
