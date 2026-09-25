**Status:** live

# Leaderboards page

The `/leaderboards` page in the [operations](/documentation_v2/glossary.md#operations) section,
titled "Global Leaderboards": a signed-in member ranks the community's players by one of five
statistics from [match telemetry](/documentation_v2/glossary.md#match-telemetry), searches them
by name, and opens one player's full statistics in a slide-over dossier.

## Where it lives

- Code: [`apps/website/frontend/src/v2/pages/operations/leaderboards/`](/apps/website/frontend/src/v2/pages/operations/leaderboards/):
  `page.rs` holds the route component `LeaderboardsPage`, the tabs, the search field, the board
  fetch and the row reader; `board_table.rs` the podium and the roster rows;
  `operator_dossier.rs` the dossier sheet. The folder's
  [README](/apps/website/frontend/src/v2/pages/operations/leaderboards/README.md) describes each
  file.
- Entry: the route, its tier and its layout are in the README's
  [Routes](/apps/website/frontend/src/v2/pages/operations/leaderboards/README.md#routes). The
  sidebar lists the page as "Global Leaderboards" in the "Operations" section.
- Related: the [deployments page](/documentation_v2/website/frontend/pages/operations/deployments/deployments_page.md),
  whose API reads the same aggregate view for the viewer's own figures; the
  [API](/documentation_v2/glossary.md#api)'s
  [command center domain](/apps/website/api_v2/src/command_center/README.md), which serves the
  board and the dossier.

## Behaviour

### The board

1. The page body sits in `AuthGate` (`apps/website/frontend/src/v2/core/ui/gates.rs`), which shows
   the session states of the README's
   [States](/apps/website/frontend/src/v2/pages/operations/leaderboards/README.md#states) until the
   viewer is signed in.
2. The header reads "Global Leaderboards" and "Real-time tactical performance metrics across all
   active theaters.", with the five tabs "K/D Ratio" (the default), "Command Win Rate",
   "Missions Played", "Longest Kill" and "Wall of Shame", and the "Search operators..." field.
3. Every tab click and every keystroke fetches the board again, with the category and, when the
   trimmed text is not empty, the search. The API orders and filters, so the page never re-sorts
   rows. The controls sit outside the loading boundary: the field keeps its focus and the last
   board stays on screen until the next one arrives; only the first load shows "Loading…".
4. The first three rows stand on a podium, first in the middle, second on the left and third on
   the right, with a rank badge, the avatar, the name and the category's statistic pair. The rest
   follow as roster rows with the rank as two digits ("04"), the avatar, the name and the pair.
5. The pair per tab: the ratio and "N Kills"; the win rate as a percentage and "N Ops"; the
   missions played and "N Kills"; "Nm" and "N Kills"; the team kills and "N Ops".
6. An avatar loads only from an `http(s)` address; otherwise the row shows the player's initials.
7. An empty board reads "No ranked operators yet — telemetry has not reported any tracked
   matches."; a search that matches nobody reads "No operators match your search."; a failed fetch
   reads "Failed to load the leaderboard."

### The dossier

1. A roster row click, or "[ VIEW DOSSIER ]" under the first podium place, opens the
   "Operator Dossier" sheet: the avatar, the name and "RANK #N" from the clicked row, and a fetch
   of that player's statistics ("Loading…", "Failed to load this operator's record.").
2. The sheet shows the tiles "Kills", "Deaths", "K/D Ratio", "Team Kills", "Longest Kill",
   "Vehicles Destroyed", "Missions Played", "Command Wins", "Command Win Rate", "Total Operations"
   and "Attendance". The command win rate arrives as a fraction and shows as a percentage; the
   attendance rate arrives already multiplied out.

### Known discrepancies

- The second and third podium places have no dossier control: only the first place carries
  "[ VIEW DOSSIER ]", and the podium places are not clickable (`podium_place` in
  `apps/website/frontend/src/v2/pages/operations/leaderboards/board_table.rs`), so the dossiers of
  the second and third players cannot be opened from the page.
- A search re-ranks: the API numbers the matching rows from 1 (`get_leaderboards` in
  `apps/website/api_v2/src/command_center/handlers/leaderboards.rs`), so a searched player shows on
  the podium as "#1" and the dossier says "RANK #1" whatever their place on the full board.
- The board shows the first 20 players: the page sends no `limit` or `offset` and has no pager,
  and the API returns 20 rows by default (`get_leaderboards`).

## Data

The README's [Data](/apps/website/frontend/src/v2/pages/operations/leaderboards/README.md#data)
lists each call with the fields the page reads. Server-side:

- `GET /api/v1/leaderboards?category=<category>&q=<text>` (`get_leaderboards` in
  `apps/website/api_v2/src/command_center/handlers/leaderboards.rs`), for any signed-in member:
  rows of the `leaderboard_totals` materialized view joined to the players who are not deleted,
  filtered by a case-insensitive username match, ordered by the category (K/D and the command win
  rate with unmeasured values last) with the Discord id breaking ties, `limit` rows (20 by
  default, at most 50) from `offset`, each numbered `rank` from `offset + 1`. An unknown category
  is refused with 400 "unknown category". A K/D nobody measured is `null`.
- `GET /api/v1/users/{discordId}/stats` (`get_user_stats` in
  `apps/website/api_v2/src/command_center/handlers/user_stats_card.rs`): one player's row of the
  same view, all zeros for a player with no matches, and their total operations and attendance
  rate; 404 "user not found" for an unknown player.
- The view aggregates the per-match player statistics that match results carry, and is refreshed
  in the same transaction as every match ingest and identity link that changes them
  (`apps/website/api_v2/src/command_center/services/leaderboard_view.rs`).

The page writes nothing and stores nothing in the browser.

## Design

- A page header, the tab strip and the search field, the podium, then the roster; the dossier is
  a `Sheet` sliding over the page.
- Design target: the [leaderboards blueprint](/documentation_v2/website/frontend/pages/operations/leaderboards/visual_references/leaderboards_blueprint/README.md),
  a design-phase reference, and the archived platform spec's
  [Global Leaderboards section](/documentation_v2/archive/go_and_react_era_design/platform_context_handoff.md#4-global-leaderboards).
  The built page follows the blueprint closely and differs:
  - the fifth tab reads "Wall of Shame" rather than "Wall of Shame (Team Kills)", and the search
    placeholder "Search operators..." rather than "Search operatives...";
  - avatars fall back to initials where the blueprint shows portraits;
  - the roster shows 20 players at most, where the archived spec asked for ranks 4 to 50.
- A design-phase inventory planned a "Load More Intel" button for paging and a table with
  "Kills", "Deaths" and "K/D Ratio" columns; the built roster is a list of rows with one statistic
  pair and no paging.

## Open work

- [T-949 — Leaderboard paging test cannot fail on its named claim](/.ai/tickets/T-949.toml)
  (idea, no plan): the API's paging test gains the tied rows that let it fail, guarding the
  tie-break order the board relies on.

## Decisions

- Ranking and search stay on the server: the page never re-sorts a page of rows, so the order
  holds across the whole table (`parse_row_prefers_the_server_rank_over_position` in
  `apps/website/frontend/src/v2/pages/operations/leaderboards/tests/leaderboards.rs`).
- The command win rate is a command win rate: the API counts only matches where the player held a
  command slot, and the page labels it so rather than as a general win rate, which the telemetry
  cannot derive (`command_win_rate_renders_the_wire_fraction_as_a_percentage`, same test file).
- A player's avatar loads only from an `http(s)` address, else initials show
  (`avatar_img_emits_src_only_for_http_urls`, same test file).
- The dossier adds no shape to the shared DTOs: it reads the statistics untyped, and takes the
  avatar, name and rank from the clicked row.
