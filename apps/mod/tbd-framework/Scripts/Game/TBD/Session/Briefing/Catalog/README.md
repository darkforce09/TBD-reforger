# Briefing screen data

The data the Briefing screen's pages draw: one catalog per briefing, holding both factions, the
radio nets, objectives, rules, lore, parameters, each side's vehicles and uniforms, and the tactical
plans. It serves mock content until something replaces it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/Catalog/
├── TBD_AssetTypeInfo.c     one vehicle type and, as `TBD_AssetInstanceInfo`, each vehicle of it
├── TBD_BriefingCatalog.c   the catalog: `Get()`, `Set()` and the per-page accessors
├── TBD_BriefingFaction.c   one side: key, name, role chip and tint
├── TBD_NetInfo.c           one radio net with its auxiliary channels and display text
├── TBD_ObjectiveInfo.c     one objective card with its map position
├── TBD_ParamInfo.c         one parameters row
├── TBD_PlanInfo.c          one tactical plan for the Markers panel
├── TBD_RuleGroup.c         a collapsible group of numbered rules, each a `TBD_RuleInfo`
└── TBD_UniformInfo.c       one uniform card and its doll prefab
```

## How it works

`TBD_BriefingCatalog.Get()` returns the served catalog, building it from `TBD_BriefingMock` on
first use; `Set()` replaces it, and null restores the mock on the next `Get()`. No script calls
`Set()`, so the pages show mock content. Accessors that take `friendly` return the reader's side for
true and the other side for false. `TBD_UniformInfo.Prefab()` returns a pinned prefab, else the kit
alias resolved through `TBD_Registry`.

## Authority

- Server: nothing here.
- Client: everything; the catalog is read by the screen on the local player's machine.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BriefingMock` in `apps/mod/tbd-framework/Scripts/Game/TBD/UI/Mock/`;
  `TBD_SessionSelection` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`;
  `TBD_Registry` and `TBD_KitEntry` in `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: the Briefing screen, its pages and the Markers panel in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/Briefing/UI/`; `TBD_BriefingMock`.
- Rules: pages hold only `TBD_BriefingCatalog`, never the mock class; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.
