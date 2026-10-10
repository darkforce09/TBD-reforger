**Status:** live

# Glossary

The project's terms and abbreviations, one entry each, split into three files by the term's first
letter. A document links a term's first use to its entry in the letter-range file, as
`[mission](/documentation/glossary/g_to_m.md#mission)`; code identifiers keep their own
spelling, and an entry says where the code differs.

## Contents

```text
documentation/glossary/
├── a_to_f.md  the terms from A to F, administration to frame packet
├── g_to_m.md  the terms from G to M, game runtime to modpack
└── n_to_z.md  the terms from N to Z, operations to Workbench
```

## How it works

Each file holds the entries of its letter range in alphabetical order, ignoring case. An entry
follows the [glossary entry template](/documentation/standards/templates/glossary_entry.md): a
`###` heading with the term as prose writes it, a one-to-three-sentence definition, `In code:`
with the identifiers and paths that carry the concept, and `See:` with the related entries and
the documents that go deeper. The heading's anchor is its text lowercased with spaces made
hyphens, so `### mission header` is linked as `#mission-header`; a See link to an entry in the
same file is the bare anchor, and one to an entry in another file is a repository-root link to
that file.

A new term goes into the file of its first letter, in alphabetical order, with a line in the
index below. When a file nears the 500-line limit, its range splits into two files named the same
way (`<first letter>_to_<last letter>.md`) and every link to the entries that move is rewritten
across the repository.

### Index

