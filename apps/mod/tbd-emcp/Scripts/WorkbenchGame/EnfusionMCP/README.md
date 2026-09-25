# Enfusion MCP Workbench handlers

The Workbench side of the Enfusion MCP bridge: nineteen Net API handlers that let the `wb_*` tools
of the pinned `enfusion-mcp` package, and `cargo xtask mcp wbcall`, read and drive a running
[Workbench](/documentation_v2/glossary.md#workbench). They are the handlers `enfusion-mcp@0.6.1`
ships, with one local addition in `EMCP_WB_ScriptEditor.c`.

## Contents

```text
apps/mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/
├── EMCP_WB_Clipboard.c      copy, cut, paste, paste at cursor and duplicate the editor selection
├── EMCP_WB_Components.c     add, remove and list the components of an entity
├── EMCP_WB_CreateEntity.c   create an entity from a prefab at a position and rotation
├── EMCP_WB_DeleteEntity.c   delete an entity by name
├── EMCP_WB_EditorControl.c  play, stop, save, save as, undo, redo and open a resource
├── EMCP_WB_ExecuteAction.c  run any World Editor menu action by its comma-separated menu path
├── EMCP_WB_GetEntity.c      one entity's details, found by name or index
├── EMCP_WB_GetState.c       a snapshot of the editor: mode, terrain bounds, selection
├── EMCP_WB_Layers.c         list layers, the active layer and an entity's layer
├── EMCP_WB_ListEntities.c   the editor's entities, paginated and filtered by name
├── EMCP_WB_Localization.c   insert, delete, modify and read localization table rows
├── EMCP_WB_ModifyEntity.c   move, rotate, rename, reparent and edit an entity's properties and arrays
├── EMCP_WB_Ping.c           the bridge health check, reporting edit or game mode
├── EMCP_WB_Prefabs.c        create a prefab from an entity, save it, read its ancestor
├── EMCP_WB_Reload.c         trigger a script compile or a plugin reload
├── EMCP_WB_Resources.c      register, rebuild and open resource files
├── EMCP_WB_ScriptEditor.c   read and edit the open script line by line, or dump it whole
├── EMCP_WB_SelectEntity.c   select, deselect, clear and read the entity selection
└── EMCP_WB_Terrain.c        terrain height at a point and the terrain bounds
```

## How it works

Each file declares a request and a response `JsonApiStruct` and one `NetApiHandler` subclass
named `EMCP_WB_<Name>`. Workbench's Net API, listening on TCP port 5775 by default once it is
enabled in Workbench's options, dispatches a call whose `APIFunc` is that class name to the handler:
`GetRequest` supplies the request struct Workbench fills from the call's JSON, and `GetResponse`
does the work through a Workbench module and returns the response struct as JSON. Sixteen of the
module lookups are `WorldEditor`; the rest are `ScriptEditor`, `ResourceManager` and
`LocalizationEditor`. A handler with several operations switches on the request's `action` string
and answers an unknown one with `status` `error` and the list of valid actions.

The `enfusion-mcp` tools reach the handlers as follows:

| MCP tool | Handler |
|---|---|
| `wb_connect` | `EMCP_WB_Ping` |
| `wb_state` | `EMCP_WB_GetState` |
| `wb_play`, `wb_stop`, `wb_save`, `wb_undo_redo`, `wb_open_resource`, `wb_projects` (open) | `EMCP_WB_EditorControl` |
| `wb_entity_create`, `wb_entity_delete`, `wb_entity_inspect`, `wb_entity_list`, `wb_entity_modify`, `wb_entity_select` | `EMCP_WB_CreateEntity`, `EMCP_WB_DeleteEntity`, `EMCP_WB_GetEntity`, `EMCP_WB_ListEntities`, `EMCP_WB_ModifyEntity`, `EMCP_WB_SelectEntity` |
| `wb_component`, `wb_clipboard`, `wb_layers`, `wb_prefabs`, `wb_resources`, `wb_terrain`, `wb_localization`, `wb_reload`, `wb_script_editor`, `wb_execute_action` | the handler of the same subject |

`EMCP_WB_ScriptEditor.c` differs from the package copy by one action, `getAllText`, which returns
every line of the open file in `text`. The package's `wb_script_editor` tool does not offer it;
`cargo xtask mcp wbcall EMCP_WB_ScriptEditor '{"action":"getAllText"}'` calls it directly.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's script API (`NetApiHandler`, `JsonApiStruct`, the `WorldEditor`,
  `ScriptEditor`, `ResourceManager` and `LocalizationEditor` modules) and nothing of the framework.
- Used by: the `wb_*` tools of `enfusion-mcp`, pinned in
  `tools_v2/enfusion_mcp_node_package/package.json`, which `cargo xtask mcp call` runs through the
  MCP daemon; `cargo xtask mcp smoke` (`wb_connect`, `wb_state`); `cargo xtask mod dev-bootstrap`,
  which refuses a checkout without `EMCP_WB_Ping.c` and then calls `wb_connect`;
  `cargo xtask mod spawn-verify` and `cargo xtask mod spawn-determinism` (`wb_play`, `wb_stop`);
  and `cargo xtask mcp wbcall`, which sends a raw Net API call.
- Rules: exactly one copy of these handlers is loaded, or Workbench fails the WorkbenchGame module
  on "Multiple declaration": the MCP's `wb_launch` with `gprojPath` copies a second set into the
  addon it names, and `cargo xtask mod compile` exits 1 when one lands in a
  `Scripts/WorkbenchGame/` folder of `apps/mod/tbd-framework/`; the MCP's `wb_cleanup` deletes
  this folder, so it is never pointed at `apps/mod/tbd-emcp`; the files stay byte-identical to the pinned package's
  copies apart from the `getAllText` action.

## Related documentation

- [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the MCP call
  path, the daemon, exit codes and the live checks.
