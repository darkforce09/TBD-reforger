# Entity modification handler

The Net API handler behind the `wb_entity_modify` MCP tool: it moves, rotates, renames and
reparents a named entity in the open world, and sets, clears, reads and lists its properties,
object-array members and object classes, whenever a call reaches a running
[Workbench](/documentation/glossary/n_to_z.md#workbench).

## Contents

```text
mod/tbd-emcp/Scripts/WorkbenchGame/EnfusionMCP/ModifyEntity/
├── EMCP_WB_ModifyEntity.c                    the handler and its request and response wires: validates, finds the entity, dispatches the action
├── EMCP_WB_ModifyEntityArrayMemberActions.c  listArrayItems, addArrayItem and removeArrayItem on an array-of-objects property
├── EMCP_WB_ModifyEntityHierarchyActions.c    rename and reparent
├── EMCP_WB_ModifyEntityObjectClassActions.c  setObjectClass: swap the class of an object property or array member
├── EMCP_WB_ModifyEntityPropertyActions.c     setProperty, clearProperty, getProperty and listProperties
└── EMCP_WB_ModifyEntityTransformActions.c    move and rotate
```

## How it works

Workbench's Net API dispatches a call whose `APIFunc` is `EMCP_WB_ModifyEntity` to the handler
class of that name. `GetRequest` hands Workbench an `EMCP_WB_ModifyEntityRequestWire` to fill from
the call's JSON (`name`, `action`, `value`, `propertyPath`, `propertyKey`, `memberIndex`).
`GetResponse` answers `status` `error` when `name` is empty or the World Editor, its API or the
entity is missing; otherwise it switches on `action` and calls one static method of the action
class for that family, passing the WorldEditorAPI, the entity source and both wires. The method
fills `status` and `message` on the response wire, which the handler returns as JSON. An unknown
action is answered `error` with the twelve valid names.

The handler also holds the lookups the action classes share: `FindEntityByName`,
`FindComponentByClassName`, `BuildPathEntries` (a dot-separated `propertyPath` to a container
path) and `ParseVectorString` (an `"x y z"` string to a vector). Every write except `setProperty`
and `clearProperty` runs inside one `BeginEntityAction`/`EndEntityAction` pair, so Workbench
undoes it as one step. `propertyPath` is a component class name for the read, list and array
actions, and a dot-separated container path for `setProperty`, `clearProperty` and
`setObjectClass`; the array add and remove actions fall back to a container path when no
component has that class name.

## Authority

None: Workbench runs these scripts in the editor.

## Boundaries

- Depends on: Workbench's script API (`NetApiHandler`, `JsonApiStruct`, `WorldEditor`,
  `WorldEditorAPI`, `IEntitySource`, `IEntityComponentSource`, `ContainerIdPathEntry`).
- Used by: the `wb_entity_modify` tool of the pinned `enfusion-mcp` package and
  `cargo xtask mcp wbcall EMCP_WB_ModifyEntity`, both through Workbench's Net API.
- Rules: the class name `EMCP_WB_ModifyEntity`, the JSON keys and the action names are the Net
  API surface the package calls and stay unchanged; the files derive from `enfusion-mcp@0.6.1`
  and differ from its single `EMCP_WB_ModifyEntity.c`, so an upgrade ports changes by hand
  (see the [addon README](/mod/tbd-emcp/README.md#upgrading-enfusion-mcp));
  `cargo xtask verify file-length` reports the files' length, and a new file needs a Workbench
  restart before the handler compiles.

## Related documentation

- [Enfusion MCP bridge](/documentation/mod/tbd-emcp/workbench_mcp_bridge.md) — the call path
  from the MCP tools to these handlers, and the upgrade rules.
