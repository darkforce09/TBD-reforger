**Status:** live

# Glossary

The project's terms and abbreviations, one entry each, split into three files by the term's first
letter. A document links a term's first use to its entry in the letter-range file, as
`[mission](/documentation_v2/glossary/g_to_m.md#mission)`; code identifiers keep their own
spelling, and an entry says where the code differs.

## Contents

```text
documentation_v2/glossary/
├── a_to_f.md  the terms from A to F, administration to frame packet
├── g_to_m.md  the terms from G to M, game runtime to modpack
└── n_to_z.md  the terms from N to Z, operations to Workbench
```

## How it works

Each file holds the entries of its letter range in alphabetical order, ignoring case. An entry
follows the [glossary entry template](/documentation_v2/standards/templates/glossary_entry.md): a
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

- [administration](/documentation_v2/glossary/a_to_f.md#administration)
- [after-action review](/documentation_v2/glossary/a_to_f.md#after-action-review)
- [API](/documentation_v2/glossary/a_to_f.md#api)
- [approvals](/documentation_v2/glossary/a_to_f.md#approvals)
- [armory](/documentation_v2/glossary/a_to_f.md#armory)
- [arsenal](/documentation_v2/glossary/a_to_f.md#arsenal)
- [artifact](/documentation_v2/glossary/a_to_f.md#artifact)
- [audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs)
- [background workers](/documentation_v2/glossary/a_to_f.md#background-workers)
- [command center](/documentation_v2/glossary/a_to_f.md#command-center)
- [community content](/documentation_v2/glossary/a_to_f.md#community-content)
- [content manager](/documentation_v2/glossary/a_to_f.md#content-manager)
- [damage-driven render](/documentation_v2/glossary/a_to_f.md#damage-driven-render)
- [DEM](/documentation_v2/glossary/a_to_f.md#dem)
- [deployment](/documentation_v2/glossary/a_to_f.md#deployment)
- [dev login](/documentation_v2/glossary/a_to_f.md#dev-login)
- [Eden](/documentation_v2/glossary/a_to_f.md#eden)
- [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript)
- [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion)
- [event](/documentation_v2/glossary/a_to_f.md#event)
- [event manager](/documentation_v2/glossary/a_to_f.md#event-manager)
- [factory](/documentation_v2/glossary/a_to_f.md#factory)
- [feature doc](/documentation_v2/glossary/a_to_f.md#feature-doc)
- [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command)
- [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)
- [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario)
- [frame packet](/documentation_v2/glossary/a_to_f.md#frame-packet)
- [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime)
- [gate](/documentation_v2/glossary/g_to_m.md#gate)
- [identity and access](/documentation_v2/glossary/g_to_m.md#identity-and-access)
- [lane](/documentation_v2/glossary/g_to_m.md#lane)
- [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential)
- [match telemetry](/documentation_v2/glossary/g_to_m.md#match-telemetry)
- [mission](/documentation_v2/glossary/g_to_m.md#mission)
- [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)
- [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment)
- [mission header](/documentation_v2/glossary/g_to_m.md#mission-header)
- [missions](/documentation_v2/glossary/g_to_m.md#missions)
- [mod](/documentation_v2/glossary/g_to_m.md#mod)
- [modpack](/documentation_v2/glossary/g_to_m.md#modpack)
- [operations](/documentation_v2/glossary/n_to_z.md#operations)
- [oracle](/documentation_v2/glossary/n_to_z.md#oracle)
- [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat)
- [orchestrator](/documentation_v2/glossary/n_to_z.md#orchestrator)
- [personnel](/documentation_v2/glossary/n_to_z.md#personnel)
- [RCON](/documentation_v2/glossary/n_to_z.md#rcon)
- [registry](/documentation_v2/glossary/n_to_z.md#registry)
- [render engine](/documentation_v2/glossary/n_to_z.md#render-engine)
- [role](/documentation_v2/glossary/n_to_z.md#role)
- [runtime session](/documentation_v2/glossary/n_to_z.md#runtime-session)
- [safe start](/documentation_v2/glossary/n_to_z.md#safe-start)
- [scenario](/documentation_v2/glossary/n_to_z.md#scenario)
- [server control](/documentation_v2/glossary/n_to_z.md#server-control)
- [server infrastructure](/documentation_v2/glossary/n_to_z.md#server-infrastructure)
- [service record](/documentation_v2/glossary/n_to_z.md#service-record)
- [slice](/documentation_v2/glossary/n_to_z.md#slice)
- [slot](/documentation_v2/glossary/n_to_z.md#slot)
- [SSE](/documentation_v2/glossary/n_to_z.md#sse)
- [Stitch visual reference](/documentation_v2/glossary/n_to_z.md#stitch-visual-reference)
- [ticket](/documentation_v2/glossary/n_to_z.md#ticket)
- [ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard)
- [wave](/documentation_v2/glossary/n_to_z.md#wave)
- [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)

## Code

- [Website](/apps/website/README.md) — the API domains, pages, Mission Creator and engines most
  entries name.
- [Mod suite](/apps/mod/README.md) — the addons, EnfScript, safe start and the game runtime.
- [Developer tools](/tools_v2/README.md) — tickets, waves, slices, gates and the oracles.
- [Fleet host agent](/apps/fleet_host_agent/README.md) — the fleet host agent and RCON.
- [Ticketboard](/apps/ticketboard/README.md) — the ticket viewer.

## Boundaries

- Depends on: the [glossary entry template](/documentation_v2/standards/templates/glossary_entry.md),
  which fixes each entry's shape, and the code each entry names, which it is checked against.
- Used by: nearly every README in the code trees and every live document under
  `documentation_v2/`, which link a term's first use here (the style lock's glossary rule); the
  [README standard](/documentation_v2/standards/readme_standard.md) and the
  [documentation standards](/documentation_v2/standards/documentation_standards.md), which send
  writers here.
- Rules: one entry per term, in the file of its first letter; a heading, and so its anchor, never
  changes once documents link it; every link and backticked path resolves
  (`cargo xtask verify link-check`); each file stays within 500 lines
  (`cargo xtask verify markdown-placement`).

## Related documentation

- [Documentation](/documentation_v2/README.md) — the map of the documentation tree.
- [README standard](/documentation_v2/standards/readme_standard.md) — the terminology every
  document follows.
