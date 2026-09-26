**Status:** live

# Account pages

The three pages that act on the viewer's own session: `/login` starts the Discord sign-in,
`/auth/callback` installs the session the sign-in redirect carries, and `/settings` shows the
signed-in viewer's profile and [role](/documentation_v2/glossary/n_to_z.md#role), links their Discord
account to their Arma identity, and gives their attendance figures.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/account/`](/apps/website/frontend/src/v2/pages/account/):
  `login/page.rs` holds `LoginPage`, `auth_callback/page.rs` holds `AuthCallbackPage` with the
  fragment parser and the failure lines, and `settings/page.rs` holds `SettingsPage` with its two
  fetches, the link and unlink actions and the three cards. The folder's
  [README](/apps/website/frontend/src/v2/pages/account/README.md) describes each folder.
- Entry: the three routes, their tier and their layout are in the
  [sign-in page](/apps/website/frontend/src/v2/pages/account/login/README.md#routes),
  [sign-in callback page](/apps/website/frontend/src/v2/pages/account/auth_callback/README.md#routes)
  and [account settings page](/apps/website/frontend/src/v2/pages/account/settings/README.md#routes)
  READMEs.
- Related: the [app layout and navigation](/documentation_v2/website/frontend/pages/navigation/app_layout_and_navigation.md)
  doc, whose frame renders the first two pages bare and whose top bar links `/login` and
  `/settings`; the [session and access](/apps/website/frontend/src/v2/core/auth/README.md) code,
  which holds the session store, its persistence and the role ladder; the
  [API](/documentation_v2/glossary/a_to_f.md#api)'s
  [identity and access](/documentation_v2/glossary/g_to_m.md#identity-and-access) domain
  ([README](/apps/website/api_v2/src/identity_and_access/README.md)), which runs the sign-in and
  the link.

## Behaviour

### Sign-in

1. A visitor reaches `/login` from the top bar's "Sign in with Discord" link, from the sign-in
   prompt of `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`) or from the callback's
   "Back to login" link. The frame renders the page bare, with no sidebar or top bar.
2. The card offers "Sign in with Discord" and a "Continue browsing without signing in" link to
   `/`. It renders the same card for a viewer who is already signed in; nothing redirects them.
3. "Sign in with Discord" sets the browser's location to `/api/v1/auth/discord/login`: a
   full-page navigation, never a request, because the flow continues off-site.
4. The API sets a host-only `oauth_state` cookie valid for ten minutes and redirects (307) to
   Discord's consent page. It sends the browser straight back to `/auth/callback#error=<code>`
   instead when Discord sign-in has no client id (`oauth_unconfigured`), or, in development only,
   when `FRONTEND_URL` and `DISCORD_REDIRECT_URL` name different hosts, since the state cookie
   would never return (`oauth_host_mismatch`).
5. Discord returns the browser to `/api/v1/auth/discord/callback`. The API checks the code and
   the state cookie (`missing_code`, `invalid_state`), exchanges the code with Discord
   (`discord_unreachable`), records the account and its guild membership, refuses a banned
   account (`banned`) and issues a session. It redirects (302) to `/auth/callback` with
   `access_token`, `refresh_token`, `expires_at` and `arma_linked` in the URL fragment, or with
   `error=server_error` when a step on its side fails. Every exit after the state check clears
   the state cookie.

### Dev login

