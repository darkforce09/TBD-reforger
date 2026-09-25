**Status:** live

# App layout and navigation

The frame every page renders in: the choice of frame per route, the sidebar and its links, the top
bar with the breadcrumb and the account menu, the membership status panel, and the page shown when
no route matches. Every visitor meets it on every route; administrators also use its panel to
extend cached access during a Discord outage.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/navigation/`](/apps/website/frontend/src/v2/pages/navigation/):
  `layout.rs` holds `AppLayout`, the frame classifier and the active-link rule; `nav_config.rs`
  the `NAVIGATION` table; `sidebar.rs` the sidebar, the drawer toggle and the link list;
  `top_nav.rs` the top bar and sign-out; `membership_status.rs` the membership panel;
  `not_found.rs` the fallback page. The folder's
  [README](/apps/website/frontend/src/v2/pages/navigation/README.md) describes each file.
- Entry: `apps/website/frontend/src/main.rs` mounts `AppLayout` once, inside the router. The one
  route of its own, the fallback, is in the README's
  [Routes](/apps/website/frontend/src/v2/pages/navigation/README.md#routes).
- Related: the [account pages](/documentation_v2/website/frontend/pages/account/account_pages.md),
  which the frame renders bare or links from its account menu; the
  [session and access](/apps/website/frontend/src/v2/core/auth/README.md) code, whose store the
  frame creates and whose [role](/documentation_v2/glossary.md#role) ladder the sidebar applies;
  the route table in `apps/website/frontend/src/router.rs`, which declares each route's layout
  flags and breadcrumb.

## Behaviour

### Frame choice

1. `AppLayout` creates the one `AuthStore` every page reads, and the toast queue. On every path
   but `/auth/callback` it starts the stored-session restore; on that path the callback page
   installs the session itself.
2. `classify_frame` picks one of three frames from the pathname:

   ```text
   /login, /auth/callback     ──▶ bare: the page alone, no wrapper
   router::chromeless(path)   ──▶ chromeless: one full-viewport container
   any other path             ──▶ chrome: sidebar + top bar + <main>
                                  <main> overflow-hidden if router::full_bleed(path), else padded
   ```

3. The route table flags four chromeless routes: the
   [Mission Creator](/documentation_v2/glossary.md#mission-creator) at `/missions/:id/edit`, the
   review workspace and the two debug benches. The frame is rebuilt only when the kind changes, so
   moving between two chromed routes swaps the page inside `<main>` and leaves the sidebar and the
   top bar mounted.
4. The membership panel and the toast viewport sit beside the frame, so they show in all three
   frames and a frame swap never unmounts them.

### Sidebar

1. The sidebar lists the six sections of `NAVIGATION` top to bottom, each link with its Material
   Symbols icon; the README's
   [How it works](/apps/website/frontend/src/v2/pages/navigation/README.md#how-it-works) has the
   table of sections, labels and paths.
2. Links are filtered per render through `has_min_role`, the browse-mode check: a signed-out
   visitor, whose role is unknown, sees every section, "Administration" included. A signed-in
   viewer sees the links their role clears: every link outside "Administration" asks for
   `enlisted`, and the six "Administration" links for `admin`. A signed-in `guest` therefore sees
   an empty sidebar, and a section left without links is dropped.
3. The sidebar is not an access boundary. Each page gates its own data: the route guard, `AuthGate`
   and `AdminGate` refuse a viewer whatever the sidebar showed.
4. One link is active, marked by style and `aria-current="page"`: "Dashboard" only on `/`, any
   other link on its own path and every path below it, so `/missions/abc` keeps "Mission Library"
   lit while `/missions-archive` does not.
5. Below the `lg` breakpoint (1024px) the sidebar gives way to a fixed "Open menu" toggle. It opens
   a drawer holding the same brand block and links, which a backdrop click, a link click or the
   Escape key closes.
6. Creating a [mission](/documentation_v2/glossary.md#mission) is not a sidebar item: the Mission
   Library's "New Mission" button opens the create dialog.

### Top bar

1. The left side shows the route's breadcrumb from `router::breadcrumb`, parent / current (Mission
   Hub / Mission Overview on `/missions/abc`), or "TBD Reforger" on a route without one, such as
   the not-found page.
2. A viewer without a session sees a "Sign in with Discord" link to `/login`. A signed-in viewer
   sees the identity pill, "Linked: <first 8 characters of the Arma id>..." or "Unlinked", then
   the avatar and the name.
3. The avatar button opens the account menu: "Settings" (`/settings`), "Link Arma Identity"
   (`/settings#arma-link`, the settings page's link card) and "Sign Out". A click anywhere, a menu
   item or the Escape key closes it. The account area is rebuilt only when the name, the avatar or
   the linked identity changes, so a profile poll leaves an open menu alone.

