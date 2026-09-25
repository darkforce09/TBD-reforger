**Status:** live

# Server intel blueprint

Design-phase reference for the server intel page at `/server-intel`: one game server's panel with
its connect header, a telemetry grid and an intelligence strip, over a map backdrop. It gives
colour and layout context and is not an implementation source; the built UI is the Leptos code
under `apps/website/frontend/src/v2/pages/command_center/server_intel/`.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/server_intel/visual_references/server_intel_blueprint/
├── server_intel_blueprint.html  the Stitch export of the server panel
└── server_intel_blueprint.png   its screenshot
```

## How it works

The blueprint shows a server name with a status dot, an address chip with a copy icon and a
"LAUNCH & CONNECT" button; "ACTIVE PERSONNEL" with "55 / 64", "Uptime:" and "Server FPS:" marked
"(Optimal)"; "THEATER OF OPERATIONS", a forest image captioned with a terrain and an operation's
name; "SIMULATED TIME", "CONDITIONS" and "MOD CONFIGURATION" with a modpack marked "(Synced)"; and
"RECENT INTELLIGENCE", two timestamped lines.

The built page keeps this layout and its labels and differs: the theatre tile's second line is
"Match <id>" or "No Active Mission" rather than an operation's name, and the tile links to the
event schedule; every readout shows "—" while the server has not reported; and the launch button
only shows a toast. The built page copies the blueprint's two intelligence lines as fixed text.
The [server intel page](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md)
feature doc holds the full comparison.

## Code

- [Server intel page](/apps/website/frontend/src/v2/pages/command_center/server_intel/) — the
  page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and a Google-hosted image, which the html loads
  when opened; the png needs nothing.
- Used by: the server intel feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