- [acknowledgement-dropping relay](/documentation/glossary/a_to_f.md#acknowledgement-dropping-relay)
- [administration](/documentation/glossary/a_to_f.md#administration)
- [after-action review](/documentation/glossary/a_to_f.md#after-action-review)
- [API](/documentation/glossary/a_to_f.md#api)
- [approvals](/documentation/glossary/a_to_f.md#approvals)
- [armory](/documentation/glossary/a_to_f.md#armory)
- [arsenal](/documentation/glossary/a_to_f.md#arsenal)
- [artifact](/documentation/glossary/a_to_f.md#artifact)
- [audit logs](/documentation/glossary/a_to_f.md#audit-logs)
- [background workers](/documentation/glossary/a_to_f.md#background-workers)
- [charge ring](/documentation/glossary/a_to_f.md#charge-ring)
- [command center](/documentation/glossary/a_to_f.md#command-center)
- [community content](/documentation/glossary/a_to_f.md#community-content)
- [console command](/documentation/glossary/a_to_f.md#console-command)
- [content manager](/documentation/glossary/a_to_f.md#content-manager)
- [damage-driven render](/documentation/glossary/a_to_f.md#damage-driven-render)
- [DEM](/documentation/glossary/a_to_f.md#dem)
- [deployment](/documentation/glossary/a_to_f.md#deployment)
- [dev login](/documentation/glossary/a_to_f.md#dev-login)
- [Eden](/documentation/glossary/a_to_f.md#eden)
- [EnfScript](/documentation/glossary/a_to_f.md#enfscript)
- [Enfusion](/documentation/glossary/a_to_f.md#enfusion)
- [event](/documentation/glossary/a_to_f.md#event)
- [event manager](/documentation/glossary/a_to_f.md#event-manager)
- [factory](/documentation/glossary/a_to_f.md#factory)
- [feature doc](/documentation/glossary/a_to_f.md#feature-doc)
- [fleet command](/documentation/glossary/a_to_f.md#fleet-command)
- [fleet instance](/documentation/glossary/a_to_f.md#fleet-instance)
- [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario)
- [frame packet](/documentation/glossary/a_to_f.md#frame-packet)
- [game runtime](/documentation/glossary/g_to_m.md#game-runtime)
- [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent)
- [gate](/documentation/glossary/g_to_m.md#gate)
- [graphics engine](/documentation/glossary/g_to_m.md#graphics-engine)
- [identity and access](/documentation/glossary/g_to_m.md#identity-and-access)
- [lane](/documentation/glossary/g_to_m.md#lane)
- [load workload](/documentation/glossary/g_to_m.md#load-workload)
- [machine credential](/documentation/glossary/g_to_m.md#machine-credential)
- [map engine](/documentation/glossary/g_to_m.md#map-engine)
- [match telemetry](/documentation/glossary/g_to_m.md#match-telemetry)
- [mission](/documentation/glossary/g_to_m.md#mission)
- [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
- [mission deployment](/documentation/glossary/g_to_m.md#mission-deployment)
- [mission header](/documentation/glossary/g_to_m.md#mission-header)
- [missions](/documentation/glossary/g_to_m.md#missions)
- [mod](/documentation/glossary/g_to_m.md#mod)
- [modpack](/documentation/glossary/g_to_m.md#modpack)
- [operational receipt](/documentation/glossary/n_to_z.md#operational-receipt)
- [operations](/documentation/glossary/n_to_z.md#operations)
- [oracle](/documentation/glossary/n_to_z.md#oracle)
- [ORBAT](/documentation/glossary/n_to_z.md#orbat)
- [orchestrator](/documentation/glossary/n_to_z.md#orchestrator)
- [personnel](/documentation/glossary/n_to_z.md#personnel)
- [probable error](/documentation/glossary/n_to_z.md#probable-error)
- [RCON](/documentation/glossary/n_to_z.md#rcon)
- [registry](/documentation/glossary/n_to_z.md#registry)
- [render engine](/documentation/glossary/n_to_z.md#render-engine)
- [role](/documentation/glossary/n_to_z.md#role)
- [runtime session](/documentation/glossary/n_to_z.md#runtime-session)
- [safe start](/documentation/glossary/n_to_z.md#safe-start)
- [scenario](/documentation/glossary/n_to_z.md#scenario)
- [server control](/documentation/glossary/n_to_z.md#server-control)
- [server infrastructure](/documentation/glossary/n_to_z.md#server-infrastructure)
- [service record](/documentation/glossary/n_to_z.md#service-record)
- [service worker pack](/documentation/glossary/n_to_z.md#service-worker-pack)
- [slice](/documentation/glossary/n_to_z.md#slice)
- [slot](/documentation/glossary/n_to_z.md#slot)
- [SSE](/documentation/glossary/n_to_z.md#sse)
- [staging harness](/documentation/glossary/n_to_z.md#staging-harness)
- [Stitch visual reference](/documentation/glossary/n_to_z.md#stitch-visual-reference)
- [synthetic load account](/documentation/glossary/n_to_z.md#synthetic-load-account)
- [ticket](/documentation/glossary/n_to_z.md#ticket)
- [time fuze](/documentation/glossary/n_to_z.md#time-fuze)
- [wave](/documentation/glossary/n_to_z.md#wave)
- [Workbench](/documentation/glossary/n_to_z.md#workbench)

## Code

- [Library crates](/crates/README.md) — the API server and its domains, the single-page app and
  its pages, the Mission Creator, the game server host agent and the map crates most entries name.
- [Mod suite](/mod/README.md) — the addons, EnfScript, safe start and the game runtime.
- [Developer tools](/tools/README.md) — tickets, waves, slices, gates and the oracles.
- [Website API](/crates/api/api_server/README.md) — the API server crate and its binaries.
- [Game server host agent](/crates/fleet/game_server_host_agent/README.md) — the game server host
  agent and RCON.

## Boundaries

- Depends on: the [glossary entry template](/documentation/standards/templates/glossary_entry.md),
  which fixes each entry's shape, and the code each entry names, which it is checked against.
- Used by: nearly every README in the code trees and every live document under
  `documentation/`, which link a term's first use here (the style lock's glossary rule); the
  [README standard](/documentation/standards/readme_standard.md) and the
  [documentation standards](/documentation/standards/documentation_standards.md), which send
  writers here.
- Rules: one entry per term, in the file of its first letter; a heading, and so its anchor, never
  changes once documents link it; every link and backticked path resolves
  (`cargo xtask verify link-check`); each file stays within 500 lines.

## Related documentation

- [Documentation](/documentation/README.md) — the map of the documentation tree.
- [README standard](/documentation/standards/readme_standard.md) — the terminology every
  document follows.