With `APP_ENV=development`, `GET /api/v1/auth/dev-login?role=<role>` is the
[dev login](/documentation_v2/glossary/a_to_f.md#dev-login): it signs in without Discord as a fixed local
account of the role (`guest`, `enlisted`, `leader`, `mission_maker` or `admin`; any other value
signs in as `admin`) and redirects to `/auth/callback` with the same fragment the Discord callback
sends, so the callback page runs unchanged. Outside development the route is not registered and
the handler answers 404. The [local development runbook](/documentation_v2/runbooks/local_development.md)
gives the commands.

### Sign-in callback

1. The page does its work once, when it mounts, and only in the browser build. The frame skips
   its stored-session restore on this path, because the page installs the session it is handed.
2. With `error=<code>` in the fragment, the page removes the fragment and shows "Sign-in failed"
   with the line for that code and a "Back to login" link. A code the page does not know gets the
   generic line.
3. With the three tokens, it removes the fragment by replacing the history entry, never pushing
   one, so the back button cannot return to the credentials and a second parse finds nothing. It
   clears any session held, installs the new tokens and saves them in local storage under the
   cross-tab refresh lock. A browser that cannot store them gets the failure line saying so.
4. It fetches the profile with the new access token. The store adopts it only while the request
   still belongs to the current session; then the page saves the profile beside the tokens and
   makes a full-page load of `/`, whatever page the sign-in started from.
5. When the profile fetch fails, the page shows the generic line and keeps the stored tokens, so
   a reload restores the session. Until then the store holds tokens without a profile, and the
   top bar offers "Sign in with Discord".
6. The page reads `arma_linked` and ignores it: the profile says whether an Arma identity is
   linked.

Every status and failure text is in the callback README's
[States](/apps/website/frontend/src/v2/pages/account/auth_callback/README.md#states).

### Settings

1. `/settings` renders inside the navigation frame with the breadcrumb Account / Settings. Its
   route tier is `none`, and `AuthGate` shows "Loading session…" while the stored session is
   restored and the sign-in prompt to a signed-out viewer. Every signed-in role, `guest`
   included, sees the cards.
2. The page fetches the profile and the link status at once and renders the cards only when both
   have answered, so the status never reads "Unlinked" because its fetch has not arrived. A failed
   profile fetch shows "Failed to load data."; a failed link status reads as unlinked.
3. The Profile card shows the avatar (any value that is not an `http` or `https` URL gives way to
   the shipped placeholder), the username, the Discord handle and the role as its wire value.
4. The Arma Identity card, id `arma-link`, is where the top bar's "Link Arma Identity" item lands
   (`/settings#arma-link`). It shows "Status: Linked (<character or Arma id>)" or
   "Status: Unlinked".
5. "Generate Link Code" requests a code, shows it as "Link code: <code>", toasts and refetches the
   link status. The player types the code in the game's chat as `#tbd link <code>`; the game
   server spends it through the API, which then records the Arma identity. The page does not poll,
   so the status changes after a reload. The card shows no expiry, although the code lapses after
   ten minutes.
6. When the server reports a pending code and none was generated on this visit, the card says a
   code is pending and must be generated again to be shown: the API never returns a code twice.
   A code generated on this visit outranks that notice and survives the refetch it causes.
7. "Unlink Arma ID" shows only while linked. It removes the link, clears the code and refetches
   both the profile and the link status.
8. Each button ignores a second click while its request runs. Every result shows as a toast.
9. Service Stats shows "Total Operations" and "Attendance" as a percentage.

The page's states and toasts are in the settings README's
[States](/apps/website/frontend/src/v2/pages/account/settings/README.md#states).

### Known discrepancies

- The callback page shows "Something went wrong completing sign-in. Please try again." for
  `oauth_host_mismatch` and `server_error`, since `auth_error_copy` has no line for either
  (`apps/website/frontend/src/v2/pages/account/auth_callback/page.rs`). The API sends
  `oauth_host_mismatch` in development for a configuration fault that no retry fixes: the two
  URLs in `apps/website/api_v2/.env` name different hosts (`reject_login_on_host_mismatch` in
  `apps/website/api_v2/src/identity_and_access/handlers/oauth_host_guard.rs`).
- `AuthCallbackPage`'s doc comment says the page shows "Completing sign in…" and navigates to the
  destination the sign-in started from (`auth_callback/page.rs`). The page shows
  "Completing sign-in…", nothing records where the sign-in started, and the page always loads `/`.

## Data

The READMEs' Data sections list each call with the DTO the pages read:
[sign-in](/apps/website/frontend/src/v2/pages/account/login/README.md#data),
[callback](/apps/website/frontend/src/v2/pages/account/auth_callback/README.md#data) and
[settings](/apps/website/frontend/src/v2/pages/account/settings/README.md#data). Server-side:

- `GET /api/v1/auth/discord/login` (`discord_login` in
  `apps/website/api_v2/src/identity_and_access/handlers/discord_oauth.rs`): sets the state cookie
  and redirects to Discord, or to the callback page with an error, as the sign-in steps say.
- `GET /api/v1/auth/discord/callback` (`discord_callback`, same file): compares the state in
  constant time, upserts the `users` row, records the guild membership observation, issues the
  session and redirects with the token fragment; `expires_at` there is RFC 3339 in whole seconds
  (`session_redirect` in `apps/website/api_v2/src/identity_and_access/services/session_issuance.rs`).
- `GET /api/v1/auth/dev-login` (`dev_login` in
  `apps/website/api_v2/src/identity_and_access/handlers/developer_login.rs`): upserts the role's
  own local account, each role with its own Discord id and Arma id, and issues a session that
  carries the role.
- `GET /api/v1/me` (`get_me` in `apps/website/api_v2/src/identity_and_access/handlers/member_profile.rs`):
  the stored account with the session's effective role in place of the stored one,
  `arma_linked`, and the membership flags the navigation frame reads. `attendance_rate` is
  computed on each read from the decided attendance records; `total_deployments` is the number of
  distinct matches with recorded statistics for the account, which the match results ingest and
  the link and unlink steps recompute.
- `GET /api/v1/me/link/status` (`link_status` in
  `apps/website/api_v2/src/identity_and_access/handlers/arma_link_codes.rs`): `linked` when the
  stored Arma id is not blank, `arma_id`, `arma_character`, and `pending_code` when an unspent,
  uncancelled, unexpired code exists.
- `POST /api/v1/me/link` (`create_link_code`, same file): 201 with a six-digit `code` and its
  `expires_at`, ten minutes ahead; the call cancels the caller's previous pending code.
- `DELETE /api/v1/me/link` (`unlink`, same file): removes the Arma identity and every pending code
  while keeping sign-up and attendance records, and answers `{linked: false}`.
- `POST /api/v1/ingest/link-confirm` (`ingest_link_confirm` in
  `apps/website/api_v2/src/identity_and_access/handlers/arma_link_confirmation.rs`): the game
  server's half of the link, which spends a code with `{code, arma_id, arma_character}`; the
  pages never send it.
- `PATCH /api/v1/me` (`update_me` in `member_profile.rs`) is never called: every profile field
  belongs to the Discord sign-in or the link flow, so it answers the stored account unchanged.
- `POST /api/v1/auth/refresh` belongs to the API client, not to these pages: the session store
  README's [How it works](/apps/website/frontend/src/v2/core/auth/README.md#how-it-works) gives
  the refresh rotation and the `tbd-auth` blob the callback writes.

## Design

- Sign-in: one centred card, at most 28rem wide, on the page background: "TBD" in the primary
  colour before " Reforger", the purpose line, a full-width primary button and a muted link
  below it.
- Callback: the same card holding either the status heading or the failure heading in the error
  colour, its line and the link back to `/login`.
- Settings: a column at most 42rem wide under the page header, holding three glass cards; the role
  shows as a primary chip, the link code in monospace on a primary tint, and the two figures in
  large monospace, primary and success.
- Design target: no visual reference set exists for the account pages. The archived platform
  spec's [Discord integration section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#a-discord-integration)
  sets the one rule they follow: Discord is the only sign-in, with no local passwords.

## Open work

- [T-1036 — Fix sign-in callback return path, status text and error copy](/.ai/tickets/T-1036.toml)
  (idea, no plan): the callback's doc comment and its behaviour agree on where the page goes and
  what it says, and `server_error` and `oauth_host_mismatch` get their own lines.
- [T-946.81 — Auth callback persists a token with no user](/.ai/tickets/T-946.81.toml) (idea, no
  plan): the callback stops leaving the store with tokens and no profile when the profile fetch
  is interrupted.

## Decisions

- Discord is the only way to sign in: accounts, roles and guild membership all come from
  Discord, so the platform keeps no passwords.
- The sign-in starts with a full-page navigation: the API sets the state cookie on its own
  response and the flow leaves the application for Discord.
- Tokens reach the page only in the URL fragment, never in a query string: a fragment is not sent
  to any server, and the page removes it from history before it installs the session.
- The settings cards wait for both fetches: a profile above an "Unlinked" line that has not been
  fetched would state a link status that may be false.
- The profile is read-only: the Discord sign-in and the link flow own every field, so the page
  offers no edit form and the role shows where it comes from without a control to change it.
