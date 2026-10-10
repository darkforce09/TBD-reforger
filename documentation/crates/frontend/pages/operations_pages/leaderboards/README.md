**Status:** live

# Leaderboards page documentation

The feature documentation of the `/leaderboards` page, titled "Global Leaderboards", where a
member ranks and searches the community's players by statistics from
[match telemetry](/documentation/glossary/g_to_m.md#match-telemetry) and opens one player's dossier,
with the page's design-phase reference.

## Contents

```text
documentation/crates/frontend/pages/operations_pages/leaderboards/
├── leaderboards_page.md  the feature doc: the five ladders, search, the dossier and the API
└── visual_references/    the design-phase blueprint of the tabs, the podium and the roster
```

## How it works

Read [leaderboards_page.md](/documentation/crates/frontend/pages/operations_pages/leaderboards/leaderboards_page.md)
first. It follows the [feature doc template](/documentation/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the board and the dossier calls mean in the
[API](/documentation/glossary/a_to_f.md#api), lists where the page and the API disagree, and compares
the built page with the blueprint in `visual_references/`. The code folder's README lists the
page's files, calls and states.

## Code

- [Leaderboards page](/crates/frontend/pages/operations_pages/src/leaderboards/) — the route
  component `LeaderboardsPage`, the podium, the roster and the dossier.
- [Command center domain](/crates/api/api_command_center/src/) — the leaderboard and the
  per-player statistics the page reads.

## Boundaries

- Depends on: the feature doc template; the page code, the command center handlers and the ticket
  manager (`ttm`), which the feature doc is written from.
- Used by: the page's in-code README and the operations pages README, which link the feature doc;
  the deployments feature doc; the web app README's page table in
  `documentation/crates/frontend/shell/frontend_application/`.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Command center domain](/crates/api/api_command_center/src/README.md) — the API side of the
  leaderboard and the statistics card.
