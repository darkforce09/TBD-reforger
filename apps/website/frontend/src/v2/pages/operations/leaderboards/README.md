# Leaderboards page

The `/leaderboards` page, titled Global Leaderboards: five orderings of the same operator table,
ranked and searched by the [API](/documentation_v2/glossary.md#api), with a slide-over dossier of
one operator's statistics behind every row.

## Contents

```text
apps/website/frontend/src/v2/pages/operations/leaderboards/
├── board_table.rs       the podium and roster rows, per-category statistics and the avatar glyph
├── mod.rs               the module tree; re-exports `LeaderboardsPage`
├── operator_dossier.rs  `OperatorDossier`: one operator's stat card, fetched when a row opens it
├── page.rs              the route component: category and search controls, board fetch and sheet
└── tests/               unit tests for the row reader, win-rate scaling, initials and avatar sink
```

## How it works

`LeaderboardsPage` renders inside `AuthGate`. The signed-in half owns the category and query
signals and keys the board fetch on both, so every tab click and every keystroke fetches again;
the API orders and filters, so the ranking holds across the whole table rather than within one
page of rows. The controls sit outside the `Transition`, so the search field keeps its focus and
the resolved board stays on screen while the next one is in flight.

`page.rs` reads each untyped wire row into its `Row` (the server's `rank`, or the row's position
when the rank is missing or zero) and holds the win-rate scaling the other files share: the
command win rate arrives as a fraction and shows as a percentage, while the attendance rate
arrives multiplied out. `board_table.rs` puts the top three on a podium and the rest in roster
rows, each with the statistic pair of the active category; an avatar emits an `<img src>` only for
an `http(s)` URL and falls back to initials otherwise. A row click opens the "Operator Dossier"
sheet, whose `OperatorDossier` fetches that operator's card.

| Tab | `category` | Statistic pair on a row |
|---|---|---|
| "K/D Ratio" | `kd` | the ratio and "N Kills" |
| "Command Win Rate" | `command_win` | the win rate and "N Ops" |
| "Missions Played" | `missions` | the played count and "N Kills" |
| "Longest Kill" | `longest_kill` | "Nm" and "N Kills" |
| "Wall of Shame" | `team_kills` | the team kills and "N Ops" |

## Routes

| Route | Component | Access | Layout |
|---|---|---|---|
| `/leaderboards` | `LeaderboardsPage` | route tier `none`; the data renders only for a signed-in viewer | full-bleed inside the navigation frame; breadcrumb Operations › Global Leaderboards |

## Data

- `GET /api/v1/leaderboards?category=<category>&q=<query>`: the board, read as `Leaderboard`
  (`category` and untyped `data` rows); `q` is sent URL-encoded and only when the trimmed query is
  not empty.
- `GET /api/v1/users/{discordId}/stats`: the opened operator's card, read untyped (`stats`,
  `total_operations`, `attendance_rate`), so the page adds no shape to the shared DTOs; the sheet's
  avatar, name and rank come from the clicked row.
- The page reads the session from the `AuthStore` context and writes nothing. Both fetches run in
  the browser build only.

## States

| State | What the viewer sees |
|---|---|
| session restoring | "Loading session…" |
| signed out | "Sign in to load live data from the platform." and a "Sign in with Discord" link to `/login` |
| loaded controls | the "Global Leaderboards" header with "Real-time tactical performance metrics across all active theaters.", the five tabs and the "Search operators..." field |
| board loading | "Loading…" on the first fetch; afterwards the previous board stays until the next arrives |
| board failed | "Failed to load the leaderboard." |
| empty ladder | "No ranked operators yet — telemetry has not reported any tracked matches." |
| no search match | "No operators match your search." |
| loaded | the podium, each place with "[ VIEW DOSSIER ]", and the roster rows |
| dossier loading | "Loading…" in the sheet |
| dossier failed | "Failed to load this operator's record." |
| dossier loaded | the avatar, name and "RANK #N", and the tiles "Kills", "Deaths", "K/D Ratio", "Team Kills", "Longest Kill", "Vehicles Destroyed", "Missions Played", "Command Wins", "Command Win Rate", "Total Operations" and "Attendance" |

## Boundaries

- Depends on: `crate::v2::core::api` (the `api_get` client, `Leaderboard`), `crate::v2::core::ui`
  (`AuthGate`, `PageHeader`, `Sheet`, `MaterialIcon`, `cn`), `crate::v2::core::auth::url_guard`
  and the `AuthStore` context.
- Used by: the `/leaderboards` route in `apps/website/frontend/src/app_routes.rs`.
- Rules: ordering and filtering stay on the server, so nothing here re-sorts a page of rows; the
  row reader prefers the server's rank (`parse_row_prefers_the_server_rank_over_position` in
  `tests/leaderboards.rs`); the command win rate renders its wire fraction as a percentage
  (`command_win_rate_renders_the_wire_fraction_as_a_percentage`); an avatar `src` is only ever an
  `http(s)` URL (`avatar_img_emits_src_only_for_http_urls`).

## Related documentation

- [Leaderboards page](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md)
  — the page's behaviour and design.
