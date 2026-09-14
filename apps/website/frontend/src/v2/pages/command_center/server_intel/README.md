# Server Intel Page (`/server-intel`)

## Architecture
- **`page.rs`**: Main layout file.
- **`server_list.rs`**: Grid of active Reforger dedicated game servers.
- **`player_census.rs`**: Real-time connected player list and faction breakdown.
- **`direct_connect.rs`**: 1-click IP/Port/Password copy widget and Steam protocol launcher.

## Not present in the legacy page
- **Server grid** — `server_list.rs` picks one server (the first active row, else the first row) and draws the panel shell for it; there is no grid of servers.
- **Player list and faction breakdown** — `player_census.rs` reports an aggregate head count with uptime, frame rate, the theatre tile and the environment readouts; no per-player roster and no per-faction split exist.
- **Password copy and Steam protocol launcher** — `direct_connect.rs` copies `ip:port` only, and the launch button raises a notice that the game client is required.
- The intelligence strip at the foot of the panel is fixed placeholder text, not a feed; it lives in `server_list.rs` with the section it belongs to.