### Sign-out

1. "Sign Out" clears the local session at once, which also purges the departing account's local
   Mission Creator drafts.
2. Under the cross-tab refresh lock it removes the stored session, unless another tab has already
   stored a different session in its place.
3. It asks the [API](/documentation_v2/glossary.md#api) to revoke the refresh token. A storage
   failure or a failed revocation shows a toast; the viewer stays signed out locally either way.

### Membership status panel

1. While a session is held, the panel refetches the profile every 30 seconds, so the membership
   flags stay current.
2. It shows at the bottom left while the profile reports the membership stale, or while the
   viewer may extend access:
   - stale: the API last verified the viewer's Discord membership more than 60 seconds ago, or
     never. The viewer keeps their cached role for 48 hours after that verification, and
     afterwards drops to `guest` unless an administrator extends access. A
     [dev login](/documentation_v2/glossary.md#dev-login) session is never stale.
   - "An administrative access extension is active." shows while an extension keeps the cached
     role.
   - A verified administrator whose guild membership is confirmed sees "Extend cached access"
     whenever signed in, in every frame, the Mission Creator included.
3. The extension form takes a Discord account id (blank means the viewer) and a reason, which
   must hold more than whitespace. An id with anything but digits is refused in the browser.
   "Extend for 48 hours" sends the request, reports the outcome in the panel and refetches the
   profile.

Every text of the top bar, the sign-out toasts and the panel is in the README's
[States](/apps/website/frontend/src/v2/pages/navigation/README.md#states).

### Not-found page

A path no route matches renders `NotFoundPage` inside the chromed frame, padded, so the visitor
keeps the sidebar and the top bar: "404", "Sector Not Found", "The requested route does not exist
in this AO." and a "Return to Dashboard" link to `/`. The server answers every path with the
application, so the fallback is resolved in the browser and the response is never a 404.

### Known discrepancies

- The frame's module header says the chromeless routes are "today the mission editor"
  (`apps/website/frontend/src/v2/pages/navigation/layout.rs`); the route table flags four
  (`apps/website/frontend/src/router.rs`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/navigation/README.md#data) lists each call
with the body it sends or the DTO it reads. Server-side:

- `POST /api/v1/auth/logout` (`logout` in
  `apps/website/api_v2/src/identity_and_access/handlers/session_tokens.rs`): revokes the session
  the refresh token belongs to and answers 204, also for an unknown token; a missing token is
  refused with 400 "refresh_token required".
- `GET /api/v1/me` (`get_me` in `apps/website/api_v2/src/identity_and_access/handlers/member_profile.rs`):
  the profile the store adopts, with `membership_stale`, `membership_override_active` and
  `can_manage_membership_override`, which the API derives from the account's last verified
  Discord membership on every authenticated request
  (`apps/website/api_v2/src/identity_and_access/services/session_authorization.rs`,
  `cached_membership_permissions.rs`). A background worker re-reads each member's Discord
  membership.
- `POST /api/v1/admin/users/{discordId}/membership-grace` (`extend_grace` in
  `apps/website/api_v2/src/administration/handlers/membership_grace_overrides.rs`): takes
  `duration_hours` from 1 to 48 and a reason of 1 to 2000 bytes. The caller must hold a live
  non-development session and be a verified administrator who is still a guild member; the
  target must be a verified guild member who is neither banned nor deleted (409 otherwise). In
  one transaction the API writes the override, records `membership.grace_extended` in the
  [audit logs](/documentation_v2/glossary.md#audit-logs) with the previous and the new expiry and
  the reason, and answers `{discord_id, expires_at}`.
- The sidebar, the breadcrumb and the not-found page make no call: `NAVIGATION` and the route
  table are static.

## Design

- Chromed frame: a full-height row. On a wide viewport the sidebar is a fixed 320px column on the
  low surface: the brand block ("TBD" in the primary colour, "Reforger" in the text colour) over
  a scrolling link list. Section headings are small bold capitals in grey; the Administration
  section sits in a red-tinted, red-bordered box with a red heading. The active link carries a
  faint gradient, a 2px primary bar at its left edge and primary text.
- Top bar: 64px high, the low surface at 70% opacity with a backdrop blur and a faint bottom
  border; below `lg` its content is inset to clear the fixed toggle. The breadcrumb parent is
  muted and the current page bright and semibold, with a "/" between them. The pill is a rounded
  capsule, green on a dark green fill when linked. The menu is a translucent glass card with a
  faint border, an icon before each item, a divider, and "Sign Out" in the error colour.
- `<main>` is padded and scrolls, or is `overflow-hidden` for a full-bleed route, which fills it
  and scrolls itself.
- Drawer: 320px, sliding in from the left over a half-black backdrop.
- Membership panel: a card fixed at the bottom left, at most 28rem wide, with the form in a
  disclosure.
- Not-found: centred, "404" in large primary type over the heading.
- Design target: the [topbar blueprint](/documentation_v2/website/frontend/pages/navigation/visual_references/topbar_blueprint/README.md),
  a design-phase reference holding the Stitch design brief and its tokens, with no export or
  screenshot. The built top bar differs from it:
  - the bar is the theme's translucent low surface with a blur, not the solid `#0b1120` with a
    `#1e3a5f` border;
  - the theme's primary is `#adc6ff`, where the blueprint's is `#3b82f6`; that blue is the theme's
    `action` colour;
  - the linked pill fills with `success-muted` (`#064e3b`) rather than green at 20% opacity;
  - the menu is the theme's translucent glass with a grey border rather than a solid `#1f2937`
    card with a blue border; the avatar button adds a chevron, and the menu items carry icons;
  - the built bar adds the "Unlinked" pill, the signed-out "Sign in with Discord" link and the
    "TBD Reforger" fallback, none of which the blueprint shows.
- The sidebar has no design set; its Stitch export went with the retired React client. The
  design-phase spec called for a textured gradient panel under a 2px primary top line; the built
  sidebar is a flat surface with neither.

## Open work

- [T-1045 — Rewrite stale frontend doc comments naming missing code and behaviour](/.ai/tickets/T-1045.toml)
  (idea, no plan): the frame's module header names all four chromeless routes.

## Decisions

- One classifier picks the frame, and it names only the two sign-in paths: every other layout is
  a flag in the route table, so a new full-viewport route is a table edit, not a frame change.
- The frame is rebuilt only when its kind changes: navigation between standard pages keeps the
  sidebar and the top bar from remounting.
- Signed-out visitors browse the full navigation: the site stays explorable before sign-in, and
  each page's gate, not the sidebar, refuses access; `has_min_role` serves chrome only, and
  actions use `has_min_role_authed`, which never admits an unknown role.
- The dashboard link matches `/` exactly and every other link by path prefix: every path starts
  with `/`, and a page's sub-routes keep their section lit.
- Sign-out clears the local session before it asks the API: the viewer is signed out even when
  the API cannot be reached, and a failure is reported rather than blocking.
- The not-found page renders inside the frame: a mistyped path keeps the navigation, and the
  server's catch-all already answered the request.
- The membership panel lives beside the frame: a Discord outage warning and the recovery form
  must stay reachable on every route, the Mission Creator included.
