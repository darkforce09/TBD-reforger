**Status:** live

# Template: glossary entry

**When to use:** a term that documents use with a project-specific meaning, or an abbreviation
they use, defined once in the glossary (`glossary.md` at the documentation root). Every document
links the first use of such a term to its entry, and every entry is checked against the code it
names. The [README standard](/documentation_v2/standards/readme_standard.md) holds the terminology
every document follows.

## Skeleton

Copy the block and replace every `<…>` placeholder; each one says what goes there. The heading is
the term as prose writes it, in lowercase unless it is a name, so its anchor stays stable: the
entry `### mission header` is linked as `#mission-header`.

````markdown
### <term>

<Definition: one to three sentences saying what the term means in this project, and what it is
not when another term sits close to it.>

In code: <the identifiers, files, routes and settings that carry the concept, in backticks with
their repository paths; where the code spells the concept another way, say so>

See: <links to the related entries and to the documents that go deeper>
````

A document links the term's first use to the entry, with the glossary's repository-root path and
the anchor:

```text
[mission header](/documentation_v2/glossary.md#mission-header)
```

## Worked sample

Written from the writing brief's terminology and checked against the code it names: the mod's
mission header configs, the dedicated-server profile, the deploy settings and the API's fleet
scenario registry. The sample sits in a fenced block, so no gate reads its links.

````markdown
### mission header

Enfusion's world plus game-mode configuration that a dedicated server boots: an
`SCR_MissionHeader` config naming the world, the game mode, and the name and description players
see. It is not a [mission](#mission), the platform document a mission maker authors in the
[Mission Creator](#mission-creator), which the mod's mission loader loads into the running game.

In code: the `SCR_MissionHeader` configs in `apps/mod/tbd-framework/Missions/`, such as
`TBD_Dev_POC.conf`, resource `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf`; the dedicated-server
config's `game.scenarioId`, which names that resource
(`tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`), and `TBD_SCENARIO` in
`tools_v2/xtask/deploy/deploy.env`; the API's fleet scenario registry, `/api/v1/fleet/scenarios`
(`apps/website/api_v2/src/server_infrastructure/handlers/fleet_scenarios.rs`), which records the
mission header the fleet runs for each terrain. These code identifiers say scenario, and
Enfusion's own names (`scenarioId`, the `SCR_EScenario*` types) keep that spelling.

See: [mission](#mission), [event](#event),
[Game server staging](/documentation_v2/runbooks/game_server_staging/README.md).
````
