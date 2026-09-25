**Status:** live

# Server intel page documentation

The feature documentation of the `/server-intel` page in the
[command center](/documentation_v2/glossary.md#command-center), where a member watches one game
server's live state through its [SSE](/documentation_v2/glossary.md#sse) status stream, with the
page's design-phase reference.

## Contents

```text
documentation_v2/website/frontend/pages/command_center/server_intel/
├── server_intel_page.md  the feature doc: the server pick, the live stream, the panel and the API
└── visual_references/    the design-phase blueprint of one server's panel
```

## How it works

Read [server_intel_page.md](/documentation_v2/website/frontend/pages/command_center/server_intel/server_intel_page.md)
first. It follows the [feature doc template](/documentation_v2/standards/templates/feature_doc.md):
it quotes the page's interface text, gives what the server list and the status stream mean in
the [API](/documentation_v2/glossary.md#api), names the fixed text the page shows as live, and
compares the built page with the blueprint in `visual_references/`. The code folder's README lists
the page's files, calls and states.

## Code

- [Server intel page](/apps/website/frontend/src/v2/pages/command_center/server_intel/) — the
  route component `ServerIntelPage`, the panel and the stream subscription.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/) — the server
  list and the status stream the page reads.

## Boundaries

- Depends on: the feature doc template; the page code, the server infrastructure handlers and the
  ticket registry in `.ai/tickets/`, which the feature doc is written from.
- Used by: the page's in-code README, the command center pages README and the frontend API client
  README in `apps/website/frontend/src/v2/core/api/`, which link the feature doc; the server
  control feature doc; the web app README's page table in `documentation_v2/website/frontend/`.
- Rules: the feature doc keeps its name, which those links use; the blueprint set stays as it was
  captured and is never edited to match the built page.

## Related documentation

- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — the
  API side of the server list and the status stream.
