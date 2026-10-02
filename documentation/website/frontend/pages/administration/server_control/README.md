**Status:** live

# Server control page documentation

The feature documentation of the `/admin/server` page, where administrators follow the game servers,
issue [fleet commands](/documentation/glossary/a_to_f.md#fleet-command), deploy approved
[missions](/documentation/glossary/g_to_m.md#mission), keep the
[registry](/documentation/glossary/n_to_z.md#registry) of
[fleet scenarios](/documentation/glossary/a_to_f.md#fleet-scenario) and manage
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential). The page has no design set:
its built layout, drawn in the feature doc, is the reference.

## Contents

```text
documentation/website/frontend/pages/administration/server_control/
└── server_control_page.md  the feature doc: the server card, its four panels and their API calls
```

## Code

- [Server control page](/apps/website/frontend/src/v2/pages/administration/server_control/) — the
  route component `ServerControlPage`, the server card and its fleet command, deployment, fleet
  scenario and credential panels.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/) — the server,
  fleet command, fleet scenario and machine credential routes.
- [Missions domain](/apps/website/api_v2/src/missions/) — the
  [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) routes.
- [Fleet host agent](/apps/fleet_host_agent/) — the executor of process control and the player
  list.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  page code and the server infrastructure and missions handlers the feature doc is written from.
- Used by: the [server control](/documentation/glossary/n_to_z.md#server-control) glossary entry, the
  page's in-code README and the server infrastructure domain README, which link the feature doc;
  the administration pages README.
- Rules: the feature doc keeps its name, which those links use, and stays within 500 lines; past
  that it splits into one document per panel folder of the code, with this folder as their index.

## Related documentation

- [Fleet command ledger evidence](/documentation/website/api_v2/verification_evidence/fleet_command_ledger.md)
  — the verification of the command ledger the console follows.
- [Machine credentials evidence](/documentation/website/api_v2/verification_evidence/machine_credentials.md)
  — the verification of issuing, using and revoking credentials.
