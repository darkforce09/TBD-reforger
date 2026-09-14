# Server Control & RCON (`/admin/server`)

The configured game servers, the state of the one selected, and the console that commands it.

## Architecture
- **`page.rs`**: route component — fetches `GET /servers`, puts it behind the administrator gate,
  owns the transcript and the command line, and arranges the picker beside the server card.
- **`server_cards.rs`**: the selectable list of servers with their status dots, the selected
  server's header and controls, the three telemetry columns, and the readings they format.
- **`rcon_console.rs`**: the quick actions, the scrolling transcript, and the raw command line.
- **`rcon.rs`**: the channel behind all of them — the reply's shape, the four request bodies, the
  reading of a reply into success or failure, the sentence the operator is shown, and the send.
- **`tests/server_control.rs`**: the request and reply contract, and the reply path itself, lifted
  out of this folder and compiled and run against a recording stand-in.

## Not present in the legacy page
- **A Start control**: there is no start endpoint of any kind. Launch is a client-side action that
  says so, and Stop is disabled with copy naming the same reason.
- **`#kick` / `#ban` / `#missions` quick actions**: the wired quick actions are Change Map and
  Force Restart. Swap Modpack and Global Broadcast are disabled because the action set has no verb
  for them; anything else goes through the raw command line.
- **Terrain and the running mission on the server card**: the server payload carries neither, so
  both read as a dash rather than as invented values.
