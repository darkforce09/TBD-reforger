**Status:** live

# Discord identity link

How a player ties their game identity to their TBD platform account, and so to their Discord
identity: a code made on the website, typed in game. The link lets the platform credit a
player's attendance and statistics from the round results the server reports. In game it is a
private chat command; the [mod](/documentation_v2/glossary.md#mod) draws no dialog.

## Where it lives

- Code: [`apps/mod/tbd-framework/Scripts/Game/TBD/API/`](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md):
  `TBD_IdentityLink.c` (the `#tbd link` command, the request queue and every reply) and
  `TBD_PlayerIdentity.c` (`GetArmaId`, the one identity accessor it shares with
  `TBD_ResultsReporter`).
- Entry: `#tbd link <code>` in chat, intercepted on the server by `TBD_AdminCommands` before the
  game forwards the line; unlike the other `#tbd` commands it is open to every player.
- Website half: the account settings page's "Generate Link Code", reached from the top bar's
  "Link Arma Identity"; the [account pages](/documentation_v2/website/frontend/pages/account/account_pages.md)
  document it.
- Layout: none; replies go to the player alone through `SCR_ChatComponent.SendPrivateMessage`.

## Behaviour

### Linking

1. On the website, a signed-in member generates a link code: 6 digits, valid for 10 minutes.
2. In game, the player types `#tbd link <code>`. The line is consumed before the game's chat
   forwards it, so no other player sees the code, and it is never echoed in chat or logs.
3. The server checks, in order, and refuses with a reply when a check fails:
   - the code is not too long ("that does not look like a link code (too long). It is the 6
     digits the website showed you.");
   - the server issued the player a durable game identity; a player with none, or with a
     name-derived identity on a listen host, is refused, since a link made there would bind the
     account to a seat or a name;
   - the server has a backend URL and service token ("cannot link: this server is not connected
     to the TBD website, so it cannot confirm your code. Tell an admin. Your code was not used.");
   - the request queue has room (16 requests; "too many link requests queued right now - try
     again in a minute.").
4. The server posts the code, the identity and the character name to the platform, one request at
   a time, each with a 15 s timeout and a 25 s watchdog.
5. On success the player reads "TBD: linked. Your game identity is now attached to your TBD
   account — attendance and stats count from your next round.".
6. On failure the reply names the cause and what to do: an invalid, used or expired code (make a
   new one on the website); an identity already linked to a different account (unlink it there
   with "Unlink Arma ID", or contact an admin); a rejected service token or a website error (tell
   an admin). Each failure reply ends "You are NOT linked.".

### Status and help

- `#tbd link` alone prints the usage and the steps: "usage: #tbd link <code>   (also: #tbd link
  status)", where to generate the code, the 10-minute limit, and that the command is private.
- `#tbd link status` reports the player's identity ("ok (<id>)", "NONE" or "NOT DURABLE
  (name-derived)") and whether the server reaches the website.
- The Mission Selector's deploy relay tells an unlinked admin to link with `#tbd link` first.

A bare code typed without the `#tbd link` prefix is ordinary public chat: the mod filters only
the command.

### Known discrepancies

None found: each reply matches the status the platform's handler returns.

## Data

- `POST /api/v1/me/link` (`create_link_code` in
  `apps/website/api_v2/src/identity_and_access/handlers/arma_link_codes.rs`): signed-in member
  tier; issues the 6-digit code with a 10-minute expiry.
- `POST /api/v1/ingest/link-confirm` (`ingest_link_confirm` in
  `apps/website/api_v2/src/identity_and_access/handlers/arma_link_confirmation.rs`): service-token
  tier, called by the game server with `X-Service-Token`; body `code`, `arma_id` and
  `arma_character`. It consumes the code, sets the member's game identity, attributes their
  earlier history and recomputes statistics, and answers `linked`, `discord_id`, `arma_id` and
  `arma_character`.
- `GET /api/v1/me/link/status` and `DELETE /api/v1/me/link`: the website's link status and unlink,
  used by the settings page only.
- The identity sent is `TBD_PlayerIdentity.GetArmaId`, byte for byte the value
  `TBD_ResultsReporter` posts with each round's results, so the platform's join on it matches.

## Design

- As built: private chat lines only; no dialog, no automatic prompt on joining and no pause menu
  entry.
- Design target: the [identity link modal sheet mockup](/documentation_v2/mod/tbd-framework/UI/discord_identity_link/visual_references/identity_link_macos_modal_sheet_mockup/README.md),
  a design-phase reference, and the specification's wireframe of a "COMMUNITY IDENTITY LINK"
  dialog over a dark scrim. The built flow differs throughout:
  - the target generates the code in game and shows it (`TBD-8X2K`, "Expires in: 09:42") with
    "COPY CODE" and "REGENERATE"; the built flow runs the other way, with the code made on the
    website and typed in game;
  - the target completes the link on a web page at a public link address or with a Discord bot's
    `/link` command; neither exists, and the platform has no Discord bot command;
  - the target shows a status chip ("UNLINKED", or "LINKED AS <user>") and the member's Discord
    role chips, polls every 3 s while open, and flashes green on success; the built flow answers
    once per command;
  - the target opens itself for an unlinked player on joining, has a pause menu entry "LINK
    DISCORD" and a "Do not show again on connect" option; none is built.
- No open ticket covers the dialog.

## Open work

- [T-327 — Chat shows `#tbd link` before TBD can suppress](/.ai/tickets/T-327.toml) (deferred,
  no plan): the typed code could show in chat before the server consumes the line.

## Decisions

- The code is made on the website and confirmed by the game server, never by the client: the
  server alone holds the service token and the player's identity, so a client cannot claim an
  account.
- A player without a durable identity cannot link: the platform's game identity is unique, so
  binding a seat number or a name to an account would hand it to whoever holds that seat or name
  next, and block every other account from it.
- One request in flight at a time: a REST callback carries no player, so only a serial queue can
  answer the right player; linking is once per player, so the wait costs nothing.
- Identity and results share one accessor: the platform joins them on this value, and a mismatch
  would fail silently with success codes at both ends.
