# Global Leaderboards (`/leaderboards`)

Five orderings of the same operator table — kill/death, command win rate, missions played,
longest kill and team kills — ranked and searched on the server, with a slide-over dossier behind
every row.

## Architecture
- **`page.rs`**: the route component. Owns the category and search controls, keys
  `GET /leaderboards` on both, reads the wire rows, and mounts the dossier sheet. Also holds the
  row type, the untyped-field readers and the win-rate scaling the other two shards share.
- **`board_table.rs`**: the board — the three-place podium and the roster rows under it — plus the
  per-category statistic pair, the podium tier styling, and the avatar glyph with its scheme
  guard and initials fallback.
- **`operator_dossier.rs`**: the slide-over stat card, fetched from
  `GET /users/:discord_id/stats`.
- **`tests/leaderboards.rs`**: the wire row reader including its rank fallback, the win-rate
  scaling, the initials fallback, and the avatar image sink over the shared adversarial corpus.

## Not present in the legacy page
- **A fabricated ladder for an empty board**: an empty board says it is empty, and says it two
  ways — a search that matched nobody reads differently from a ladder with nothing in it yet.
